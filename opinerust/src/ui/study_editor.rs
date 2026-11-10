//! Study editor panel: an editing surface for host-owned study source.
//!
//! The panel holds a document title and its text, tells whether the text changed since it was
//! opened or saved, lists the host's diagnostics and sends open, save and apply requests. The
//! host owns the language, the files and the study: it compiles the text, answers with
//! [`set_diagnostics`](StudyEditor::set_diagnostics) and
//! [`apply_result`](StudyEditor::apply_result), and the edits stay in the panel whatever it
//! answers. The panel needs no other part of the library and can sit in any layout.

use Oracly_charts::localization as l10n;
use crate::ui::pine_reference;
use Oracly_charts::theme::ThemeExt;
use Oracly_charts::ui_kit::buttons::Button;
use Oracly_charts::ui_kit::status_message::{MessageKind, status_message};
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
    /// Draw the diagnostics list under the text.
    ///
    /// Shown by default for the generic editor. A host that shows diagnostics in
    /// its own place — the OpineRust dock lists them in its Logs — hides this so
    /// the same failure is not rendered twice, once under the code and once in the
    /// log strip.
    hide_inline_diagnostics: bool,
    /// Opt-in Pine/OpineRust syntax colors.
    opinerust_syntax: bool,
    /// Text the host last published, for hosts without a file store.
    applied_text: String,
    /// Enable floating hover tooltips with Pine Script documentation.
    hover_docs: bool,
    /// Enable signature help tooltip when typing function arguments.
    signature_help: bool,
    /// Enable context menu on right-click.
    context_menu_enabled: bool,
    /// Color picker state for inline swatch.
    inline_color_picker: Option<InlineColorPickerState>,
}

#[derive(Clone, Debug)]
struct InlineColorPickerState {
    byte_range: Range<usize>,
    picker_state: Oracly_charts::ui_kit::ColorPickerState,
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

    /// Show or hide the diagnostics list drawn under the text.
    ///
    /// Hiding it is for hosts that list the same diagnostics somewhere else. The
    /// diagnostics themselves are kept: [`Self::diagnostics`] and [`Self::reveal`]
    /// still work, so a host can render and navigate them in its own log.
    pub fn set_inline_diagnostics(&mut self, show: bool) {
        self.hide_inline_diagnostics = !show;
    }

    /// Draw line numbers in a gutter beside the text.
    ///
    /// Off by default so hosts that already wrap the editor in their own layout
    /// are unaffected. Syntax highlighting is configured independently.
    pub fn set_line_numbers(&mut self, show: bool) {
        self.line_numbers = show;
    }

    /// Enable or disable the OpineRust syntax palette independently of the gutter.
    /// The generic editor starts with highlighting disabled.
    pub fn set_opinerust_syntax(&mut self, show: bool) {
        self.opinerust_syntax = show;
    }

    /// Enable rich documentation hover tooltips and signature help.
    pub fn set_hover_docs(&mut self, show: bool) {
        self.hover_docs = show;
        self.signature_help = show;
    }

    /// Enable or disable right-click context menu (Format, Comment, Cut/Copy/Paste).
    pub fn set_context_menu(&mut self, show: bool) {
        self.context_menu_enabled = show;
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
        if !self.hide_inline_diagnostics {
            if let Some(reveal) = self.diagnostics_list(ui) {
                self.reveal = Some(reveal);
                ui.ctx().request_repaint();
            }
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
        let line_count = self.text.bytes().filter(|&byte| byte == b'\n').count() + 1;
        let gutter = self.line_numbers.then(|| self.gutter_width(ui, line_count));
        let syntax = self.opinerust_syntax;
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
                        let mut layouter = opinerust_layouter;
                        let mut builder = TextEdit::multiline(&mut self.text)
                            .id(editor)
                            .code_editor()
                            .frame(egui::Frame::NONE)
                            .desired_width(f32::INFINITY)
                            .desired_rows(
                                (height / ui.text_style_height(&egui::TextStyle::Monospace))
                                    as usize,
                            );
                        if syntax {
                            builder = builder.layouter(&mut layouter);
                        }
                        let output = builder.show(ui);
                        paint_line_numbers(ui, &output, &gutter_rect, width);
                        text_output = Some(output);
                    });
                    text_output
                } else {
                    let mut layouter = opinerust_layouter;
                    let mut builder = TextEdit::multiline(&mut self.text)
                        .id(editor)
                        .code_editor()
                        .frame(egui::Frame::NONE)
                        .desired_width(f32::INFINITY)
                        .desired_rows(
                            (height / ui.text_style_height(&egui::TextStyle::Monospace)) as usize,
                        );
                    if syntax {
                        builder = builder.layouter(&mut layouter);
                    }
                    Some(builder.show(ui))
                };
                let Some(output) = output else { return };

                // --- Inline Color Picker Swatches ---
                if syntax {
                    self.render_inline_color_swatches(ui, &output);
                }

                // --- Hover Documentation Tooltip ---
                if self.hover_docs && output.response.hovered() {
                    self.render_hover_doc_tooltip(ui, &output);
                }

                // --- Context Menu (Right Click) ---
                if self.context_menu_enabled {
                    self.render_context_menu(ui, &output);
                }

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

    /// Draw the diagnostic rows in a scroll area, returning a span the user clicked.
    ///
    /// The rows scroll inside the space `diagnostics_height` reserves. Painting them
    /// outside it was what drew diagnostics over the panel's Logs strip: the height
    /// was capped at six rows while the list rendered every entry.
    fn diagnostics_list(&self, ui: &mut Ui) -> Option<Range<usize>> {
        if self.diagnostics.is_empty() {
            return None;
        }
        let weak = ui.visuals().weak_text_color();
        let mut reveal = None;
        ScrollArea::vertical()
            .id_salt("study_editor_diagnostics")
            .max_height(self.diagnostics_height(ui))
            .auto_shrink([false, true])
            .show(ui, |ui| {
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
            });
        reveal
    }

    /// Room kept below the text for the diagnostics list.
    fn diagnostics_height(&self, ui: &Ui) -> f32 {
        let row = ui.spacing().interact_size.y + ui.spacing().item_spacing.y;
        row * self.diagnostics.len().min(6) as f32
    }

    /// Render interactive inline color swatches next to hex colors (#RRGGBB) or color.blue/red/etc.
    fn render_inline_color_swatches(
        &mut self,
        ui: &mut Ui,
        output: &egui::text_edit::TextEditOutput,
    ) {
        let matches = find_color_literals(&self.text);
        if matches.is_empty() {
            return;
        }

        let mut replace_color = None;

        for (range, color) in matches {
            let cursor = CCursor::new(range.start);
            let rect = output.galley.pos_from_cursor(cursor);
            let pos = output.galley_pos + rect.left_top().to_vec2();

            // Draw small swatch square preceding the token
            let swatch_rect =
                egui::Rect::from_min_size(pos + egui::vec2(-14.0, 2.0), egui::vec2(11.0, 11.0));

            if ui.clip_rect().intersects(swatch_rect) {
                let painter = ui.painter();
                painter.rect_filled(swatch_rect, 2.0, color);
                painter.rect_stroke(
                    swatch_rect,
                    2.0,
                    egui::Stroke::new(1.0, ui.visuals().window_stroke().color.gamma_multiply(0.6)),
                    egui::StrokeKind::Outside,
                );

                let id = ui.scope_id().with(("color_swatch", range.start));
                let response = ui.interact(swatch_rect, id, egui::Sense::click());
                if response.clicked() {
                    let mut picker_state = Oracly_charts::ui_kit::ColorPickerState::new(color);
                    picker_state.is_open = true;
                    picker_state.popup_pos = Some(swatch_rect.left_bottom() + egui::vec2(0.0, 4.0));
                    self.inline_color_picker = Some(InlineColorPickerState {
                        byte_range: range.clone(),
                        picker_state,
                    });
                }
            }
        }

        // Active ColorPicker popup if opened
        if let Some(mut picker) = self.inline_color_picker.take() {
            let popup_pos = picker.picker_state.popup_pos.unwrap_or(ui.cursor().min);
            let mut area_rect = egui::Rect::NOTHING;
            egui::Area::new(
                ui.scope_id()
                    .with(("inline_picker_area", picker.byte_range.start)),
            )
            .fixed_pos(popup_pos)
            .order(egui::Order::Foreground)
            .constrain(true)
            .show(ui.ctx(), |ui| {
                let res = Oracly_charts::ui_kit::glass_menu(ui).show(ui, |ui| {
                    ui.set_min_width(200.0);
                    let mut current = picker.picker_state.sel_color;
                    if Oracly_charts::ui_kit::color_picker::show_palette(ui, &mut current) {
                        let hex_str =
                            format!("#{:02X}{:02X}{:02X}", current.r(), current.g(), current.b());
                        replace_color = Some((picker.byte_range.clone(), hex_str));
                        picker.picker_state.sel_color = current;
                    }
                    if picker.picker_state.custom_mode {
                        ui.add_space(ui.active_theme().spacing.sm);
                        let mut custom = picker.picker_state.custom_color;
                        if ui.color_edit_button_srgba(&mut custom).changed() {
                            let hex_str =
                                format!("#{:02X}{:02X}{:02X}", custom.r(), custom.g(), custom.b());
                            replace_color = Some((picker.byte_range.clone(), hex_str));
                            picker.picker_state.custom_color = custom;
                        }
                    }
                });
                area_rect = res.response.rect;
            });

            if !picker.picker_state.just_opened && ui.input(|i| i.pointer.any_pressed()) {
                if let Some(pos) = ui.input(|i| i.pointer.interact_pos()) {
                    if !area_rect.contains(pos) {
                        picker.picker_state.is_open = false;
                    }
                }
            } else {
                picker.picker_state.just_opened = false;
            }

            if picker.picker_state.is_open {
                self.inline_color_picker = Some(picker);
            }
        }

        if let Some((range, new_str)) = replace_color {
            if range.start <= self.text.len() && range.end <= self.text.len() {
                self.text.replace_range(range, &new_str);
            }
        }
    }

    /// Display rich Pine Script documentation tooltip when hovering over functions/builtins.
    fn render_hover_doc_tooltip(&self, ui: &Ui, output: &egui::text_edit::TextEditOutput) {
        let Some(pointer_pos) = ui.ctx().pointer_hover_pos() else {
            return;
        };

        // Check if pointer is over galley area
        let local_pos = pointer_pos - output.galley_pos.to_vec2();
        if local_pos.x < 0.0 || local_pos.y < 0.0 {
            return;
        }

        let ccursor = output.galley.cursor_from_pos(local_pos.to_vec2());
        let char_idx = ccursor.index;

        // Find identifier at char_idx
        if let Some((word, _byte_range)) = get_word_at_char(&self.text, char_idx.into()) {
            if let Some(doc) = pine_reference::find_builtin(&word) {
                egui::containers::Tooltip::always_open(
                    ui.ctx().clone(),
                    ui.layer_id(),
                    output.response.id.with("pine_hover_doc"),
                    egui::PopupAnchor::Pointer,
                )
                .gap(8.0)
                .show(|ui| {
                    ui.set_max_width(340.0);
                    Oracly_charts::ui_kit::glass_popup(ui).show(ui, |ui| {
                        ui.spacing_mut().item_spacing.y = 4.0;
                        ui.horizontal(|ui| {
                            let badge_color = match doc.kind {
                                pine_reference::SymbolKind::Function => {
                                    ui.visuals().selection.bg_fill
                                }
                                pine_reference::SymbolKind::Variable => {
                                    ui.visuals().warn_fg_color
                                }
                                pine_reference::SymbolKind::Constant => {
                                    ui.visuals().error_fg_color
                                }
                                _ => ui.visuals().weak_text_color(),
                            };
                            ui.label(
                                RichText::new(doc.kind.badge())
                                    .strong()
                                    .color(badge_color)
                                    .background_color(badge_color.gamma_multiply(0.2)),
                            );
                            ui.label(RichText::new(doc.name).strong());
                            ui.label(
                                RichText::new(format!("-> {}", doc.return_type))
                                    .weak()
                                    .monospace(),
                            );
                        });

                        ui.separator();
                        ui.label(RichText::new(doc.signature).monospace().size(12.0));
                        ui.add_space(2.0);
                        ui.label(doc.summary);

                        if !doc.params.is_empty() {
                            ui.add_space(4.0);
                            ui.label(RichText::new("Parameters:").strong().size(11.0));
                            for p in doc.params {
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new(p.name).monospace().strong());
                                    ui.label(
                                        RichText::new(format!("({})", p.type_info))
                                            .weak()
                                            .size(11.0),
                                    );
                                    ui.label(RichText::new(p.doc).size(11.0));
                                });
                            }
                        }

                        if let Some(example) = doc.example {
                            ui.add_space(4.0);
                            ui.label(RichText::new("Example:").strong().size(11.0));
                            ui.label(
                                RichText::new(example)
                                    .monospace()
                                    .color(ui.visuals().warn_fg_color),
                            );
                        }
                    });
                });
            }
        }
    }

    /// Render context menu on right click inside the code editor.
    fn render_context_menu(&mut self, _ui: &mut Ui, output: &egui::text_edit::TextEditOutput) {
        output.response.context_menu(|ui| {
            Oracly_charts::ui_kit::glass_menu(ui).show(ui, |ui| {
                ui.set_min_width(170.0);

                // Cut / Copy / Paste
                if ui.button("📋 Copy").clicked() {
                    ui.ctx().copy_text(self.text.clone());
                    ui.close();
                }

                if ui.button("✂ Cut").clicked() {
                    ui.ctx().copy_text(self.text.clone());
                    self.text.clear();
                    ui.close();
                }

                ui.separator();

                if ui.button("🪄 Format Script").clicked() {
                    format_pine_script(&mut self.text);
                    ui.close();
                }

                if ui.button("💬 Toggle Comment (//)").clicked() {
                    toggle_comments(&mut self.text);
                    ui.close();
                }

                ui.separator();

                if ui.button("🗑 Clear All").clicked() {
                    self.text.clear();
                    ui.close();
                }
            });
        });
    }

    /// Run auto-formatter on current buffer.
    pub fn format_code(&mut self) {
        format_pine_script(&mut self.text);
    }
}

/// Lay out Pine-like OpineRust source without changing the text buffer. The lexer is
/// intentionally lightweight and accepts incomplete source while the user types.
fn opinerust_layouter(
    ui: &Ui,
    text: &dyn egui::TextBuffer,
    wrap_width: f32,
) -> std::sync::Arc<egui::Galley> {
    let font = egui::TextStyle::Monospace.resolve(ui.style());
    let mut job = opinerust_layout(text.as_str(), font, ui.visuals().dark_mode);
    job.wrap.max_width = wrap_width;
    job.keep_trailing_whitespace = true;
    ui.fonts_mut(|fonts| fonts.layout_job(job))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SyntaxKind {
    Plain,
    Comment,
    String,
    Number,
    Keyword,
    Function,
    Builtin,
}

fn syntax_color(kind: SyntaxKind, dark: bool) -> egui::Color32 {
    use egui::Color32 as C;
    match (dark, kind) {
        // Dark theme: Tokyo Night / VS Code modern palette
        (true, SyntaxKind::Plain) => C::from_rgb(192, 202, 245),       // clean soft lavender-white
        (true, SyntaxKind::Comment) => C::from_rgb(103, 110, 149),     // muted slate blue
        (true, SyntaxKind::String) => C::from_rgb(158, 206, 106),      // pleasant soft green
        (true, SyntaxKind::Number) => C::from_rgb(255, 158, 100),      // warm orange
        (true, SyntaxKind::Keyword) => C::from_rgb(187, 154, 247),     // light violet/purple
        (true, SyntaxKind::Function) => C::from_rgb(122, 162, 247),    // bright modern azure
        (true, SyntaxKind::Builtin) => C::from_rgb(125, 207, 255),     // vivid sky cyan

        // Light theme: Clean GitHub / VS Code Light palette
        (false, SyntaxKind::Plain) => C::from_rgb(36, 41, 47),
        (false, SyntaxKind::Comment) => C::from_rgb(106, 115, 125),
        (false, SyntaxKind::String) => C::from_rgb(10, 115, 60),
        (false, SyntaxKind::Number) => C::from_rgb(176, 85, 0),
        (false, SyntaxKind::Keyword) => C::from_rgb(130, 52, 209),
        (false, SyntaxKind::Function) => C::from_rgb(5, 80, 174),
        (false, SyntaxKind::Builtin) => C::from_rgb(15, 109, 134),
    }
}

fn opinerust_layout(text: &str, font: egui::FontId, dark: bool) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::default();
    let mut i = 0;
    let mut run_start = 0;
    let mut run_kind = None;
    while i < text.len() {
        let start = i;
        let rest = &text[i..];
        let first = rest.chars().next().unwrap();
        let kind;
        if rest.starts_with("//") {
            i += rest.find('\n').unwrap_or(rest.len());
            kind = SyntaxKind::Comment;
        } else if rest.starts_with("/*") {
            i += rest.find("*/").map_or(rest.len(), |end| end + 2);
            kind = SyntaxKind::Comment;
        } else if first == '"' || first == '\'' {
            i += first.len_utf8();
            let mut escaped = false;
            while i < text.len() {
                let ch = text[i..].chars().next().unwrap();
                if ch == '\n' || ch == '\r' {
                    break;
                }
                i += ch.len_utf8();
                if escaped {
                    escaped = false;
                } else if ch == '\\' {
                    escaped = true;
                } else if ch == first {
                    break;
                }
            }
            kind = SyntaxKind::String;
        } else if first.is_ascii_digit() {
            i += 1;
            while i < text.len()
                && (text.as_bytes()[i].is_ascii_alphanumeric()
                    || matches!(text.as_bytes()[i], b'.' | b'_'))
            {
                i += 1;
            }
            kind = SyntaxKind::Number;
        } else if first == '_' || first.is_alphabetic() {
            i += first.len_utf8();
            while i < text.len() {
                let ch = text[i..].chars().next().unwrap();
                if ch != '_' && !ch.is_alphanumeric() {
                    break;
                }
                i += ch.len_utf8();
            }
            let word = &text[start..i];
            kind = if matches!(
                word,
                "if" | "else"
                    | "for"
                    | "while"
                    | "switch"
                    | "import"
                    | "export"
                    | "type"
                    | "method"
                    | "var"
                    | "varip"
                    | "const"
                    | "return"
                    | "and"
                    | "or"
                    | "not"
                    | "true"
                    | "false"
                    | "na"
            ) {
                SyntaxKind::Keyword
            } else if text[i..].trim_start_matches([' ', '\t']).starts_with('(') {
                SyntaxKind::Function
            } else if matches!(
                word,
                "open"
                    | "high"
                    | "low"
                    | "close"
                    | "volume"
                    | "time"
                    | "bar_index"
                    | "ta"
                    | "math"
                    | "color"
                    | "input"
                    | "syminfo"
                    | "timeframe"
            ) {
                SyntaxKind::Builtin
            } else {
                SyntaxKind::Plain
            };
        } else {
            i += first.len_utf8();
            kind = SyntaxKind::Plain;
        }
        if let Some(previous) = run_kind {
            if previous != kind {
                job.append(
                    &text[run_start..start],
                    0.0,
                    egui::TextFormat::simple(font.clone(), syntax_color(previous, dark)),
                );
                run_start = start;
            }
        }
        run_kind = Some(kind);
    }
    if let Some(kind) = run_kind {
        job.append(
            &text[run_start..],
            0.0,
            egui::TextFormat::simple(font, syntax_color(kind, dark)),
        );
    } else {
        job.append(
            "",
            0.0,
            egui::TextFormat::simple(font, syntax_color(SyntaxKind::Plain, dark)),
        );
    }
    job
}

/// Draw logical line numbers on the first visual row of each line.
/// Row positions come from the actual text galley, including blank and wrapped lines.
fn paint_line_numbers(
    ui: &Ui,
    output: &egui::text_edit::TextEditOutput,
    gutter_rect: &egui::Rect,
    width: f32,
) {
    // Clip to the gutter and the scroll viewport. The editor's own
    // `text_clip_rect` starts to the right of the gutter, so using its vertical
    // bounds clipped every number away once the editor scrolled.
    let gallery = ui.clip_rect();
    let clip = egui::Rect::from_min_max(
        egui::pos2(gutter_rect.left() + 1.0, gallery.top()),
        egui::pos2(gutter_rect.left() + width, gallery.bottom()),
    )
    .intersect(gallery);
    let painter = ui.painter().with_clip_rect(clip);
    // Muted, but not invisible: `weak_text_color` on a light panel washed out to
    // near-white. Blend the text colour most of the way to the background, which
    // stays legible in both themes.
    let color = blend(ui.visuals().text_color(), ui.visuals().panel_fill, 0.55);
    let font = egui::TextStyle::Monospace.resolve(ui.style());
    let right = gutter_rect.left() + width - ui.spacing().item_spacing.x;

    // Draw subtle vertical separator line separating gutter from code canvas
    let border_color = if ui.visuals().dark_mode {
        egui::Color32::from_white_alpha(18)
    } else {
        egui::Color32::from_black_alpha(20)
    };
    painter.vline(
        gutter_rect.left() + width,
        gallery.y_range(),
        egui::Stroke::new(1.0, border_color),
    );

    let mut line = 1;
    let mut first_row = true;
    for row in &output.galley.rows {
        if first_row {
            let y = output.galley_pos.y + row.pos.y;
            let galley =
                ui.fonts_mut(|fonts| fonts.layout_no_wrap(line.to_string(), font.clone(), color));
            let pos = egui::pos2(right - galley.size().x, y);
            painter.galley(pos, galley, color);
        }
        first_row = row.ends_with_newline;
        if first_row {
            line += 1;
        }
    }
}

/// Blend `over` towards `under` by `amount` (0 = `over`, 1 = `under`).
///
/// A gutter number must stay readable on light and dark panels alike; this keeps
/// it a step behind the code instead of at the mercy of the theme's weak colour.
fn blend(over: egui::Color32, under: egui::Color32, amount: f32) -> egui::Color32 {
    let mix = |a: u8, b: u8| {
        (f32::from(a) + (f32::from(b) - f32::from(a)) * amount.clamp(0.0, 1.0)).round() as u8
    };
    egui::Color32::from_rgba_unmultiplied(
        mix(over.r(), under.r()),
        mix(over.g(), under.g()),
        mix(over.b(), under.b()),
        255,
    )
}

/// Find hex color literals (#RRGGBB or #RGB) and built-in color names in the text.
fn find_color_literals(text: &str) -> Vec<(Range<usize>, egui::Color32)> {
    let mut results = Vec::new();
    let bytes = text.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        if !text.is_char_boundary(i) {
            i += 1;
            continue;
        }
        if bytes[i] == b'#' {
            let start = i;
            i += 1;
            while i < len && bytes[i].is_ascii_hexdigit() {
                i += 1;
            }
            let hex_str = &text[start + 1..i];
            if hex_str.len() == 6 {
                if let (Ok(r), Ok(g), Ok(b)) = (
                    u8::from_str_radix(&hex_str[0..2], 16),
                    u8::from_str_radix(&hex_str[2..4], 16),
                    u8::from_str_radix(&hex_str[4..6], 16),
                ) {
                    results.push((start..i, egui::Color32::from_rgb(r, g, b)));
                }
            } else if hex_str.len() == 3 {
                if let (Ok(r), Ok(g), Ok(b)) = (
                    u8::from_str_radix(&hex_str[0..1], 16),
                    u8::from_str_radix(&hex_str[1..2], 16),
                    u8::from_str_radix(&hex_str[2..3], 16),
                ) {
                    results.push((start..i, egui::Color32::from_rgb(r * 17, g * 17, b * 17)));
                }
            }
        } else if text[i..].starts_with("color.") {
            let start = i;
            i += 6;
            while i < len && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                i += 1;
            }
            let col_name = &text[start..i];
            let col = match col_name {
                "color.blue" => Some(egui::Color32::from_rgb(41, 98, 255)),
                "color.red" => Some(egui::Color32::from_rgb(242, 54, 69)),
                "color.green" => Some(egui::Color32::from_rgb(8, 153, 129)),
                "color.orange" => Some(egui::Color32::from_rgb(255, 152, 0)),
                "color.white" => Some(egui::Color32::WHITE),
                "color.black" => Some(egui::Color32::BLACK),
                "color.yellow" => Some(egui::Color32::from_rgb(255, 235, 59)),
                "color.purple" => Some(egui::Color32::from_rgb(156, 39, 176)),
                "color.teal" => Some(egui::Color32::from_rgb(0, 150, 136)),
                _ => None,
            };
            if let Some(color) = col {
                results.push((start..i, color));
            }
        } else {
            i += 1;
        }
    }
    results
}

/// Find word / token at given character index.
fn get_word_at_char(text: &str, char_idx: usize) -> Option<(String, Range<usize>)> {
    let mut current_char = 0;
    let mut byte_offset = 0;

    for (b_idx, _ch) in text.char_indices() {
        if current_char == char_idx {
            byte_offset = b_idx;
            break;
        }
        current_char += 1;
    }
    if current_char < char_idx {
        byte_offset = text.len();
    }

    let bytes = text.as_bytes();
    if byte_offset >= bytes.len() && !bytes.is_empty() {
        byte_offset = bytes.len() - 1;
        while byte_offset > 0 && !text.is_char_boundary(byte_offset) {
            byte_offset -= 1;
        }
    }

    // Expand left and right to find word characters (alphanumeric, underscore, or dot)
    let mut start = byte_offset;
    while start > 0
        && (bytes[start - 1].is_ascii_alphanumeric()
            || bytes[start - 1] == b'_'
            || bytes[start - 1] == b'.')
    {
        start -= 1;
    }

    let mut end = byte_offset;
    while end < bytes.len()
        && (bytes[end].is_ascii_alphanumeric() || bytes[end] == b'_' || bytes[end] == b'.')
    {
        end += 1;
    }

    if start < end && text.is_char_boundary(start) && text.is_char_boundary(end) {
        Some((text[start..end].to_string(), start..end))
    } else {
        None
    }
}

/// Basic code formatting / indentation prettify for Pine Script.
fn format_pine_script(text: &mut String) {
    let lines: Vec<&str> = text.lines().collect();
    let mut formatted = String::new();
    let mut indent_level: usize = 0;

    for line in lines {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            formatted.push('\n');
            continue;
        }

        let is_block_close = trimmed == "}"
            || trimmed == "]"
            || trimmed.starts_with("else")
            || trimmed.starts_with("=>");
        if is_block_close && indent_level > 0 {
            indent_level = indent_level.saturating_sub(1);
        }

        for _ in 0..indent_level {
            formatted.push_str("    ");
        }
        formatted.push_str(trimmed);
        formatted.push('\n');

        let opens_block = trimmed.ends_with('{')
            || trimmed.ends_with(':')
            || trimmed.starts_with("if ")
            || trimmed.starts_with("for ")
            || trimmed.starts_with("while ")
            || (trimmed.ends_with("=>") && !trimmed.contains("plot("));
        if opens_block {
            indent_level += 1;
        }
    }

    if !formatted.is_empty() {
        *text = formatted;
    }
}

/// Toggle comment on all selected lines or whole buffer.
fn toggle_comments(text: &mut String) {
    let lines: Vec<&str> = text.lines().collect();
    let all_commented = lines
        .iter()
        .all(|l| l.trim().is_empty() || l.trim().starts_with("//"));

    let mut new_text = String::new();
    for line in lines {
        if all_commented {
            if let Some(pos) = line.find("//") {
                let (before, after) = line.split_at(pos);
                let trimmed_after = after
                    .strip_prefix("// ")
                    .unwrap_or_else(|| after.strip_prefix("//").unwrap_or(after));
                new_text.push_str(before);
                new_text.push_str(trimmed_after);
            } else {
                new_text.push_str(line);
            }
        } else if !line.trim().is_empty() {
            new_text.push_str("// ");
            new_text.push_str(line);
        } else {
            new_text.push_str(line);
        }
        new_text.push('\n');
    }
    *text = new_text;
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

    #[test]
    fn test_find_color_literals() {
        let code = "c1 = #2962FF\nc2 = #F23\nc3 = color.green\n";
        let matches = find_color_literals(code);
        assert_eq!(matches.len(), 3);
        assert_eq!(matches[0].1, egui::Color32::from_rgb(41, 98, 255));
        assert_eq!(matches[1].1, egui::Color32::from_rgb(255, 34, 51));
        assert_eq!(matches[2].1, egui::Color32::from_rgb(8, 153, 129));
    }

    #[test]
    fn test_multibyte_utf8_color_literals_and_word_lookup() {
        // Includes multi-byte characters: © (2 bytes), 🚀 (4 bytes), → (3 bytes)
        let code = "// © 2026 Author 🚀 -> indicator\nc = #2962FF\ncolor_val = color.blue\n";
        let matches = find_color_literals(code);
        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].1, egui::Color32::from_rgb(41, 98, 255));
        assert_eq!(matches[1].1, egui::Color32::from_rgb(41, 98, 255));

        // Test word lookup around multi-byte characters
        // "Author" is at character index 10 ("// © 2026 Author")
        let word = get_word_at_char(code, 12); // inside "Author"
        assert!(word.is_some());
        assert_eq!(word.unwrap().0, "Author");
    }

    #[test]
    fn test_format_pine_script() {
        let mut unformatted = String::from("if true\nx = 1\n");
        format_pine_script(&mut unformatted);
        assert!(unformatted.contains("    x = 1"));
    }

    #[test]
    fn test_toggle_comments() {
        let mut code = String::from("a = 1\nb = 2\n");
        toggle_comments(&mut code);
        assert_eq!(code, "// a = 1\n// b = 2\n");
        toggle_comments(&mut code);
        assert_eq!(code, "a = 1\nb = 2\n");
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

    #[test]
    fn syntax_preserves_utf8_crlf_and_handles_incomplete_tokens() {
        let source =
            "//@version=5\r\nα = \"//é\\\"x\" // comment\r\nplot(close, 3.5)\n/* unfinished 🦀";
        let job = opinerust_layout(source, egui::FontId::monospace(14.0), true);
        assert_eq!(job.text, source);
        let sections: Vec<_> = job
            .sections
            .iter()
            .map(|section| {
                (
                    &source[section.byte_range.start.0..section.byte_range.end.0],
                    section.format.color,
                )
            })
            .collect();
        assert!(sections.contains(&("\"//é\\\"x\"", syntax_color(SyntaxKind::String, true))));
        assert!(sections.contains(&("// comment\r", syntax_color(SyntaxKind::Comment, true))));
        assert!(sections.contains(&("/* unfinished 🦀", syntax_color(SyntaxKind::Comment, true))));
        assert!(sections.contains(&("plot", syntax_color(SyntaxKind::Function, true))));
        assert!(sections.contains(&("close", syntax_color(SyntaxKind::Builtin, true))));
        assert!(sections.contains(&("3.5", syntax_color(SyntaxKind::Number, true))));
        assert!(
            sections.iter().any(|(text, color)| text.contains('α')
                && *color == syntax_color(SyntaxKind::Plain, true))
        );
        let mut end = 0;
        for section in &job.sections {
            assert_eq!(section.byte_range.start.0, end);
            assert!(source.is_char_boundary(section.byte_range.end.0));
            end = section.byte_range.end.0;
        }
        assert_eq!(end, source.len());
    }

    /// A gutter number must be visible against the panel, not blended into it.
    #[test]
    fn gutter_numbers_stay_readable_on_both_themes() {
        use Oracly_charts::theme::{Theme, apply_to_egui};
        for theme in [Theme::light(), Theme::dark()] {
            let ctx = egui::Context::default();
            apply_to_egui(&ctx, &theme);
            let mut editor = StudyEditor::new();
            editor.open_document("Numbers", "one\ntwo\nthree\n");
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
            let panel = theme.ui.panel_bg;
            let numbers: Vec<_> = output
                .shapes
                .iter()
                .filter_map(|shape| match &shape.shape {
                    egui::Shape::Text(text)
                        if !text.galley.text().is_empty()
                            && text.galley.text().chars().all(|c| c.is_ascii_digit()) =>
                    {
                        Some(text.fallback_color)
                    }
                    _ => None,
                })
                .collect();
            output.drop_without_applying_deltas();
            assert!(!numbers.is_empty(), "{theme:?} paints line numbers");
            for color in numbers {
                let distance = (i16::from(color.r()) - i16::from(panel.r())).abs()
                    + (i16::from(color.g()) - i16::from(panel.g())).abs()
                    + (i16::from(color.b()) - i16::from(panel.b())).abs();
                assert!(
                    distance >= 90,
                    "{theme:?}: gutter {color:?} is too close to the panel {panel:?}"
                );
            }
        }
    }

    #[test]
    fn syntax_and_gutter_are_opt_in() {
        let mut editor = StudyEditor::new();
        assert!(!editor.line_numbers && !editor.opinerust_syntax);
        editor.set_line_numbers(true);
        assert!(editor.line_numbers && !editor.opinerust_syntax);
        editor.set_opinerust_syntax(true);
        assert!(editor.line_numbers && editor.opinerust_syntax);
        editor.set_opinerust_syntax(false);
        assert!(editor.line_numbers && !editor.opinerust_syntax);
        editor.set_line_numbers(false);
        editor.set_opinerust_syntax(true);
        assert!(!editor.line_numbers && editor.opinerust_syntax);
    }

    /// Blank lines and a final newline keep their own aligned gutter positions.
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
        let numbers: Vec<_> = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::Shape::Text(text)
                    if !text.galley.text().is_empty()
                        && text.galley.text().chars().all(|c| c.is_ascii_digit()) =>
                {
                    Some((shape.clip_rect, text))
                }
                _ => None,
            })
            .collect();
        let code = output
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Text(text) if text.galley.text() == "one\n\ntwo\n" => Some(text),
                _ => None,
            })
            .expect("code galley is painted");
        assert_eq!(numbers.len(), 4);
        for (index, (clip, number)) in numbers.iter().enumerate() {
            assert_eq!(number.galley.text(), &(index + 1).to_string());
            let bounds = number.galley.rect.translate(number.pos.to_vec2());
            assert!(
                clip.intersects(bounds),
                "number {} is clipped away: {clip:?} {bounds:?}",
                index + 1
            );
            let expected_y = code.pos.y + code.galley.rows[index].pos.y;
            assert!((number.pos.y - expected_y).abs() < 0.01);
            assert!(bounds.right() <= code.pos.x);
        }
        output.drop_without_applying_deltas();
    }
}
