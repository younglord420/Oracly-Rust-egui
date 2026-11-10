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
use crate::engine::{
    execute_with_context, execute_with_inputs, Bar, Diagnostic, EngineError, EngineOutput,
};
use egui::Color32;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use Oracly_charts::model::Bar as ChartBar;
use Oracly_charts::model::BarClock as ChartClock;
use Oracly_charts::model::Timeframe as ChartTimeframe;
use Oracly_charts::studies::{CustomIndicator, IndicatorValue, InputValue, PlotType, StudyStatus};

/// The recalculation the study runs on every refresh.
type RecalculateFn = Arc<dyn Fn(&[ChartBar]) -> Vec<IndicatorValue> + Send + Sync>;

/// Why a study could not be built.
#[derive(Clone, Debug, PartialEq)]
pub enum StudyBuildError {
    /// The script failed to compile or run.
    Engine(EngineError),
    /// The script produced no drawable plot.
    NoPlots,
    /// The mapped `input.*` schema was rejected by the chart.
    InvalidInputs,
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
/// `now_ms` decides whether the last candle is still forming, and the **chart's own
/// bar clock** supplies that candle's span. Deriving the span from neighbouring
/// timestamps was a guess: a session gap, a DST change or a monthly boundary all
/// break the median, and the guess then decides `barstate.isrealtime` wrongly. The
/// clock the chart uses to bucket bars is the only authority on where a candle ends.
///
/// Without a clock (a plain unit test, or a host that never supplied one) the
/// timestamp-derived span is still used, because there is nothing better to ask.
/// That fallback is a heuristic and the doc says so rather than presenting it as
/// the same thing.
fn engine_bars(
    bars: &[ChartBar],
    now_ms: i64,
    bar_ms: i64,
    clock: Option<&ChartClock>,
) -> Vec<Bar> {
    let last = bars.len().saturating_sub(1);
    bars.iter()
        .enumerate()
        .map(|(index, bar)| {
            let open_ms = bar.time.timestamp_millis();
            let span = clock
                .and_then(|clock| candle_span_ms(clock, bar.time))
                .filter(|span| *span > 0)
                .unwrap_or(bar_ms);
            let forming = index == last
                && span > 0
                && now_ms >= open_ms
                && now_ms < open_ms + span
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

/// How long the candle that opens at `open` lasts, according to the chart's clock.
///
/// Asked of the clock instead of computed here: only the clock knows about session
/// windows, the exchange timezone and monthly boundaries. `None` when the clock
/// cannot answer — a trade-based interval has no duration, and a bar outside every
/// session window has no close — which callers must not turn into `0`, because
/// `0` would silently make every candle look closed.
fn candle_span_ms(clock: &ChartClock, open: chrono::DateTime<chrono::Utc>) -> Option<i64> {
    let next = clock.next_open_after(open)?;
    let span = (next - open).num_milliseconds();
    (span > 0).then_some(span)
}

/// A clock for the chart's interval alone, used until the host supplies its own.
///
/// Built from the chart's own `Timeframe`, not from the Pine multiplier: only the
/// chart type knows whether an interval is trade-based or how its buckets align.
/// The chart replaces this through `Indicator::set_bar_clock` as soon as it has the
/// exchange timezone and session, so this is a starting point, not an assumption.
fn default_clock(timeframe: Option<ChartTimeframe>) -> Option<ChartClock> {
    timeframe.map(ChartClock::new)
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
    // The chart's interval alone gives a usable clock for the common case; the
    // session/timezone-aware clock arrives later through `Indicator::set_bar_clock`.
    let live_clock = Arc::new(Mutex::new(default_clock(context.chart_timeframe)));
    let charts = {
        let clock = live_clock
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        engine_bars(bars, now_ms, bar_span_ms(bars), clock.as_ref())
    };
    let output: EngineOutput = execute_with_context(source, &charts, now_ms, &context)?;
    study_from_output(
        source.to_owned(),
        now_ms,
        context,
        clock,
        live_clock,
        output,
    )
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
    context: RunContext,
    clock: Option<Arc<dyn Fn() -> i64 + Send + Sync>>,
    live_clock: Arc<Mutex<Option<ChartClock>>>,
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
    // `syminfo`/`timeframe` are chart state, not script state: the chart can
    // change either while the study stays installed. Keep the live context in a
    // cell the closure reads on every recalculation, so a symbol or interval
    // change does not leave the script running with the previous chart's values.
    let live_context = Arc::new(Mutex::new(context));

    // Inputs are chart settings, not script state: the editor can change them
    // while the study stays installed. The schema is mapped once from the run's
    // declarations, and edits land in a shared map the closure reads on every
    // recalculation.
    let mapped = crate::settings::map_inputs(&output.inputs);
    let declared_inputs: Arc<Vec<Oracly_charts::studies::StudyInput>> = Arc::new(mapped.inputs);
    // `title` is how an edited setting reaches `input.*`: the schema is keyed by a
    // positional id so it stays stable, but Pinecone looks overrides up by title.
    let input_titles: Arc<Vec<String>> = Arc::new(mapped.titles);
    let overrides: Arc<Mutex<HashMap<String, pine_core::InputValue>>> =
        Arc::new(Mutex::new(HashMap::new()));

    // One closure shared by both calculation paths: the plain one and the
    // input-aware one. `with_inputs` replaces the calculate callback, so the
    // study would lose the input-aware path unless both point at this same
    // shared closure.
    let recalculate: RecalculateFn = {
        let snapshot = Arc::clone(&published);
        let errors = Arc::clone(&last_error);
        let clock = clock.clone();
        let context = Arc::clone(&live_context);
        let overrides = Arc::clone(&overrides);
        let live_clock = Arc::clone(&live_clock);
        Arc::new(move |bars: &[ChartBar]| -> Vec<IndicatorValue> {
            let now = clock.as_ref().map_or(now_ms, |clock| clock());
            let context = context
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .clone();
            let overrides = overrides
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .clone();
            // The candle span is answered by the chart's clock, read fresh on every
            // run: the interval, timezone and session can all change while the study
            // stays installed, and a cached span would then decide "forming" for the
            // previous interval. The timestamp fallback only applies when no clock
            // was ever supplied.
            let bar_ms = bar_span_ms(bars);
            let live_clock = live_clock
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .clone();
            let fresh = execute_with_inputs(
                &source,
                &engine_bars(bars, now, bar_ms, live_clock.as_ref()),
                now,
                &context,
                &overrides,
            );
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
        })
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
    //
    // This reads the error without consuming it. The panel separately drains the
    // same slot to avoid logging one failure twice, so a `take()` here would make
    // the chart's error disappear the moment the panel looked at it.
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

    let calculate = Arc::clone(&recalculate);
    let mut study = CustomIndicator::new(name.clone(), move |bars: &[ChartBar]| calculate(bars))
        .with_line_count(count)
        .with_line_names(titles)
        .with_colors(declared_colors)
        .with_overlay(overlay)
        .with_plot_types(vec![PlotType::Line; count])
        .with_plot_color(move |line, bar, _values| per_bar(line, bar))
        .with_status_source(status_source)
        // The chart supplies live metadata through `Indicator::set_instrument`
        // and `set_timeframe`; the study forwards a missing interval verbatim so
        // scripts see an unset `timeframe.*` rather than a stale period.
        .with_context_update({
            let context = Arc::clone(&live_context);
            move |instrument, timeframe| {
                let mut slot = context
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                let next = RunContext::from_chart(instrument, timeframe);
                // Only a real metadata change needs a full replay. Comparing the
                // derived context also covers the case where the chart reports a
                // symbol and interval separately, one call after the other.
                if *slot == next {
                    return false;
                }
                *slot = next;
                true
            }
        })
        // The chart's own clock, once it supplies one, is the only authority on
        // where a candle ends; the interval-only default above is just a start.
        .with_bar_clock(Arc::clone(&live_clock))
        .with_input_override({
            let overrides = Arc::clone(&overrides);
            let titles = Arc::clone(&input_titles);
            let declared_inputs = Arc::clone(&declared_inputs);
            move |id: &str, value: &InputValue| {
                let index = declared_inputs
                    .iter()
                    .position(|input| input.id == id)
                    .ok_or("unknown study input")?;
                let Some(title) = titles.get(index) else {
                    return Err("study input has no declaration");
                };
                let Some(engine) = crate::settings::to_engine_value(value) else {
                    return Err("this input type cannot change the running script");
                };
                overrides
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .insert(title.clone(), engine);
                Ok(())
            }
        });

    if !declared_inputs.is_empty() {
        // The closure ignores the passed settings: overrides travel through the
        // shared map so a recalculation reads the same values the editor shows.
        let schema = declared_inputs.as_ref().clone();
        let inputs_calculate = Arc::clone(&recalculate);
        study = study
            .with_inputs(schema, move |bars, _inputs| inputs_calculate(bars))
            .map_err(|_| StudyBuildError::InvalidInputs)?;
    }

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

    /// An interval-only clock would also pass the previous test, so this one uses
    /// the session clock the chart actually has: a candle opened before the session
    /// closes must end at the close, not one interval later.
    ///
    /// The guess this replaces (median gap between bars) cannot see the gap at all.
    #[test]
    fn a_session_close_ends_the_last_candle() {
        use chrono::{NaiveTime, Weekday};
        use Oracly_charts::model::{Session, SessionWindow, TimezoneId};

        let source = r#"//@version=5
indicator("state")
plot(barstate.isrealtime ? 1 : 0, title = "Realtime")
plot(barstate.isconfirmed ? 1 : 0, title = "Confirmed")
"#;
        // A fixed exchange offset keeps this test free of the `timezones` feature;
        // the session window rule being tested does not depend on IANA lookups.
        // 09:30–16:00 at UTC-4 is 13:30–20:00 UTC.
        let zone = TimezoneId::fixed_offset(-4 * 3600).expect("valid offset");
        let session = Session::new([SessionWindow::new(
            [
                Weekday::Mon,
                Weekday::Tue,
                Weekday::Wed,
                Weekday::Thu,
                Weekday::Fri,
            ],
            NaiveTime::from_hms_opt(9, 30, 0).expect("time"),
            NaiveTime::from_hms_opt(16, 0, 0).expect("time"),
        )]);
        let clock = ChartClock::new(Oracly_charts::model::Timeframe::Min5)
            .with_session(zone, session, false);

        let open = chrono::DateTime::parse_from_rfc3339("2024-06-03T19:56:00Z")
            .expect("timestamp")
            .with_timezone(&chrono::Utc);
        let close = chrono::DateTime::parse_from_rfc3339("2024-06-03T20:00:00Z")
            .expect("timestamp")
            .with_timezone(&chrono::Utc);
        assert_eq!(
            candle_span_ms(&clock, open),
            Some((close - open).num_milliseconds()),
            "the candle ends at the session close"
        );

        // The chart supplies its own session-aware clock through `set_bar_clock`,
        // and the host clock moves between runs — exactly the production path.
        let bars = vec![
            Bar::new(
                open - chrono::TimeDelta::minutes(5),
                100.0,
                101.0,
                99.0,
                100.0,
                1.0,
            ),
            Bar::new(open, 100.0, 101.0, 99.0, 100.5, 1.0),
        ];
        let now = Arc::new(Mutex::new(open.timestamp_millis()));
        let clock_now = Arc::clone(&now);
        let mut study = build_study_with_clock(
            source,
            &bars,
            open.timestamp_millis(),
            Some(Arc::new(move || {
                *clock_now
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
            })),
        )
        .expect("builds")
        .study;
        study.set_bar_clock(&clock);
        study.set_timeframe(Oracly_charts::model::Timeframe::Min5);

        // 15:58 local (two minutes before the close): the 15:56 candle is open.
        let inside = open + chrono::TimeDelta::minutes(2);
        *now.lock().unwrap() = inside.timestamp_millis();
        study.calculate(&bars);
        assert_eq!(
            study.values().value(0, 1),
            Some(1.0),
            "the shortened candle is realtime before the session close"
        );

        // 16:00:01 local: the same candle is confirmed. A fixed 5-minute span would
        // still call it realtime until 16:01.
        let after = close + chrono::TimeDelta::seconds(1);
        *now.lock().unwrap() = after.timestamp_millis();
        study.calculate(&bars);
        assert_eq!(
            study.values().value(0, 1),
            Some(0.0),
            "the candle is confirmed at the close, not one interval later"
        );
        assert_eq!(study.values().value(1, 1), Some(1.0), "now confirmed");
    }

    /// A monthly chart must close its candle at the start of the next month, not
    /// after a fixed 30 days. This is the `next_open_after` path the bridge uses to
    /// decide `barstate` on calendar intervals.
    #[test]
    fn a_monthly_candle_ends_at_the_next_month() {
        use chrono::TimeZone;
        let clock = ChartClock::new(Oracly_charts::model::Timeframe::Month1);
        let january = chrono::Utc
            .with_ymd_and_hms(2024, 1, 1, 0, 0, 0)
            .single()
            .expect("timestamp");
        let february = chrono::Utc
            .with_ymd_and_hms(2024, 2, 1, 0, 0, 0)
            .single()
            .expect("timestamp");
        assert_eq!(
            clock.next_open_after(january),
            Some(february),
            "January is 31 days, not 30"
        );
        assert_eq!(
            candle_span_ms(&clock, january),
            Some(31 * 24 * 60 * 60 * 1000),
            "the span is the real month length"
        );

        // February 2024 is a leap month: 29 days.
        let march = chrono::Utc
            .with_ymd_and_hms(2024, 3, 1, 0, 0, 0)
            .single()
            .expect("timestamp");
        assert_eq!(clock.next_open_after(february), Some(march));
        assert_eq!(
            candle_span_ms(&clock, february),
            Some(29 * 24 * 60 * 60 * 1000),
            "a leap February is 29 days"
        );
    }

    /// A trade-based interval has no clock span. The bridge must not turn that into
    /// `0`, which would make every candle look closed.
    #[test]
    fn a_trade_based_interval_has_no_candle_span() {
        use std::num::NonZeroU64;
        let clock = ChartClock::new(Oracly_charts::model::Timeframe::Ticks(
            NonZeroU64::new(500).expect("nonzero"),
        ));
        let open = chrono::DateTime::UNIX_EPOCH + chrono::TimeDelta::minutes(1);
        assert_eq!(candle_span_ms(&clock, open), None);
    }

    /// An edited input must change what the next recalculation computes. The
    /// override travels by title, so a script whose plot depends on `input.int`
    /// proves the whole path: schema -> set_input -> override map -> interpreter.
    #[test]
    fn an_edited_input_changes_the_next_recalculation() {
        let source = r#"//@version=5
indicator("Length", overlay = true)
len = input.int(3, "Length", minval=1, maxval=200)
plot(close * 0 + len, title = "Len")
"#;
        let study = build_study(source, &chart_bars(5), 0).expect("builds");
        assert_eq!(study.study.inputs().len(), 1);
        let id = study.study.inputs()[0].id.clone();
        assert_eq!(study.study.inputs()[0].label, "Length");

        let mut indicator = study.study;
        indicator.calculate(&chart_bars(5));
        assert_eq!(
            indicator.values().value(0, 4),
            Some(3.0),
            "declared default"
        );

        // An edit the schema refuses changes nothing.
        assert!(indicator
            .set_input(&id, Oracly_charts::studies::InputValue::Length(500))
            .is_err());
        indicator.calculate(&chart_bars(5));
        assert_eq!(indicator.values().value(0, 4), Some(3.0), "refused edit");

        // An accepted edit reaches the script's `input.int`.
        indicator
            .set_input(&id, Oracly_charts::studies::InputValue::Length(7))
            .expect("accepted");
        indicator.calculate(&chart_bars(5));
        assert_eq!(
            indicator.values().value(0, 4),
            Some(7.0),
            "the recalculation must use the edited input"
        );
    }

    /// An override for a title the script never declared cannot silently change a
    /// different input; Pinecone resolves by title, so an unknown key is inert.
    #[test]
    fn an_input_override_only_touches_its_own_declaration() {
        let source = r#"//@version=5
indicator("Two", overlay = true)
a = input.int(2, "A")
b = input.int(9, "B")
plot(close * 0 + a, title = "A plot")
plot(close * 0 + b, title = "B plot")
"#;
        let mut study = build_study(source, &chart_bars(4), 0)
            .expect("builds")
            .study;
        let inputs = study.inputs();
        assert_eq!(inputs.len(), 2);
        let first = inputs[0].id.clone();
        study.calculate(&chart_bars(4));
        assert_eq!(study.values().value(0, 3), Some(2.0));
        assert_eq!(study.values().value(1, 3), Some(9.0));

        study
            .set_input(&first, Oracly_charts::studies::InputValue::Length(5))
            .expect("accepted");
        study.calculate(&chart_bars(4));
        assert_eq!(study.values().value(0, 3), Some(5.0), "edited input");
        assert_eq!(
            study.values().value(1, 3),
            Some(9.0),
            "the other declaration is untouched"
        );
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
