//! Pine Script built-in reference documentation and signatures.
//!
//! Provides reference information (function signatures, return types, descriptions,
//! and parameter lists) for hover tooltips, autocompletion, and signature help in the
//! OpineRust code editor.

/// Information about a parameter in a built-in function.
#[derive(Clone, Debug, PartialEq)]
pub struct ParamDoc {
    pub name: &'static str,
    pub type_info: &'static str,
    pub doc: &'static str,
    pub optional: bool,
}

/// Category of a Pine Script built-in symbol.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SymbolKind {
    Function,
    Variable,
    Constant,
    Keyword,
    Namespace,
}

impl SymbolKind {
    pub fn badge(&self) -> &'static str {
        match self {
            SymbolKind::Function => "ƒ",
            SymbolKind::Variable => "v",
            SymbolKind::Constant => "c",
            SymbolKind::Keyword => "k",
            SymbolKind::Namespace => "m",
        }
    }
}

/// Documentation entry for a built-in symbol or function.
#[derive(Clone, Debug, PartialEq)]
pub struct BuiltinDoc {
    pub name: &'static str,
    pub kind: SymbolKind,
    pub signature: &'static str,
    pub return_type: &'static str,
    pub summary: &'static str,
    pub params: &'static [ParamDoc],
    pub example: Option<&'static str>,
}

/// Lookup a built-in symbol by its exact name or namespace qualified name (e.g., "ta.sma" or "close").
pub fn find_builtin(name: &str) -> Option<&'static BuiltinDoc> {
    BUILTIN_REGISTRY.iter().find(|entry| entry.name == name)
}

/// Find completions starting with a given prefix (e.g. "ta." or "inp").
pub fn suggest_builtins(prefix: &str) -> Vec<&'static BuiltinDoc> {
    if prefix.is_empty() {
        return BUILTIN_REGISTRY.iter().collect();
    }
    BUILTIN_REGISTRY
        .iter()
        .filter(|entry| entry.name.starts_with(prefix))
        .collect()
}

static BUILTIN_REGISTRY: &[BuiltinDoc] = &[
    // --- Core Variables ---
    BuiltinDoc {
        name: "open",
        kind: SymbolKind::Variable,
        signature: "open: series float",
        return_type: "series float",
        summary: "Current bar open price.",
        params: &[],
        example: Some("plot(open)"),
    },
    BuiltinDoc {
        name: "high",
        kind: SymbolKind::Variable,
        signature: "high: series float",
        return_type: "series float",
        summary: "Current bar high price.",
        params: &[],
        example: Some("plot(high)"),
    },
    BuiltinDoc {
        name: "low",
        kind: SymbolKind::Variable,
        signature: "low: series float",
        return_type: "series float",
        summary: "Current bar low price.",
        params: &[],
        example: Some("plot(low)"),
    },
    BuiltinDoc {
        name: "close",
        kind: SymbolKind::Variable,
        signature: "close: series float",
        return_type: "series float",
        summary: "Current bar close price (or latest price of unconfirmed realtime bar).",
        params: &[],
        example: Some("plot(close)"),
    },
    BuiltinDoc {
        name: "volume",
        kind: SymbolKind::Variable,
        signature: "volume: series float",
        return_type: "series float",
        summary: "Current bar volume.",
        params: &[],
        example: Some("plot(volume)"),
    },
    BuiltinDoc {
        name: "time",
        kind: SymbolKind::Variable,
        signature: "time: series int",
        return_type: "series int",
        summary: "Current bar opening time in UNIX milliseconds.",
        params: &[],
        example: None,
    },
    BuiltinDoc {
        name: "timenow",
        kind: SymbolKind::Variable,
        signature: "timenow: simple int",
        return_type: "simple int",
        summary: "Current time in UNIX milliseconds (supplied by chart host).",
        params: &[],
        example: None,
    },
    BuiltinDoc {
        name: "bar_index",
        kind: SymbolKind::Variable,
        signature: "bar_index: series int",
        return_type: "series int",
        summary: "Current bar index, 0-based from first bar.",
        params: &[],
        example: Some("plot(bar_index)"),
    },
    BuiltinDoc {
        name: "barstate.isconfirmed",
        kind: SymbolKind::Variable,
        signature: "barstate.isconfirmed: series bool",
        return_type: "series bool",
        summary: "Returns true if calculating on the last closing tick of the current bar.",
        params: &[],
        example: None,
    },
    BuiltinDoc {
        name: "barstate.isfirst",
        kind: SymbolKind::Variable,
        signature: "barstate.isfirst: series bool",
        return_type: "series bool",
        summary: "Returns true if current bar is the first bar in the dataset.",
        params: &[],
        example: None,
    },
    BuiltinDoc {
        name: "barstate.ishistory",
        kind: SymbolKind::Variable,
        signature: "barstate.ishistory: series bool",
        return_type: "series bool",
        summary: "Returns true if current bar is a historical confirmed bar.",
        params: &[],
        example: None,
    },
    BuiltinDoc {
        name: "barstate.islast",
        kind: SymbolKind::Variable,
        signature: "barstate.islast: series bool",
        return_type: "series bool",
        summary: "Returns true if current bar is the last bar in the dataset.",
        params: &[],
        example: None,
    },
    BuiltinDoc {
        name: "barstate.isnew",
        kind: SymbolKind::Variable,
        signature: "barstate.isnew: series bool",
        return_type: "series bool",
        summary: "Returns true if calculating on the first tick of a new bar.",
        params: &[],
        example: None,
    },
    BuiltinDoc {
        name: "barstate.isrealtime",
        kind: SymbolKind::Variable,
        signature: "barstate.isrealtime: series bool",
        return_type: "series bool",
        summary: "Returns true if current bar is still forming in real time.",
        params: &[],
        example: None,
    },
    // --- Declarations & Core Functions ---
    BuiltinDoc {
        name: "indicator",
        kind: SymbolKind::Function,
        signature: "indicator(title, shorttitle, overlay, format, precision, max_bars_back)",
        return_type: "void",
        summary: "Declares script properties for an indicator study.",
        params: &[
            ParamDoc {
                name: "title",
                type_info: "const string",
                doc: "Display title of the indicator.",
                optional: false,
            },
            ParamDoc {
                name: "shorttitle",
                type_info: "const string",
                doc: "Short title displayed in chart legend.",
                optional: true,
            },
            ParamDoc {
                name: "overlay",
                type_info: "const bool",
                doc: "If true, plots over main price chart. If false, plots in a separate pane.",
                optional: true,
            },
        ],
        example: Some("indicator(\"My Script\", overlay = true)"),
    },
    BuiltinDoc {
        name: "plot",
        kind: SymbolKind::Function,
        signature: "plot(series, title, color, linewidth, style, trackprice, histbase, offset, join, editable, show_last, display)",
        return_type: "plot",
        summary: "Plots a curve or values on the chart.",
        params: &[
            ParamDoc {
                name: "series",
                type_info: "series float",
                doc: "Series of data to plot.",
                optional: false,
            },
            ParamDoc {
                name: "title",
                type_info: "const string",
                doc: "Title of the plot.",
                optional: true,
            },
            ParamDoc {
                name: "color",
                type_info: "series color",
                doc: "Color of the plot curve.",
                optional: true,
            },
        ],
        example: Some("plot(close, title = \"Close\", color = color.blue)"),
    },
    // --- input.* family ---
    BuiltinDoc {
        name: "input.int",
        kind: SymbolKind::Function,
        signature: "input.int(defval, title, minval, maxval, step, tooltip, group, inline)",
        return_type: "input int",
        summary: "Adds an integer input setting to the study settings dialog.",
        params: &[
            ParamDoc {
                name: "defval",
                type_info: "simple int",
                doc: "Default value.",
                optional: false,
            },
            ParamDoc {
                name: "title",
                type_info: "const string",
                doc: "Label displayed in the settings dialog.",
                optional: true,
            },
            ParamDoc {
                name: "minval",
                type_info: "const int",
                doc: "Minimum allowed value.",
                optional: true,
            },
            ParamDoc {
                name: "maxval",
                type_info: "const int",
                doc: "Maximum allowed value.",
                optional: true,
            },
        ],
        example: Some("len = input.int(14, \"Length\", minval = 1, maxval = 200)"),
    },
    BuiltinDoc {
        name: "input.float",
        kind: SymbolKind::Function,
        signature: "input.float(defval, title, minval, maxval, step, tooltip, group, inline)",
        return_type: "input float",
        summary: "Adds a floating-point input setting to the study settings dialog.",
        params: &[
            ParamDoc {
                name: "defval",
                type_info: "simple float",
                doc: "Default value.",
                optional: false,
            },
            ParamDoc {
                name: "title",
                type_info: "const string",
                doc: "Label displayed in the settings dialog.",
                optional: true,
            },
            ParamDoc {
                name: "minval",
                type_info: "const float",
                doc: "Minimum allowed value.",
                optional: true,
            },
            ParamDoc {
                name: "maxval",
                type_info: "const float",
                doc: "Maximum allowed value.",
                optional: true,
            },
        ],
        example: Some("mult = input.float(2.0, \"Multiplier\", minval = 0.1)"),
    },
    BuiltinDoc {
        name: "input.bool",
        kind: SymbolKind::Function,
        signature: "input.bool(defval, title, tooltip, group, inline)",
        return_type: "input bool",
        summary: "Adds a checkbox boolean toggle input setting to the study settings dialog.",
        params: &[
            ParamDoc {
                name: "defval",
                type_info: "simple bool",
                doc: "Default boolean value.",
                optional: false,
            },
            ParamDoc {
                name: "title",
                type_info: "const string",
                doc: "Label displayed in the settings dialog.",
                optional: true,
            },
        ],
        example: Some("show_ma = input.bool(true, \"Show Moving Average\")"),
    },
    BuiltinDoc {
        name: "input.string",
        kind: SymbolKind::Function,
        signature: "input.string(defval, title, options, tooltip, group, inline)",
        return_type: "input string",
        summary: "Adds a string text input or dropdown select setting to the study settings dialog.",
        params: &[
            ParamDoc {
                name: "defval",
                type_info: "simple string",
                doc: "Default string value.",
                optional: false,
            },
            ParamDoc {
                name: "title",
                type_info: "const string",
                doc: "Label displayed in the settings dialog.",
                optional: true,
            },
        ],
        example: Some("txt = input.string(\"Default\", \"Label\")"),
    },
    BuiltinDoc {
        name: "input.color",
        kind: SymbolKind::Function,
        signature: "input.color(defval, title, tooltip, group, inline)",
        return_type: "input color",
        summary: "Adds a color picker input setting to the study settings dialog.",
        params: &[
            ParamDoc {
                name: "defval",
                type_info: "series color",
                doc: "Default color value.",
                optional: false,
            },
            ParamDoc {
                name: "title",
                type_info: "const string",
                doc: "Label displayed in the settings dialog.",
                optional: true,
            },
        ],
        example: Some("col = input.color(color.blue, \"Line Color\")"),
    },
    // --- ta.* technical analysis builtins ---
    BuiltinDoc {
        name: "ta.sma",
        kind: SymbolKind::Function,
        signature: "ta.sma(source, length) -> series float",
        return_type: "series float",
        summary: "The simple moving average (SMA) calculates the average of the last `length` values of `source`.",
        params: &[
            ParamDoc {
                name: "source",
                type_info: "series float",
                doc: "Series of values to process.",
                optional: false,
            },
            ParamDoc {
                name: "length",
                type_info: "simple int",
                doc: "Number of bars (length).",
                optional: false,
            },
        ],
        example: Some("fast_ma = ta.sma(close, 14)"),
    },
    BuiltinDoc {
        name: "ta.ema",
        kind: SymbolKind::Function,
        signature: "ta.ema(source, length) -> series float",
        return_type: "series float",
        summary: "The exponential moving average (EMA) calculates the exponentially weighted average of `source`.",
        params: &[
            ParamDoc {
                name: "source",
                type_info: "series float",
                doc: "Series of values to process.",
                optional: false,
            },
            ParamDoc {
                name: "length",
                type_info: "simple int",
                doc: "Number of bars (length).",
                optional: false,
            },
        ],
        example: Some("ema20 = ta.ema(close, 20)"),
    },
    BuiltinDoc {
        name: "ta.rsi",
        kind: SymbolKind::Function,
        signature: "ta.rsi(source, length) -> series float",
        return_type: "series float",
        summary: "Relative Strength Index (RSI) measures price momentum and oscillates between 0 and 100.",
        params: &[
            ParamDoc {
                name: "source",
                type_info: "series float",
                doc: "Series of values to process.",
                optional: false,
            },
            ParamDoc {
                name: "length",
                type_info: "simple int",
                doc: "RSI lookback period.",
                optional: false,
            },
        ],
        example: Some("rsi14 = ta.rsi(close, 14)"),
    },
    BuiltinDoc {
        name: "ta.macd",
        kind: SymbolKind::Function,
        signature: "ta.macd(source, fastlen, slowlen, siglen) -> [series float, series float, series float]",
        return_type: "tuple [macd, signal, hist]",
        summary: "Moving Average Convergence Divergence (MACD) trend and momentum indicator.",
        params: &[
            ParamDoc {
                name: "source",
                type_info: "series float",
                doc: "Series of values to process.",
                optional: false,
            },
            ParamDoc {
                name: "fastlen",
                type_info: "simple int",
                doc: "Fast EMA length.",
                optional: false,
            },
            ParamDoc {
                name: "slowlen",
                type_info: "simple int",
                doc: "Slow EMA length.",
                optional: false,
            },
            ParamDoc {
                name: "siglen",
                type_info: "simple int",
                doc: "Signal smoothing length.",
                optional: false,
            },
        ],
        example: Some("[macdLine, signalLine, histLine] = ta.macd(close, 12, 26, 9)"),
    },
    BuiltinDoc {
        name: "ta.atr",
        kind: SymbolKind::Function,
        signature: "ta.atr(length) -> series float",
        return_type: "series float",
        summary: "Average True Range indicator for market volatility.",
        params: &[ParamDoc {
            name: "length",
            type_info: "simple int",
            doc: "ATR lookback period.",
            optional: false,
        }],
        example: Some("volatility = ta.atr(14)"),
    },
    BuiltinDoc {
        name: "ta.crossover",
        kind: SymbolKind::Function,
        signature: "ta.crossover(source1, source2) -> series bool",
        return_type: "series bool",
        summary: "Returns true if `source1` crossed over `source2` on the current bar.",
        params: &[
            ParamDoc {
                name: "source1",
                type_info: "series float",
                doc: "First series.",
                optional: false,
            },
            ParamDoc {
                name: "source2",
                type_info: "series float",
                doc: "Second series.",
                optional: false,
            },
        ],
        example: Some("bullish = ta.crossover(ta.sma(close, 10), ta.sma(close, 50))"),
    },
    BuiltinDoc {
        name: "ta.crossunder",
        kind: SymbolKind::Function,
        signature: "ta.crossunder(source1, source2) -> series bool",
        return_type: "series bool",
        summary: "Returns true if `source1` crossed under `source2` on the current bar.",
        params: &[
            ParamDoc {
                name: "source1",
                type_info: "series float",
                doc: "First series.",
                optional: false,
            },
            ParamDoc {
                name: "source2",
                type_info: "series float",
                doc: "Second series.",
                optional: false,
            },
        ],
        example: Some("bearish = ta.crossunder(ta.sma(close, 10), ta.sma(close, 50))"),
    },
    BuiltinDoc {
        name: "ta.highest",
        kind: SymbolKind::Function,
        signature: "ta.highest(source, length) -> series float",
        return_type: "series float",
        summary: "Highest value for a given number of bars back.",
        params: &[
            ParamDoc {
                name: "source",
                type_info: "series float",
                doc: "Series to evaluate.",
                optional: false,
            },
            ParamDoc {
                name: "length",
                type_info: "simple int",
                doc: "Lookback length.",
                optional: false,
            },
        ],
        example: Some("high_20 = ta.highest(high, 20)"),
    },
    BuiltinDoc {
        name: "ta.lowest",
        kind: SymbolKind::Function,
        signature: "ta.lowest(source, length) -> series float",
        return_type: "series float",
        summary: "Lowest value for a given number of bars back.",
        params: &[
            ParamDoc {
                name: "source",
                type_info: "series float",
                doc: "Series to evaluate.",
                optional: false,
            },
            ParamDoc {
                name: "length",
                type_info: "simple int",
                doc: "Lookback length.",
                optional: false,
            },
        ],
        example: Some("low_20 = ta.lowest(low, 20)"),
    },
    // --- math.* builtins ---
    BuiltinDoc {
        name: "math.abs",
        kind: SymbolKind::Function,
        signature: "math.abs(number) -> float",
        return_type: "float",
        summary: "Absolute value of `number`.",
        params: &[ParamDoc {
            name: "number",
            type_info: "series float",
            doc: "Value to process.",
            optional: false,
        }],
        example: Some("diff = math.abs(close - open)"),
    },
    BuiltinDoc {
        name: "math.max",
        kind: SymbolKind::Function,
        signature: "math.max(arg1, arg2, ...) -> float",
        return_type: "float",
        summary: "Returns the greatest of given values.",
        params: &[
            ParamDoc {
                name: "arg1",
                type_info: "series float",
                doc: "First value.",
                optional: false,
            },
            ParamDoc {
                name: "arg2",
                type_info: "series float",
                doc: "Second value.",
                optional: false,
            },
        ],
        example: Some("m = math.max(high, close)"),
    },
    BuiltinDoc {
        name: "math.min",
        kind: SymbolKind::Function,
        signature: "math.min(arg1, arg2, ...) -> float",
        return_type: "float",
        summary: "Returns the smallest of given values.",
        params: &[
            ParamDoc {
                name: "arg1",
                type_info: "series float",
                doc: "First value.",
                optional: false,
            },
            ParamDoc {
                name: "arg2",
                type_info: "series float",
                doc: "Second value.",
                optional: false,
            },
        ],
        example: Some("m = math.min(low, open)"),
    },
    BuiltinDoc {
        name: "math.round",
        kind: SymbolKind::Function,
        signature: "math.round(number, precision) -> float",
        return_type: "float",
        summary: "Rounds a decimal value to nearest integer or specified decimal precision.",
        params: &[
            ParamDoc {
                name: "number",
                type_info: "series float",
                doc: "Value to round.",
                optional: false,
            },
            ParamDoc {
                name: "precision",
                type_info: "simple int",
                doc: "Decimal places.",
                optional: true,
            },
        ],
        example: Some("rounded = math.round(close, 2)"),
    },
    // --- color.* builtins ---
    BuiltinDoc {
        name: "color.rgb",
        kind: SymbolKind::Function,
        signature: "color.rgb(red, green, blue, transp) -> color",
        return_type: "color",
        summary: "Constructs a color from Red, Green, Blue, and transparency values.",
        params: &[
            ParamDoc {
                name: "red",
                type_info: "series int",
                doc: "Red component (0 to 255).",
                optional: false,
            },
            ParamDoc {
                name: "green",
                type_info: "series int",
                doc: "Green component (0 to 255).",
                optional: false,
            },
            ParamDoc {
                name: "blue",
                type_info: "series int",
                doc: "Blue component (0 to 255).",
                optional: false,
            },
            ParamDoc {
                name: "transp",
                type_info: "series int",
                doc: "Transparency percentage (0 to 100).",
                optional: true,
            },
        ],
        example: Some("custom_col = color.rgb(41, 98, 255, 20)"),
    },
    BuiltinDoc {
        name: "color.new",
        kind: SymbolKind::Function,
        signature: "color.new(color, transp) -> color",
        return_type: "color",
        summary: "Applies transparency (0-100) to an existing color.",
        params: &[
            ParamDoc {
                name: "color",
                type_info: "series color",
                doc: "Base color.",
                optional: false,
            },
            ParamDoc {
                name: "transp",
                type_info: "series int",
                doc: "Transparency percentage (0 to 100).",
                optional: false,
            },
        ],
        example: Some("transparent_blue = color.new(color.blue, 50)"),
    },
    BuiltinDoc {
        name: "color.blue",
        kind: SymbolKind::Constant,
        signature: "color.blue: const color = #2962FF",
        return_type: "const color",
        summary: "Predefined blue color.",
        params: &[],
        example: None,
    },
    BuiltinDoc {
        name: "color.red",
        kind: SymbolKind::Constant,
        signature: "color.red: const color = #F23645",
        return_type: "const color",
        summary: "Predefined red color.",
        params: &[],
        example: None,
    },
    BuiltinDoc {
        name: "color.green",
        kind: SymbolKind::Constant,
        signature: "color.green: const color = #089981",
        return_type: "const color",
        summary: "Predefined green color.",
        params: &[],
        example: None,
    },
    BuiltinDoc {
        name: "color.orange",
        kind: SymbolKind::Constant,
        signature: "color.orange: const color = #FF9800",
        return_type: "const color",
        summary: "Predefined orange color.",
        params: &[],
        example: None,
    },
    BuiltinDoc {
        name: "color.white",
        kind: SymbolKind::Constant,
        signature: "color.white: const color = #FFFFFF",
        return_type: "const color",
        summary: "Predefined white color.",
        params: &[],
        example: None,
    },
    BuiltinDoc {
        name: "color.black",
        kind: SymbolKind::Constant,
        signature: "color.black: const color = #000000",
        return_type: "const color",
        summary: "Predefined black color.",
        params: &[],
        example: None,
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_builtin() {
        let sma = find_builtin("ta.sma").expect("ta.sma should exist");
        assert_eq!(sma.kind, SymbolKind::Function);
        assert_eq!(sma.params.len(), 2);
        assert!(sma.summary.contains("moving average"));

        let col = find_builtin("color.green").expect("color.green should exist");
        assert_eq!(col.kind, SymbolKind::Constant);

        assert!(find_builtin("non_existent").is_none());
    }

    #[test]
    fn test_suggest_builtins() {
        let ta_suggestions = suggest_builtins("ta.");
        assert!(ta_suggestions.iter().any(|b| b.name == "ta.sma"));
        assert!(ta_suggestions.iter().any(|b| b.name == "ta.rsi"));
        assert!(!ta_suggestions.iter().any(|b| b.name == "math.abs"));

        let math_suggestions = suggest_builtins("math.");
        assert!(math_suggestions.iter().any(|b| b.name == "math.max"));
    }
}
