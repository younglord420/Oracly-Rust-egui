//! Host-supplied data the script asks for: other feeds and imported libraries.
//!
//! OpineRust never fetches anything itself. `request.security` and `import` are
//! answered from what the host already has — the chart's own bar cache and its
//! library directory — so the same code path works in a browser where a socket
//! is a thread block away from a frozen tab, and on a server where a fetch would
//! be a surprise.
//!
//! A host that supplies nothing keeps the old behaviour: `request.*` reads `na`
//! and an `import` fails with a message naming the path, instead of the bridge
//! refusing the script at lex time.

use pine_core::{Data, DataProvider, LibraryLoader, ProviderError, SymInfo, Timeframe};
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

/// Bars for another symbol or interval, from the host's own cache.
///
/// Returning `None` is the honest answer for a feed the host does not have: the
/// script reads `na`, exactly as Pine does for an unavailable symbol with
/// `ignore_invalid_symbol = true`. `Err` reports a real failure the host wants
/// surfaced.
pub trait SecurityProvider: Send + Sync {
    /// The bars of `symbol` at `timeframe`, oldest first.
    ///
    /// `timeframe` is the Pine period string the script asked for (`"60"`,
    /// `"1D"`), not the chart's own interval.
    fn request(&self, symbol: &str, timeframe: &str) -> Option<Data>;
}

/// Source text of an `import`ed library, keyed by its import path.
///
/// The path is what the script wrote (`"user/Lib/1"`), so one resolver can serve
/// a directory of `.pine` files or an in-memory map.
pub trait LibrarySource: Send + Sync {
    /// The library's source, or `None` when this host does not know the path.
    fn load(&self, path: &str) -> Option<String>;
}

/// Everything a script may ask the host for.
///
/// Cheap to clone: the two hooks are shared, and an absent hook is `None` rather
/// than a stub that pretends to answer.
#[derive(Clone, Default)]
pub struct HostEnvironment {
    security: Option<Arc<dyn SecurityProvider>>,
    libraries: Option<Arc<dyn LibrarySource>>,
}

impl std::fmt::Debug for HostEnvironment {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("HostEnvironment")
            .field("security", &self.security.is_some())
            .field("libraries", &self.libraries.is_some())
            .finish()
    }
}

impl HostEnvironment {
    /// A host with no extra data: `request.*` reads `na`, `import` fails.
    pub fn empty() -> Self {
        Self::default()
    }

    /// Answer `request.*` from `provider`.
    pub fn with_security(mut self, provider: Arc<dyn SecurityProvider>) -> Self {
        self.security = Some(provider);
        self
    }

    /// Resolve `import` from `source`.
    pub fn with_libraries(mut self, source: Arc<dyn LibrarySource>) -> Self {
        self.libraries = Some(source);
        self
    }

    /// Whether the host answers `request.*`.
    pub fn has_security(&self) -> bool {
        self.security.is_some()
    }

    /// The provider answering `request.*`, when the host has one.
    pub(crate) fn security(&self) -> Option<&Arc<dyn SecurityProvider>> {
        self.security.as_ref()
    }

    /// The resolver `import` uses, always present.
    ///
    /// With no host library source this is an empty set, so a missing import is a
    /// diagnostic naming the path rather than "no loader configured": the script's
    /// mistake and the host's are different problems.
    pub(crate) fn library_hook(&self) -> Arc<dyn LibrarySource> {
        self.libraries
            .clone()
            .unwrap_or_else(|| Arc::new(MemoryLibraries::new()))
    }

    /// Whether the host resolves `import`.
    pub fn has_libraries(&self) -> bool {
        self.libraries.is_some()
    }
}

/// A [`SecurityProvider`] that only knows the chart's own symbol, at the chart's
/// own interval. Useful for a `request.security(syminfo.tickerid, "5", ...)`
/// style call that the host can answer without a second feed.
#[derive(Clone, Debug, Default)]
pub struct ChartOnlyProvider {
    symbol: String,
    timeframe: String,
    bars: Option<Data>,
}

impl ChartOnlyProvider {
    /// Serve `bars` only when asked for `symbol` at `timeframe`.
    pub fn new(symbol: impl Into<String>, timeframe: impl Into<String>, bars: Data) -> Self {
        Self {
            symbol: symbol.into(),
            timeframe: timeframe.into(),
            bars: Some(bars),
        }
    }
}

impl SecurityProvider for ChartOnlyProvider {
    fn request(&self, symbol: &str, timeframe: &str) -> Option<Data> {
        let same_symbol = symbol == self.symbol
            || symbol
                .rsplit_once(':')
                .is_some_and(|(_, ticker)| ticker == self.symbol);
        (same_symbol && timeframe == self.timeframe)
            .then(|| self.bars.clone())
            .flatten()
    }
}

/// A [`LibrarySource`] backed by an in-memory map of import path to source.
#[derive(Clone, Debug, Default)]
pub struct MemoryLibraries {
    files: HashMap<String, String>,
}

impl MemoryLibraries {
    /// An empty set.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register `source` under `path`, for chaining.
    pub fn with_library(mut self, path: impl Into<String>, source: impl Into<String>) -> Self {
        self.files.insert(path.into(), source.into());
        self
    }
}

impl LibrarySource for MemoryLibraries {
    fn load(&self, path: &str) -> Option<String> {
        self.files.get(path).cloned()
    }
}

/// The `pine_core::DataProvider` the interpreter calls, backed by our hook.
///
/// Buffers a requested dataset per `(symbol, timeframe)` so a script that asks
/// once per bar does not re-run the host's lookup each time.
pub(crate) struct InterpreterFeed {
    host: Arc<dyn SecurityProvider>,
    syminfo: SymInfo,
    cache: RefCell<HashMap<(String, String), Option<Data>>>,
}

impl InterpreterFeed {
    /// Wrap `host`; `syminfo` is stamped onto a dataset the provider returned
    /// without one, so `syminfo.*` inside the requested context is not empty.
    pub(crate) fn new(host: Arc<dyn SecurityProvider>, syminfo: SymInfo) -> Self {
        Self {
            host,
            syminfo,
            cache: RefCell::new(HashMap::new()),
        }
    }
}

impl DataProvider for InterpreterFeed {
    fn request(&self, symbol: &str, timeframe: Timeframe) -> Result<Data, ProviderError> {
        let key = (symbol.to_owned(), timeframe.period());
        if let Some(cached) = self.cache.borrow().get(&key) {
            return cached
                .clone()
                .ok_or_else(|| format!("no data for {symbol} at {key}", key = key.1).into());
        }
        let mut data = self.host.request(symbol, &key.1);
        if let Some(data) = &mut data {
            // The provider may return bars without describing them; fill in what
            // the chart knows so `syminfo.tickerid` inside a `request` is not "".
            if data.syminfo.ticker.is_empty() {
                data.syminfo.ticker = symbol
                    .rsplit_once(':')
                    .map_or(symbol, |(_, ticker)| ticker)
                    .to_owned();
            }
            if data.syminfo.tickerid.is_empty() {
                data.syminfo.tickerid = symbol.to_owned();
            }
            if data.syminfo.prefix.is_empty() {
                data.syminfo.prefix = self.syminfo.prefix.clone();
            }
        }
        self.cache.borrow_mut().insert(key, data.clone());
        data.ok_or_else(|| format!("no data for {symbol} at {}", timeframe.period()).into())
    }
}

/// A `pine_core::LibraryLoader` backed by our hook.
pub(crate) struct InterpreterLibraries {
    host: Arc<dyn LibrarySource>,
    /// Sources already resolved, so the sema pass can attribute a diagnostic to
    /// the same text the run used.
    loaded: RefCell<HashMap<String, String>>,
}

impl InterpreterLibraries {
    pub(crate) fn new(host: Arc<dyn LibrarySource>) -> Self {
        Self {
            host,
            loaded: RefCell::new(HashMap::new()),
        }
    }
}

impl LibraryLoader for InterpreterLibraries {
    fn load_library(&self, path: &str) -> Result<String, String> {
        self.host
            .load(path)
            .inspect(|source| {
                // Remember what was served, so a diagnostic can quote the same
                // text the run used.
                self.loaded
                    .borrow_mut()
                    .insert(path.to_owned(), source.clone());
            })
            .ok_or_else(|| format!("this host has no library at '{path}'"))
    }
}

/// The `pine_core::LibraryLoader` the interpreter calls, backed by our hook.
#[cfg(test)]
mod tests {
    use super::*;
    use pine_core::{Bar, Data, Timeframe};

    fn data(close: f64) -> Data {
        Data::new(vec![Bar {
            close,
            ..Bar::default()
        }])
    }

    #[test]
    fn an_empty_host_answers_nothing() {
        let environment = HostEnvironment::empty();
        assert!(!environment.has_security());
        assert!(!environment.has_libraries());
    }

    #[test]
    fn chart_only_provider_serves_only_its_own_pair() {
        let provider = ChartOnlyProvider::new("AAPL", "5", data(1.0));
        assert!(provider.request("AAPL", "5").is_some());
        assert!(provider.request("NASDAQ:AAPL", "5").is_some());
        assert!(provider.request("AAPL", "15").is_none());
        assert!(provider.request("MSFT", "5").is_none());
    }

    #[test]
    fn memory_libraries_answer_registered_paths_only() {
        let libraries = MemoryLibraries::new().with_library("acme/Stats/1", "// source");
        assert_eq!(libraries.load("acme/Stats/1").as_deref(), Some("// source"));
        assert!(libraries.load("acme/Missing/1").is_none());
    }

    #[test]
    fn the_feed_caches_and_stamps_syminfo() {
        let provider = Arc::new(ChartOnlyProvider::new("AAPL", "5", data(42.0)));
        let feed = InterpreterFeed::new(
            provider,
            SymInfo {
                prefix: "NASDAQ".into(),
                ..Default::default()
            },
        );
        let timeframe: Timeframe = "5".parse().expect("a 5m period");
        let first = feed
            .request("NASDAQ:AAPL", timeframe.clone())
            .expect("served");
        assert_eq!(first.bars[0].close, 42.0);
        assert_eq!(first.syminfo.ticker, "AAPL", "the ticker is derived");
        assert_eq!(first.syminfo.tickerid, "NASDAQ:AAPL");
        let again = feed.request("NASDAQ:AAPL", timeframe).expect("cached");
        assert_eq!(again.bars[0].close, 42.0);
    }

    #[test]
    fn a_missing_feed_is_an_error_not_a_panic() {
        let feed = InterpreterFeed::new(
            Arc::new(ChartOnlyProvider::new("AAPL", "5", data(1.0))),
            SymInfo::default(),
        );
        let timeframe: Timeframe = "15".parse().expect("a 15m period");
        assert!(feed.request("AAPL", timeframe).is_err());
    }

    #[test]
    fn the_library_adapter_reports_an_unknown_path() {
        let loader = InterpreterLibraries::new(Arc::new(MemoryLibraries::new()));
        let error = loader.load_library("nobody/Lib/1").expect_err("missing");
        assert!(error.contains("nobody/Lib/1"), "{error}");
    }
}
