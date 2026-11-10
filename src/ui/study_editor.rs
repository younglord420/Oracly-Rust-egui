//! Study editor panel: an editing surface for host-owned study source.
//!
//! The panel holds a document title and its text, tells whether the text changed since it was
//! opened or saved, lists the host's diagnostics and sends open, save and apply requests. The
//! host owns the language, the files and the study: it compiles the text, answers with
//! [`set_diagnostics`](StudyEditor::set_diagnostics) and
//! [`apply_result`](StudyEditor::apply_result), and the edits stay in the panel whatever it
//! answers. The panel needs no other part of the library and can sit in any layout.

use crate::localization as l10n;
use crate::ui_kit::buttons::Button;
use crate::ui_kit::status_message::{MessageKind, status_message};
use egui::text::{CCursor, CCursorRange};
use egui::{Align, RichText, ScrollArea, TextEdit, Ui};
use std::ops::Range;

/// How serious a [`Diagnostic`] is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[non_exhaustive]
pub enum Severity {
    /// A hint.
    Info,
    /// The text works but may not do what was meant.
    Warning,
    /// The text cannot be applied.
    Error,
}

/// A host message about a span of the document.
#[derive(Clone, Debug, PartialEq)]
pub struct Diagnostic {
    /// How serious it is.
    pub severity: Severity,
    /// What the host found.
    pub message: String,
    /// The byte range of the text it concerns. A range past the text or inside a character is
    /// clamped to the text and to character boundaries.
    pub span: Range<usize>,
}

/// What the user asked of the host.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum StudyEditorAction {
    /// Nothing was asked.
    None,
    /// The user wants to open another document: the host picks one and calls
    /// [`StudyEditor::open_document`]. Unsaved edits are still in the panel until it does.
    Open,
    /// The user wants the document stored: the host saves it and calls
    /// [`StudyEditor::mark_saved`].
    Save {
        /// The document title.
        title: String,
        /// The document text.
        text: String,
    },
    /// The user wants the text applied as a study: the host compiles it and calls
    /// [`StudyEditor::apply_result`].
    Apply {
        /// The document title.
        title: String,
        /// The document text.
        text: String,
    },
}

/// The host's answer to an apply request.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Status {
    Applied,
    Rejected(String),
}

/// The study editor panel.
#[derive(Default)]
pub struct StudyEditor {
    title: String,
    text: String,
    /// The text as last opened or saved, to tell whether it was modified.
    saved: String,
    diagnostics: Vec<Diagnostic>,
    status: Option<Status>,
    /// A span to select and scroll to in the next frame.
    reveal: Option<Range<usize>>,
    /// Hide Open and Save for hosts that cannot honour them.
    hide_io: bool,
    /// Label of the apply button, empty for the localized default.
    apply_label: String,
    /// Hide the apply button for hosts that draw their own Run action.
    hide_apply: bool,
    /// Hide the title row, modified note and status banner.
    hide_chrome: bool,
    /// Draw line numbers beside the text.
    line_numbers: bool,
    /// Text the host last published, for hosts without a file store.
    applied_text: String,
}

impl StudyEditor {
    /// An empty, untitled document.
    pub fn new() -> Self {
        Self::default()
    }

    /// Replaces the document, clearing diagnostics and status and the modified state.
    pub fn open_document(&mut self, title: impl Into<String>, text: impl Into<String>) {
        self.title = title.into();
        self.text = text.into();
        self.saved = self.text.clone();
        self.diagnostics.clear();
        self.status = None;
        self.reveal = None;
    }

    /// The document title.
    pub fn title(&self) -> &str {
        &self.title
    }

    /// The document text, with the user's edits.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Replaces the text without changing the saved baseline, as typing does.
    /// Hosts and tests use this when they do not drive the widget directly.
    pub fn replace_text(&mut self, text: impl Into<String>) {
        self.text = text.into();
    }

    /// Whether the text differs from the text last opened or saved.
    pub fn is_modified(&self) -> bool {
        self.text != self.saved
    }

    /// Records that the host stored `text`, which clears the modified state if the user has not
    /// edited the document since the save request.
    pub fn mark_saved(&mut self, text: &str) {
        self.saved = text.to_owned();
    }

    /// Replaces the diagnostics shown, most serious first as the host sends them.
    pub fn set_diagnostics(&mut self, diagnostics: Vec<Diagnostic>) {
        self.diagnostics = diagnostics;
    }

    /// The diagnostics shown.
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// Records the host's answer to [`StudyEditorAction::Apply`]: `Ok` shows the text as applied,
    /// `Err` shows the reason. Neither changes the text.
    pub fn apply_result(&mut self, result: Result<(), String>) {
        self.status = Some(match result {
            Ok(()) => Status::Applied,
            Err(reason) => Status::Rejected(reason),
        });
    }

    /// Selects the byte `span` in the text and scrolls to it in the next frame.
    pub fn reveal(&mut self, span: Range<usize>) {
        self.reveal = Some(span);
    }

    /// Hides Open and Save. Use when the host has no document store: showing the
    /// buttons would promise something the host cannot do.
    pub fn set_io_actions(&mut self, show: bool) {
        self.hide_io = !show;
    }

    /// Hide the title field, the modified note and the status banner.
    ///
    /// A host that draws its own header — the OpineRust dock shows the script
    /// name, the run state and the Run action itself — would otherwise repeat
    /// every one of them. The text, diagnostics and actions keep working.
    pub fn set_chrome(&mut self, show: bool) {
        self.hide_chrome = !show;
    }

    /// The id of the code text widget, for hosts that track focus.
    ///
    /// Stable for the same enclosing `Ui`, so a host can tell whether the code has
    /// focus before accepting a keyboard shortcut meant only for it.
    pub fn text_id(&self, ui: &Ui) -> egui::Id {
        ui.scope_id().with("study_editor_text")
    }

    /// Draw line numbers in a gutter beside the text.
    ///
    /// Off by default so hosts that already wrap the editor in their own layout
    /// are unaffected.
    pub fn set_line_numbers(&mut self, show: bool) {
        self.line_numbers = show;
    }

    /// Whether the text changed since the document was opened or last saved.
    ///
    /// Unlike [`Self::is_modified`], this compares against a baseline the host
    /// chooses: a host with no file store uses [`Self::mark_applied`] to tell
    /// "this draft is not what the chart runs" from "this file is not saved".
    pub fn is_dirty_against(&self, baseline: &str) -> bool {
        self.text != baseline
    }

    /// Record the text the host published, so a later edit is visible as such.
    ///
    /// This never claims the text is saved: [`Self::is_modified`] keeps its own
    /// file-oriented baseline.
    pub fn mark_applied(&mut self, text: &str) {
        self.applied_text = text.to_owned();
    }

    /// The text as last published by the host.
    pub fn applied_text(&self) -> &str {
        &self.applied_text
    }

    /// Renames the apply button, for hosts whose action is "Run".
    pub fn set_apply_label(&mut self, label: impl Into<String>) {
        self.apply_label = label.into();
    }

    /// Hides the apply button when the host owns the Run action in its own header.
    pub fn set_apply_button(&mut self, show: bool) {
        self.hide_apply = !show;
    }

    /// The span as character offsets, clamped to the text.
    fn char_range(&self, span: &Range<usize>) -> Range<usize> {
        let at = |byte: usize| {
            let mut byte = byte.min(self.text.len());
            while !self.text.is_char_boundary(byte) {
                byte -= 1;
            }
            self.text[..byte].chars().count()
        };
        at(span.start)..at(span.end.max(span.start))
    }

    /// Shows the panel in `ui` and returns what the user asked of the host.
    pub fn show(&mut self, ui: &mut Ui) -> StudyEditorAction {
        let mut action = StudyEditorAction::None;
        if !self.hide_chrome {
            action = self.chrome(ui);
            self.status_banner(ui);
        }
        self.text_area(ui);
        if let Some(reveal) = self.diagnostics_list(ui) {
            self.reveal = Some(reveal);
            ui.ctx().request_repaint();
        }
        action
    }

    /// Title field, modified note, and the Open/Save/Apply actions.
    fn chrome(&mut self, ui: &mut Ui) -> StudyEditorAction {
        let mut action = StudyEditorAction::None;
        let weak = ui.visuals().weak_text_color();
        ui.horizontal(|ui| {
            ui.label(RichText::new(l10n::text(ui.ctx(), "Title")).color(weak));
            ui.add(TextEdit::singleline(&mut self.title).desired_width(220.0));
            if self.is_modified() {
                ui.label(RichText::new(l10n::text(ui.ctx(), "Unsaved changes")).color(weak));
            }
            let apply_label = if self.apply_label.is_empty() {
                l10n::text(ui.ctx(), "Apply")
            } else {
                self.apply_label.clone()
            };
            let hide_io = self.hide_io;
            let hide_apply = self.hide_apply;
            ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                if !hide_apply && Button::new(&apply_label).show(ui).clicked() {
                    action = StudyEditorAction::Apply {
                        title: self.title.clone(),
                        text: self.text.clone(),
                    };
                }
                if !hide_io {
                    if Button::new("Save").show(ui).clicked() {
                        action = StudyEditorAction::Save {
                            title: self.title.clone(),
                            text: self.text.clone(),
                        };
                    }
                    if Button::new("Open").show(ui).clicked() {
                        action = StudyEditorAction::Open;
                    }
                }
            });
        });
        action
    }

    /// The host's answer to the last apply request.
    fn status_banner(&self, ui: &mut Ui) {
        match &self.status {
            Some(Status::Applied) => status_message(
                ui,
                MessageKind::Success,
                &l10n::text(ui.ctx(), "OpineRust: script applied"),
            ),
            Some(Status::Rejected(reason)) => {
                status_message(ui, MessageKind::Error, reason);
            }
            None => {}
        }
    }

    /// The scrollable text, with an optional line-number gutter.
    fn text_area(&mut self, ui: &mut Ui) {
        let editor = ui.scope_id().with("study_editor_text");
        let height = (ui.available_height() - self.diagnostics_height(ui)).max(80.0);
        let reveal = self.reveal.take().map(|span| self.char_range(&span));
        let line_count = self.text.lines().count().max(1);
        let gutter = self.line_numbers.then(|| self.gutter_width(ui, line_count));
        ScrollArea::both()
            .id_salt("study_editor_scroll")
            .max_height(height)
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                let output = if let Some(width) = gutter {
                    let mut text_output = None;
                    // The gutter is allocated first so the text starts to its right,
                    // and painted afterwards from the galley rows: that keeps every
                    // number on its own baseline without re-laying out the text.
                    ui.horizontal_top(|ui| {
                        let (gutter_rect, _) =
                            ui.allocate_exact_size(egui::vec2(width, 0.0), egui::Sense::hover());
                        let builder = TextEdit::multiline(&mut self.text)
                            .id(editor)
                            .code_editor()
                            .desired_width(f32::INFINITY)
                            .desired_rows(
                                (height / ui.text_style_height(&egui::TextStyle::Monospace))
                                    as usize,
                            );
                        let output = builder.show(ui);
                        paint_line_numbers(ui, &output, &gutter_rect, width);
                        text_output = Some(output);
                    });
                    text_output
                } else {
                    Some(
                        TextEdit::multiline(&mut self.text)
                            .id(editor)
                            .code_editor()
                            .desired_width(f32::INFINITY)
                            .desired_rows(
                                (height / ui.text_style_height(&egui::TextStyle::Monospace))
                                    as usize,
                            )
                            .show(ui),
                    )
                };
                let Some(output) = output else { return };
                if let Some(range) = reveal {
                    let cursors =
                        CCursorRange::two(CCursor::new(range.start), CCursor::new(range.end));
                    let mut state = output.state.clone();
                    state.cursor.set_char_range(Some(cursors));
                    state.store(ui.ctx(), editor);
                    ui.memory_mut(|memory| memory.request_focus(editor));
                    let at = output.galley.pos_from_cursor(CCursor::new(range.start));
                    ui.scroll_to_rect(
                        at.translate(output.galley_pos.to_vec2()),
                        Some(Align::Center),
                    );
                }
            });
    }

    /// Width a gutter needs for `lines` line numbers.
    fn gutter_width(&self, ui: &Ui, lines: usize) -> f32 {
        let digits = lines.to_string().len().max(2) as f32;
        ui.fonts_mut(|fonts| {
            let font = egui::TextStyle::Monospace.resolve(ui.style());
            fonts.glyph_width(&font, '0') * digits
        }) + ui.spacing().item_spacing.x
    }

    /// Draw the diagnostic rows, returning a span the user clicked.
    fn diagnostics_list(&self, ui: &mut Ui) -> Option<Range<usize>> {
        let weak = ui.visuals().weak_text_color();
        let mut reveal = None;
        for diagnostic in &self.diagnostics {
            let color = match diagnostic.severity {
                Severity::Error => ui.visuals().error_fg_color,
                Severity::Warning => ui.visuals().warn_fg_color,
                Severity::Info => weak,
            };
            let label = match diagnostic.severity {
                Severity::Error => "Error",
                Severity::Warning => "Warning",
                Severity::Info => "Info",
            };
            let line = ui.horizontal(|ui| {
                ui.label(RichText::new(l10n::text(ui.ctx(), label)).color(color));
                ui.add(
                    egui::Label::new(&diagnostic.message)
                        .sense(egui::Sense::click())
                        .truncate(),
                )
                .clicked()
            });
            if line.inner {
                reveal = Some(diagnostic.span.clone());
            }
        }
        reveal
    }

    /// Room kept below the text for the diagnostics list.
    fn diagnostics_height(&self, ui: &Ui) -> f32 {
        let row = ui.spacing().interact_size.y + ui.spacing().item_spacing.y;
        row * self.diagnostics.len().min(6) as f32
    }
}

/// Draw one number per visual row, right-aligned in `gutter_rect`.
///
/// Positions come from the galley rows rather than from an assumed line height,
/// so wrapped rows, blank lines, CRLF and a taller font stay aligned. The gutter
/// shares the text's clip rectangle, which is what makes it scroll with the code
/// instead of drifting.
fn paint_line_numbers(
    ui: &Ui,
    output: &egui::text_edit::TextEditOutput,
    gutter_rect: &egui::Rect,
    width: f32,
) {
    let painter = ui.painter().with_clip_rect(output.text_clip_rect);
    let color = ui.visuals().weak_text_color();
    let font = egui::TextStyle::Monospace.resolve(ui.style());
    let right = gutter_rect.left() + width - ui.spacing().item_spacing.x;
    for (index, row) in output.galley.rows.iter().enumerate() {
        let y = output.galley_pos.y + row.pos.y;
        let number = (index + 1).to_string();
        let galley = ui.fonts_mut(|fonts| fonts.layout_no_wrap(number, font.clone(), color));
        let pos = egui::pos2(right - galley.size().x, y);
        painter.galley(pos, galley, color);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_rejected_apply_keeps_the_edits_and_a_save_clears_the_modified_state() {
        let mut editor = StudyEditor::new();
        editor.open_document("RSI cross", "plot(rsi)");
        assert!(!editor.is_modified());
        editor.text.push_str("\nplot(sma)");
        assert!(editor.is_modified());
        editor.apply_result(Err("line 2: unknown name".into()));
        assert_eq!(editor.text(), "plot(rsi)\nplot(sma)");
        assert!(editor.is_modified());
        let saved = editor.text().to_owned();
        editor.mark_saved(&saved);
        assert!(!editor.is_modified());
    }

    #[test]
    fn spans_are_clamped_to_characters_of_the_text() {
        let mut editor = StudyEditor::new();
        editor.open_document("t", "aé=1");
        // Bytes: a=0, é=1..3, '='=3, '1'=4.
        assert_eq!(editor.char_range(&(1..3)), 1..2);
        assert_eq!(editor.char_range(&(2..99)), 1..4);
    }

    /// A host with its own header hides the editor chrome, and the applied baseline
    /// is separate from the file baseline: neither may pretend the other happened.
    #[test]
    fn chrome_can_be_hidden_without_touching_the_file_baseline() {
        let mut editor = StudyEditor::new();
        editor.open_document("Dock", "plot(close)");
        editor.set_chrome(false);
        editor.set_line_numbers(true);

        assert!(!editor.is_modified(), "a fresh document is not edited");
        editor.text.push_str("\nplot(open)");
        assert!(editor.is_modified(), "typing still marks the file dirty");
        assert!(
            editor.is_dirty_against("plot(close)"),
            "and the draft differs from what the host published"
        );

        // Publishing is not saving: the file baseline is untouched.
        editor.mark_applied(&editor.text().to_owned());
        assert!(
            !editor.is_dirty_against(editor.applied_text()),
            "the published text matches the draft"
        );
        assert!(
            editor.is_modified(),
            "mark_applied must never claim the file was saved"
        );
        editor.mark_saved(&editor.text().to_owned());
        assert!(
            !editor.is_modified(),
            "a real save clears the file baseline"
        );
    }

    /// The dock draws no title, modified note or status banner of its own; the
    /// generic editor keeps them for hosts like the terminal.
    #[test]
    fn the_dock_chrome_is_absent_and_the_generic_chrome_stays() {
        let ctx = egui::Context::default();
        let size = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(480.0, 400.0));

        let texts = |chrome: bool| {
            let mut editor = StudyEditor::new();
            editor.open_document("Script", "plot(close)\nplot(open)\n");
            editor.set_chrome(chrome);
            editor.set_apply_button(false);
            editor.apply_result(Ok(()));
            let output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(size),
                    ..Default::default()
                },
                |ui| {
                    editor.show(ui);
                },
            );
            let found: Vec<String> = output
                .shapes
                .iter()
                .filter_map(|shape| match &shape.shape {
                    egui::Shape::Text(text) => Some(text.galley.text().to_owned()),
                    _ => None,
                })
                .collect();
            output.drop_without_applying_deltas();
            found
        };

        let dock = texts(false);
        assert!(
            !dock.iter().any(|text| text == "Title"),
            "no title field in the dock: {dock:?}"
        );
        assert!(
            !dock.iter().any(|text| text.contains("applied")),
            "no status banner duplicating the header: {dock:?}"
        );

        let generic = texts(true);
        assert!(
            generic.iter().any(|text| text == "Title"),
            "the generic editor keeps its title field: {generic:?}"
        );
    }

    /// Line numbers are drawn per visual row, so a blank line, a wrapped row or a
    /// newline at the end still lines up with the text it belongs to.
    #[test]
    fn line_numbers_follow_the_rows_they_belong_to() {
        let ctx = egui::Context::default();
        let mut editor = StudyEditor::new();
        editor.open_document("Numbers", "one\n\ntwo\n");
        editor.set_chrome(false);
        editor.set_line_numbers(true);
        editor.set_apply_button(false);
        let output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(320.0, 200.0),
                )),
                ..Default::default()
            },
            |ui| {
                editor.show(ui);
            },
        );
        let numbers: Vec<String> = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::Shape::Text(text) => Some(text.galley.text().to_owned()),
                _ => None,
            })
            .filter(|text| text.chars().all(|c| c.is_ascii_digit()))
            .collect();
        output.drop_without_applying_deltas();
        assert!(
            numbers.contains(&"1".to_owned()) && numbers.contains(&"3".to_owned()),
            "each row gets its own number, including after a blank line: {numbers:?}"
        );
    }
}
