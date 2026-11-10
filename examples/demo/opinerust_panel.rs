//! OpineRust editor panel for the demo.
//!
//! The panel is a side dock, modelled on Vela's editor: a header with the panel
//! identity and run state, the code body whose primary action is **Run**, and a
//! collapsible Logs strip pinned at the bottom so the editor cannot push it out
//! of view.
//!
//! Three things stay separate:
//!
//! - **Prepare** compiles and runs the script off-chart. A failure only writes
//!   diagnostics; the chart is untouched.
//! - **Commit** happens after the host installed the study in one undoable
//!   action, and only then does the panel say “Applied”.
//! - **Save** is never implied by Run. Open/Save are hidden because this host
//!   has no document store for scripts, and a button that cannot keep its
//!   promise is worse than none.

use egui_charts::Chart;
use egui_charts::model::Bar as ChartBar;
use egui_charts::studies::Indicator;
use egui_charts::theme::ThemeExt;
use egui_charts::ui::study_editor::{Diagnostic as EditorDiagnostic, Severity, StudyEditor};
use egui_charts::ui_kit::buttons::{Button, ButtonSize, IconButton};
use egui_charts::icons::icons;
use egui_charts::widget::SeriesId;
use opinerust::{DiagnosticSeverity, RunContext, StudyBuildError, build_study_in_context};
use std::ops::Range;
use std::sync::{Arc, Mutex};

/// Script shown on first open. Kept simple so it runs before anything else works.
const DEFAULT_SCRIPT: &str = r#"//@version=5
indicator("Fast and Slow", overlay = true)

fast = ta.sma(close, 3)
slow = ta.sma(close, 10)

plot(fast, title = "Fast", color = color.orange)
plot(slow, title = "Slow", color = color.blue)
"#;

/// One entry in the run log.
#[derive(Clone, Debug, PartialEq)]
struct LogEntry {
    severity: Severity,
    message: String,
    /// Where the problem is, when the runtime reported a location.
    ///
    /// Kept as a byte span into the source that produced it; a message without a
    /// location has none, rather than a made-up position.
    span: Option<Range<usize>>,
}

/// What the panel is showing about the script right now.
///
/// The order is the priority: a live failure outranks a successful install, and an
/// installed study whose draft has since changed is not the same as one that is
/// still live. Written as one enum so a frame cannot show two of them at once.
#[derive(Clone, Debug, PartialEq)]
enum RunState {
    /// Nothing installed by this panel.
    Draft,
    /// Installed, and the editor holds exactly what the chart runs.
    Applied(String),
    /// Installed, but the draft has changed since.
    Edited(String),
    /// The last run or recalculation failed.
    Failed(String),
    /// The chart has no bars to run over.
    NoData,
}

/// The OpineRust panel's state.
pub struct OpineRustPanel {
    editor: StudyEditor,
    /// Title of the script the host last installed successfully.
    applied: Option<String>,
    /// The text at the last successful apply, to tell edits from it.
    applied_text: String,
    /// One log entry per event, newest last.
    logs: Vec<LogEntry>,
    /// True when a study from this panel is on the chart, so the next Run
    /// replaces rather than adds.
    applied_before: bool,
    /// Where the installed study records recalculation failures.
    watching: Option<Arc<Mutex<Option<String>>>>,
    /// The failure already written to the log, so it is logged once.
    logged_error: Option<String>,
    /// Title waiting for the host's install result.
    pending: Option<String>,
    /// Id of the study this panel installed, once the chart accepted it.
    installed: Option<SeriesId>,
    /// Every study this panel installed, newest last, while the chart still holds it.
    ///
    /// Undo can bring back a study the panel installed before a replace. Forgetting
    /// it would make the next Run add a duplicate instead of replacing it, so the
    /// panel remembers the ids it created — with the title and draft text of that
    /// install, so adopting one does not silently claim the current draft is live.
    installed_history: Vec<Installed>,
    /// Symbol and interval the scripts should see. Empty until the host sets it.
    context: RunContext,
    /// Whether the Logs body is open.
    logs_open: bool,
    /// Where the header's Run button was drawn last frame, for hosts and tests
    /// that need to press the real control instead of guessing a point.
    run_rect: Option<egui::Rect>,
    /// Why the current script is failing, cleared when a run or recalculation
    /// succeeds again.
    ///
    /// Kept apart from the installed title so a failure never reads as success,
    /// and cleared on recovery so a stale error does not outlive its cause.
    last_failure: Option<String>,
}

impl Default for OpineRustPanel {
    fn default() -> Self {
        let mut editor = StudyEditor::new();
        editor.open_document("My script", DEFAULT_SCRIPT);
        editor.set_io_actions(false);
        editor.set_apply_label("Run");
        // The dock draws its own header, status and Run action; repeating them in
        // the editor would be two places to read the same fact.
        editor.set_chrome(false);
        editor.set_line_numbers(true);
        editor.set_opinerust_syntax(true);
        editor.set_hover_docs(true);
        editor.set_context_menu(true);
        // Diagnostics are listed in the Logs strip, not under the code: two lists
        // of the same failures is what made the dock look broken.
        editor.set_inline_diagnostics(false);
        Self {
            editor,
            applied: None,
            applied_text: String::new(),
            logs: Vec::new(),
            applied_before: false,
            watching: None,
            logged_error: None,
            pending: None,
            installed: None,
            installed_history: Vec::new(),
            context: RunContext::default(),
            logs_open: false,
            run_rect: None,
            last_failure: None,
        }
    }
}

/// What the host should do with a successful Run.
pub struct InstallStudy {
    /// The study to install.
    pub study: Box<dyn Indicator>,
    /// Replace the previously applied study instead of adding another one.
    pub replace: bool,
    /// The slot where later recalculation failures are published.
    pub last_error: Arc<Mutex<Option<String>>>,
    /// Script title, for the host's status line.
    pub title: String,
}

/// A study this panel installed, kept so undo can hand it back.
#[derive(Clone, Debug)]
struct Installed {
    id: SeriesId,
    title: String,
    /// Draft text at that install, so adopting it later restores an honest state.
    text: String,
}

impl OpineRustPanel {
    /// The editor, for tests that drive it directly.
    #[cfg(test)]
    pub fn editor_mut(&mut self) -> &mut StudyEditor {
        &mut self.editor
    }

    /// The run log, for tests that check what the user is told.
    #[cfg(test)]
    pub fn logs(&self) -> Vec<String> {
        self.logs
            .iter()
            .map(|entry| entry.message.clone())
            .collect()
    }

    /// Title the panel currently claims to have applied, for tests.
    #[cfg(test)]
    pub fn applied_title(&self) -> Option<&str> {
        self.applied.as_deref()
    }

    /// Publish a prepared study on the chart as one undoable action.
    ///
    /// The candidate is added **before** the previous study is removed, so a
    /// refused candidate (the study limit) leaves the chart exactly as it was:
    /// there is no window in which the user's study is gone. The capacity check
    /// runs first so a chart already at its limit never reaches the mutation.
    /// The panel only says “Applied” after the chart accepted the study.
    pub fn install(
        &mut self,
        chart: &mut Chart,
        previous: Option<SeriesId>,
        prepared: InstallStudy,
    ) -> Result<SeriesId, String> {
        let InstallStudy {
            study,
            replace,
            last_error,
            title,
        } = prepared;
        self.pending = Some(title.clone());
        // Replacing frees a slot, so only an add can be refused for capacity.
        // During a replace the new study is added before the old is removed (so
        // there is never a frame without a study), which needs one temporary
        // slot; the limit is restored before returning.
        let replaces = replace && previous.is_some();
        if !replaces && chart.study_limit() == Some(chart.indicators().count()) {
            let reason = "Study limit reached".to_owned();
            self.commit_failed(reason.clone());
            return Err(reason);
        }
        let limit = chart.study_limit();
        if replaces && limit.is_some() {
            chart.set_study_limit(limit.map(|limit| limit + 1));
        }

        chart.begin_user_action();
        match chart.try_add_indicator(study) {
            Ok(id) => {
                // `add_indicator` computes the candidate through an infallible
                // closure, so `Ok(id)` only means "registered". A script that
                // preflighted fine can still fail on the chart's first
                // calculation (fewer bars than the preflight data, a different
                // context, an output shape that changed). Publishing "Applied"
                // there would tell the user a broken study is live. Roll the
                // candidate back and keep the previous study.
                if let Some(failure) = install_failure(chart, id) {
                    chart.remove_series(id);
                    // The candidate never became the applied study, so the whole
                    // action is discarded rather than pushed onto undo history.
                    // Otherwise the user would undo a failure into an empty slot.
                    chart.abandon_user_action();
                    chart.set_study_limit(limit);
                    self.commit_failed(failure.clone());
                    return Err(failure);
                }
                if let Some(old) = previous.filter(|_| replaces) {
                    chart.remove_series(old);
                }
                chart.end_user_action();
                chart.set_study_limit(limit);
                self.watch(last_error);
                self.installed = Some(id);
                self.installed_history.push(Installed {
                    id,
                    title: title.clone(),
                    text: self.editor.text().to_owned(),
                });
                self.confirm_installed(title, replaces);
                Ok(id)
            }
            Err(reason) => {
                // Nothing was removed yet: the refused candidate is the only
                // change and the chart keeps the study the user already had.
                chart.end_user_action();
                chart.set_study_limit(limit);
                self.commit_failed(reason.to_owned());
                Err(reason.to_owned())
            }
        }
    }

    /// Record that the host installed the study successfully. Only then does the
    /// panel say “Applied”.
    pub fn confirm_installed(&mut self, title: String, replaced: bool) {
        if replaced {
            self.log(Severity::Info, format!("Replaced “{title}”."));
        } else {
            self.log(Severity::Info, format!("Applied “{title}”."));
        }
        self.applied = Some(title);
        self.applied_text = self.editor.text().to_owned();
        self.last_failure = None;
        self.applied_before = true;
        self.pending = None;
        self.editor.apply_result(Ok(()));
    }

    /// The host could not install a prepared study. Nothing was applied, so the
    /// panel must not claim success.
    pub fn commit_failed(&mut self, reason: String) {
        self.last_failure = Some(reason.clone());
        self.log(Severity::Error, format!("Install failed: {reason}"));
        self.pending = None;
        self.editor.apply_result(Err(reason));
    }

    /// Reconcile the panel with a study the host believes is installed.
    ///
    /// Undo and redo change the chart without going through the panel: they can
    /// bring back a study this panel installed earlier (a different script, a
    /// different title) or remove the one it is watching. Keeping the old title
    /// would make the header claim "Applied" for a study that is not on the chart,
    /// and the next Run would then replace the *wrong* study. Declaring the study
    /// gone on a mismatch is the honest option: the draft and its text survive, and
    /// the next Run adds a study that matches what the editor shows.
    pub fn forget_missing_study(&mut self, chart: &Chart) {
        // Drop ids the chart no longer holds, so the list stays a set of studies the
        // chart can actually give back.
        self.installed_history
            .retain(|entry| chart.indicators().any(|(candidate, _)| candidate == entry.id));
        let on_chart = self.installed.and_then(|id| {
            self.installed_history
                .iter()
                .find(|entry| entry.id == id)
                .map(|entry| entry.title.clone())
        });
        let matches = match (&on_chart, &self.applied) {
            (Some(title), Some(applied)) => title == applied,
            // The host never recorded a title, so an existing study is all we can check.
            (Some(_), None) => true,
            _ => false,
        };
        if matches {
            return;
        }
        self.installed = None;
        if self.applied.take().is_some() {
            self.log(
                Severity::Info,
                "The chart no longer holds this script (undo or redo changed it). Run installs it again.",
            );
        }
        self.applied_before = false;
        self.applied_text.clear();
        self.watching = None;
        self.logged_error = None;
        // Adopt the study this panel created that the chart holds now, if any. This
        // is what makes a Run after an undo replace the restored study instead of
        // adding a second copy of it.
        if let Some(entry) = self
            .installed_history
            .iter()
            .rev()
            .find(|entry| chart.indicators().any(|(candidate, _)| candidate == entry.id))
            .cloned()
        {
            self.installed = Some(entry.id);
            self.applied = Some(entry.title.clone());
            self.applied_before = true;
            self.applied_text = entry.text;
            self.log(
                Severity::Info,
                format!("The chart holds “{}” again; Run replaces it.", entry.title),
            );
        }
    }

    /// The study id this panel installed, if the chart still holds it.
    ///
    /// The host uses this after undo/redo, which change the chart without going
    /// through the panel: the id it remembers can then belong to a study the chart
    /// no longer has.
    pub fn installed_study(&self, chart: &Chart) -> Option<SeriesId> {
        let id = self.installed?;
        chart
            .indicators()
            .any(|(candidate, _)| candidate == id)
            .then_some(id)
    }

    /// Follow the installed study so recalculation failures reach the logs.
    pub fn watch(&mut self, last_error: Arc<Mutex<Option<String>>>) {
        self.watching = Some(last_error);
        // A new study's slot starts empty; forget the previous one's message.
        self.logged_error = None;
    }

    /// The last recalculation failure, logged once.
    ///
    /// The slot is read, not emptied: the chart reads the same value to show a
    /// stale study, so a `take()` here would make the error disappear from the
    /// chart the moment the panel looked at it. `logged_error` tracks what has
    /// already been written to the log so one failure is not printed every frame.
    fn drain_recalculation_error(&mut self) {
        let Some(slot) = &self.watching else { return };
        let message = slot
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone();
        let Some(message) = message else {
            // The study calculated cleanly again: the failure it reported is over.
            if self.logged_error.take().is_some() {
                self.last_failure = None;
            }
            return;
        };
        if self.logged_error.as_deref() == Some(message.as_str()) {
            return;
        }
        self.last_failure = Some(message.clone());
        self.last_failure = Some(message.clone());
        self.log(Severity::Error, format!("Recalculation failed: {message}"));
        self.editor.apply_result(Err(message.clone()));
        self.logged_error = Some(message);
    }

    /// Draw the panel and return a study to install when Run succeeded.
    ///
    /// Also reports whether the user closed the panel, so the host owns the dock
    /// instead of the editor hiding itself.
    pub fn show_with_close(
        &mut self,
        ui: &mut egui::Ui,
        bars: &[ChartBar],
        now_ms: i64,
    ) -> (Option<InstallStudy>, bool) {
        self.drain_recalculation_error();
        let has_data = !bars.is_empty();
        let (mut run, close) = self.header(ui, has_data);
        // Only while the code editor has focus: typing must not be a chart
        // shortcut, and the shortcut must not fire from the chart or the header.
        // The key event carries its own modifiers, so this works even when the
        // pass-level modifier state was not updated.
        let editor_id = self.editor.text_id(ui);
        let editor_focused = ui.memory(|memory| memory.focused().is_some_and(|id| id == editor_id));
        let shortcut = editor_focused
            && ui.input(|input| {
                input.events.iter().any(|event| {
                    matches!(
                        event,
                        egui::Event::Key {
                            key: egui::Key::Enter,
                            pressed: true,
                            modifiers,
                            ..
                        } if modifiers.command || modifiers.ctrl
                    )
                })
            });
        run |= shortcut;
        run &= has_data;
        self.logs_ui_pinned(ui);
        if !has_data {
            ui.label(egui::RichText::new("Load a symbol to run the script.").weak());
        }
        
        // Studio Quick Action Shelf
        self.action_shelf(ui);

        // The draft stays editable even when the chart has no data.
        self.editor.set_apply_button(false);
        let _ = self.editor.show(ui);
        if run {
            let text = self.editor.text().to_owned();
            return (self.prepare(&text, bars, now_ms), close);
        }
        (None, close)
    }

    /// Draw the panel and return a study to install when Run succeeded.
    ///
    /// The demo host uses [`Self::show_with_close`] so it can honour the header's
    /// close button; this convenience form is kept for hosts that draw their own
    /// close affordance and for the panel's tests.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "the demo host uses show_with_close; other hosts and tests use this"
        )
    )]
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        bars: &[ChartBar],
        now_ms: i64,
    ) -> Option<InstallStudy> {
        self.show_with_close(ui, bars, now_ms).0
    }

    /// Pin the Logs to the bottom so an editor that fills the panel cannot push
    /// them out of view.
    fn logs_ui_pinned(&mut self, ui: &mut egui::Ui) {
        let height = logs_panel_height(self.logs_open, ui.available_height());
        egui::Panel::bottom("opinerust_logs")
            .exact_size(height)
            .resizable(false)
            .frame(egui::Frame::NONE)
            .show(ui, |ui| {
                ui.set_clip_rect(ui.max_rect());
                self.logs_ui(ui);
            });
    }

    /// Quick Action Shelf right above the code editor (Format code, copy, version pill, lines counter).
    fn action_shelf(&mut self, ui: &mut egui::Ui) {
        let theme = ui.active_theme();
        let version = if self.editor.text().contains("@version=6") {
            "Pine v6"
        } else if self.editor.text().contains("@version=5") {
            "Pine v5"
        } else {
            "Pine Script"
        };
        let line_count = self.editor.text().lines().count();

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;

            // Auto-format code button
            if Button::new("🪄 Format")
                .small()
                .tooltip("Format code indentation")
                .show(ui)
                .clicked()
            {
                self.editor.format_code();
            }

            // Copy text button
            if Button::new("📋 Copy")
                .small()
                .tooltip("Copy entire script to clipboard")
                .show(ui)
                .clicked()
            {
                ui.ctx().copy_text(self.editor.text().to_owned());
            }

            ui.separator();

            // Version Pill badge
            ui.label(
                egui::RichText::new(version)
                    .size(10.0)
                    .strong()
                    .color(theme.ui.accent)
                    .background_color(theme.ui.accent.gamma_multiply(0.12)),
            );

            // Line count info
            ui.label(
                egui::RichText::new(format!("{line_count} lines"))
                    .size(11.0)
                    .weak(),
            );
        });

        let (rule, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 1.0), egui::Sense::hover());
        ui.painter().hline(rule.x_range(), rule.center().y, egui::Stroke::new(1.0, theme.ui.border.gamma_multiply(0.4)));
        ui.add_space(theme.spacing.xs);
    }

    /// Chart metadata (`syminfo.*`, `timeframe.*`) the panel should report.
    ///
    /// The host sets this before drawing the panel. Leaving it unset keeps the
    /// previous behaviour: the script still runs, with empty metadata.
    pub fn set_context(&mut self, context: RunContext) {
        self.context = context;
    }

    /// Centre of the header's Run button, once it has been drawn.
    ///
    /// Exposed so a host or test can press the real control instead of guessing a
    /// point; the demo's own frame tests are the current caller.
    #[cfg(test)]
    pub fn run_button_center(&self) -> Option<egui::Pos2> {
        self.run_rect.map(|rect| rect.center())
    }

    /// What the panel claims about the script right now.
    fn run_state(&self) -> RunState {
        // A failure is the current truth about the chart: whatever title was
        // installed before is not what the user should read as live.
        if let Some(reason) = &self.last_failure {
            return RunState::Failed(reason.clone());
        }
        let Some(title) = self.applied.clone() else {
            return RunState::Draft;
        };
        if self.editor.text() != self.applied_text {
            RunState::Edited(title)
        } else {
            RunState::Applied(title)
        }
    }

    /// The single header: identity, run state, Run and Close.
    ///
    /// Returns `(run, close)`. Run and the shortcut share the flag so both go
    /// through the host's install path; close is the host's to act on.
    fn header(&mut self, ui: &mut egui::Ui, has_data: bool) -> (bool, bool) {
        let mut run = false;
        let mut close = false;
        let theme = ui.active_theme();
        let state = if has_data { self.run_state() } else { RunState::NoData };
        let (label, tooltip) = match &state {
            RunState::Draft => ("Draft".to_owned(), "Not applied yet"),
            RunState::Applied(title) => (title.clone(), "Running on the chart"),
            RunState::Edited(title) => (
                format!("{title} · edited"),
                "The chart still runs the previous script",
            ),
            RunState::Failed(reason) => ("Run failed".to_owned(), reason.as_str()),
            RunState::NoData => ("No data".to_owned(), "Load a symbol before running"),
        };
        // Allocate the actions first, so a long script title cannot push them out
        // of the panel. Use the same buttons, typography and hairline as Oracly.
        ui.allocate_ui_with_layout(
            egui::vec2(ui.available_width(), 44.0),
            egui::Layout::right_to_left(egui::Align::Center),
            |ui| {
                close = IconButton::new(&icons::CLOSE)
                    .size(ButtonSize::SM)
                    .tooltip("Close OpineRust editor")
                    .show(ui)
                    .clicked();
                let run_button = Button::new("Run")
                    .primary()
                    .small()
                    .enabled(has_data)
                    .tooltip(if has_data { "Run script · Ctrl/Command+Enter" } else { "Load a symbol to run" })
                    .show(ui);
                self.run_rect = Some(run_button.rect);
                run = run_button.clicked();
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    ui.spacing_mut().item_spacing.x = theme.spacing.sm;
                    ui.label(egui::RichText::new("OpineRust").strong().size(theme.text.title));
                    let (color, bg) = match state {
                        RunState::Failed(_) => (ui.visuals().error_fg_color, ui.visuals().error_fg_color.gamma_multiply(0.12)),
                        RunState::Applied(_) => (egui::Color32::from_rgb(8, 153, 129), egui::Color32::from_rgb(8, 153, 129).gamma_multiply(0.12)),
                        RunState::Edited(_) => (egui::Color32::from_rgb(255, 152, 0), egui::Color32::from_rgb(255, 152, 0).gamma_multiply(0.12)),
                        _ => (theme.ui.text_muted, egui::Color32::TRANSPARENT),
                    };
                    ui.label(
                        egui::RichText::new(format!(" {label} "))
                            .size(theme.text.small)
                            .color(color)
                            .background_color(bg)
                    ).on_hover_text(format!("{label} — {tooltip}"));
                });
            },
        );
        let (rule, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 1.0), egui::Sense::hover());
        ui.painter().hline(rule.x_range(), rule.center().y, egui::Stroke::new(1.0, theme.ui.border.gamma_multiply(0.5)));
        ui.add_space(theme.spacing.xs);
        (run, close)
    }

    /// The Logs summary row, plus the body when open.
    ///
    /// Collapsed it costs one row: an empty 110 px strip pushed the code out of the
    /// panel for no reason.
    fn logs_ui(&mut self, ui: &mut egui::Ui) {
        let errors = self
            .logs
            .iter()
            .filter(|entry| entry.severity == Severity::Error)
            .count();
        let warnings = self
            .logs
            .iter()
            .filter(|entry| entry.severity == Severity::Warning)
            .count();
        let mut summary = String::from("Logs");
        if errors > 0 {
            summary.push_str(&format!(" · {errors} error"));
            if errors > 1 {
                summary.push('s');
            }
        }
        if warnings > 0 {
            summary.push_str(&format!(" · {warnings} warning"));
            if warnings > 1 {
                summary.push('s');
            }
        }
        let theme = ui.active_theme();
        let (rule, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 1.0), egui::Sense::hover());
        ui.painter().hline(rule.x_range(), rule.center().y, egui::Stroke::new(1.0, theme.ui.border.gamma_multiply(0.5)));
        let icon = if self.logs_open { &icons::CHEVRON_DOWN } else { &icons::CHEVRON_UP };
        ui.horizontal(|ui| {
            if Button::icon_and_text(icon, summary)
                .small()
                .tooltip("Show or hide the run log")
                .show(ui)
                .clicked()
            {
                self.logs_open = !self.logs_open;
            }
            if self.logs_open && !self.logs.is_empty() {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if Button::new("Clear")
                        .small()
                        .tooltip("Clear log entries")
                        .show(ui)
                        .clicked()
                    {
                        self.logs.clear();
                    }
                });
            }
        });
        if !self.logs_open {
            return;
        }
        let height = ui.available_height().max(60.0);
        egui::ScrollArea::vertical()
            .id_salt("opinerust_log_scroll")
            .max_height(height)
            .auto_shrink([false; 2])
            .stick_to_bottom(true)
            .show(ui, |ui| {
                if self.logs.is_empty() {
                    ui.label(egui::RichText::new("No runs yet.").weak());
                }
                let mut clicked = None;
                for entry in &self.logs {
                    let (color, label) = match entry.severity {
                        Severity::Error => (ui.visuals().error_fg_color, "Error"),
                        Severity::Warning => (ui.visuals().warn_fg_color, "Warning"),
                        // `Severity` is non-exhaustive upstream: an unknown level
                        // is shown as a hint rather than dropped.
                        Severity::Info | _ => (ui.visuals().weak_text_color(), "Info"),
                    };
                    let row = ui.horizontal_wrapped(|ui| {
                        ui.label(
                            egui::RichText::new(format!(" {label} "))
                                .size(10.0)
                                .color(color)
                                .background_color(color.gamma_multiply(0.12)),
                        );
                        let text = ui.add(
                            egui::Label::new(&entry.message)
                                .sense(if entry.span.is_some() {
                                    egui::Sense::click()
                                } else {
                                    egui::Sense::hover()
                                })
                                .wrap(),
                        );
                        if entry.span.is_some() {
                            text.on_hover_text("Go to the reported line")
                        } else {
                            text
                        }
                    });
                    if entry.span.is_some() && row.inner.clicked() {
                        clicked = entry.span.clone();
                    }
                }
                if let Some(span) = clicked {
                    self.editor.reveal(span);
                    ui.ctx().request_repaint();
                }
            });
    }

    /// Append a log entry.
    fn log(&mut self, severity: Severity, message: impl Into<String>) {
        self.log_at(severity, message, None);
    }

    /// Append a log entry that points at a place in the source.
    fn log_at(
        &mut self,
        severity: Severity,
        message: impl Into<String>,
        span: Option<Range<usize>>,
    ) {
        self.logs.push(LogEntry {
            severity,
            message: message.into(),
            span,
        });
        // A failure is the reason the panel matters; open the log so it is not
        // hidden behind a collapsed row the user has to discover.
        if severity == Severity::Error {
            self.logs_open = true;
        }
    }

    /// Compile and run `source`, returning the prepared study on success.
    ///
    /// This is the exact path the host uses; tests drive it directly. The panel
    /// does not claim success here: the host calls [`Self::confirm_installed`]
    /// after `Chart::add_indicator` returns.
    pub fn prepare(
        &mut self,
        source: &str,
        bars: &[ChartBar],
        now_ms: i64,
    ) -> Option<InstallStudy> {
        // Recalculations use the host clock, so a still-forming candle reports
        // `barstate.isrealtime` and a closed one reports confirmed history. The
        // chart's symbol and interval travel with the study so `syminfo.*` and
        // `timeframe.*` read real values.
        match build_study_in_context(
            source,
            bars,
            now_ms,
            self.context.clone(),
            Some(host_clock()),
        ) {
            Ok(built) => {
                let title = built.info.title.clone();
                let lines = built.study.line_names().join(", ");
                self.log(
                    Severity::Info,
                    format!(
                        "Prepared “{title}” ({} line(s): {lines}).",
                        built.study.line_count()
                    ),
                );
                for diagnostic in built.diagnostics {
                    self.log(
                        severity_of(diagnostic.severity),
                        diagnostic.message.clone(),
                    );
                }
                // Diagnostics live in the Logs strip, so the editor keeps none of
                // its own; two lists showed the same failure twice.
                self.editor.set_diagnostics(Vec::new());
                self.pending = Some(title.clone());
                Some(InstallStudy {
                    study: Box::new(built.study),
                    replace: self.applied_before,
                    last_error: built.last_error,
                    title,
                })
            }
            Err(error) => {
                let (message, edits) = describe(error, source);
                self.last_failure = Some(message.clone());
                // One grouped entry per problem, each carrying every location, so a
                // script with nine `line` errors reads as one line, not nine. The
                // grouped entries already say what failed, so the bare summary is
                // only added when the runtime gave no located diagnostic at all.
                let groups = group_diagnostics(&edits);
                if groups.is_empty() {
                    self.log(Severity::Error, format!("Run failed: {message}"));
                } else {
                    for group in &groups {
                        self.log_at(Severity::Error, group.summary(source), group.span());
                    }
                    // A closing line that names the run's outcome once, without
                    // repeating any group's message.
                    let count = groups.len();
                    self.log(
                        Severity::Error,
                        if count == 1 {
                            format!("Run failed: {message}")
                        } else {
                            format!("Run failed · {count} distinct problems")
                        },
                    );
                }
                // Diagnostics live in the Logs strip; the editor keeps none, so the
                // same failure is not painted twice.
                self.editor.set_diagnostics(Vec::new());
                self.editor.apply_result(Err(message));
                None
            }
        }
    }
}

/// Height the pinned Logs area asks for.
///
/// Collapsed it is the summary row alone; an empty strip of a fixed height used to
/// push the code out of a short panel. Expanded it adds the body, capped so the
/// editor keeps most of the panel.
fn logs_panel_height(open: bool, available: f32) -> f32 {
    let summary = 32.0_f32;
    if !open {
        return summary.min(available);
    }
    (summary + 140.0).min(available * 0.6).max(summary.min(available))
}

/// The host clock the engine reads on recalculation, in UNIX milliseconds.
fn host_clock() -> Arc<dyn Fn() -> i64 + Send + Sync> {
    Arc::new(|| chrono::Utc::now().timestamp_millis())
}

/// The editor's severity for an engine diagnostic, so one log list can count them.
fn severity_of(severity: DiagnosticSeverity) -> Severity {
    match severity {
        DiagnosticSeverity::Info => Severity::Info,
        DiagnosticSeverity::Warning => Severity::Warning,
        DiagnosticSeverity::Error => Severity::Error,
    }
}

/// Diagnostics that share one cause, so the log shows one line instead of many.
///
/// A script that uses `box` in six places produced six identical errors, which
/// buried everything else. Entries are grouped by message text: identical text means
/// one cause reported at several places. Different messages stay separate even when
/// they look similar, because merging them would hide a distinct problem.
#[derive(Clone, Debug, PartialEq)]
struct DiagnosticGroup {
    message: String,
    /// One location per occurrence, in source order.
    spans: Vec<Range<usize>>,
}

impl DiagnosticGroup {
    /// Group `diagnostics` in first-seen order, keeping each occurrence's span.
    fn group(diagnostics: &[EditorDiagnostic]) -> Vec<Self> {
        let mut groups: Vec<Self> = Vec::new();
        for diagnostic in diagnostics {
            match groups
                .iter_mut()
                .find(|group| group.message == diagnostic.message)
            {
                Some(group) => group.spans.push(diagnostic.span.clone()),
                None => groups.push(Self {
                    message: diagnostic.message.clone(),
                    spans: vec![diagnostic.span.clone()],
                }),
            }
        }
        groups
    }

    /// One readable line: the message, with a count and the first line when it repeats.
    fn summary(&self, source: &str) -> String {
        match self.spans.len() {
            1 => self.message.clone(),
            count => format!(
                "{} ({} locations, first at line {})",
                self.message,
                count,
                line_of(source, self.spans[0].start)
            ),
        }
    }

    /// The first occurrence, which a click in the log navigates to.
    fn span(&self) -> Option<Range<usize>> {
        self.spans.first().cloned()
    }
}

/// Group editor diagnostics for the log.
fn group_diagnostics(diagnostics: &[EditorDiagnostic]) -> Vec<DiagnosticGroup> {
    DiagnosticGroup::group(diagnostics)
}

/// One-based line number of a byte offset, for a log summary.
fn line_of(source: &str, byte: usize) -> usize {
    let byte = byte.min(source.len());
    source[..byte].bytes().filter(|b| *b == b'\n').count() + 1
}

/// The failure a freshly installed study reports, if its first calculation failed.
///
/// `Chart::study_status` answers "what does the chart have to say about this
/// study" and reports `WaitingForData` while values are still missing, so the
/// study's own status is the one that distinguishes a failed calculation from a
/// study that simply has nothing to draw yet. Reads without mutating, so no undo
/// step is recorded.
fn install_failure(chart: &Chart, id: SeriesId) -> Option<String> {
    use egui_charts::studies::StudyStatus;
    match chart.indicators().find(|(candidate, _)| *candidate == id)?.1.study_status() {
        Some(StudyStatus::DataError { message }) => Some(message),
        _ => None,
    }
}

fn describe(error: StudyBuildError, source: &str) -> (String, Vec<EditorDiagnostic>) {
    let engine = match error {
        StudyBuildError::NoPlots => {
            return (
                "The script declares no plot(...) output.".to_owned(),
                Vec::new(),
            );
        }
        StudyBuildError::Engine(engine) => engine,
        StudyBuildError::InvalidInputs => {
            // The chart refused the settings schema built from the script's
            // `input.*` declarations. The script itself compiled, so this is a
            // host-side mapping problem, not a user script error.
            return (
                "The script's input declarations could not become chart settings.".to_owned(),
                Vec::new(),
            );
        }
    };
    let diagnostics = match engine {
        opinerust::EngineError::Compile(list) => list,
        opinerust::EngineError::Runtime(diagnostic) => vec![diagnostic],
        opinerust::EngineError::NoBars => {
            return ("The chart has no bars to run over.".to_owned(), Vec::new());
        }
        opinerust::EngineError::Internal(message) => return (message, Vec::new()),
    };
    let message = diagnostics
        .first()
        .map(|diagnostic| diagnostic.message.clone())
        .unwrap_or_else(|| "The script could not run.".to_owned());
    let edits = diagnostics
        .into_iter()
        .filter_map(|diagnostic| {
            let line = diagnostic.line?;
            Some(EditorDiagnostic {
                severity: Severity::Error,
                message: diagnostic.message,
                span: source_span(source, line, diagnostic.column),
            })
        })
        .collect();
    (message, edits)
}

/// Map a one-based line (and optional column) to a byte span in `source`.
///
/// Lines split on `\n`; a trailing `\r` is not part of the line, so a CRLF
/// document reveals the text and not the newline. Out-of-range lines clamp to
/// the end of the text, and the span always lands on character boundaries.
pub fn source_span(source: &str, line: u32, column: Option<u32>) -> Range<usize> {
    let start = line_start(source, line);
    let end = match source[start..].find('\n') {
        Some(offset) => {
            let mut end = start + offset;
            // A CRLF's `\r` belongs to the line ending, not the line.
            if end > start && source.as_bytes().get(end - 1) == Some(&b'\r') {
                end -= 1;
            }
            end
        }
        None => source.len(),
    };
    let column = column
        .filter(|column| *column > 0)
        .map(|column| column as usize - 1);
    if let Some(column) = column {
        let offset = char_offset(source, start, end, column);
        let next = source[offset..]
            .chars()
            .next()
            .map_or(offset, |character| offset + character.len_utf8());
        return offset..next;
    }
    start..end
}

/// Byte offset of a one-based line, clamped to the text.
fn line_start(source: &str, line: u32) -> usize {
    if line <= 1 {
        return 0;
    }
    let mut remaining = line as usize - 1;
    for (offset, byte) in source.bytes().enumerate() {
        if byte == b'\n' {
            remaining -= 1;
            if remaining == 0 {
                return offset + 1;
            }
        }
    }
    source.len()
}

/// Byte offset `column` characters into `start..end`, clamped and boundary-safe.
fn char_offset(source: &str, start: usize, end: usize, column: usize) -> usize {
    source[start..end]
        .char_indices()
        .nth(column)
        .map_or(end, |(offset, _)| start + offset)
}

#[cfg(test)]
mod tests {
    //! Tests for the OpineRust panel: run/replace lifecycle and diagnostic spans.

    use super::*;
    use egui_charts::Chart;
    use egui_charts::model::{Bar, BarData};
    use egui_charts::widget::SeriesId;
    use opinerust::build_study;

    fn bars(count: usize) -> Vec<Bar> {
        (0..count)
            .map(|i| {
                let close = 100.0 + (i as f64).sin() * 5.0 + i as f64 * 0.1;
                Bar::new(
                    chrono::DateTime::UNIX_EPOCH + chrono::TimeDelta::minutes(i as i64),
                    close,
                    close + 1.0,
                    close - 1.0,
                    close,
                    10.0,
                )
            })
            .collect()
    }

    fn install(chart: &mut Chart, panel: &mut OpineRustPanel, prepared: InstallStudy) -> SeriesId {
        panel
            .install(chart, None, prepared)
            .expect("the sample study installs")
    }

    /// A study whose first on-chart calculation fails after a successful
    /// preflight. Preflight runs over ten bars; the chart then holds none, which
    /// the engine rejects with `NoBars` instead of producing a plot.
    fn study_failing_on_install(panel: &mut OpineRustPanel) -> InstallStudy {
        let script =
            "//@version=5\nindicator(\"Fragile\", overlay = true)\nplot(close, title = \"C\")\n";
        panel
            .prepare(script, &bars(10), 0)
            .expect("the script preflights over 10 bars")
    }

    /// A candidate that fails its first calculation must not be reported as
    /// applied, and the study it was replacing must stay on the chart.
    #[test]
    fn a_failed_first_calculation_keeps_the_previous_study() {
        let bars = bars(10);
        let mut chart = Chart::new(BarData::from_bars(bars.clone()));
        let mut panel = OpineRustPanel::default();
        let good = panel
            .prepare(
                "//@version=5\nindicator(\"Good\", overlay = true)\nplot(close, title = \"C\")\n",
                &bars,
                0,
            )
            .expect("runs");
        let first = install(&mut chart, &mut panel, good);
        let edits = chart.edit_count();

        // Preflight succeeds over ten bars, but the chart now has none to calculate over.
        chart.set_data(SeriesId::MAIN, Vec::new());
        let fragile = study_failing_on_install(&mut panel);

        let result = panel.install(&mut chart, Some(first), fragile);
        assert!(result.is_err(), "a failed calculation must not install");
        assert!(
            chart.indicators().any(|(id, _)| id == first),
            "the previous study must stay on the chart"
        );
        assert_eq!(
            chart.indicators().count(),
            1,
            "the failed candidate must not be left behind"
        );
        assert_eq!(
            chart.edit_count(),
            edits,
            "a discarded install must not leave an undo step"
        );
        assert_eq!(
            panel.applied.as_deref(),
            Some("Good"),
            "the failed candidate must not replace the applied title"
        );
        assert!(
            !panel.logs().iter().any(|line| line.contains("Applied “Fragile”")),
            "the failed candidate must never be announced as applied: {:?}",
            panel.logs
        );
        assert!(
            panel.logs().iter().any(|line| line.contains("Install failed")),
            "{:?}",
            panel.logs
        );
    }

    #[test]
    fn default_script_is_v5_and_applies() {
        assert!(DEFAULT_SCRIPT.starts_with("//@version=5"));
        let built = build_study(DEFAULT_SCRIPT, &bars(20), 0).expect("sample runs");
        assert_eq!(built.study.line_count(), 2);
        assert!(built.study.is_overlay());
    }

    /// A script using one unsupported feature in many places must read as one log
    /// line, not one line per occurrence: multiple import errors used to bury the cause
    /// and overflow the strip.
    #[test]
    fn repeated_unsupported_uses_collapse_into_one_log_entry() {
        let mut panel = OpineRustPanel::default();
        let mut script = String::from("//@version=6\nindicator(\"Imports\")\nplot(close)\n");
        for i in 0..6 {
            script.push_str(&format!("import username/lib{i}/1\n"));
        }
        script.push_str("strategy(\"bad\")\n");
        assert!(panel.prepare(&script, &bars(10), 0).is_none());

        let import_entries: Vec<_> = panel
            .logs()
            .into_iter()
            .filter(|entry| entry.contains("`import` is not supported"))
            .collect();
        assert_eq!(
            import_entries.len(),
            1,
            "six `import` uses are one cause: {:?}",
            panel.logs()
        );
        assert!(
            import_entries[0].contains("6 locations"),
            "the entry says how often it occurred: {:?}",
            import_entries
        );
        assert!(
            import_entries[0].contains("first at line 4"),
            "and where to look first: {:?}",
            import_entries
        );
        assert!(
            panel
                .logs()
                .iter()
                .any(|entry| entry.contains("`strategy` scripts are not supported")),
            "a different cause keeps its own entry: {:?}",
            panel.logs()
        );
    }

    /// Diagnostics belong in the Logs strip; the editor must not draw a second copy
    /// under the code, which is what overlapped the strip in a long panel.
    #[test]
    fn a_failed_run_lists_diagnostics_only_in_the_logs() {
        let mut panel = OpineRustPanel::default();
        let script = "//@version=6\nindicator(\"Bad\")\nplot(close)\nx = request.security(\"X\", \"D\", close)\n";
        assert!(panel.prepare(script, &bars(10), 0).is_none());
        assert!(
            panel.editor.diagnostics().is_empty(),
            "the dock shows diagnostics in Logs, so the editor keeps none: {:?}",
            panel.editor.diagnostics()
        );
        assert!(
            panel.logs_open,
            "the log opens itself so the failure is not hidden"
        );
    }

    #[test]
    fn a_failed_run_reports_errors_and_yields_no_study() {
        let mut panel = OpineRustPanel::default();
        let broken = "//@version=5\nindicator(\"Broken\"\nplot(close\n";
        assert!(panel.prepare(broken, &bars(10), 0).is_none());
        assert!(panel.applied.is_none(), "nothing is applied on failure");
        assert!(
            panel.logs().iter().any(|line| line.contains("Run failed")),
            "{:?}",
            panel.logs
        );

        let good = "//@version=5\nindicator(\"Good\")\nplot(close, title = \"C\")\n";
        let prepared = panel.prepare(good, &bars(10), 0).expect("runs");
        assert!(
            panel.applied.is_none(),
            "preparation alone must not claim Applied"
        );
        assert!(!prepared.replace, "the first install adds a study");
    }

    #[test]
    fn a_second_successful_run_asks_for_replace() {
        let mut panel = OpineRustPanel::default();
        let good = "//@version=5\nindicator(\"Good\")\nplot(close, title = \"C\")\n";
        let first = panel.prepare(good, &bars(8), 0).expect("runs");
        assert!(!first.replace);
        panel.confirm_installed(first.title.clone(), false);
        let again = panel.prepare(good, &bars(8), 0).expect("runs");
        assert!(again.replace, "a later Run replaces instead of adding");
    }

    /// The whole host lifecycle: first Run adds, a later Run replaces, and a failing
    /// Run keeps the chart exactly as it was, including its undo history.
    #[test]
    fn rerunning_replaces_the_study_and_failures_keep_it() {
        let bars = bars(40);
        let mut chart = Chart::new(BarData::from_bars(bars.clone()));
        let mut panel = OpineRustPanel::default();
        let baseline = chart.indicators().count();

        let edits = chart.edit_count();
        assert!(
            panel
                .prepare("//@version=5\nindicator(\"Broken\"\nplot(close\n", &bars, 0)
                .is_none()
        );
        assert_eq!(chart.indicators().count(), baseline);
        assert_eq!(chart.edit_count(), edits, "a failure records no undo step");

        let one = panel
            .prepare(
                "//@version=5\nindicator(\"One\", overlay = true)\nplot(close, title = \"C\")\n",
                &bars,
                0,
            )
            .expect("runs");
        let first_id = install(&mut chart, &mut panel, one);
        assert_eq!(chart.indicators().count(), baseline + 1);

        let two = panel
            .prepare(
                "//@version=5\nindicator(\"Two\", overlay = true)\nplot(open, title = \"C\")\n",
                &bars,
                0,
            )
            .expect("runs");
        assert!(two.replace, "the second run must replace");
        let second_id = panel
            .install(&mut chart, Some(first_id), two)
            .expect("the replacement installs");
        assert_eq!(
            chart.indicators().count(),
            baseline + 1,
            "no duplicate study"
        );
        assert_ne!(second_id, first_id);

        let after_edits = chart.edit_count();
        assert!(
            panel
                .prepare("//@version=5\nindicator(\"Broken\"\nplot(close\n", &bars, 0)
                .is_none()
        );
        assert_eq!(chart.indicators().count(), baseline + 1);
        assert_eq!(chart.edit_count(), after_edits);
        let (_, last) = chart.indicators().last().expect("study present");
        assert_eq!(last.name(), "Two");
    }

    #[test]
    fn undo_restores_the_previous_study_after_a_replace() {
        let bars = bars(30);
        let mut chart = Chart::new(BarData::from_bars(bars.clone()));
        let mut panel = OpineRustPanel::default();
        let baseline = chart.indicators().count();

        let one = panel
            .prepare(
                "//@version=5\nindicator(\"One\", overlay = true)\nplot(close, title = \"C\")\n",
                &bars,
                0,
            )
            .expect("runs");
        let first_id = install(&mut chart, &mut panel, one);

        let two = panel
            .prepare(
                "//@version=5\nindicator(\"Two\", overlay = true)\nplot(open, title = \"C\")\n",
                &bars,
                0,
            )
            .expect("runs");
        let second_id = panel
            .install(&mut chart, Some(first_id), two)
            .expect("the replacement installs");
        assert_eq!(chart.indicators().count(), baseline + 1);

        // One replace is one undo step: the old study comes back and the new goes.
        assert!(chart.undo(), "replace must be undoable");
        assert_eq!(chart.indicators().count(), baseline + 1);
        assert_eq!(
            chart.indicators().last().expect("restored study").1.name(),
            "One",
            "undo restores the replaced study"
        );
        assert!(chart.redo(), "replace must be redoable");
        assert_eq!(
            chart.indicators().last().expect("redone study").1.name(),
            "Two"
        );
        let _ = second_id;
    }

    #[test]
    fn a_chart_at_its_study_limit_keeps_the_existing_study() {
        let bars = bars(10);
        let mut chart = Chart::new(BarData::from_bars(bars.clone()));
        chart.set_study_limit(Some(1));
        let mut panel = OpineRustPanel::default();
        let one = panel
            .prepare(
                "//@version=5\nindicator(\"One\", overlay = true)\nplot(close, title = \"C\")\n",
                &bars,
                0,
            )
            .expect("runs");
        let first = install(&mut chart, &mut panel, one);
        assert_eq!(chart.indicators().count(), 1);

        // A different study that is not a replacement must be refused, and the
        // chart must keep the study the user already had.
        let extra = panel
            .prepare(
                "//@version=5\nindicator(\"Extra\", overlay = true)\nplot(open, title = \"C\")\n",
                &bars,
                0,
            )
            .expect("runs");
        // Pretend the host lost track of the previous id so the panel reports "add".
        panel.applied_before = false;
        let error = panel
            .install(&mut chart, None, extra)
            .expect_err("the limit refuses the study");
        assert!(error.contains("limit"), "{error}");
        assert_eq!(chart.indicators().count(), 1);
        assert_eq!(chart.indicators().next().expect("study kept").0, first);
        assert_eq!(
            panel.applied.as_deref(),
            Some("One"),
            "a refused install leaves the applied title alone"
        );
        assert!(
            panel
                .logs()
                .iter()
                .any(|line| line.contains("Install failed: Study limit reached")),
            "{:?}",
            panel.logs
        );
    }

    /// A replace works even when the chart is exactly at its study limit: the new
    /// study takes the old one's place instead of being refused for capacity.
    #[test]
    fn a_replace_succeeds_at_the_study_limit() {
        let bars = bars(10);
        let mut chart = Chart::new(BarData::from_bars(bars.clone()));
        chart.set_study_limit(Some(1));
        let mut panel = OpineRustPanel::default();
        let one = panel
            .prepare(
                "//@version=5\nindicator(\"One\", overlay = true)\nplot(close, title = \"C\")\n",
                &bars,
                0,
            )
            .expect("runs");
        let first = install(&mut chart, &mut panel, one);
        let two = panel
            .prepare(
                "//@version=5\nindicator(\"Two\", overlay = true)\nplot(open, title = \"C\")\n",
                &bars,
                0,
            )
            .expect("runs");
        assert!(two.replace);
        let second = panel
            .install(&mut chart, Some(first), two)
            .expect("a replace is not blocked by the limit");
        assert_eq!(chart.indicators().count(), 1);
        assert_eq!(chart.indicators().next().expect("study").1.name(), "Two");
        assert_eq!(chart.study_limit(), Some(1), "the limit is restored");
        let _ = second;
    }

    /// The panel draws in a real egui pass: the logs strip keeps its height, the
    /// editor does not eat it, and Ctrl+Enter runs the same path as the button.
    #[test]
    fn the_panel_draws_and_ctrl_enter_runs() {
        let ctx = egui::Context::default();
        let bars = bars(20);
        let mut panel = OpineRustPanel::default();
        let mut installed = false;
        let frame = |events: Vec<egui::Event>, panel: &mut OpineRustPanel| {
            let mut ran = false;
            ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(420.0, 700.0),
                    )),
                    events,
                    ..Default::default()
                },
                |ui| {
                    if panel.show(ui, &bars, 0).is_some() {
                        ran = true;
                    }
                },
            )
            .drop_without_applying_deltas();
            ran
        };

        // A plain pass must not run anything.
        assert!(!frame(Vec::new(), &mut panel));
        assert!(panel.logs().is_empty(), "opening the panel runs nothing");

        // Ctrl+Enter is the Run shortcut, and it only fires while the code editor
        // has focus: a click inside the text is what gives it that focus.
        let click_at = |pos: egui::Pos2, panel: &mut OpineRustPanel| {
            let events = vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: Default::default(),
                },
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers: Default::default(),
                },
            ];
            ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(420.0, 700.0),
                    )),
                    events,
                    ..Default::default()
                },
                |ui| {
                    panel.show(ui, &bars, 0);
                },
            )
            .drop_without_applying_deltas();
        };
        // Two warm-up passes so the text area has a real rect, then click into it.
        frame(Vec::new(), &mut panel);
        frame(Vec::new(), &mut panel);
        click_at(egui::pos2(120.0, 150.0), &mut panel);
        frame(Vec::new(), &mut panel);

        let enter = vec![egui::Event::Key {
            key: egui::Key::Enter,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::CTRL,
        }];
        installed |= frame(enter, &mut panel);
        assert!(installed, "Ctrl+Enter must prepare the study");
        assert!(
            panel.logs().iter().any(|line| line.contains("Prepared")),
            "{:?}",
            panel.logs
        );
    }

    /// Closing and reopening the panel keeps the draft: the panel is a dock, not
    /// a modal that throws the text away.
    #[test]
    fn closing_the_panel_keeps_the_draft() {
        let mut panel = OpineRustPanel::default();
        panel
            .editor_mut()
            .replace_text("//@version=5\nindicator(\"Draft\")\nplot(close)\n");
        let draft = panel.editor.text().to_owned();
        // The host closes the panel by clearing `open_panel`; the panel struct and
        // its editor are untouched, which this models: reopening shows the draft.
        assert_eq!(panel.editor.text(), draft);
        assert!(panel.editor.is_modified(), "the draft is still unsaved");
    }

    /// The Run button in the panel header installs a study, and running the same
    /// document again replaces it instead of adding a second one. This drives the
    /// real widget tree and the real chart, not the prepare path alone.
    #[test]
    fn the_run_button_installs_then_replaces() {
        let ctx = egui::Context::default();
        let bars = bars(40);
        let mut chart = Chart::new(BarData::from_bars(bars.clone()));
        let mut panel = OpineRustPanel::default();
        let mut installed: Option<SeriesId> = None;
        let baseline = chart.indicators().count();

        let mut frame = |panel: &mut OpineRustPanel, chart: &mut Chart, pos: Option<egui::Pos2>| {
            let mut prepared = None;
            let mut events = Vec::new();
            if let Some(pos) = pos {
                events.push(egui::Event::PointerMoved(pos));
                events.push(egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: Default::default(),
                });
                events.push(egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers: Default::default(),
                });
            }
            ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(460.0, 780.0),
                    )),
                    events,
                    ..Default::default()
                },
                |ui| {
                    if let Some(install) = panel.show(ui, &bars, 0) {
                        prepared = Some(install);
                    }
                },
            )
            .drop_without_applying_deltas();
            if let Some(prepared) = prepared {
                installed = Some(
                    panel
                        .install(chart, installed, prepared)
                        .expect("the chart accepts the study"),
                );
            }
        };

        // Warm up so widget positions exist, then click the first header button.
        frame(&mut panel, &mut chart, None);
        // Run is the rightmost action in the header; the close button sits beside it.
        let run_button = egui::pos2(420.0, 18.0);
        frame(&mut panel, &mut chart, Some(run_button));
        assert_eq!(
            chart.indicators().count(),
            baseline + 1,
            "the Run button installs one study"
        );
        frame(&mut panel, &mut chart, Some(run_button));
        assert_eq!(
            chart.indicators().count(),
            baseline + 1,
            "a second Run replaces the study"
        );
    }

    #[test]
    fn a_script_without_plots_yields_no_study() {
        let mut panel = OpineRustPanel::default();
        let source = "//@version=5\nindicator(\"Empty\")\nx = close\n";
        assert!(panel.prepare(source, &bars(5), 0).is_none());
        assert!(
            panel.logs().iter().any(|line| line.contains("no plot")),
            "{:?}",
            panel.logs
        );
    }

    #[test]
    fn an_unsupported_script_is_rejected_with_its_reason() {
        let mut panel = OpineRustPanel::default();
        let source = "//@version=5\nstrategy(\"s\")\n";
        assert!(panel.prepare(source, &bars(5), 0).is_none());
        assert!(
            panel
                .logs()
                .iter()
                .any(|line| line.contains("`strategy` scripts are not supported")),
            "{:?}",
            panel.logs
        );
    }

    #[test]
    fn a_commit_failure_does_not_claim_applied() {
        let mut panel = OpineRustPanel::default();
        let good = "//@version=5\nindicator(\"Good\")\nplot(close, title = \"C\")\n";
        assert!(panel.prepare(good, &bars(5), 0).is_some());
        panel.commit_failed("chart refused the study".to_owned());
        assert!(panel.applied.is_none(), "a failed install is not Applied");
        assert!(
            panel
                .logs()
                .iter()
                .any(|line| line.contains("Install failed"))
        );
    }

    #[test]
    fn an_edit_after_apply_is_reported_as_edited() {
        let mut panel = OpineRustPanel::default();
        let good = "//@version=5\nindicator(\"Good\")\nplot(close, title = \"C\")\n";
        panel.prepare(good, &bars(5), 0).expect("runs");
        panel.confirm_installed("Good".to_owned(), false);
        let edited = format!("{}\n// note", panel.editor.text());
        panel.editor.replace_text(edited);
        assert_ne!(panel.editor.text(), panel.applied_text.as_str());
    }

    // --- diagnostic spans -------------------------------------------------------

    #[test]
    fn a_span_covers_the_whole_line() {
        let source = "one\ntwo\nthree";
        assert_eq!(source_span(source, 1, None), 0..3);
        assert_eq!(source_span(source, 2, None), 4..7);
        assert_eq!(source_span(source, 3, None), 8..13);
        assert_eq!(source_span(source, 99, None), 13..13, "clamped");
    }

    #[test]
    fn crlf_lines_exclude_the_carriage_return() {
        let source = "one\r\ntwo\r\nthree";
        assert_eq!(source_span(source, 1, None), 0..3);
        assert_eq!(source_span(source, 2, None), 5..8);
    }

    #[test]
    fn a_column_is_character_based_on_utf8_text() {
        // é is two bytes, so a naive byte offset would land inside a character.
        let source = "é = close";
        let span = source_span(source, 1, Some(3));
        assert!(source.is_char_boundary(span.start));
        assert!(source.is_char_boundary(span.end));
        assert_eq!(&source[span.clone()], "=");
    }

    #[test]
    fn an_absent_location_has_no_span() {
        let (_, edits) = describe(
            StudyBuildError::Engine(opinerust::EngineError::Runtime(opinerust::Diagnostic {
                severity: DiagnosticSeverity::Error,
                message: "boom".to_owned(),
                line: None,
                column: None,
            })),
            "source",
        );
        assert!(edits.is_empty(), "no location means no fake span");
    }

    #[test]
    fn a_recalculation_failure_reaches_the_logs() {
        let mut panel = OpineRustPanel::default();
        let slot: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(Some("bar failed".to_owned())));
        panel.watch(slot.clone());
        panel.drain_recalculation_error();
        assert!(
            panel
                .logs()
                .iter()
                .any(|line| line.contains("Recalculation failed: bar failed")),
            "{:?}",
            panel.logs
        );
        panel.drain_recalculation_error();
        assert_eq!(
            panel
                .logs()
                .iter()
                .filter(|line| line.contains("Recalculation failed"))
                .count(),
            1,
            "each failure is logged once"
        );
        assert_eq!(
            slot.lock().unwrap().as_deref(),
            Some("bar failed"),
            "logging must not empty the slot the chart reads"
        );
    }

    /// A live failure must outrank the installed title, and a clean recalculation
    /// must clear it again: the header is where the user decides whether the chart
    /// is showing current data.
    #[test]
    fn a_failure_shows_then_clears_when_the_study_recovers() {
        let mut panel = OpineRustPanel::default();
        panel
            .editor_mut()
            .replace_text("//@version=5\nindicator(\"Live\", overlay = true)\nplot(close)\n");
        let bars = bars(10);
        let source = panel.editor.text().to_owned();
        let prepared = panel.prepare(&source, &bars, 0).expect("runs");
        let mut chart = Chart::new(BarData::from_bars(bars));
        panel.install(&mut chart, None, prepared).expect("installs");
        assert_eq!(panel.run_state(), RunState::Applied("Live".to_owned()));

        let slot: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        panel.watch(slot.clone());
        *slot.lock().unwrap() = Some("history reload failed".to_owned());
        panel.drain_recalculation_error();
        assert_eq!(
            panel.run_state(),
            RunState::Failed("history reload failed".to_owned()),
            "a failure is not reported as the applied title"
        );

        // The study calculates cleanly again.
        *slot.lock().unwrap() = None;
        panel.drain_recalculation_error();
        assert_eq!(
            panel.run_state(),
            RunState::Applied("Live".to_owned()),
            "recovery clears the failure instead of leaving a stale error"
        );
    }

    /// Collapsed Logs cost one row. An empty 110 px strip used to push the code out
    /// of a short panel for no reason.
    #[test]
    fn collapsed_logs_cost_only_the_summary_row() {
        let ctx = egui::Context::default();
        let bars = bars(20);
        let mut panel = OpineRustPanel::default();
        // The rule itself: collapsed is the summary row only, expanded is the body
        // plus that row and never more than part of the panel.
        assert_eq!(logs_panel_height(false, 700.0), 32.0);
        assert_eq!(logs_panel_height(false, 100.0), 32.0);
        let open = logs_panel_height(true, 700.0);
        assert!(
            (100.0..=172.0).contains(&open),
            "an open log keeps the editor usable: {open}"
        );
        assert!(
            logs_panel_height(true, 120.0) < 120.0,
            "a short panel does not give the log more room than it has"
        );

        // And the panel still draws both states without panicking.
        let draw = |panel: &mut OpineRustPanel| {
            ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(420.0, 700.0),
                    )),
                    ..Default::default()
                },
                |ui| {
                    panel.show(ui, &bars, 0);
                },
            )
            .drop_without_applying_deltas();
        };
        draw(&mut panel);
        panel.logs_open = true;
        draw(&mut panel);
        panel.logs_open = false;
        draw(&mut panel);
    }

    #[test]
    fn empty_chart_keeps_draft_visible_and_run_disabled() {
        let ctx = egui::Context::default();
        let mut panel = OpineRustPanel::default();
        let mut prepared = false;
        let output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(460.0, 700.0),
                )),
                ..Default::default()
            },
            |ui| {
                prepared = panel.show_with_close(ui, &[], 0).0.is_some();
            },
        );
        let texts: Vec<_> = output.shapes.iter().filter_map(|shape| {
            if let egui::Shape::Text(text) = &shape.shape {
                Some(text.galley.text().to_owned())
            } else {
                None
            }
        }).collect();
        output.drop_without_applying_deltas();
        assert!(!prepared);
        assert!(texts.iter().any(|text| text.contains("indicator(\"Fast and Slow\"")), "draft must remain visible: {texts:?}");
        assert!(texts.iter().any(|text| text.contains("Load a symbol")), "explain why Run is unavailable: {texts:?}");
    }

    /// The header's close button asks the host to hide the dock.
    #[test]
    fn the_header_close_button_asks_the_host_to_close() {
        let ctx = egui::Context::default();
        let bars = bars(20);
        let mut panel = OpineRustPanel::default();
        let click = |panel: &mut OpineRustPanel, pos: egui::Pos2| {
            let events = vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: Default::default(),
                },
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers: Default::default(),
                },
            ];
            let mut result = (None, false);
            ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(460.0, 700.0),
                    )),
                    events,
                    ..Default::default()
                },
                |ui| {
                    result = panel.show_with_close(ui, &bars, 0);
                },
            )
            .drop_without_applying_deltas();
            result
        };
        // Warm up so the header has a rect, then click the close button at the
        // far right; Run sits to its left and must not fire.
        click(&mut panel, egui::pos2(-1.0, -1.0));
        let (install, close) = click(&mut panel, egui::pos2(452.0, 18.0));
        assert!(install.is_none(), "closing must not run the script");
        assert!(close, "the close button reports a close request");
        assert!(
            panel.logs().is_empty(),
            "nothing was prepared by a close: {:?}",
            panel.logs()
        );
    }
}
