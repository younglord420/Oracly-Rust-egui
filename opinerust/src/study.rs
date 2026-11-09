//! Turn an [`EngineOutput`] into an Oracly study.
//!
//! Values and per-bar colours are published together: a calculation builds one
//! [`Snapshot`] and stores it atomically, so `values()` and `plot_color()` can
//! never come from different runs. The study re-executes the script against the
//! bars it receives (append, forming-candle update, history reload, symbol or
//! timeframe change), so output never replays a stale build-time snapshot.
//!
//! A calculation that fails keeps the previous snapshot and records the error
//! for the host instead of silently drawing gaps as if the script worked.

use crate::context::RunContext;
use crate::engine::{execute_with_context, Bar, Diagnostic, EngineError, EngineOutput};
use egui::Color32;
use std::sync::{Arc, Mutex};
use Oracly_charts::model::Bar as ChartBar;
use Oracly_charts::studies::{CustomIndicator, IndicatorValue, PlotType, StudyStatus};

/// Why a study could not be built.
#[derive(Clone, Debug, PartialEq)]
pub enum StudyBuildError {
    /// The script failed to compile or run.
    Engine(EngineError),
    /// The script produced no drawable plot.
    NoPlots,
}

impl From<EngineError> for StudyBuildError {
    fn from(error: EngineError) -> Self {
        Self::Engine(error)
    }
}

/// A study ready to hand to the chart.
pub struct OpineRustStudy {
    /// The study itself, named with the script title.
    pub study: CustomIndicator,
    /// Declaration metadata resolved from the script.
    pub info: crate::engine::ScriptInfo,
    /// Non-fatal script messages, for the editor's log.
    pub diagnostics: Vec<Diagnostic>,
    /// Last recalculation error, shared with the host so the panel can show it.
    pub last_error: Arc<Mutex<Option<String>>>,
}

/// One run's values and colours, published together.
#[derive(Clone)]
struct Snapshot {
    /// One row per bar; `IndicatorValue::Multiple` for the declared plot count.
    rows: Vec<IndicatorValue>,
    /// `[plot][bar]` colours, aligned with `rows`.
    colors: Vec<Vec<Option<Color32>>>,
    /// Number of plots this snapshot was built for.
    plot_count: usize,
}

/// The candle duration implied by the chart's own bar spacing.
///
/// Derived from timestamps rather than the timeframe enum so the bridge does not
/// need a chart handle: the most common positive gap between neighbouring bars is
/// the candle span, which survives weekends and session breaks.
fn bar_span_ms(bars: &[ChartBar]) -> i64 {
    let mut gaps: Vec<i64> = bars
        .windows(2)
        .map(|pair| pair[1].time.timestamp_millis() - pair[0].time.timestamp_millis())
        .filter(|gap| *gap > 0)
        .collect();
    if gaps.is_empty() {
        return 0;
    }
    gaps.sort_unstable();
    gaps[gaps.len() / 2]
}

/// Convert chart bars to engine bars, keeping one entry per chart bar.
///
/// Dropping a bar would shift every following value relative to the chart's
/// candle index, so whitespace bars are passed through: the interpreter treats
/// them as `na` instead of silently renumbering the series.
///
/// `now_ms` decides whether the last candle is still forming: when the clock is
/// inside the last candle's own window it is realtime and unconfirmed, which is
/// what `barstate.isrealtime` and `barstate.isconfirmed` must report on a live
/// chart. A clock before the window (a test fixture, or a replay cursor) leaves
/// the bar historical instead of inventing a forming candle.
fn engine_bars(bars: &[ChartBar], now_ms: i64, bar_ms: i64) -> Vec<Bar> {
    let last = bars.len().saturating_sub(1);
    bars.iter()
        .enumerate()
        .map(|(index, bar)| {
            let open_ms = bar.time.timestamp_millis();
            let forming = index == last
                && bar_ms > 0
                && now_ms >= open_ms
                && now_ms < open_ms + bar_ms
                && !bar.is_whitespace();
            Bar {
                time_ms: open_ms,
                open: bar.open,
                high: bar.high,
                low: bar.low,
                close: bar.close,
                volume: bar.volume,
                forming,
                history: !forming,
            }
        })
        .collect()
}

fn color_from_packed(packed: u32) -> Color32 {
    Color32::from_rgba_unmultiplied(
        (packed >> 24) as u8,
        (packed >> 16) as u8,
        (packed >> 8) as u8,
        (packed & 0xFF) as u8,
    )
}

/// Compile and run `source`, then build a study for `bars`.
///
/// Nothing is mutated on the chart. `now_ms` seeds the first calculation's
/// `timenow`; later recalculations use the newest bar's time.
pub fn build_study(
    source: &str,
    bars: &[ChartBar],
    now_ms: i64,
) -> Result<OpineRustStudy, StudyBuildError> {
    build_study_with_clock(source, bars, now_ms, None)
}

/// Like [`build_study`] with the chart's symbol and interval available to the
/// script as `syminfo.*` and `timeframe.*`.
///
/// The context is carried into every recalculation, so a symbol or interval
/// change is reflected without rebuilding the study by hand.
pub fn build_study_in_context(
    source: &str,
    bars: &[ChartBar],
    now_ms: i64,
    context: RunContext,
    clock: Option<Arc<dyn Fn() -> i64 + Send + Sync>>,
) -> Result<OpineRustStudy, StudyBuildError> {
    let bar_ms = bar_span_ms(bars);
    let engine_bars = engine_bars(bars, now_ms, bar_ms);
    let output: EngineOutput = execute_with_context(source, &engine_bars, now_ms, &context)?;
    study_from_output(source.to_owned(), now_ms, bar_ms, context, clock, output)
}

/// Like [`build_study`], but later recalculations read the host clock through
/// `clock` instead of replaying the initial instant. Pass the chart host's own
/// clock so a live candle is reported as forming (`barstate.isrealtime`) and a
/// closed one as confirmed history.
pub fn build_study_with_clock(
    source: &str,
    bars: &[ChartBar],
    now_ms: i64,
    clock: Option<Arc<dyn Fn() -> i64 + Send + Sync>>,
) -> Result<OpineRustStudy, StudyBuildError> {
    build_study_in_context(source, bars, now_ms, RunContext::default(), clock)
}

fn study_from_output(
    source: String,
    now_ms: i64,
    bar_ms: i64,
    context: RunContext,
    clock: Option<Arc<dyn Fn() -> i64 + Send + Sync>>,
    output: EngineOutput,
) -> Result<OpineRustStudy, StudyBuildError> {
    if output.plots.is_empty() {
        return Err(StudyBuildError::NoPlots);
    }
    let count = output.plots.len();
    let name = output.info.title.clone();
    let titles: Vec<String> = output.plots.iter().map(|plot| plot.title.clone()).collect();
    let declared_colors: Vec<Color32> = output
        .plots
        .iter()
        .map(|plot| {
            plot.colors
                .iter()
                .flatten()
                .next()
                .copied()
                .map_or(Color32::from_rgb(0x29, 0x62, 0xFF), color_from_packed)
        })
        .collect();
    let overlay = output.info.overlay;
    let info = output.info.clone();
    let diagnostics = output.diagnostics.clone();
    let initial = snapshot_from(&output);

    let published = Arc::new(Mutex::new(initial));
    let last_error: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));

    let recalculate = {
        let snapshot = Arc::clone(&published);
        let errors = Arc::clone(&last_error);
        let clock = clock.clone();
        move |bars: &[ChartBar]| -> Vec<IndicatorValue> {
            let now = clock.as_ref().map_or(now_ms, |clock| clock());
            let fresh =
                execute_with_context(&source, &engine_bars(bars, now, bar_ms), now, &context);
            let mut slot = snapshot
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            match fresh {
                Ok(run) if run.plots.len() == slot.plot_count => {
                    let next = snapshot_from_len(&run, bars.len());
                    let rows = next.rows.clone();
                    *slot = next;
                    *errors
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner()) = None;
                    rows
                }
                Ok(_) => {
                    *errors.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(
                        "The script changed the number of plots while recalculating; keeping the last good result.".to_owned(),
                    );
                    slot.rows.clone()
                }
                Err(error) => {
                    let message = match &error {
                        EngineError::Compile(list) => list
                            .first()
                            .map(|diagnostic| diagnostic.message.clone())
                            .unwrap_or_else(|| "The script failed to compile.".to_owned()),
                        EngineError::Runtime(diagnostic) => diagnostic.message.clone(),
                        EngineError::NoBars => "No bars to calculate over.".to_owned(),
                        EngineError::Internal(message) => message.clone(),
                    };
                    *errors
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(message);
                    // Keep the previous good result rather than drawing a
                    // misleading all-gap series as if the script succeeded.
                    slot.rows.clone()
                }
            }
        }
    };

    let colors_source = Arc::clone(&published);
    let per_bar_colors = move |line: usize, bar: usize| -> Option<Color32> {
        let slot = colors_source
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        slot.colors
            .get(line)
            .and_then(|row| row.get(bar))
            .copied()
            .flatten()
    };
    let per_bar = Arc::new(per_bar_colors);

    // A failed recalculation must reach the chart, not only the panel. The chart
    // recomputes through an infallible closure, so the study keeps the last good
    // snapshot and publishes the error as its own status; otherwise a stale line
    // would look exactly like a live one.
    let status_source = {
        let errors = Arc::clone(&last_error);
        move || {
            errors
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .as_ref()
                .map(|message| StudyStatus::DataError {
                    message: message.clone(),
                })
        }
    };

    let mut study = CustomIndicator::new(name.clone(), recalculate)
        .with_line_count(count)
        .with_line_names(titles)
        .with_colors(declared_colors)
        .with_overlay(overlay)
        .with_plot_types(vec![PlotType::Line; count])
        .with_plot_color(move |line, bar, _values| per_bar(line, bar))
        .with_status_source(status_source);

    if let Some(short) = &info.short_title {
        study = study.with_description(short.clone());
    }

    Ok(OpineRustStudy {
        study,
        info,
        diagnostics,
        last_error,
    })
}

/// Rows aligned to `len` bars from a completed run.
fn snapshot_from(output: &EngineOutput) -> Snapshot {
    let len = output.plots.first().map_or(0, |plot| plot.values.len());
    snapshot_from_len(output, len)
}

fn snapshot_from_len(output: &EngineOutput, len: usize) -> Snapshot {
    let plot_count = output.plots.len();
    let rows = (0..len)
        .map(|bar| {
            let row: Vec<f64> = output
                .plots
                .iter()
                .map(|plot| plot.values.get(bar).copied().flatten().unwrap_or(f64::NAN))
                .collect();
            if row.is_empty() || row.iter().all(|value| value.is_nan()) {
                IndicatorValue::None
            } else {
                IndicatorValue::Multiple(row)
            }
        })
        .collect();
    let colors = output
        .plots
        .iter()
        .map(|plot| {
            plot.colors
                .iter()
                .map(|color| color.map(color_from_packed))
                .collect()
        })
        .collect();
    Snapshot {
        rows,
        colors,
        plot_count,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use Oracly_charts::model::{Bar, Timeframe};
    use Oracly_charts::studies::Indicator;

    fn chart_bars(count: usize) -> Vec<Bar> {
        (0..count)
            .map(|i| {
                let close = 100.0 + i as f64;
                Bar::new(
                    chrono::DateTime::UNIX_EPOCH + chrono::TimeDelta::minutes(i as i64),
                    close - 1.0,
                    close + 1.0,
                    close - 2.0,
                    close,
                    10.0,
                )
            })
            .collect()
    }

    const SMALL: &str = r#"//@version=5
indicator("Fast and Slow", overlay = true)
plot(ta.sma(close, 3), title = "Fast")
plot(ta.sma(close, 10), title = "Slow")
"#;

    #[test]
    fn builds_a_two_line_overlay_study() {
        let built = build_study(SMALL, &chart_bars(30), 0).expect("builds");
        let study = built.study;
        assert_eq!(study.line_count(), 2);
        assert_eq!(study.line_names(), ["Fast", "Slow"]);
        assert!(study.is_overlay());
        assert_eq!(study.name(), "Fast and Slow");

        let mut study = study;
        study.calculate(&chart_bars(30));
        let values = study.values();
        assert_eq!(values.len(), 30);
        assert!(values.value(0, 0).is_none(), "warm-up is a gap");
        assert!(values.value(0, 29).is_some());
        assert!(values.value(1, 5).is_none(), "slow line still warming up");
    }

    #[test]
    fn a_new_candle_changes_the_output() {
        let mut study = build_study(SMALL, &chart_bars(30), 0)
            .expect("builds")
            .study;
        study.calculate(&chart_bars(30));
        let before = study.values().value(0, 29).expect("value at the last bar");

        let mut extended = chart_bars(31);
        extended[30] = Bar::new(
            chrono::DateTime::UNIX_EPOCH + chrono::TimeDelta::minutes(30),
            200.0,
            201.0,
            199.0,
            200.0,
            10.0,
        );
        study.calculate(&extended);
        let after = study.values().value(0, 30).expect("value at the new bar");
        assert_ne!(before, after, "the new candle must be calculated");
        assert_eq!(study.values().len(), 31, "one value per chart bar");
    }

    /// Colours must come from the same run as the values, including a new candle.
    #[test]
    fn colours_follow_the_same_run_as_values() {
        let source = r#"//@version=5
indicator("Patched", overlay = true)
plot(close - open, title = "Delta", color = close > open ? color.green : color.red)
"#;
        let mut study = build_study(source, &chart_bars(5), 0)
            .expect("builds")
            .study;
        study.calculate(&chart_bars(5));
        let before = study.plot_color(0, 4);
        assert!(before.is_some(), "a per-bar colour is present");

        let mut extended = chart_bars(6);
        // A falling candle on the new bar must select the red colour.
        extended[5] = Bar::new(
            chrono::DateTime::UNIX_EPOCH + chrono::TimeDelta::minutes(5),
            100.0,
            100.0,
            90.0,
            90.0,
            1.0,
        );
        study.calculate(&extended);
        let after = study.plot_color(0, 5).expect("a colour for the new bar");
        assert_eq!(after, egui::Color32::RED, "the new bar's own run decides");
    }

    /// The chart mutates its bar array in place: append, update the last candle,
    /// and prepend older history. Every case must match a full replay of the
    /// same bars, with one row per chart bar and the same timestamps.
    #[test]
    fn incremental_bar_changes_match_a_full_replay() {
        let base = chart_bars(40);
        let mut study = build_study(SMALL, &base, 0).expect("builds").study;
        let mut bars = base.clone();
        study.calculate(&bars);
        assert_matches_replay(&study, SMALL, &bars);

        // Append one candle.
        bars.push(step_bar(40, 150.0));
        study.calculate(&bars);
        assert_matches_replay(&study, SMALL, &bars);
        assert_eq!(study.values().len(), 41, "one row per bar");

        // Update the forming candle several times.
        for close in [151.0, 120.0, 180.0] {
            let last = bars.len() - 1;
            bars[last] = step_bar(40, close);
            study.calculate(&bars);
            assert_matches_replay(&study, SMALL, &bars);
        }

        // Prepend older history, as a history reload does.
        let mut older: Vec<_> = (0..10usize).map(|i| step_bar(i, 80.0 + i as f64)).collect();
        older.extend(bars.clone());
        bars = older;
        study.calculate(&bars);
        assert_matches_replay(&study, SMALL, &bars);
        assert_eq!(study.values().len(), 51);
    }

    /// A symbol or timeframe change replaces every bar. Nothing from the previous
    /// context may survive: the output must equal a run on the new bars alone.
    #[test]
    fn changing_the_data_context_replaces_the_output() {
        let base = chart_bars(30);
        let mut study = build_study(SMALL, &base, 0).expect("builds").study;
        study.calculate(&base);
        let before = study.values().value(0, 29);

        let other: Vec<_> = (0..25)
            .map(|i| {
                let close = 500.0 - i as f64 * 3.0;
                Bar::new(
                    chrono::DateTime::UNIX_EPOCH + chrono::TimeDelta::hours(i as i64),
                    close + 1.0,
                    close + 2.0,
                    close - 1.0,
                    close,
                    5.0,
                )
            })
            .collect();
        study.calculate(&other);
        assert_matches_replay(&study, SMALL, &other);
        assert_eq!(study.values().len(), 25, "old rows do not linger");
        assert_ne!(study.values().value(0, 24), before);
    }

    /// Replay walks the same bars back and forth; at every step the study must
    /// equal a fresh run over exactly those bars.
    #[test]
    fn replay_windows_match_a_full_replay() {
        let all = chart_bars(50);
        let mut study = build_study(SMALL, &all, 0).expect("builds").study;
        for end in [10usize, 18, 25, 20, 12, 33, 50] {
            let window = &all[..end];
            study.calculate(window);
            assert_matches_replay(&study, SMALL, window);
            assert_eq!(study.values().len(), end, "replay window {end}");
        }
    }

    /// Every bar of the window is finite, so the values must be exactly the
    /// fresh run's, not a subset or a shifted array.
    fn assert_matches_replay(study: &CustomIndicator, source: &str, bars: &[Bar]) {
        use Oracly_charts::studies::Indicator;
        let mut fresh = build_study(source, bars, 0).expect("replay builds").study;
        fresh.calculate(bars);
        assert_eq!(study.values().len(), bars.len());
        for line in 0..study.line_count() {
            for bar in 0..bars.len() {
                assert_eq!(
                    study.values().value(line, bar),
                    fresh.values().value(line, bar),
                    "line {line} bar {bar}"
                );
                assert_eq!(
                    study.plot_color(line, bar),
                    fresh.plot_color(line, bar),
                    "colour line {line} bar {bar}"
                );
            }
        }
    }

    fn step_bar(index: usize, close: f64) -> Bar {
        Bar::new(
            chrono::DateTime::UNIX_EPOCH + chrono::TimeDelta::minutes(index as i64),
            close - 1.0,
            close + 1.0,
            close - 2.0,
            close,
            10.0,
        )
    }

    /// On a live chart the newest candle is still forming; once the clock passes
    /// its close it becomes confirmed history. Both must be visible to scripts and
    /// must not change the earlier bars.
    #[test]
    fn a_forming_candle_becomes_history_when_the_clock_moves_on() {
        let source = r#"//@version=5
indicator("state")
plot(barstate.isrealtime ? 1 : 0, title = "Realtime")
plot(barstate.isconfirmed ? 1 : 0, title = "Confirmed")
"#;
        let bars = chart_bars(10);
        let last_open = bars[9].time.timestamp_millis();
        let minute = 60_000;

        // Clock inside the last candle's window: realtime, unconfirmed.
        let live = |now: i64| {
            let mut study = build_study_with_clock(
                source,
                &bars,
                now,
                Some(Arc::new(move || now) as Arc<dyn Fn() -> i64 + Send + Sync>),
            )
            .expect("builds")
            .study;
            study.calculate(&bars);
            study
        };
        let forming = live(last_open + minute / 2);
        assert_eq!(forming.values().value(0, 9), Some(1.0), "realtime");
        assert_eq!(forming.values().value(1, 9), Some(0.0), "unconfirmed");
        assert_eq!(
            forming.values().value(1, 8),
            Some(1.0),
            "earlier bars closed"
        );

        // Clock past the close: the same bar is now confirmed history.
        let closed = live(last_open + minute + 1);
        assert_eq!(closed.values().value(0, 9), Some(0.0), "no longer realtime");
        assert_eq!(closed.values().value(1, 9), Some(1.0), "now confirmed");
    }

    #[test]
    fn pane_script_is_not_an_overlay() {
        let source = r#"//@version=5
indicator("Pane", overlay = false)
plot(close - open, title = "Delta")
"#;
        let built = build_study(source, &chart_bars(5), 0).expect("builds");
        assert!(!built.study.is_overlay());
        assert_eq!(built.study.line_count(), 1);
    }

    #[test]
    fn a_script_without_plots_is_rejected() {
        let source = "//@version=5\nindicator(\"Nothing\")\nx = close\n";
        assert_eq!(
            build_study(source, &chart_bars(5), 0).err(),
            Some(StudyBuildError::NoPlots)
        );
    }

    #[test]
    fn compile_errors_do_not_produce_a_study() {
        let error = build_study("//@version=5\nplot(\n", &chart_bars(5), 0).err();
        assert!(matches!(
            error,
            Some(StudyBuildError::Engine(EngineError::Compile(_)))
        ));
    }

    #[test]
    fn study_keeps_the_declared_timeframe_units() {
        let built = build_study(SMALL, &chart_bars(30), 0).expect("builds");
        assert_eq!(
            built.study.value_format(),
            Oracly_charts::scales::StudyFormat::Price
        );
        let _ = Timeframe::Min1;
    }

    #[test]
    fn unsupported_features_are_reported() {
        let error = build_study("//@version=5\nstrategy(\"x\")\n", &chart_bars(5), 0).err();
        match error {
            Some(StudyBuildError::Engine(EngineError::Compile(diagnostics))) => {
                assert_eq!(diagnostics[0].line, Some(2));
                assert!(diagnostics[0].message.contains("not supported"));
            }
            other => panic!("expected an unsupported-feature error, got {other:?}"),
        }
    }
}
