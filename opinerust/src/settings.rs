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
/// Returns `None` for kinds the runtime cannot receive (a colour or a symbol has
/// no byte-level override here), which keeps the caller from writing an override
/// the script would misread.
pub fn to_engine_value(value: &InputValue) -> Option<pine_core::InputValue> {
    match value {
        InputValue::Length(value) => Some(pine_core::InputValue::Int(*value as i64)),
        InputValue::Integer(value) => Some(pine_core::InputValue::Int(i64::from(*value))),
        InputValue::Number(value) if value.is_finite() => {
            Some(pine_core::InputValue::Float(*value))
        }
        InputValue::Toggle(value) => Some(pine_core::InputValue::Bool(*value)),
        InputValue::Text(value) => Some(pine_core::InputValue::Str(value.clone())),
        InputValue::Choice(index) => Some(pine_core::InputValue::Int(*index as i64)),
        _ => None,
    }
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

        let default = input_from_default(&input.default).unwrap_or_else(|| value.clone());
        mapped.inputs.push(
            StudyInput::new(id, label, value, kind)
                .with_default(default)
                .with_group(&input.group),
        );
        mapped.titles.push(input.title.clone());
    }
    mapped
}

fn value_and_kind(input: &ScriptInput) -> Option<(InputValue, InputKind)> {
    match input.kind.as_str() {
        "int" | "integer" => {
            let value = match &input.value {
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
            let value = match &input.value {
                InputDefault::Float(value) => *value,
                InputDefault::Int(value) => *value as f64,
                _ => return None,
            };
            let lower = input.min.unwrap_or(-f64::MAX);
            let upper = input.max.unwrap_or(f64::MAX);
            Some((InputValue::Number(value), InputKind::Number(lower, upper)))
        }
        "bool" => {
            let InputDefault::Bool(value) = &input.value else {
                return None;
            };
            Some((InputValue::Toggle(*value), InputKind::Toggle))
        }
        "string" => {
            let InputDefault::Text(value) = &input.value else {
                return None;
            };
            Some((InputValue::Text(value.clone()), InputKind::Text))
        }
        "color" => {
            let InputDefault::Color([r, g, b, t]) = &input.value else {
                return None;
            };
            Some((
                InputValue::Color(color_to_egui(*r, *g, *b, *t)),
                InputKind::Color,
            ))
        }
        // `source`, `session` and `time` need a resolver or a widget the chart does
        // not have yet. Omitting the field is safer than letting a user edit a
        // value the script then ignores.
        _ => None,
    }
}

fn input_from_default(default: &InputDefault) -> Option<InputValue> {
    match default {
        InputDefault::Int(value) => Some(InputValue::Length((*value).max(0) as usize)),
        InputDefault::Float(value) => Some(InputValue::Number(*value)),
        InputDefault::Bool(value) => Some(InputValue::Toggle(*value)),
        InputDefault::Text(value) => Some(InputValue::Text(value.clone())),
        InputDefault::Color([r, g, b, t]) => Some(InputValue::Color(color_to_egui(*r, *g, *b, *t))),
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
}
