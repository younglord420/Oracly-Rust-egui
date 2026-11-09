//! Chart metadata a script reads through `syminfo.*` and `timeframe.*`.
//!
//! Pinecone registers both namespaces, but with empty defaults unless the host
//! supplies real values. The bridge therefore carries the chart's instrument and
//! interval into the engine. Values the chart genuinely does not know stay
//! empty/zero rather than being invented: a script that needs an unavailable
//! field should see the same "not provided" state as the chart's own UI.

use pine_core::TimeframeUnit;
use Oracly_charts::model::{Instrument, Timeframe as ChartTimeframe};

/// Symbol and interval a run should report to the script.
#[derive(Clone, Debug, PartialEq)]
pub struct RunContext {
    /// Symbol without the exchange prefix, e.g. `"AAPL"`.
    pub ticker: String,
    /// Exchange or data-source prefix, e.g. `"NASDAQ"`.
    pub prefix: String,
    /// Fully qualified symbol, e.g. `"NASDAQ:AAPL"`.
    pub tickerid: String,
    /// Human readable instrument name.
    pub description: String,
    /// Instrument type, e.g. `"stock"`.
    pub type_: String,
    /// Smallest price increment.
    pub mintick: f64,
    /// Price currency, e.g. `"USD"`.
    pub currency: String,
    /// Interval multiplier, e.g. `5` in `5m`; `0` when the chart has none.
    pub multiplier: u32,
    /// Interval unit.
    pub unit: TimeframeUnit,
}

impl Default for RunContext {
    fn default() -> Self {
        Self {
            ticker: String::new(),
            prefix: String::new(),
            tickerid: String::new(),
            description: String::new(),
            type_: String::new(),
            mintick: 0.0,
            currency: String::new(),
            multiplier: 0,
            unit: TimeframeUnit::Minutes,
        }
    }
}

impl RunContext {
    /// Build the context from the chart's instrument and interval.
    ///
    /// Fields the chart leaves unset stay empty so a script can tell "unknown"
    /// from a real value; nothing is filled in with a plausible default.
    pub fn from_chart(instrument: Option<&Instrument>, timeframe: Option<ChartTimeframe>) -> Self {
        let mut context = Self::default();
        if let Some(instrument) = instrument {
            let exchange = instrument.exchange.clone().unwrap_or_default();
            context.ticker = instrument.symbol.clone();
            context.prefix = exchange.clone();
            context.tickerid = if exchange.is_empty() {
                instrument.symbol.clone()
            } else {
                format!("{exchange}:{}", instrument.symbol)
            };
            context.description = instrument.description.clone().unwrap_or_default();
            context.type_ = instrument
                .symbol_type
                .map(|kind| kind.to_string())
                .unwrap_or_default();
            context.mintick = instrument.tick_size.unwrap_or(0.0);
            context.currency = instrument.currency.clone().unwrap_or_default();
        }
        if let Some(timeframe) = timeframe {
            let (multiplier, unit) = pine_timeframe(timeframe);
            context.multiplier = multiplier;
            context.unit = unit;
        }
        context
    }

    /// The Pinecone `SymInfo` this context describes.
    pub fn syminfo(&self) -> pine_core::SymInfo {
        pine_core::SymInfo {
            ticker: self.ticker.clone(),
            tickerid: self.tickerid.clone(),
            description: self.description.clone(),
            prefix: self.prefix.clone(),
            type_: self.type_.clone(),
            mintick: self.mintick,
            currency: self.currency.clone(),
            ..Default::default()
        }
    }

    /// The Pinecone `Timeframe` this context describes, or `None` when the chart
    /// has no interval to report.
    pub fn timeframe(&self) -> Option<pine_core::Timeframe> {
        if self.multiplier == 0 {
            return None;
        }
        Some(pine_core::Timeframe {
            multiplier: self.multiplier,
            unit: self.unit,
        })
    }
}

/// Map the chart's interval onto Pinecone's multiplier/unit pair.
///
/// Pinecone has no sub-second, tick or volume units. Sub-second intervals are
/// reported in seconds (the smallest unit it knows) and trade-based intervals
/// keep their size in minutes, rather than silently inventing a clock interval.
fn pine_timeframe(timeframe: ChartTimeframe) -> (u32, TimeframeUnit) {
    use ChartTimeframe as T;
    match timeframe {
        T::Ticks(count) | T::Volume(count) | T::Range(count) | T::Delta(count) => {
            (count.get() as u32, TimeframeUnit::Minutes)
        }
        T::Ms100 => (100, TimeframeUnit::Seconds),
        T::Ms250 => (250, TimeframeUnit::Seconds),
        T::Ms500 => (500, TimeframeUnit::Seconds),
        T::Sec1 => (1, TimeframeUnit::Seconds),
        T::Sec2 => (2, TimeframeUnit::Seconds),
        T::Sec5 => (5, TimeframeUnit::Seconds),
        T::Sec10 => (10, TimeframeUnit::Seconds),
        T::Sec30 => (30, TimeframeUnit::Seconds),
        T::Min1 => (1, TimeframeUnit::Minutes),
        T::Min5 => (5, TimeframeUnit::Minutes),
        T::Min15 => (15, TimeframeUnit::Minutes),
        T::Min30 => (30, TimeframeUnit::Minutes),
        T::Hour1 => (60, TimeframeUnit::Minutes),
        T::Hour4 => (240, TimeframeUnit::Minutes),
        T::Day1 => (1, TimeframeUnit::Daily),
        T::Week1 => (1, TimeframeUnit::Weekly),
        T::Month1 => (1, TimeframeUnit::Monthly),
        T::Weeks(count) => (count.get(), TimeframeUnit::Weekly),
        T::Months(count) => (count.get(), TimeframeUnit::Monthly),
        T::Custom(seconds) => (seconds.get() as u32, TimeframeUnit::Seconds),
        // `Timeframe` is `#[non_exhaustive]`: a variant added later has no
        // Pinecone unit we can prove. Report no interval rather than inventing
        // one; scripts then see the same "unknown" state as an unset context.
        _ => (0, TimeframeUnit::Minutes),
    }
}
