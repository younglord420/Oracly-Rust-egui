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
use egui_charts::ui::study_editor::{Diagnostic as EditorDiagnostic, Severity, StudyEditor};
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

/// The OpineRust panel's state.
pub struct OpineRustPanel {
    editor: StudyEditor,
    /// Title of the script the host last installed successfully.
    applied: Option<String>,
    /// The text at the last successful apply, to tell edits from it.
    applied_text: String,
    /// One log line per run, newest last.
    logs: Vec<String>,
    /// True when a study from this panel is on the chart, so the next Run
    /// replaces rather than adds.
    applied_before: bool,
    /// Where the installed study records recalculation failures.
    watching: Option<Arc<Mutex<Option<String>>>>,
    /// The failure already written to the log, so it is logged once.
    logged_error: Option<String>,
    /// Title waiting for the host's install result.
    pending: Option<String>,
    /// Symbol and interval the scripts should see. Empty until the host sets it.
    context: RunContext,
}

impl Default for OpineRustPanel {
    fn default() -> Self {
        let mut editor = StudyEditor::new();
        editor.open_document("My script", DEFAULT_SCRIPT);
        editor.set_io_actions(false);
        editor.set_apply_label("Run");
        Self {
            editor,
            applied: None,
            applied_text: String::new(),
            logs: Vec::new(),
            applied_before: false,
            watching: None,
            logged_error: None,
            pending: None,
            context: RunContext::default(),
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

impl OpineRustPanel {
    /// The editor, for tests that drive it directly.
    #[cfg(test)]
    pub fn editor_mut(&mut self) -> &mut StudyEditor {
        &mut self.editor
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
                if let Some(old) = previous.filter(|_| replaces) {
                    chart.remove_series(old);
                }
                chart.end_user_action();
                chart.set_study_limit(limit);
                self.watch(last_error);
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
        self.logs.push(if replaced {
            format!("Replaced “{title}”.")
        } else {
            format!("Applied “{title}”.")
        });
        self.applied = Some(title);
        self.applied_text = self.editor.text().to_owned();
        self.applied_before = true;
        self.pending = None;
        self.editor.apply_result(Ok(()));
    }

    /// The host could not install a prepared study. Nothing was applied, so the
    /// panel must not claim success.
    pub fn commit_failed(&mut self, reason: String) {
        self.logs.push(format!("Install failed: {reason}"));
        self.pending = None;
        self.editor.apply_result(Err(reason));
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
        let Some(message) = message else { return };
        if self.logged_error.as_deref() == Some(message.as_str()) {
            return;
        }
        self.logs.push(format!("Recalculation failed: {message}"));
        self.editor.apply_result(Err(message.clone()));
        self.logged_error = Some(message);
    }

    /// Draw the panel and return a study to install when Run succeeded.
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        bars: &[ChartBar],
        now_ms: i64,
    ) -> Option<InstallStudy> {
        self.drain_recalculation_error();
        // Run is the panel's primary action, in its header as in Vela. Both the
        // button and Ctrl/Cmd+Enter go through the same request slot.
        self.header(ui);
        let mut run = false;
        ui.horizontal(|ui| {
            let run_button = ui.add(
                egui::Button::new(egui::RichText::new("Run").strong())
                    .fill(ui.visuals().selection.bg_fill),
            );
            if run_button
                .on_hover_text("Compile and apply (Ctrl+Enter)")
                .clicked()
            {
                run = true;
            }
            if self.editor.is_modified() {
                ui.label(egui::RichText::new("Unsaved changes").weak());
            }
        })
        .response
        .on_hover_text("Run compiles and applies the draft; a later Run replaces the study");
        // Only while the editor has focus: typing in the code must not be a
        // chart shortcut, and the shortcut must not fire from the chart. The key
        // event carries its own modifiers, so this works even when the pass-level
        // modifier state was not updated.
        let shortcut = ui.input(|input| {
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

        // Pin the logs first: an editor that grows to fill the panel would push
        // a bottom strip out of view.
        let logs_height = 110.0_f32.min(ui.available_height() * 0.4);
        egui::Panel::bottom("opinerust_logs")
            .exact_size(logs_height)
            .resizable(false)
            .frame(egui::Frame::NONE)
            .show(ui, |ui| self.logs_ui(ui));

        // The editor no longer carries an Apply button of its own: Run is here,
        // and a second button with the same meaning would be a trap.
        self.editor.set_apply_button(false);
        let _ = self.editor.show(ui);
        if run {
            let text = self.editor.text().to_owned();
            return self.prepare(&text, bars, now_ms);
        }
        None
    }

    /// Chart metadata (`syminfo.*`, `timeframe.*`) the panel should report.
    ///
    /// The host sets this before drawing the panel. Leaving it unset keeps the
    /// previous behaviour: the script still runs, with empty metadata.
    pub fn set_context(&mut self, context: RunContext) {
        self.context = context;
    }

    /// Panel identity and run state. Run lives in the editor header beside it.
    fn header(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("OpineRust").strong());
            let edited = self.applied.is_some() && self.editor.text() != self.applied_text;
            let state = match &self.applied {
                Some(title) if edited => format!("{title} — edited"),
                Some(title) => title.clone(),
                None => "Draft".to_owned(),
            };
            ui.label(egui::RichText::new(state).weak()).on_hover_text(
                "Run compiles and applies. A later Run replaces the applied study in place.",
            );
            if self.pending.is_some() {
                ui.spinner();
            }
        });
        ui.add_space(2.0);
    }

    fn logs_ui(&mut self, ui: &mut egui::Ui) {
        let errors = self
            .logs
            .iter()
            .filter(|line| line.contains("failed"))
            .count();
        let title = format!("Logs ({}) — {errors} error(s)", self.logs.len());
        egui::CollapsingHeader::new(title)
            .id_salt("opinerust_logs_header")
            .default_open(true)
            .show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("opinerust_log_scroll")
                    .auto_shrink([false; 2])
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        if self.logs.is_empty() {
                            ui.label(egui::RichText::new("No runs yet.").weak());
                        }
                        for line in &self.logs {
                            ui.label(line.as_str());
                        }
                    });
            });
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
                self.logs.push(format!(
                    "Prepared “{title}” ({} line(s): {lines}).",
                    built.study.line_count()
                ));
                for diagnostic in built.diagnostics {
                    self.logs.push(format!(
                        "{}: {}",
                        label(diagnostic.severity),
                        diagnostic.message
                    ));
                }
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
                self.logs.push(format!("Run failed: {message}"));
                self.editor.set_diagnostics(edits);
                self.editor.apply_result(Err(message));
                None
            }
        }
    }
}

/// The host clock the engine reads on recalculation, in UNIX milliseconds.
fn host_clock() -> Arc<dyn Fn() -> i64 + Send + Sync> {
    Arc::new(|| chrono::Utc::now().timestamp_millis())
}

fn label(severity: DiagnosticSeverity) -> &'static str {
    match severity {
        DiagnosticSeverity::Info => "Info",
        DiagnosticSeverity::Warning => "Warning",
        DiagnosticSeverity::Error => "Error",
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

    #[test]
    fn default_script_is_v5_and_applies() {
        assert!(DEFAULT_SCRIPT.starts_with("//@version=5"));
        let built = build_study(DEFAULT_SCRIPT, &bars(20), 0).expect("sample runs");
        assert_eq!(built.study.line_count(), 2);
        assert!(built.study.is_overlay());
    }

    #[test]
    fn a_failed_run_reports_errors_and_yields_no_study() {
        let mut panel = OpineRustPanel::default();
        let broken = "//@version=5\nindicator(\"Broken\"\nplot(close\n";
        assert!(panel.prepare(broken, &bars(10), 0).is_none());
        assert!(panel.applied.is_none(), "nothing is applied on failure");
        assert!(
            panel.logs.iter().any(|line| line.contains("Run failed")),
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
                .logs
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
        assert!(panel.logs.is_empty(), "opening the panel runs nothing");

        // Ctrl+Enter is the Run shortcut.
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
            panel.logs.iter().any(|line| line.contains("Prepared")),
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
        frame(&mut panel, &mut chart, Some(egui::pos2(12.0, 40.0)));
        assert_eq!(
            chart.indicators().count(),
            baseline + 1,
            "the Run button installs one study"
        );
        frame(&mut panel, &mut chart, Some(egui::pos2(12.0, 40.0)));
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
            panel.logs.iter().any(|line| line.contains("no plot")),
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
                .logs
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
                .logs
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
                .logs
                .iter()
                .any(|line| line.contains("Recalculation failed: bar failed")),
            "{:?}",
            panel.logs
        );
        panel.drain_recalculation_error();
        assert_eq!(
            panel
                .logs
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
}
