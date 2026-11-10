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
use pine_core::{
    Bar as PineBar, Color, Data, DefaultPineOutput, InputOutput, MetadataOutput, PineVersion,
    PlotOutput,
};
use pine_interpreter::{Interpreter, Series, Value};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

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
    /// `indicator(...)` title, or the fallback title.
    pub title: String,
    /// Short title, when declared.
    pub short_title: Option<String>,
    /// `overlay = true` in the declaration.
    pub overlay: bool,
    /// Version the script declared or the host default.
    pub version: u8,
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
    /// Non-fatal messages.
    pub diagnostics: Vec<Diagnostic>,
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
fn per_bar(
    bar: &PineBar,
    last: Option<&PineBar>,
    now_ms: f64,
) -> Vec<(String, Value<DefaultPineOutput>)> {
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
        ("bar_index".to_string(), Value::Number(bar.index as f64)),
        (
            "last_bar_index".to_string(),
            last.map_or(Value::Na, |b| Value::Number(b.index as f64)),
        ),
        (
            "last_bar_time".to_string(),
            last.map_or(Value::Na, |b| Value::Number(b.time as f64)),
        ),
    ];
    let members: HashMap<String, Value<DefaultPineOutput>> = [
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

/// Validate lexer tokens, including references to unsupported functions (aliases).
/// Plot identity is only provable for direct, unconditional top-level calls.
fn validate_tokens(tokens: &[pine_lexer::Token]) -> (Vec<Diagnostic>, usize) {
    use pine_lexer::TokenType as T;
    let tokens: Vec<_> = tokens
        .iter()
        .filter(|t| !matches!(t.typ, T::Comment(_) | T::BlankLine))
        .collect();
    let mut diagnostics = Vec::new();
    let mut indent = 0usize;
    let mut plot_count = 0;
    for (i, token) in tokens.iter().enumerate() {
        match &token.typ {
            T::Indent => indent += 1,
            T::Dedent => indent = indent.saturating_sub(1),
            _ => {}
        }
        let name = match &token.typ {
            T::Import => Some("import"),
            T::Ident(name) => Some(name.as_str()),
            _ => None,
        };
        let Some(name) = name else { continue };
        let forbidden = matches!(
            name,
            "import"
                | "strategy"
                | "library"
                | "request"
                | "label"
                | "box"
                | "table"
                | "line"
                | "linefill"
                | "polyline"
                | "fill"
                | "bgcolor"
                | "barcolor"
                | "plotshape"
                | "plotchar"
                | "plotarrow"
                | "plotbar"
                | "plotcandle"
                | "hline"
                | "alert"
                | "alertcondition"
        );
        let reason = if forbidden {
            Some(match name {
                "strategy" => "`strategy` scripts are not supported by the OpineRust bridge; only `indicator` scripts run.".to_string(),
                "library" => "`library` scripts are not supported by the OpineRust bridge; only `indicator` scripts run.".to_string(),
                "import" => "`import` is not supported by the OpineRust bridge; imported libraries cannot be resolved.".to_string(),
                "request" => format!("`{name}.*` is not supported by the OpineRust bridge; this build has no cross-symbol or cross-timeframe data."),
                other => format!("`{other}` is not supported by the OpineRust bridge"),
            })
        } else if name == "plot" {
            let direct = matches!(tokens.get(i + 1).map(|t| &t.typ), Some(T::LParen));
            let start = i == 0 || matches!(tokens[i - 1].typ, T::Newline | T::Dedent);
            if indent != 0 || !direct || !start {
                Some("plot identity is ambiguous: only direct unconditional top-level plot calls are supported".to_string())
            } else {
                plot_count += 1;
                None
            }
        } else {
            None
        };
        if let Some(message) = reason {
            diagnostics.push(Diagnostic {
                severity: DiagnosticSeverity::Error,
                message,
                line: Some(token.line as u32),
                column: Some(token.column as u32),
            });
        }
    }
    (diagnostics, plot_count)
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
    let (unsupported, declared_plot_count) = validate_tokens(&tokens);
    if !unsupported.is_empty() {
        return Err(EngineError::Compile(unsupported));
    }
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
    let (consts, _advances) = pine_builtins::register_namespace_objects::<DefaultPineOutput>(
        version,
        Some(context.syminfo()),
        context.timeframe(),
    );
    let mut builtins = consts.clone();
    for (name, value) in per_bar(&PineBar::default(), None, now_ms as f64) {
        builtins.insert(name, value);
    }
    let diagnostics = pine_sema::analyze(&program, &builtins, None);
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

    let mut interpreter: Interpreter<DefaultPineOutput> = Interpreter::new();
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

    let last = data.bars.last().cloned();
    let mut plots: Vec<PlotSeries> = Vec::new();
    let mut declared_title = String::new();
    let mut short_title = None;
    let mut overlay = false;
    let mut seen = 0usize;
    let mut inputs: Vec<ScriptInput> = Vec::new();

    for bar in &data.bars {
        interpreter.current_time = Some(bar.time);
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
                })
                .collect();
        }
        if let Some(indicator) = output.indicator() {
            if !indicator.timeframe.is_empty()
                || !indicator.format.is_empty()
                || indicator.precision.is_some()
            {
                return Err(EngineError::Runtime(Diagnostic {
                    severity: DiagnosticSeverity::Error,
                    message: "indicator timeframe, format, and precision are not supported by the OpineRust bridge".into(),
                    line: None,
                    column: None,
                }));
            }
            if declared_title.is_empty() && !indicator.title.is_empty() {
                declared_title = indicator.title.clone();
            }
            if !indicator.shorttitle.is_empty() {
                short_title = Some(indicator.shorttitle.clone());
            }
            overlay = indicator.overlay;
        }

        // Pinecone does not expose a call-site ID. A different count, order or
        // title is unsafe, not a missing value that can be guessed from titles.
        if output.plots().len() != declared_plot_count {
            return Err(EngineError::Runtime(Diagnostic {
                severity: DiagnosticSeverity::Error,
                message:
                    "plot identity is ambiguous: runtime output count differs from declarations"
                        .into(),
                line: None,
                column: None,
            }));
        }
        let mut occurrences: HashMap<&str, usize> = HashMap::new();
        for (position, plot) in output.plots().iter().enumerate() {
            if plot.linewidth != 1.0
                || plot.style != "line"
                || plot.trackprice
                || plot.histbase != 0.0
                || plot.offset != 0.0
                || plot.join
                || plot.show_last.is_some()
                || plot.display != "all"
                || plot.format.is_some()
                || plot.precision.is_some()
                || plot.force_overlay
                || plot.linestyle != "solid"
            {
                return Err(EngineError::Runtime(Diagnostic {
                    severity: DiagnosticSeverity::Error,
                    message: "plot styling or positioning is not supported by the OpineRust bridge"
                        .into(),
                    line: None,
                    column: None,
                }));
            }
            let occurrence = occurrences.entry(plot.title.as_str()).or_insert(0);
            if seen == 0 {
                plots.push(PlotSeries {
                    title: plot.title.clone(),
                    occurrence: *occurrence,
                    values: Vec::new(),
                    colors: Vec::new(),
                    has_values: false,
                });
            }
            let column = &mut plots[position];
            if column.title != plot.title || column.occurrence != *occurrence {
                return Err(EngineError::Runtime(Diagnostic {
                    severity: DiagnosticSeverity::Error,
                    message: "plot identity changed between bars".into(),
                    line: None,
                    column: None,
                }));
            }
            *occurrence += 1;
            column.has_values |= plot.series.is_finite();
            column
                .values
                .push(plot.series.is_finite().then_some(plot.series));
            column.colors.push(plot.color.as_ref().map(pack_color));
        }
        seen += 1;
    }

    for series in &mut plots {
        series.values.truncate(seen);
        series.colors.truncate(seen);
    }

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
        },
        plots,
        inputs,
        diagnostics: Vec::new(),
    })
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

    #[test]
    fn unsupported_features_are_rejected_with_a_location() {
        for source in [
            "//@version=5\nstrategy(\"s\")\n",
            "//@version=5\nindicator(\"x\")\na = request.security(\"X\", \"D\", close)\n",
            "//@version=5\nindicator(\"x\")\nlabel.new(bar_index, close, \"hi\")\n",
        ] {
            match execute(source, &bars(3), 0) {
                Err(EngineError::Compile(diagnostics)) => {
                    assert!(
                        diagnostics.iter().any(|d| d.line.is_some()),
                        "{source}: {diagnostics:?}"
                    );
                }
                other => panic!("{source} must be rejected, got {other:?}"),
            }
        }
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

    #[test]
    fn unsupported_references_and_aliases_are_rejected() {
        for name in [
            "strategy",
            "request",
            "label",
            "box",
            "table",
            "line",
            "linefill",
            "polyline",
            "fill",
            "bgcolor",
            "barcolor",
            "plotshape",
            "plotchar",
            "plotarrow",
            "plotbar",
            "plotcandle",
            "hline",
            "alert",
            "alertcondition",
            "library",
        ] {
            let source = format!("//@version=5\nindicator(\"x\")\nf = {name} \nplot(close)\n");
            let error = execute(&source, &bars(2), 0).expect_err(name);
            assert!(
                matches!(error, EngineError::Compile(ref d) if d.iter().any(|d| d.message.contains("not supported") && d.line == Some(3))),
                "{name}: {error:?}"
            );
        }
        for source in [
            "import foo/bar/1",
            "strategy (\"s\")",
            "x = request . security(\"X\", \"D\", close)",
        ] {
            assert!(
                matches!(execute(source, &bars(2), 0), Err(EngineError::Compile(ref d)) if d.iter().any(|d| d.message.contains("not supported"))),
                "{source}"
            );
        }
    }

    #[test]
    fn ambiguous_plot_declarations_are_rejected() {
        for source in [
            "indicator(\"x\")\nif bar_index >= 2\n    plot(close, title=\"x\")\n",
            "indicator(\"x\")\nif bar_index != 1\n    plot(close, title=\"same\")\nplot(open, title=\"same\")\n",
            "indicator(\"x\")\np = plot\np(close)\n",
            "indicator(\"x\")\nf() => plot(close)\nf()\n",
        ] {
            assert!(matches!(execute(source, &bars(4), 0), Err(EngineError::Compile(ref d)) if d.iter().any(|d| d.message.contains("ambiguous"))), "{source}");
        }
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
    fn non_default_plot_rendering_is_rejected() {
        for argument in [
            "linewidth=2",
            "offset=1",
            "style=plot.style_histogram",
            "display=display.none",
            "show_last=1",
        ] {
            let source = format!("indicator(\"x\")\nplot(close, {argument})\n");
            assert!(execute(&source, &bars(2), 0).is_err(), "{argument}");
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
    fn semantic_pass_does_not_need_a_system_clock() {
        // Building the sema builtins with `per_bar` is what keeps this working on
        // wasm32-unknown-unknown, where `SystemTime::now()` panics.
        let source = "//@version=5\nindicator(\"t\")\nplot(timenow)\n";
        assert!(execute(source, &bars(2), 42).is_ok());
    }
}
