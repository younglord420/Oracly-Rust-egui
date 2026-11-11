//! Map a script's `input.*` declarations onto Oracly's study settings.
//!
//! Pinecone records every declaration with its type, default, value and numeric
//! bounds, so the bridge can offer real settings instead of running every script
//! with its defaults. Two rules keep this honest:
//!
//! - A declaration the chart cannot represent is **omitted with a reason**, not
//!   guessed at. `input.time`/`input.session` have no `StudyInput` equivalent
//!   yet, and a text field the runtime cannot validate is worse than no field.
//! - Ids are derived from the declaration order, so they stay stable across runs
//!   even when titles repeat. Pinecone resolves overrides by title, so duplicate
//!   titles share one override; the bridge reports that rather than pretending
//!   each declaration is independent.

use crate::engine::{InputDefault, ScriptInput};
use Oracly_charts::studies::{InputKind, InputValue, StudyInput};

/// Outcome of mapping a run's declarations.
#[derive(Clone, Debug, Default)]
pub struct MappedInputs {
    /// Settings the host can edit.
    pub inputs: Vec<StudyInput>,
    /// Declaration title for each entry of [`Self::inputs`], same order.
    ///
    /// Pinecone resolves overrides by title, so the title has to travel with the
    /// setting; it is not recoverable from [`StudyInput`], whose id is positional.
    pub titles: Vec<String>,
    /// Declarations that could not be represented, with the reason.
    pub skipped: Vec<String>,
}

/// Override one input by its id, as a host would after the user edits a field.
pub fn apply_override(input: &mut StudyInput, value: InputValue) -> Result<(), String> {
    if !input.accepts(&value) {
        return Err(format!(
            "`{}` does not accept that value for {}",
            input.id, input.label
        ));
    }
    input.value = value;
    Ok(())
}

/// Translate an edited setting into the value Pinecone's `input.*` reads.
///
/// A colour has no byte-level override the runtime will apply, so it returns
/// `None` and the caller keeps the script's declared colour. A choice is the
/// option's text, not its index: `input.string` compares the override against
/// the option strings.
pub fn to_engine_value(input: &StudyInput, value: &InputValue) -> Option<pine_core::InputValue> {
    match value {
        InputValue::Length(value) => Some(pine_core::InputValue::Int(*value as i64)),
        InputValue::Integer(value) => Some(pine_core::InputValue::Int(i64::from(*value))),
        InputValue::Number(value) if value.is_finite() => {
            Some(pine_core::InputValue::Float(*value))
        }
        InputValue::Toggle(value) => Some(pine_core::InputValue::Bool(*value)),
        InputValue::Text(value) | InputValue::Symbol(value) => {
            Some(pine_core::InputValue::Str(value.clone()))
        }
        InputValue::Choice(index) => {
            let InputKind::Choice(choices) = &input.kind else {
                return None;
            };
            choices
                .get(*index)
                .map(|choice| pine_core::InputValue::Str(choice.clone()))
        }
        InputValue::Timeframe(Some(timeframe)) => {
            pine_period(*timeframe).map(pine_core::InputValue::Str)
        }
        InputValue::Timeframe(None) => Some(pine_core::InputValue::Str(String::new())),
        InputValue::Color(_) => None,
        _ => None,
    }
}

/// The Pine period string for a chart interval the bridge offered as a setting.
///
/// Intervals Pine writes as a bare number of minutes stay bare (`"15"`, `"60"`).
/// An interval with no such spelling returns `None`, so the edit is refused
/// instead of being sent as a period the script would misread.
fn pine_period(timeframe: Oracly_charts::model::Timeframe) -> Option<String> {
    use Oracly_charts::model::Timeframe as T;
    Some(
        match timeframe {
            T::Sec1 => "1S",
            T::Sec2 => "2S",
            T::Sec5 => "5S",
            T::Sec10 => "10S",
            T::Sec30 => "30S",
            T::Min1 => "1",
            T::Min5 => "5",
            T::Min15 => "15",
            T::Min30 => "30",
            T::Hour1 => "60",
            T::Hour4 => "240",
            T::Day1 => "1D",
            T::Week1 => "1W",
            T::Month1 => "1M",
            _ => return None,
        }
        .to_string(),
    )
}

/// A chart interval for a Pine period, when one of the chart's presets is that
/// period exactly. `"3"` is three minutes and the chart has no three-minute
/// preset, so it stays text rather than being rounded to five.
fn chart_timeframe(period: &str) -> Option<Oracly_charts::model::Timeframe> {
    use Oracly_charts::model::Timeframe as T;
    Some(match period {
        "1S" => T::Sec1,
        "2S" => T::Sec2,
        "5S" => T::Sec5,
        "10S" => T::Sec10,
        "30S" => T::Sec30,
        "1" => T::Min1,
        "5" => T::Min5,
        "15" => T::Min15,
        "30" => T::Min30,
        "60" => T::Hour1,
        "240" => T::Hour4,
        "D" | "1D" => T::Day1,
        "W" | "1W" => T::Week1,
        "M" | "1M" => T::Month1,
        _ => return None,
    })
}

/// Build the settings schema for a run's declarations.
pub fn map_inputs(declared: &[ScriptInput]) -> MappedInputs {
    let mut mapped = MappedInputs::default();
    let mut seen: Vec<(&str, usize)> = Vec::new();

    for (position, input) in declared.iter().enumerate() {
        let id = format!("pine_input_{position}");
        let label = if input.title.is_empty() {
            format!("Input {}", position + 1)
        } else {
            input.title.clone()
        };
        let Some((value, kind)) = value_and_kind(input) else {
            mapped.skipped.push(format!(
                "`input.{}` (\"{label}\") has no chart setting: unsupported type",
                input.kind
            ));
            continue;
        };
        if !kind_accepts(&kind, &value) {
            mapped.skipped.push(format!(
                "`input.{}` (\"{label}\") declares bounds its value violates",
                input.kind
            ));
            continue;
        }

        // Pinecone resolves an override by title. Two declarations with one title
        // therefore cannot be edited independently; say so instead of shipping two
        // fields that silently shadow each other.
        if let Some((_, first)) = seen.iter().find(|(title, _)| *title == input.title) {
            mapped.skipped.push(format!(
                "`input.{}` (\"{label}\") repeats the title of input {}, which overrides them together",
                input.kind,
                first + 1
            ));
            continue;
        }
        seen.push((input.title.as_str(), position));

        let default = setting(input, &input.default)
            .filter(|(_, default_kind)| same_kind(default_kind, &kind))
            .map(|(default, _)| default)
            .unwrap_or_else(|| value.clone());
        mapped.inputs.push(
            StudyInput::new(id, label, value, kind)
                .with_default(default)
                .with_group(&input.group),
        );
        mapped.titles.push(input.title.clone());
    }
    mapped
}

fn same_kind(left: &InputKind, right: &InputKind) -> bool {
    match (left, right) {
        (InputKind::Length(a, b), InputKind::Length(c, d)) => a == c && b == d,
        (InputKind::Integer(a, b), InputKind::Integer(c, d)) => a == c && b == d,
        (InputKind::Number(a, b), InputKind::Number(c, d)) => a == c && b == d,
        (InputKind::Choice(a), InputKind::Choice(b)) => a == b,
        (InputKind::Toggle, InputKind::Toggle)
        | (InputKind::Source, InputKind::Source)
        | (InputKind::Color, InputKind::Color)
        | (InputKind::Timeframe, InputKind::Timeframe)
        | (InputKind::Symbol, InputKind::Symbol)
        | (InputKind::Text, InputKind::Text) => true,
        _ => false,
    }
}

fn value_and_kind(input: &ScriptInput) -> Option<(InputValue, InputKind)> {
    setting(input, &input.value)
}

fn setting(input: &ScriptInput, value: &InputDefault) -> Option<(InputValue, InputKind)> {
    match input.kind.as_str() {
        "int" | "integer" => {
            let value = match value {
                InputDefault::Int(value) => *value,
                InputDefault::Float(value) => *value as i64,
                _ => return None,
            };
            let lower = input.min.map_or(0, |min| min.max(0.0) as usize);
            let upper = input.max.map_or(usize::MAX, |max| max.max(0.0) as usize);
            Some((
                InputValue::Length(value.max(0) as usize),
                InputKind::Length(lower, upper),
            ))
        }
        "float" => {
            let value = match value {
                InputDefault::Float(value) => *value,
                InputDefault::Int(value) => *value as f64,
                _ => return None,
            };
            let lower = input.min.unwrap_or(-f64::MAX);
            let upper = input.max.unwrap_or(f64::MAX);
            Some((InputValue::Number(value), InputKind::Number(lower, upper)))
        }
        "bool" => {
            let InputDefault::Bool(value) = value else {
                return None;
            };
            Some((InputValue::Toggle(*value), InputKind::Toggle))
        }
        "string" | "text_area" => {
            let InputDefault::Text(text) = value else {
                return None;
            };
            if input.options.is_empty() {
                return Some((InputValue::Text(text.clone()), InputKind::Text));
            }
            let index = input.options.iter().position(|option| option == text)?;
            Some((
                InputValue::Choice(index),
                InputKind::Choice(input.options.clone()),
            ))
        }
        "timeframe" => {
            let InputDefault::Text(period) = value else {
                return None;
            };
            if period.is_empty() {
                return Some((InputValue::Timeframe(None), InputKind::Timeframe));
            }
            match chart_timeframe(period) {
                Some(timeframe) => {
                    Some((InputValue::Timeframe(Some(timeframe)), InputKind::Timeframe))
                }
                // A period the chart has no preset for stays editable text, and
                // the override is that same string. Rounding it would lie.
                None => Some((InputValue::Text(period.clone()), InputKind::Text)),
            }
        }
        "symbol" => {
            let InputDefault::Text(symbol) = value else {
                return None;
            };
            Some((InputValue::Symbol(symbol.clone()), InputKind::Symbol))
        }
        "color" => {
            let InputDefault::Color([r, g, b, t]) = value else {
                return None;
            };
            Some((
                InputValue::Color(color_to_egui(*r, *g, *b, *t)),
                InputKind::Color,
            ))
        }
        // `source` returns the series itself and ignores a later override, so a
        // widget would edit a value the script never reads. `session` and `time`
        // have no chart widget that round-trips their text.
        _ => None,
    }
}

/// Pinecone carries transparency 0–100; Oracly wants premultiplied-free RGBA.
fn color_to_egui(r: u8, g: u8, b: u8, t: u8) -> egui::Color32 {
    let alpha = 255u8.saturating_sub((u16::from(t.min(100)) * 255 / 100) as u8);
    egui::Color32::from_rgba_unmultiplied(r, g, b, alpha)
}

/// Mirror of [`StudyInput::accepts`] for a kind built here.
fn kind_accepts(kind: &InputKind, value: &InputValue) -> bool {
    StudyInput::new("probe", "probe", value.clone(), kind.clone()).accepts(value)
}

/// Group helper: `StudyInput` has no group field, so keep the label honest.
trait WithGroup {
    fn with_group(self, group: &str) -> Self;
}

impl WithGroup for StudyInput {
    fn with_group(mut self, group: &str) -> Self {
        if !group.is_empty() {
            // Oracly has no group concept; prefixing keeps the information visible
            // instead of dropping it silently.
            self.label = format!("{group} · {}", self.label);
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn declared(kind: &str, title: &str, default: InputDefault) -> ScriptInput {
        ScriptInput {
            kind: kind.to_owned(),
            title: title.to_owned(),
            group: String::new(),
            default: default.clone(),
            value: default,
            min: None,
            max: None,
            step: None,
            options: Vec::new(),
        }
    }

    #[test]
    fn numeric_declarations_become_length_inputs() {
        let mapped = map_inputs(&[declared("int", "Length", InputDefault::Int(14))]);
        assert_eq!(mapped.inputs.len(), 1);
        assert_eq!(mapped.inputs[0].label, "Length");
        assert_eq!(mapped.inputs[0].value, InputValue::Length(14));
        assert_eq!(mapped.inputs[0].id, "pine_input_0");
    }

    #[test]
    fn declared_bounds_reach_the_schema() {
        let mut input = declared("int", "Length", InputDefault::Int(14));
        input.min = Some(1.0);
        input.max = Some(200.0);
        let mapped = map_inputs(&[input]);
        assert!(
            matches!(mapped.inputs[0].kind, InputKind::Length(1, 200)),
            "{:?}",
            mapped.inputs[0].kind
        );
    }

    #[test]
    fn unsupported_types_are_reported_not_guessed() {
        for kind in ["source", "session", "time"] {
            let mapped = map_inputs(&[declared(kind, "X", InputDefault::Int(1))]);
            assert!(mapped.inputs.is_empty(), "{kind}");
            assert_eq!(mapped.skipped.len(), 1, "{kind}");
            assert!(
                mapped.skipped[0].contains("unsupported type"),
                "{:?}",
                mapped.skipped
            );
        }
    }

    #[test]
    fn a_value_outside_its_bounds_is_reported() {
        let mut input = declared("int", "Length", InputDefault::Int(500));
        input.min = Some(1.0);
        input.max = Some(10.0);
        let mapped = map_inputs(&[input]);
        assert!(mapped.inputs.is_empty());
        assert!(mapped.skipped[0].contains("bounds"), "{:?}", mapped.skipped);
    }

    #[test]
    fn duplicate_titles_are_flagged_because_overrides_share_them() {
        let mapped = map_inputs(&[
            declared("int", "Same", InputDefault::Int(1)),
            declared("int", "Same", InputDefault::Int(2)),
        ]);
        assert_eq!(mapped.inputs.len(), 1);
        assert!(mapped.skipped[0].contains("repeats the title"));
    }

    #[test]
    fn an_override_must_satisfy_the_schema() {
        let mut input = StudyInput::new(
            "pine_input_0",
            "Length",
            InputValue::Length(14),
            InputKind::Length(1, 20),
        );
        assert!(apply_override(&mut input, InputValue::Length(5)).is_ok());
        assert_eq!(input.value, InputValue::Length(5));
        assert!(apply_override(&mut input, InputValue::Length(50)).is_err());
        assert_eq!(
            input.value,
            InputValue::Length(5),
            "a refused value changes nothing"
        );
    }

    #[test]
    fn transparency_becomes_alpha() {
        assert_eq!(color_to_egui(255, 0, 0, 0).a(), 255);
        assert_eq!(color_to_egui(255, 0, 0, 100).a(), 0);
    }

    #[test]
    fn a_preset_timeframe_becomes_the_chart_interval() {
        let mapped = map_inputs(&[declared(
            "timeframe",
            "HTF",
            InputDefault::Text("15".into()),
        )]);
        assert_eq!(mapped.inputs.len(), 1, "{:?}", mapped.skipped);
        assert_eq!(
            mapped.inputs[0].value,
            InputValue::Timeframe(Some(Oracly_charts::model::Timeframe::Min15))
        );
        assert!(matches!(mapped.inputs[0].kind, InputKind::Timeframe));
    }

    /// Three minutes is a real Pine period and not a chart preset. It stays text
    /// so the script receives `"3"`, not a rounded `"5"`.
    #[test]
    fn an_unpreset_timeframe_stays_text() {
        let mapped = map_inputs(&[declared("timeframe", "HTF", InputDefault::Text("3".into()))]);
        assert_eq!(mapped.inputs[0].value, InputValue::Text("3".into()));
        assert!(matches!(mapped.inputs[0].kind, InputKind::Text));
    }

    #[test]
    fn a_string_with_literal_options_becomes_a_choice() {
        let mut input = declared("string", "Mode", InputDefault::Text("ema".into()));
        input.options = vec!["ema".into(), "sma".into()];
        let mapped = map_inputs(&[input]);
        assert_eq!(mapped.inputs[0].value, InputValue::Choice(0));
        let InputKind::Choice(choices) = &mapped.inputs[0].kind else {
            panic!("expected a choice");
        };
        assert_eq!(choices, &["ema".to_string(), "sma".to_string()]);
        let engine = to_engine_value(&mapped.inputs[0], &InputValue::Choice(1)).expect("sent");
        assert_eq!(engine, pine_core::InputValue::Str("sma".into()));
    }

    #[test]
    fn a_symbol_input_round_trips_as_text() {
        let mapped = map_inputs(&[declared(
            "symbol",
            "Ticker",
            InputDefault::Text("AAPL".into()),
        )]);
        assert_eq!(mapped.inputs[0].value, InputValue::Symbol("AAPL".into()));
        let engine =
            to_engine_value(&mapped.inputs[0], &InputValue::Symbol("MSFT".into())).expect("sent");
        assert_eq!(engine, pine_core::InputValue::Str("MSFT".into()));
    }

    #[test]
    fn a_source_input_is_still_skipped() {
        let mapped = map_inputs(&[declared(
            "source",
            "Src",
            InputDefault::Text("close".into()),
        )]);
        assert!(mapped.inputs.is_empty());
        assert!(mapped.skipped[0].contains("unsupported type"));
    }
}
