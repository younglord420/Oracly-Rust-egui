//! Pine execution through the Pinecone core runtime, isolated from chart types.
//!
//! The engine works on plain owned data so it can run on the chart thread, in a
//! worker or on a server. It never reads the wall clock and never fetches data.
//!
//! Two upstream behaviours are handled here rather than inherited:
//!
//! - `pine_builtins::per_bar_variables` reads `SystemTime::now()` for `timenow`,
//!   which panics on `wasm32-unknown-unknown`. [`per_bar`] rebuilds those values
//!   with a host-supplied clock, and the semantic pass uses the same builder.
//! - Plot calls must be unconditional top-level declarations. Their position is
//!   their identity; ambiguous conditional or aliased calls are rejected.

use crate::context::RunContext;
use crate::host::{HostEnvironment, InterpreterFeed, InterpreterLibraries};
use crate::output::DrawingOutputSink;
use pine_core::{
    Bar as PineBar, Color, Data, Extend, InputOutput, Label, LineObject,
    LineStyle as PineLineStyle, MetadataOutput, PineBox, PineVersion, PlotOutput, Size, Table,
    XLoc, YLoc,
};
use pine_interpreter::{Interpreter, Series, Value};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

/// One OHLCV bar handed to the engine. Times are UNIX milliseconds.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bar {
    /// The candle is still forming: realtime, not confirmed, not history.
    pub forming: bool,
    /// The bar is part of completed history rather than a live tick.
    pub history: bool,
    /// Opening time in UNIX milliseconds.
    pub time_ms: i64,
    /// Open price.
    pub open: f64,
    /// High price.
    pub high: f64,
    /// Low price.
    pub low: f64,
    /// Close price.
    pub close: f64,
    /// Volume.
    pub volume: f64,
}

/// How serious a diagnostic is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum DiagnosticSeverity {
    /// A hint.
    Info,
    /// Works, but may not do what was intended.
    Warning,
    /// The script cannot run.
    Error,
}

/// A structured message about the script.
#[derive(Clone, Debug, PartialEq)]
pub struct Diagnostic {
    /// Severity.
    pub severity: DiagnosticSeverity,
    /// Message text, already stripped of engine-internal prefixes.
    pub message: String,
    /// One-based line, when the runtime reports one.
    pub line: Option<u32>,
    /// One-based column, when the runtime reports one.
    pub column: Option<u32>,
}

/// The declaration of a script's plots and identity.
#[derive(Clone, Debug, PartialEq)]
pub struct ScriptInfo {
    /// `indicator(...)` or `strategy(...)` title, or the fallback title.
    pub title: String,
    /// Short title, when declared.
    pub short_title: Option<String>,
    /// `overlay = true` in the declaration.
    pub overlay: bool,
    /// Version the script declared or the host default.
    pub version: u8,
    /// Which declaration the script used: `"indicator"` or `"strategy"`.
    ///
    /// A host reads this to tell a study that trades from one that only draws;
    /// the chart renders both the same way. The fills themselves are on
    /// [`EngineOutput::trades`], beside the plots, so one run's ledger never
    /// disagrees with its values.
    pub kind: String,
}

/// One plotted output across every bar.
#[derive(Clone, Debug, PartialEq)]
pub struct PlotSeries {
    /// Plot title from `plot(title=...)`.
    pub title: String,
    /// Position among the plots sharing this title, from zero.
    pub occurrence: usize,
    /// One entry per bar: `None` where the script produced `na`.
    pub values: Vec<Option<f64>>,
    /// The colour each bar was drawn with, packed as `0xRRGGBBAA`.
    pub colors: Vec<Option<u32>>,
    /// Whether any bar had a value.
    pub has_values: bool,
}

impl PlotSeries {
    /// Stable identity of this output within the run.
    pub fn key(&self) -> (String, usize) {
        (self.title.clone(), self.occurrence)
    }
}

/// A completed run.
#[derive(Clone, Debug, PartialEq)]
pub struct EngineOutput {
    /// Resolved declaration.
    pub info: ScriptInfo,
    /// Outputs in declaration order.
    pub plots: Vec<PlotSeries>,
    /// `input.*` declarations the script made, in declaration order.
    pub inputs: Vec<ScriptInput>,
    /// Boxes the script left on the chart (`box.new`), in creation order.
    pub boxes: Vec<ScriptBox>,
    /// Lines the script left on the chart (`line.new`, `hline`).
    pub lines: Vec<ScriptLine>,
    /// Labels the script left on the chart (`label.new`).
    pub labels: Vec<ScriptLabel>,
    /// Tables the script left on the chart (`table.new`).
    pub tables: Vec<ScriptTable>,
    /// Shapes from `plotshape`, `plotchar` and `plotarrow`, one per bar they fire.
    ///
    /// These are per-bar streams, unlike boxes: a shape that is `na` on a bar is
    /// absent here rather than stored and later deleted.
    pub marks: Vec<ScriptMark>,
    /// Order fills the script's simulated broker produced (`strategy.*`).
    pub trades: Vec<TradeMarker>,
    /// Non-fatal messages.
    pub diagnostics: Vec<Diagnostic>,
}

/// One `box.new` handle, in chart coordinates.
#[derive(Clone, Debug, PartialEq)]
pub struct ScriptBox {
    /// `xloc.bar_index`-style left edge, or a UNIX-ms time with `xloc.bar_time`.
    pub left: f64,
    /// Right edge, in the same units as [`Self::left`].
    pub right: f64,
    /// Upper price bound.
    pub top: f64,
    /// Lower price bound.
    pub bottom: f64,
    /// Whether the edges are bar indices (`false`) or UNIX ms (`true`).
    pub time_x: bool,
    /// Border colour as `0xRRGGBBAA`; `None` falls back to the plot colour.
    pub border_color: Option<u32>,
    /// Fill colour as `0xRRGGBBAA`; `None` leaves the box unfilled.
    pub fill_color: Option<u32>,
    /// Border width in logical pixels.
    pub border_width: f32,
    /// `solid`, `dotted` or `dashed`.
    pub border_style: String,
    /// Text drawn inside the box.
    pub text: String,
}

/// One `line.new`/`hline` handle, in chart coordinates.
#[derive(Clone, Debug, PartialEq)]
pub struct ScriptLine {
    /// First point's x (bar index, or UNIX ms when [`Self::time_x`]).
    pub x1: f64,
    /// First point's price.
    pub y1: f64,
    /// Second point's x.
    pub x2: f64,
    /// Second point's price.
    pub y2: f64,
    /// Whether the x values are UNIX ms rather than bar indices.
    pub time_x: bool,
    /// Stroke colour as `0xRRGGBBAA`.
    pub color: Option<u32>,
    /// `solid`, `dotted` or `dashed`, plus the arrow styles.
    pub style: String,
    /// Stroke width in logical pixels.
    pub width: f32,
    /// `none`, `left`, `right` or `both`.
    pub extend: String,
}

/// One `label.new` handle, in chart coordinates.
#[derive(Clone, Debug, PartialEq)]
pub struct ScriptLabel {
    /// Bar index, or UNIX ms when [`Self::time_x`].
    pub x: f64,
    /// The label's price, when it has one.
    pub y: f64,
    /// Whether [`Self::x`] is UNIX ms rather than a bar index.
    pub time_x: bool,
    /// `price`, `abovebar` or `belowbar`.
    pub yloc: String,
    /// Body text.
    pub text: String,
    /// Body colour as `0xRRGGBBAA`.
    pub color: Option<u32>,
    /// Text colour as `0xRRGGBBAA`.
    pub text_color: Option<u32>,
    /// `tiny`..`huge`, or `auto`.
    pub size: String,
    /// Hover tooltip text.
    pub tooltip: Option<String>,
}

/// One `table.new` overlay.
#[derive(Clone, Debug, PartialEq)]
pub struct ScriptTable {
    /// `top_left`…`bottom_right`.
    pub position: String,
    /// Column count.
    pub columns: usize,
    /// Row count.
    pub rows: usize,
    /// Table background as `0xRRGGBBAA`.
    pub background: Option<u32>,
    /// Cells in row-major order; a missing cell draws empty.
    pub cells: Vec<ScriptCell>,
}

/// One `plotshape` / `plotchar` / `plotarrow` that fired on a bar.
#[derive(Clone, Debug, PartialEq)]
pub struct ScriptMark {
    /// Bar the shape belongs to, after its `offset`.
    pub bar: usize,
    /// Price it sits on. `location.abovebar` is the bar's high, `belowbar` its low,
    /// and `location.absolute` the series value itself.
    pub price: f64,
    /// Text beside the shape, or the character a `plotchar` draws.
    pub text: String,
    /// `triangleup`, `arrowdown`, `circle`, or the character for a `plotchar`.
    pub style: String,
    /// Up-pointing shapes and positive `plotarrow` values.
    pub up: bool,
    /// Colour as `0xRRGGBBAA`.
    pub color: Option<u32>,
}

/// One occupied table cell.
#[derive(Clone, Debug, PartialEq)]
pub struct ScriptCell {
    /// Zero-based column.
    pub column: usize,
    /// Zero-based row.
    pub row: usize,
    /// Cell text.
    pub text: String,
    /// Text colour as `0xRRGGBBAA`.
    pub text_color: Option<u32>,
    /// Cell background as `0xRRGGBBAA`.
    pub background: Option<u32>,
    /// Pine `size.*` (`tiny`…`huge`). Empty keeps the chart's normal size.
    pub text_size: String,
    /// Pine `text.align_*` horizontal name. Empty means left.
    pub halign: String,
    /// Pine `text.align_*` vertical name. Empty means center.
    pub valign: String,
}

/// One simulated order fill from a `strategy.*` script.
#[derive(Clone, Debug, PartialEq)]
pub struct TradeMarker {
    /// Bar index the fill happened on.
    pub bar: usize,
    /// Fill price.
    pub price: f64,
    /// Positive for a long entry/exit covering a short; negative for a short.
    pub size: f64,
    /// Id the script gave the entry order.
    pub entry_id: String,
    /// Whether this marker closes a trade rather than opening one.
    pub exit: bool,
}

/// One `input.*` declaration, with the value the run actually used.
///
/// A host maps this onto its own settings schema; the bridge keeps it free of
/// chart types so the engine stays usable outside Oracly. Duplicate titles keep
/// their own entries because Pinecone resolves overrides by title and the last
/// declaration wins for every one of them — a host must not silently merge them.
#[derive(Clone, Debug, PartialEq)]
pub struct ScriptInput {
    /// `input.int`, `input.float`, `input.bool`, `input.string`, ... as declared.
    pub kind: String,
    /// Title the script gave it; empty when omitted.
    pub title: String,
    /// Group the declaration belongs to, when given.
    pub group: String,
    /// Declared default.
    pub default: InputDefault,
    /// Value this run used.
    pub value: InputDefault,
    /// Declared minimum, for numeric inputs.
    pub min: Option<f64>,
    /// Declared maximum, for numeric inputs.
    pub max: Option<f64>,
    /// Declared step, for numeric inputs.
    pub step: Option<f64>,
    /// `options=[...]` of an `input.string`, when every entry is a string literal.
    ///
    /// Empty when the script gave no list, or the list is computed. A computed
    /// list cannot become a dropdown without guessing which strings it holds.
    pub options: Vec<String>,
}

/// A declared input's value, kept typed so a host can build the right widget.
#[derive(Clone, Debug, PartialEq)]
pub enum InputDefault {
    /// Whole number.
    Int(i64),
    /// Fractional number.
    Float(f64),
    /// On or off.
    Bool(bool),
    /// Text.
    Text(String),
    /// An RGBA colour.
    Color([u8; 4]),
}

/// Why a script could not be run.
#[derive(Clone, Debug, PartialEq)]
pub enum EngineError {
    /// Lexing, parsing, semantic analysis, or an unsupported construct failed.
    Compile(Vec<Diagnostic>),
    /// Execution failed on a bar.
    Runtime(Diagnostic),
    /// Nothing to run over.
    NoBars,
    /// The engine itself was used incorrectly.
    Internal(String),
}

/// Rebuild the per-bar builtins without the wall clock.
///
/// Mirrors `pine_builtins::per_bar_variables` plus the private
/// `barstate::register`, but takes `timenow` from the host clock. Without this,
/// `wasm32-unknown-unknown` panics in `SystemTime::now()`.
fn per_bar<O: pine_core::PineOutput>(
    bar: &PineBar,
    last: Option<&PineBar>,
    now_ms: f64,
) -> Vec<(String, Value<O>)> {
    let series = |id: &str, value: f64| {
        (
            id.to_string(),
            Value::Series(Series {
                id: id.to_string(),
                current: Box::new(Value::Number(value)),
                history: None,
            }),
        )
    };
    let mut vars = vec![
        series("open", bar.open),
        series("high", bar.high),
        series("low", bar.low),
        series("close", bar.close),
        series("volume", bar.volume),
        series("hl2", (bar.high + bar.low) / 2.0),
        series("hlc3", (bar.high + bar.low + bar.close) / 3.0),
        series("hlcc4", (bar.high + bar.low + bar.close * 2.0) / 4.0),
        series("ohlc4", (bar.open + bar.high + bar.low + bar.close) / 4.0),
        // `bar_index` is a series in Pine, not a scalar: structure scripts read
        // `bar_index[2]` to anchor a box on the bar a gap opened on. Publishing it
        // as a plain number made every such lookback `na`, which silently emptied
        // those boxes and labels.
        series("bar_index", bar.index as f64),
        (
            "last_bar_index".to_string(),
            last.map_or(Value::Na, |b| Value::Number(b.index as f64)),
        ),
        (
            "last_bar_time".to_string(),
            last.map_or(Value::Na, |b| Value::Number(b.time as f64)),
        ),
    ];
    let members: HashMap<String, Value<O>> = [
        ("isfirst", bar.is_first),
        ("islast", bar.is_last),
        ("isnew", bar.is_new),
        ("isconfirmed", bar.is_confirmed),
        ("ishistory", bar.is_history),
        ("isrealtime", bar.is_realtime),
        ("islastconfirmedhistory", bar.is_last_confirmed_history),
    ]
    .into_iter()
    .map(|(name, flag)| (name.to_string(), Value::Bool(flag)))
    .collect();
    vars.push((
        "barstate".to_string(),
        Value::Object {
            type_name: "barstate".to_string(),
            fields: Rc::new(RefCell::new(members)),
            call: None,
            value: None,
        },
    ));
    vars.push(("timenow".to_string(), Value::Number(now_ms)));
    vars
}

fn v8(version: PineVersion) -> u8 {
    version.number()
}

/// Keep a Pinecone input value typed so a host can pick the right widget.
fn input_default(value: &pine_core::InputValue) -> InputDefault {
    match value {
        pine_core::InputValue::Bool(value) => InputDefault::Bool(*value),
        pine_core::InputValue::Int(value) => InputDefault::Int(*value),
        pine_core::InputValue::Float(value) => InputDefault::Float(*value),
        pine_core::InputValue::Str(value) => InputDefault::Text(value.clone()),
        pine_core::InputValue::Color(color) => {
            InputDefault::Color([color.r, color.g, color.b, color.t])
        }
    }
}

/// Validate lexer tokens, rejecting only what this build genuinely cannot run.
///
/// `request`, `import`, `library` and `strategy` are no longer refused here: each
/// is answered from a host hook ([`HostEnvironment`]) or executed by the
/// runtime's own broker, and a host without the hook gets `na` or a runtime
/// diagnostic instead of a blanket "unsupported".
fn validate_tokens(tokens: &[pine_lexer::Token]) -> (Vec<Diagnostic>, usize) {
    use pine_lexer::TokenType as T;
    let mut plot_count = 0;
    for token in tokens {
        if matches!(token.typ, T::Comment(_) | T::BlankLine) {
            continue;
        }
        if matches!(&token.typ, T::Ident(name) if name == "plot") {
            plot_count += 1;
        }
    }
    (Vec::new(), plot_count)
}

/// The script's own declaration, read straight from its syntax tree.
///
/// The runtime records this for `indicator(...)`, but the release this bridge is
/// pinned to does not record it for `strategy(...)`, so a strategy would install
/// as an untitled "Pine Script" study. Reading the declaration here also keeps the
/// bridge independent of which of the two the runtime happens to fill in.
#[derive(Clone, Debug, Default, PartialEq)]
struct Declaration {
    /// The `title` argument.
    title: String,
    /// The `shorttitle` argument, when given.
    short_title: Option<String>,
    /// Whether the script draws over the price pane.
    overlay: bool,
    /// `indicator` or `strategy`, as written.
    kind: String,
}

impl Declaration {
    /// Which declaration the script used, e.g. `"strategy"`, or `""` when none.
    fn kind(&self) -> &str {
        &self.kind
    }
}

/// Read `indicator(...)`/`strategy(...)` off the top level of the program.
///
/// Only the leading declarations are considered: Pine requires exactly one, and a
/// call nested inside an `if` is not one. The title is the first positional
/// argument (or `title=`), matching how the runtime's own builtins bind it.
fn declaration_of(program: &pine_ast::Program) -> Declaration {
    use pine_ast::{Argument, Expr, Literal, Stmt};

    /// The literal text of a string argument, if it is one.
    fn text(expr: &Expr) -> Option<String> {
        match expr {
            Expr::Literal(Literal::String(value)) => Some(value.clone()),
            _ => None,
        }
    }

    /// A named argument's value.
    fn named<'a>(args: &'a [Argument], name: &str) -> Option<&'a Expr> {
        args.iter().find_map(|argument| match argument {
            Argument::Named { name: key, value } if key == name => Some(value),
            _ => None,
        })
    }

    /// The first positional argument, skipping pseudo-args the caller named.
    fn positional(args: &[Argument]) -> Option<&Expr> {
        args.iter().find_map(|argument| match argument {
            Argument::Positional(value) => Some(value),
            Argument::Named { .. } => None,
        })
    }

    for statement in &program.statements {
        let Stmt::Expression(Expr::Call { callee, args, .. }) = statement else {
            continue;
        };
        let Expr::Variable { name, .. } = callee.as_ref() else {
            continue;
        };
        if name != "indicator" && name != "strategy" {
            continue;
        }
        let title = named(args, "title")
            .and_then(text)
            .or_else(|| positional(args).and_then(text))
            .unwrap_or_default();
        let short_title = named(args, "shorttitle").and_then(text);
        let overlay = matches!(
            named(args, "overlay"),
            Some(Expr::Literal(Literal::Bool(true)))
        );
        return Declaration {
            title,
            short_title,
            overlay,
            kind: name.clone(),
        };
    }
    Declaration::default()
}

/// Literal `options=[...]` lists on `input.string`, keyed by the input's title.
///
/// The runtime records the chosen string but not the list it was chosen from, so
/// a dropdown has to be read from the script text. A list that is not a literal
/// array of strings is skipped: there is nothing honest to put in the menu.
fn string_options(program: &pine_ast::Program) -> HashMap<String, Vec<String>> {
    use pine_ast::{Argument, Expr, Literal, Visitor};

    struct Scan(HashMap<String, Vec<String>>);

    impl Visitor for Scan {
        fn visit_expr(&mut self, expr: &Expr) {
            if let Expr::Call { callee, args, .. } = expr {
                if let Some((title, options)) = string_input_choices(callee, args) {
                    self.0.insert(title, options);
                }
            }
            pine_ast::walk_expr(self, expr);
        }
    }

    fn string_input_choices(callee: &Expr, args: &[Argument]) -> Option<(String, Vec<String>)> {
        let Expr::MemberAccess { object, member, .. } = callee else {
            return None;
        };
        if member != "string" {
            return None;
        }
        let Expr::Variable { name, .. } = object.as_ref() else {
            return None;
        };
        if name != "input" {
            return None;
        }
        let title = named(args, "title")
            .and_then(text)
            .or_else(|| positional(args, 1).and_then(text))?;
        let options = named(args, "options").or_else(|| positional(args, 4))?;
        let Expr::Array(items) = options else {
            return None;
        };
        let options = items.iter().map(text).collect::<Option<Vec<_>>>()?;
        (!options.is_empty()).then_some((title, options))
    }

    fn text(expr: &Expr) -> Option<String> {
        match expr {
            Expr::Literal(Literal::String(value)) => Some(value.clone()),
            _ => None,
        }
    }

    fn named<'a>(args: &'a [Argument], name: &str) -> Option<&'a Expr> {
        args.iter().find_map(|argument| match argument {
            Argument::Named { name: key, value } if key == name => Some(value),
            _ => None,
        })
    }

    fn positional(args: &[Argument], nth: usize) -> Option<&Expr> {
        args.iter()
            .filter_map(|argument| match argument {
                Argument::Positional(value) => Some(value),
                Argument::Named { .. } => None,
            })
            .nth(nth)
    }

    let mut scan = Scan(HashMap::new());
    scan.visit_program(program);
    scan.0
}

fn split_location(message: &str) -> (String, Option<u32>, Option<u32>) {
    // Runtime messages look like `error [kind] 3:9: message`; parser errors look
    // like `... at line 3`. Pull the location out so the editor can reveal it.
    let mut line = None;
    let mut column = None;
    let mut text = message.to_string();
    for token in message.split_whitespace() {
        let Some((lhs, rhs)) = token.split_once(':') else {
            continue;
        };
        let (Ok(l), Ok(c)) = (lhs.parse::<u32>(), rhs.parse::<u32>()) else {
            continue;
        };
        if l == 0 || c == 0 {
            continue;
        }
        line = Some(l);
        column = Some(c);
        text = message.replace(token, "").trim().to_string();
        break;
    }
    if line.is_none() {
        if let Some(rest) = message.rsplit(" at line ").next() {
            if let Ok(l) = rest.trim().parse::<u32>() {
                line = Some(l);
            }
        }
    }
    (text, line, column)
}

/// Pine omits the alpha channel; `t` is transparency from 0 (opaque) to 100.
fn pack_color(color: &Color) -> u32 {
    let alpha = 255u32 - (u32::from(color.t.min(100)) * 255 / 100);
    ((color.r as u32) << 24) | ((color.g as u32) << 16) | ((color.b as u32) << 8) | alpha
}

/// Compile and execute `source` over `bars`.
///
/// `now_ms` is the script-visible `timenow`, supplied by the host so the engine
/// never reads the system clock. That keeps the same code path on native and in
/// the browser.
pub fn execute(source: &str, bars: &[Bar], now_ms: i64) -> Result<EngineOutput, EngineError> {
    execute_with_context(source, bars, now_ms, &RunContext::default())
}

/// Execute with the chart's symbol and interval available as `syminfo.*` and
/// `timeframe.*`.
///
/// Without a context those namespaces keep their empty defaults, which scripts
/// cannot tell apart from "the chart did not say". A live host should pass
/// [`RunContext::from_chart`] so conditions on `syminfo.ticker` and
/// `timeframe.period` behave like they do in TradingView.
pub fn execute_with_context(
    source: &str,
    bars: &[Bar],
    now_ms: i64,
    context: &RunContext,
) -> Result<EngineOutput, EngineError> {
    execute_with_inputs(source, bars, now_ms, context, &HashMap::new())
}

/// Execute with host overrides for the script's `input.*` values.
///
/// Pinecone resolves an override by the input's `title`, so the map is keyed by
/// title rather than by declaration order. A title the script does not declare is
/// ignored: the script never asked for it. The map's values use Pinecone's own
/// `InputValue` because that is what the interpreter reads.
pub fn execute_with_inputs(
    source: &str,
    bars: &[Bar],
    now_ms: i64,
    context: &RunContext,
    overrides: &HashMap<String, pine_core::InputValue>,
) -> Result<EngineOutput, EngineError> {
    execute_with_environment(
        source,
        bars,
        now_ms,
        context,
        overrides,
        &HostEnvironment::empty(),
    )
}

/// Execute with everything a host can supply: input overrides, other feeds for
/// `request.*`, imported library sources, and the drawings/strategy output.
///
/// `environment` is what makes `request.security`, `import` and `strategy.*` work.
/// An empty one keeps the old, honest behaviour: `request.*` reads `na`, an
/// `import` fails with a runtime diagnostic naming the path, and a `strategy`
/// still runs against the runtime's own simulated broker.
pub fn execute_with_environment(
    source: &str,
    bars: &[Bar],
    now_ms: i64,
    context: &RunContext,
    overrides: &HashMap<String, pine_core::InputValue>,
    environment: &HostEnvironment,
) -> Result<EngineOutput, EngineError> {
    if bars.is_empty() {
        return Err(EngineError::NoBars);
    }
    let version = match PineVersion::detect(source) {
        Ok(Some(version)) => version,
        Ok(None) => PineVersion::V5,
        Err(error) => {
            return Err(EngineError::Compile(vec![Diagnostic {
                severity: DiagnosticSeverity::Error,
                message: error.to_string(),
                line: None,
                column: None,
            }]));
        }
    };
    let tokens = pine_lexer::Lexer::with_version(source, version)
        .tokenize()
        .map_err(|error| {
            let (message, line, column) = split_location(&error.to_string());
            EngineError::Compile(vec![Diagnostic {
                severity: DiagnosticSeverity::Error,
                message,
                line,
                column,
            }])
        })?;
    let (_none, _declared_plot_count) = validate_tokens(&tokens);
    let program = pine_parser::Parser::new(tokens)
        .parse_program()
        .map_err(|error| {
            let (message, line, column) = split_location(&error.to_string());
            EngineError::Compile(vec![Diagnostic {
                severity: DiagnosticSeverity::Error,
                message,
                line,
                column,
            }])
        })?;

    // Static analysis first: it catches unknown members and wrong arities with a
    // line/column, instead of failing halfway through the run with no location.
    // The builtins are built with [`per_bar`] so the `wasm32-unknown-unknown`
    // build never reaches `SystemTime::now()` through `per_bar_variables`.
    let (consts, _advances) = pine_builtins::register_namespace_objects::<DrawingOutputSink>(
        version,
        Some(context.syminfo()),
        context.timeframe(),
    );
    let mut builtins = consts.clone();
    for (name, value) in per_bar(&PineBar::default(), None, now_ms as f64) {
        builtins.insert(name, value);
    }
    // Sema resolves `import` against the same hook the run uses, so a library the
    // host has is analysed. The loader is always installed — an empty one for a
    // host without libraries — because that is what turns a missing import into a
    // diagnostic that names the path and the line, instead of a silent `None`.
    let libraries = environment.library_hook();
    let sema_loader = Some(Box::new(InterpreterLibraries::new(Arc::clone(&libraries)))
        as Box<dyn pine_core::LibraryLoader>);
    let diagnostics = pine_sema::analyze(&program, &builtins, sema_loader.as_deref());
    let errors: Vec<Diagnostic> = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.severity == pine_diagnostics::Severity::Error)
        .map(|diagnostic| Diagnostic {
            severity: DiagnosticSeverity::Error,
            message: diagnostic.message.clone(),
            line: diagnostic.pos.map(|(line, _)| line),
            column: diagnostic.pos.map(|(_, column)| column),
        })
        .collect();
    if !errors.is_empty() {
        return Err(EngineError::Compile(errors));
    }

    // Build the bars ourselves instead of `Data::from_ohlcv`: that helper marks
    // every bar as confirmed history, which is wrong for a forming candle.
    let last_index = bars.len().saturating_sub(1);
    // `Data::new` defaults `syminfo` to empty. Attach the context here, or the
    // runtime registers an empty namespace even though sema above saw the real
    // values — which is exactly how `syminfo.mintick` silently read `0.0`.
    let data = Data::new(
        bars.iter()
            .enumerate()
            .map(|(index, bar)| {
                let forming = bar.forming && index == last_index;
                PineBar {
                    open: bar.open,
                    high: bar.high,
                    low: bar.low,
                    close: bar.close,
                    volume: bar.volume,
                    index: index as u64,
                    time: bar.time_ms,
                    is_first: index == 0,
                    is_last: index == last_index,
                    is_new: true,
                    is_confirmed: !forming,
                    is_history: !forming && bar.history,
                    is_realtime: forming,
                    is_last_confirmed_history: index == last_index && !forming,
                }
            })
            .collect(),
    )
    .with_syminfo(context.syminfo());

    let mut interpreter: Interpreter<DrawingOutputSink> = Interpreter::new();
    let (consts, advances) = pine_builtins::register_namespace_objects(
        version,
        Some(data.syminfo.clone()),
        context.timeframe(),
    );
    interpreter.set_const_variables(consts);
    // Overrides are keyed by the declaration title, which is how Pinecone looks
    // them up when the script calls `input.*`.
    interpreter.inputs = overrides.clone();
    // The fork splits per-bar advances into a pre-execute and a post-execute
    // phase (`pine_lang` sets both). Using only `pre` would skip stateful
    // builtins that must advance after the script runs.
    interpreter.per_bar_advances = advances.pre;
    interpreter.per_bar_post_advances = advances.post;
    // The chart's interval is what the strategy's metrics annualise against.
    if let Some(timeframe) = context.timeframe() {
        interpreter.timeframe = timeframe;
    }
    // `request.*` reads from the host's own cache, never from a socket.
    if let Some(provider) = environment.security() {
        interpreter.request_provider = Some(Rc::new(InterpreterFeed::new(
            Arc::clone(provider),
            data.syminfo.clone(),
        )));
    }
    // `import` resolves against the host's library directory. The loader is
    // installed even for a host with none, so a missing library is one named
    // runtime error instead of the interpreter's generic "not configured".
    interpreter.set_library_loader(Box::new(InterpreterLibraries::new(
        environment.library_hook(),
    )));

    let last = data.bars.last().cloned();
    let mut plots: Vec<PlotSeries> = Vec::new();
    // The declaration comes from the syntax tree, so a `strategy(...)` script is
    // named exactly like an `indicator(...)` one even on a runtime release that
    // only records the indicator form.
    let declaration = declaration_of(&program);
    let mut declared_title = declaration.title.clone();
    let mut short_title = declaration.short_title.clone();
    let mut overlay = declaration.overlay;
    let mut seen = 0usize;
    let mut inputs: Vec<ScriptInput> = Vec::new();
    // The strategy's trades, accumulated as each bar closes.
    let mut trades: Vec<TradeMarker> = Vec::new();
    let mut marks: Vec<ScriptMark> = Vec::new();
    let mut seen_closed = 0usize;
    let mut seen_open: Vec<(String, f64, u64)> = Vec::new();

    for bar in &data.bars {
        interpreter.current_time = Some(bar.time);
        interpreter.current_bar = Some(bar.clone());
        for (name, value) in per_bar(bar, last.as_ref(), now_ms as f64) {
            if matches!(value, Value::Series(_)) {
                interpreter.advance_series(&name, value);
            } else {
                interpreter.set_variable(&name, value);
            }
        }
        let output = interpreter.execute(&program).map_err(|error| {
            let (message, line, column) = split_location(&error.to_string());
            EngineError::Runtime(Diagnostic {
                severity: DiagnosticSeverity::Error,
                message,
                line,
                column,
            })
        })?;
        // Collect the declared inputs from the first bar only: the declarations
        // are the same on every bar, and taking them once keeps declaration order.
        if seen == 0 {
            inputs = output
                .inputs()
                .iter()
                .map(|input| ScriptInput {
                    kind: input.kind.clone(),
                    title: input.title.clone(),
                    group: input.group.clone(),
                    default: input_default(&input.default),
                    value: input_default(&input.value),
                    min: input.min_val,
                    max: input.max_val,
                    step: input.step,
                    options: Vec::new(),
                })
                .collect();
            let choices = string_options(&program);
            for input in &mut inputs {
                if input.kind == "string" {
                    if let Some(list) = choices.get(&input.title) {
                        input.options.clone_from(list);
                    }
                }
            }
        }
        if let Some(indicator) = output.indicator() {
            // The runtime's record wins when it has one, because it reflects what
            // the script actually executed; the syntax tree above is the fallback
            // for builtins the runtime does not fill in.
            if !indicator.title.is_empty() {
                declared_title = indicator.title.clone();
            }
            if !indicator.shorttitle.is_empty() {
                short_title = Some(indicator.shorttitle.clone());
            }
            overlay = indicator.overlay;
        }
        // A `strategy(...)` script declares the same metadata shape.
        if let Some(strategy) = interpreter.broker.as_deref() {
            collect_trades(
                strategy,
                bar.index as usize,
                &mut trades,
                &mut seen_closed,
                &mut seen_open,
            );
        }

        // Ensure plot series are initialized based on output plots count
        let out_plots = output.plots();
        let mut occurrences: HashMap<&str, usize> = HashMap::new();
        for (position, plot) in out_plots.iter().enumerate() {
            let occurrence = occurrences.entry(plot.title.as_str()).or_insert(0);
            if position >= plots.len() {
                let mut prior_values = vec![None; seen];
                let mut prior_colors = vec![None; seen];
                let has_values = plot.series.is_finite();
                prior_values.push(plot.series.is_finite().then_some(plot.series));
                prior_colors.push(plot.color.as_ref().map(pack_color));
                plots.push(PlotSeries {
                    title: plot.title.clone(),
                    occurrence: *occurrence,
                    values: prior_values,
                    colors: prior_colors,
                    has_values,
                });
            } else {
                let column = &mut plots[position];
                column.has_values |= plot.series.is_finite();
                column
                    .values
                    .push(plot.series.is_finite().then_some(plot.series));
                column.colors.push(plot.color.as_ref().map(pack_color));
            }
            *occurrence += 1;
        }
        // Shapes are cleared every bar, so they have to be taken now. A later bar
        // cannot reconstruct which bars a `plotshape` fired on.
        collect_marks(&output, bar, &mut marks);
        seen += 1;
    }

    for series in &mut plots {
        while series.values.len() < seen {
            series.values.push(None);
            series.colors.push(None);
        }
        series.values.truncate(seen);
        series.colors.truncate(seen);
    }

    let drawings = interpreter.output.drawings();

    Ok(EngineOutput {
        info: ScriptInfo {
            title: if declared_title.is_empty() {
                "Pine Script".to_string()
            } else {
                declared_title
            },
            short_title,
            overlay,
            version: v8(version),
            kind: if declaration.kind().is_empty() {
                "indicator".to_string()
            } else {
                declaration.kind().to_string()
            },
        },
        plots,
        inputs,
        boxes: drawings
            .boxes
            .values()
            .map(|value| script_box(value, &data.bars))
            .collect(),
        lines: drawings
            .lines
            .values()
            .filter_map(|value| script_line(value, &data.bars))
            .collect(),
        labels: drawings
            .labels
            .values()
            .map(|value| script_label(value, &data.bars))
            .collect(),
        tables: drawings.tables.values().map(script_table).collect(),
        marks,
        trades,
        diagnostics: Vec::new(),
    })
}

/// An x coordinate in bar-index space.
///
/// `xloc.bar_time` stores UNIX milliseconds. Drawings on the chart are addressed
/// by bar index, so a time is replaced by the bar whose open time is nearest.
/// A time before the first bar or after the last stays on that end bar rather
/// than becoming a fractional index the renderer cannot see.
fn placed_x(x: f64, time_x: bool, bars: &[PineBar]) -> f64 {
    if !time_x || !x.is_finite() || bars.is_empty() {
        return x;
    }
    let mut nearest = 0usize;
    let mut nearest_distance = f64::MAX;
    for (index, bar) in bars.iter().enumerate() {
        let distance = (bar.time as f64 - x).abs();
        if distance < nearest_distance {
            nearest = index;
            nearest_distance = distance;
        }
    }
    nearest as f64
}

/// Keep the shapes that fired on this bar.
///
/// `plotshape` on a boolean is `1` when it fires and `0` when it does not. A `0`
/// at `location.abovebar` is "don't draw", not a shape at price 0. `location.absolute`
/// is the exception: there the series *is* the price, and 0 is a real price.
fn collect_marks(output: &DrawingOutputSink, bar: &PineBar, marks: &mut Vec<ScriptMark>) {
    let at = |offset: f64| -> usize {
        let shifted = bar.index as i64 + offset as i64;
        shifted.max(0) as usize
    };
    let price = |location: &str, series: f64| -> Option<f64> {
        if !series.is_finite() {
            return None;
        }
        match location {
            "abovebar" | "top" => (series != 0.0).then_some(bar.high),
            "belowbar" | "bottom" => (series != 0.0).then_some(bar.low),
            _ => Some(series),
        }
    };
    for shape in output.plotshapes() {
        if let Some(at_price) = price(&shape.location, shape.series) {
            marks.push(ScriptMark {
                bar: at(shape.offset),
                price: at_price,
                text: shape.text.clone(),
                style: shape.style.clone(),
                up: !shape.style.contains("down"),
                color: shape.color.as_ref().map(pack_color),
            });
        }
    }
    for shape in output.plotchars() {
        if let Some(at_price) = price(&shape.location, shape.series) {
            marks.push(ScriptMark {
                bar: at(shape.offset),
                price: at_price,
                text: if shape.text.is_empty() {
                    shape.char.clone()
                } else {
                    shape.text.clone()
                },
                style: shape.char.clone(),
                up: true,
                color: shape.color.as_ref().map(pack_color),
            });
        }
    }
    for arrow in output.plotarrows() {
        if !arrow.series.is_finite() || arrow.series == 0.0 {
            continue;
        }
        let up = arrow.series > 0.0;
        marks.push(ScriptMark {
            bar: at(arrow.offset),
            price: if up { bar.high } else { bar.low },
            text: String::new(),
            style: if up { "arrowup" } else { "arrowdown" }.to_string(),
            up,
            color: (if up { &arrow.colorup } else { &arrow.colordown })
                .as_ref()
                .map(pack_color),
        });
    }
}

/// Record the fills the broker produced since the last bar.
///
/// A closed trade becomes one exit marker at its exit bar; a trade still open is
/// tracked by its entry so a later bar does not record it twice.
fn collect_trades(
    broker: &dyn pine_broker::Broker,
    bar: usize,
    trades: &mut Vec<TradeMarker>,
    seen_closed: &mut usize,
    seen_open: &mut Vec<(String, f64, u64)>,
) {
    let closed = broker.closed_trades();
    for trade in closed.iter().skip(*seen_closed) {
        let exit_bar = trade.exit_bar.map_or(bar, |exit| exit as usize);
        trades.push(TradeMarker {
            bar: exit_bar,
            price: trade.exit_price.unwrap_or(trade.entry_price),
            size: -trade.size,
            entry_id: trade.entry_id.clone(),
            exit: true,
        });
    }
    *seen_closed = closed.len();
    for trade in broker.open_trades() {
        let known = seen_open.iter().any(|(id, entry, at)| {
            id == &trade.entry_id && *entry == trade.entry_price && *at == trade.entry_bar
        });
        if !known {
            seen_open.push((trade.entry_id.clone(), trade.entry_price, trade.entry_bar));
            trades.push(TradeMarker {
                bar: trade.entry_bar as usize,
                price: trade.entry_price,
                size: trade.size,
                entry_id: trade.entry_id.clone(),
                exit: false,
            });
        }
    }
}

/// Map a recorded box, keeping `na` edges so the host can drop it.
///
/// An `xloc.bar_time` edge is a UNIX millisecond. The chart addresses drawings by
/// bar index, so the edge is moved to the bar whose open time is nearest. After
/// that `time_x` is false: the coordinates are indices.
fn script_box(value: &PineBox, bars: &[PineBar]) -> ScriptBox {
    let time_x = value.xloc.contains("time");
    ScriptBox {
        left: placed_x(value.left, time_x, bars),
        right: placed_x(value.right, time_x, bars),
        top: value.top,
        bottom: value.bottom,
        time_x: false,
        border_color: value.border_color.as_ref().map(pack_color),
        fill_color: value.bgcolor.as_ref().map(pack_color),
        border_width: value.border_width as f32,
        border_style: value.border_style.clone(),
        text: value.text.clone(),
    }
}

/// Map a recorded line. `hline` stores its price in `y1` with `x1 == x2 == 0`
/// and `extend = both`, which is exactly a horizontal line at that price.
fn script_line(value: &LineObject, bars: &[PineBar]) -> Option<ScriptLine> {
    if !value.y1.is_finite() || !value.y2.is_finite() {
        return None;
    }
    let time_x = value.xloc == XLoc::BarTime;
    Some(ScriptLine {
        x1: placed_x(value.x1, time_x, bars),
        y1: value.y1,
        x2: placed_x(value.x2, time_x, bars),
        y2: value.y2,
        time_x: false,
        color: value.color.as_ref().map(pack_color),
        style: match value.style {
            PineLineStyle::StyleSolid => "solid",
            PineLineStyle::StyleDotted => "dotted",
            PineLineStyle::StyleDashed => "dashed",
            PineLineStyle::StyleArrowLeft => "arrow_left",
            PineLineStyle::StyleArrowRight => "arrow_right",
            PineLineStyle::StyleArrowBoth => "arrow_both",
        }
        .to_string(),
        width: value.width as f32,
        extend: match value.extend {
            Extend::None => "none",
            Extend::Left => "left",
            Extend::Right => "right",
            Extend::Both => "both",
        }
        .to_string(),
    })
}

/// Map a recorded label. A label without text is a marker-only handle the host
/// cannot draw, so it is kept as an empty-text label rather than dropped: a shape
/// style may still carry meaning.
fn script_label(value: &Label, bars: &[PineBar]) -> ScriptLabel {
    let time_x = value.xloc == XLoc::BarTime;
    ScriptLabel {
        x: placed_x(value.x, time_x, bars),
        y: value.y,
        time_x: false,
        yloc: match value.yloc {
            YLoc::Price => "price",
            YLoc::Abovebar => "abovebar",
            YLoc::Belowbar => "belowbar",
        }
        .to_string(),
        text: value.text.clone(),
        color: value.color.as_ref().map(pack_color),
        text_color: value.textcolor.as_ref().map(pack_color),
        size: match value.size {
            Size::Auto => "auto",
            Size::Tiny => "tiny",
            Size::Small => "small",
            Size::Normal => "normal",
            Size::Large => "large",
            Size::Huge => "huge",
        }
        .to_string(),
        tooltip: value.tooltip.clone(),
    }
}

/// Map a recorded table, in a stable row-major order.
fn script_table(value: &Table) -> ScriptTable {
    let mut cells: Vec<ScriptCell> = value
        .cells
        .iter()
        .map(|(&(column, row), cell)| ScriptCell {
            column,
            row,
            text: cell.text.clone(),
            text_color: cell.text_color.as_ref().map(pack_color),
            background: cell.bgcolor.as_ref().map(pack_color),
            text_size: cell.text_size.clone(),
            halign: cell.text_halign.clone(),
            valign: cell.text_valign.clone(),
        })
        .collect();
    cells.sort_by_key(|cell| (cell.row, cell.column));
    ScriptTable {
        position: value.position.clone(),
        columns: value.columns,
        rows: value.rows,
        background: value.bgcolor.as_ref().map(pack_color),
        cells,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bars(count: usize) -> Vec<Bar> {
        (0..count)
            .map(|i| {
                let close = 100.0 + i as f64;
                Bar {
                    time_ms: 1_700_000_000_000 + i as i64 * 60_000,
                    open: close - 1.0,
                    high: close + 1.0,
                    low: close - 2.0,
                    close,
                    volume: 10.0 + i as f64,
                    forming: false,
                    history: true,
                }
            })
            .collect()
    }

    #[test]
    fn two_plots_keep_declaration_order_and_warmup_gaps() {
        let source = r#"//@version=5
indicator("Two plots", overlay = true)
fast = ta.sma(close, 3)
slow = ta.sma(close, 10)
plot(fast, title = "Fast", color = color.orange)
plot(slow, title = "Slow", color = color.blue)
"#;
        let output = execute(source, &bars(30), 1_700_000_000_000).expect("runs");
        assert!(output.info.overlay, "overlay declaration is mapped");
        assert_eq!(output.info.version, 5);
        assert_eq!(
            output
                .plots
                .iter()
                .map(|plot| plot.title.as_str())
                .collect::<Vec<_>>(),
            ["Fast", "Slow"]
        );
        let fast = &output.plots[0];
        let slow = &output.plots[1];
        assert_eq!(fast.values.len(), 30);
        assert_eq!(fast.values.iter().filter(|v| v.is_none()).count(), 2);
        assert_eq!(slow.values.iter().filter(|v| v.is_none()).count(), 9);
        assert!(fast.values[29].is_some());
        assert_eq!(fast.colors[29], Some(0xFFA500FF), "orange, opaque");
    }

    /// `bar_index` is a series in Pine: structure scripts anchor boxes on
    /// `bar_index[2]`, the bar a three-bar gap opened on. It used to be published
    /// as a plain number, so every such lookback read `na` and the box silently
    /// spanned nothing.
    /// `xloc.bar_time` stores UNIX milliseconds. The chart addresses a drawing by
    /// bar index, so the line must come back on the bar whose open time matches.
    #[test]
    fn a_time_anchored_line_lands_on_its_bar() {
        let source = r#"//@version=5
indicator("time", overlay = true)
if bar_index == 2
    line.new(time, low, time, high, xloc = xloc.bar_time)
plot(close, title = "p")
"#;
        let output = execute(source, &bars(5), 0).expect("runs");
        assert_eq!(output.lines.len(), 1, "{:?}", output.lines);
        assert_eq!(output.lines[0].x1, 2.0, "the time resolved to bar 2");
        assert_eq!(output.lines[0].x2, 2.0);
        assert!(!output.lines[0].time_x, "the host receives an index");
    }

    /// A boolean `plotshape` fires on the true bars only, and `abovebar` sits on
    /// the bar's high rather than on the `1` the condition became.
    #[test]
    fn a_plotshape_fires_on_the_true_bars() {
        let source = r#"//@version=5
indicator("shape", overlay = true)
plotshape(bar_index == 2, title = "Mark", style = shape.triangleup, location = location.abovebar, text = "B")
plot(close, title = "p")
"#;
        let output = execute(source, &bars(5), 0).expect("runs");
        assert_eq!(output.marks.len(), 1, "{:?}", output.marks);
        assert_eq!(output.marks[0].bar, 2);
        assert_eq!(output.marks[0].price, bars(5)[2].high);
        assert_eq!(output.marks[0].text, "B");
        assert!(output.marks[0].up);
    }

    #[test]
    fn bar_index_supports_lookback() {
        let output = execute(
            "//@version=5\nindicator(\"idx\")\nplot(bar_index)\nplot(bar_index[2])\n",
            &bars(5),
            0,
        )
        .expect("runs");
        assert_eq!(
            output.plots[0].values,
            [Some(0.0), Some(1.0), Some(2.0), Some(3.0), Some(4.0)],
            "the current index is still a plain value"
        );
        assert_eq!(
            output.plots[1].values,
            [None, None, Some(0.0), Some(1.0), Some(2.0)],
            "and it keeps history"
        );
    }

    /// A box anchored on a lookback must span the bars, not collapse to `na`.
    #[test]
    fn a_box_anchored_on_a_lookback_keeps_its_left_edge() {
        let source = r#"//@version=5
indicator("anchor", overlay = true)
box.new(bar_index[2], high, bar_index, low)
plot(close, title = "p")
"#;
        let output = execute(source, &bars(5), 0).expect("runs");
        let boxed = output.boxes.last().expect("a box on the last bar");
        assert_eq!(boxed.left, 2.0, "left edge is two bars back");
        assert_eq!(boxed.right, 4.0, "right edge is the current bar");
    }

    #[test]
    fn version_six_subset_runs() {
        let source = r#"//@version=6
indicator("v6", overlay = true)
plot(ta.sma(close, 2), title = "S")
"#;
        let output = execute(source, &bars(10), 0).expect("v6 subset runs");
        assert_eq!(output.info.version, 6);
        assert_eq!(output.plots.len(), 1);
    }

    #[test]
    fn compile_error_reports_a_line() {
        let source = "//@version=5\nindicator(\"Broken\"\nplot(close\n";
        let error = execute(source, &bars(5), 0).expect_err("must fail");
        match error {
            EngineError::Compile(diagnostics) => {
                assert_eq!(diagnostics[0].severity, DiagnosticSeverity::Error);
                assert!(diagnostics[0].line.is_some(), "{diagnostics:?}");
            }
            other => panic!("expected a compile error, got {other:?}"),
        }
    }

    #[test]
    fn unsupported_member_is_a_compile_error() {
        let source = "//@version=6\nindicator(\"x\")\nplot(ta.not_a_real_builtin(close, 3))\n";
        match execute(source, &bars(5), 0) {
            Err(EngineError::Compile(diagnostics)) => {
                assert!(
                    diagnostics[0].message.contains("not_a_real_builtin"),
                    "{diagnostics:?}"
                );
            }
            other => panic!("expected a compile error, got {other:?}"),
        }
    }

    #[test]
    fn empty_series_is_rejected() {
        assert_eq!(execute("plot(close)", &[], 0), Err(EngineError::NoBars));
    }

    #[test]
    fn timenow_comes_from_the_host_clock() {
        let source = r#"//@version=5
indicator("t")
plot(timenow, title = "Now")
"#;
        let output = execute(source, &bars(3), 1_234_567).expect("runs");
        assert_eq!(output.plots[0].values[0], Some(1_234_567.0));
    }

    #[test]
    fn duplicate_titles_stay_separate() {
        let source = r#"//@version=5
indicator("dup")
plot(close, title = "Same")
plot(open, title = "Same")
"#;
        let output = execute(source, &bars(4), 0).expect("runs");
        assert_eq!(output.plots.len(), 2, "two plots, not one merged column");
        assert_eq!(output.plots[0].occurrence, 0);
        assert_eq!(output.plots[1].occurrence, 1);
        assert_ne!(
            output.plots[0].values[0], output.plots[1].values[0],
            "close and open must not merge"
        );
    }

    #[test]
    fn transparency_becomes_alpha() {
        let source = r#"//@version=5
indicator("alpha")
plot(close, title = "Faded", color = color.new(color.red, 50))
"#;
        let output = execute(source, &bars(3), 0).expect("runs");
        let packed = output.plots[0].colors[0].expect("a colour");
        let alpha = (packed & 0xFF) as u8;
        assert!(
            (120..=135).contains(&alpha),
            "50% transparency, got {alpha}"
        );
    }

    /// `request`, `import`, `library` and `strategy` are no longer compile errors:
    /// each is answered by a host hook or executed by the runtime's own broker.
    /// What a host cannot answer must still be reported, and never silently
    /// treated as a working run.
    #[test]
    fn host_dependent_constructs_run_and_report_what_the_host_cannot_answer() {
        // A strategy runs against the runtime's simulated broker even with no host
        // hooks at all.
        let strategy = execute(
            "//@version=5\nstrategy(\"s\")\nplot(close, title = \"p\")\n",
            &bars(3),
            0,
        )
        .expect("a strategy must run without host hooks");
        assert_eq!(strategy.plots.len(), 1);

        // `request.security` with no feed reads `na` instead of refusing the script.
        let requested = execute(
            "//@version=5\nindicator(\"x\")\na = request.security(\"X\", \"D\", close)\nplot(a)\n",
            &bars(3),
            0,
        )
        .expect("request.* must degrade to na, not fail");
        assert_eq!(requested.plots[0].values, [None, None, None]);

        // An `import` with no resolver is a diagnostic naming the path, with a line.
        let imported = execute(
            "//@version=5\nindicator(\"x\")\nimport username/lib/1 as lib\nplot(close)\n",
            &bars(3),
            0,
        );
        match imported {
            Err(EngineError::Compile(diagnostics)) => assert!(
                diagnostics
                    .iter()
                    .any(|d| d.message.contains("username/lib/1") && d.line.is_some()),
                "{diagnostics:?}"
            ),
            other => panic!("an unresolvable import must be reported, got {other:?}"),
        }
    }

    #[test]
    fn host_hooks_answer_request_and_import() {
        use crate::host::{HostEnvironment, MemoryLibraries, SecurityProvider};
        use pine_core::{Bar as RuntimeBar, Data};

        /// Serves one flat bar at 50.0 for any request.
        struct Flat;
        impl SecurityProvider for Flat {
            fn request(&self, _symbol: &str, _timeframe: &str) -> Option<Data> {
                Some(Data::new(vec![RuntimeBar {
                    open: 50.0,
                    high: 50.0,
                    low: 50.0,
                    close: 50.0,
                    volume: 1.0,
                    ..RuntimeBar::default()
                }]))
            }
        }

        let environment = HostEnvironment::empty()
            .with_security(Arc::new(Flat))
            .with_libraries(Arc::new(MemoryLibraries::new().with_library(
                "acme/Math/1",
                "//@version=5\nlibrary(\"Math\")\nadd(a, b) => a + b\nexport add\n",
            )));

        let requested = execute_with_environment(
            "//@version=5\nindicator(\"x\")\nsec = request.security(\"OTHER\", \"1D\", close)\nplot(sec)\n",
            &bars(2),
            0,
            &RunContext::default(),
            &HashMap::new(),
            &environment,
        )
        .expect("the host feed answers");
        assert_eq!(
            requested.plots[0].values,
            [Some(50.0), Some(50.0)],
            "the requested series replaces na"
        );

        let imported = execute_with_environment(
            "//@version=5\nindicator(\"x\")\nimport acme/Math/1 as m\nplot(m.add(close, 1))\n",
            &bars(2),
            0,
            &RunContext::default(),
            &HashMap::new(),
            &environment,
        )
        .expect("the host library resolves");
        assert_eq!(imported.plots[0].values, [Some(101.0), Some(102.0)]);
    }

    /// A script that only draws still has to run: `NoPlots` is the study layer's
    /// decision, not the engine's.
    #[test]
    fn a_drawing_only_script_runs_and_reports_its_geometry() {
        let source = r#"//@version=5
indicator("boxes", overlay = true)
var b = box.new(bar_index, high, bar_index + 1, low, border_color = color.red, bgcolor = color.new(color.blue, 80))
var l = line.new(bar_index, low, bar_index + 1, high, color = color.green)
var t = label.new(bar_index, high, "tag")
plot(close, title = "p")
"#;
        let output = execute(source, &bars(4), 0).expect("drawings must run");
        assert_eq!(output.boxes.len(), 1, "a var-held box is created once");
        assert_eq!(output.lines.len(), 1, "{:?}", output.lines);
        assert_eq!(output.labels.len(), 1, "{:?}", output.labels);
        let boxed = &output.boxes[0];
        assert!(boxed.border_color.is_some(), "a border colour survives");
        assert!(boxed.fill_color.is_some(), "a fill colour survives");
        assert_eq!(output.labels[0].text, "tag");
        assert!(
            !output.lines[0].time_x,
            "bar-index x coordinates stay indices"
        );
    }

    /// An un-held `box.new` runs on every bar, which is Pine's own behaviour: one
    /// box per bar, not one box total.
    #[test]
    fn a_box_created_without_var_accumulates_one_per_bar() {
        let source = r#"//@version=5
indicator("boxes")
box.new(bar_index, high, bar_index + 1, low)
plot(close)
"#;
        let output = execute(source, &bars(3), 0).expect("runs");
        assert_eq!(output.boxes.len(), 3, "{:?}", output.boxes);
    }

    /// A drawn table keeps its cells and their row-major order.
    #[test]
    fn a_table_reports_its_cells() {
        let source = r#"//@version=5
indicator("table")
var t = table.new(position.top_right, 2, 2)
if barstate.islast
    table.cell(t, 0, 0, "A")
    table.cell(t, 1, 1, "B")
plot(close)
"#;
        let output = execute(source, &bars(3), 0).expect("runs");
        assert_eq!(output.tables.len(), 1, "{:?}", output.tables);
        let table = &output.tables[0];
        assert_eq!((table.columns, table.rows), (2, 2));
        assert_eq!(
            table
                .cells
                .iter()
                .map(|cell| (cell.row, cell.column, cell.text.as_str()))
                .collect::<Vec<_>>(),
            [(0, 0, "A"), (1, 1, "B")]
        );
    }

    /// Size and alignment travel with the cell. The runtime already stores them;
    /// frame, border, and merge do not.
    #[test]
    fn a_cell_keeps_its_size_and_alignment() {
        let source = r#"//@version=5
indicator("table")
var t = table.new(position.bottom_left, 1, 1)
if barstate.islast
    table.cell(t, 0, 0, "Hi", text_size = size.huge, text_halign = text.align_right, text_valign = text.align_top)
plot(close)
"#;
        let output = execute(source, &bars(2), 0).expect("runs");
        let cell = &output.tables[0].cells[0];
        assert_eq!(cell.text_size, "huge");
        assert_eq!(cell.halign, "align_right");
        assert_eq!(cell.valign, "align_top");
    }

    /// The array and matrix calls LuxAlgo scripts actually use, reduced to plots.
    #[test]
    fn array_and_matrix_builtins_reduce_to_plots() {
        let source = r#"//@version=5
indicator("collections")
var a = array.from(3.0, 1.0, 2.0)
var m = matrix.new<float>(2, 2, 0.0)
if barstate.isfirst
    array.sort(a)
    array.push(a, 4.0)
    matrix.set(m, 0, 0, 1.0)
    matrix.set(m, 0, 1, 2.0)
    matrix.set(m, 1, 0, 3.0)
    matrix.set(m, 1, 1, 4.0)
added = matrix.sum(m, m)
flipped = matrix.transpose(m)
plot(array.get(a, 0))
plot(array.size(a))
plot(array.sum(a))
plot(array.avg(a))
plot(array.min(a))
plot(array.max(a))
plot(array.includes(a, 4.0) ? 1 : 0)
plot(array.indexof(a, 2.0))
plot(matrix.get(m, 1, 0))
plot(matrix.trace(m))
plot(matrix.rows(m))
plot(matrix.columns(m))
plot(matrix.get(added, 0, 1))
plot(matrix.get(flipped, 0, 1))
"#;
        let output = execute(source, &bars(2), 0).expect("collections run");
        let last = |index: usize| {
            output.plots[index]
                .values
                .last()
                .copied()
                .flatten()
                .unwrap_or(f64::NAN)
        };
        let got: Vec<f64> = (0..14).map(last).collect();
        assert_eq!(
            got,
            vec![
                1.0, 4.0, 10.0, 2.5, 1.0, 4.0, 1.0, 1.0, 3.0, 5.0, 2.0, 2.0, 4.0, 3.0
            ],
            "{got:?}"
        );
    }

    /// A `strategy.*` script reports its fills as markers on the right bars.
    #[test]
    fn a_strategy_reports_its_order_fills() {
        let source = r#"//@version=5
strategy("fills", overlay = true)
if bar_index == 1
    strategy.entry("L", strategy.long)
if bar_index == 3
    strategy.close("L")
plot(close, title = "p")
"#;
        let output = execute(source, &bars(6), 0).expect("a strategy runs");
        assert!(!output.trades.is_empty(), "{:?}", output.trades);
        let entry = output.trades.iter().find(|trade| !trade.exit);
        assert!(
            entry.is_some(),
            "an entry marker exists: {:?}",
            output.trades
        );
        assert!(output.trades.iter().any(|trade| trade.exit), "an exit");
    }

    #[test]
    fn unsupported_tokens_ignore_strings_and_comments() {
        let source = r#"//@version=5
indicator("strategy( // label.new and plotchar")
// request.security and line.new
plot(close, title="fill( // bgcolor")
"#;
        assert!(execute(source, &bars(2), 0).is_ok());
    }

    /// A literal `options` list is part of the declaration. The runtime only
    /// stores the chosen string, so the list is read from the script.
    #[test]
    fn a_literal_options_list_reaches_the_input() {
        let source = r#"//@version=5
indicator("x")
mode = input.string("ema", "Mode", options = ["ema", "sma"])
plot(mode == "sma" ? 1 : 0)
"#;
        let output = execute(source, &bars(2), 0).expect("runs");
        assert_eq!(output.inputs[0].options, ["ema", "sma"]);
        assert_eq!(output.inputs[0].value, InputDefault::Text("ema".into()));
    }

    #[test]
    fn input_declarations_run_and_reach_the_output() {
        let source = r#"//@version=5
indicator("x")
len = input.int(14, "Length", minval=1, maxval=200)
plot(close + len * 0)
"#;
        let output = execute(source, &bars(2), 0).expect("input.int must run");
        assert_eq!(output.inputs.len(), 1, "{:?}", output.inputs);
        assert_eq!(output.inputs[0].kind, "int");
        assert_eq!(output.inputs[0].title, "Length");
        assert_eq!(output.inputs[0].default, InputDefault::Int(14));
        assert_eq!(output.inputs[0].min, Some(1.0));
        assert_eq!(output.inputs[0].max, Some(200.0));
    }

    #[test]
    fn an_override_by_title_changes_the_run() {
        let source = r#"//@version=5
indicator("x")
len = input.int(14, "Length")
plot(len)
"#;
        let default = execute(source, &bars(2), 0).expect("default run");
        assert_eq!(default.plots[0].values[0], Some(14.0));

        let mut overrides = HashMap::new();
        overrides.insert("Length".to_owned(), pine_core::InputValue::Int(7));
        let overridden =
            execute_with_inputs(source, &bars(2), 0, &RunContext::default(), &overrides)
                .expect("override run");
        assert_eq!(overridden.plots[0].values[0], Some(7.0));
        assert_eq!(
            overridden.inputs[0].value,
            InputDefault::Int(7),
            "the run reports the value it actually used"
        );
        assert_eq!(
            overridden.inputs[0].default,
            InputDefault::Int(14),
            "the declared default stays the default"
        );
    }

    #[test]
    fn an_unknown_override_title_is_ignored() {
        let source = r#"//@version=5
indicator("x")
len = input.int(14, "Length")
plot(len)
"#;
        let mut overrides = HashMap::new();
        overrides.insert("Missing".to_owned(), pine_core::InputValue::Int(99));
        let output = execute_with_inputs(source, &bars(2), 0, &RunContext::default(), &overrides)
            .expect("unknown titles must not fail the run");
        assert_eq!(output.plots[0].values[0], Some(14.0));
    }

    /// Every namespace the runtime registers is reachable as a value, so a script
    /// that names one does not need the old "unsupported" filter to explain
    /// itself. A genuinely unknown member is still a compile error with a line.
    #[test]
    fn known_namespaces_are_values_and_unknown_members_are_still_errors() {
        for name in ["strategy", "request", "library"] {
            let source = format!("//@version=5\nindicator(\"x\")\nf = {name}\nplot(close)\n");
            assert!(
                execute(&source, &bars(2), 0).is_ok(),
                "`{name}` must be a readable value"
            );
        }
        let source = "//@version=5\nindicator(\"x\")\nplot(request.not_a_member(\"X\"))\n";
        assert!(
            matches!(execute(source, &bars(2), 0), Err(EngineError::Compile(ref d)) if d.iter().any(|d| d.line.is_some())),
            "an unknown member must still fail with a location"
        );
    }

    #[test]
    fn all_na_and_duplicate_empty_titles_preserve_positions() {
        let source = "indicator(\"x\")\nplot(na)\nplot(bar_index == 1 ? na : close)\nplot(open)\n";
        let out = execute(source, &bars(3), 0).unwrap();
        assert_eq!(out.plots.len(), 3);
        assert_eq!(
            out.plots.iter().map(|p| p.occurrence).collect::<Vec<_>>(),
            [0, 1, 2]
        );
        assert_eq!(out.plots[0].values, [None, None, None]);
        assert_eq!(out.plots[1].values, [Some(100.0), None, Some(102.0)]);
        assert_eq!(out.plots[2].values, [Some(99.0), Some(100.0), Some(101.0)]);
    }

    #[test]
    fn non_default_plot_rendering_is_accepted() {
        for argument in [
            "linewidth=2",
            "offset=1",
            "style=plot.style_histogram",
            "display=display.none",
            "show_last=1",
        ] {
            let source = format!("indicator(\"x\")\nplot(close, {argument})\n");
            assert!(execute(&source, &bars(2), 0).is_ok(), "{argument}");
        }
    }

    #[test]
    fn a_forming_last_candle_is_not_history() {
        let source = r#"//@version=5
indicator("state")
plot(barstate.isrealtime ? 1 : 0, title = "Realtime")
plot(barstate.isconfirmed ? 1 : 0, title = "Confirmed")
plot(barstate.ishistory ? 1 : 0, title = "History")
"#;
        let mut history = bars(4);
        let confirmed = execute(source, &history, 0).expect("runs");
        assert_eq!(confirmed.plots[0].values[3], Some(0.0));
        assert_eq!(confirmed.plots[1].values[3], Some(1.0));
        assert_eq!(confirmed.plots[2].values[3], Some(1.0));

        // The host marks the newest candle as still forming.
        history[3].forming = true;
        let live = execute(source, &history, 0).expect("runs");
        assert_eq!(live.plots[0].values[3], Some(1.0), "last bar is realtime");
        assert_eq!(live.plots[1].values[3], Some(0.0), "forming is unconfirmed");
        assert_eq!(live.plots[2].values[3], Some(0.0), "forming is not history");
        assert_eq!(
            live.plots[2].values[2],
            Some(1.0),
            "earlier bars stay history"
        );
    }

    #[test]
    fn alpha_endpoints_and_midpoint() {
        assert_eq!(pack_color(&Color::new(1, 2, 3, 0)), 0x010203ff);
        assert_eq!(pack_color(&Color::new(1, 2, 3, 50)), 0x01020380);
        assert_eq!(pack_color(&Color::new(1, 2, 3, 100)), 0x01020300);
    }

    #[test]
    fn drawings_and_styling_tokens_are_accepted_without_error() {
        let source = r#"//@version=6
indicator("Drawings test")
b = box.new(1, high, 2, low)
l = line.new(1, high, 2, low)
box.set_top(b, high)
line.set_y2(l, low)
box.delete(b)
line.delete(l)
plot(close)
"#;
        assert!(execute(source, &bars(5), 0).is_ok());
    }

    #[test]
    fn hex_color_literal_accepted_in_plot() {
        let source = r#"//@version=5
indicator("Hex Color")
plot(close, color = #089981)
"#;
        assert!(execute(source, &bars(5), 0).is_ok());
    }

    #[test]
    fn conditional_and_na_colors_are_accepted_without_error() {
        let source = r#"//@version=5
indicator("Conditional Color")
col = close > open ? #089981 : na
plot(close, color = col)
plot(open, color = color.new(col, 50))
"#;
        assert!(execute(source, &bars(5), 0).is_ok());
    }

    #[test]
    fn for_in_loop_over_map_is_accepted() {
        let source = r#"//@version=6
indicator("Map For In")
m = map.new<float, float>()
map.put(m, 1.0, 10.0)
map.put(m, 2.0, 20.0)
var total = 0.0
for [k, v] in m
    total += v
plot(total)
"#;
        assert!(execute(source, &bars(5), 0).is_ok());
    }

    #[test]
    fn semantic_pass_does_not_need_a_system_clock() {
        // Building the sema builtins with `per_bar` is what keeps this working on
        // wasm32-unknown-unknown, where `SystemTime::now()` panics.
        let source = "//@version=5\nindicator(\"t\")\nplot(timenow)\n";
        assert!(execute(source, &bars(2), 42).is_ok());
    }
}
