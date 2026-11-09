//! Regressions for chart metadata and status through the real chart wrapper.
//!
//! These deliberately use `Chart`, not a bare `CustomIndicator`. The chart wraps
//! studies that are not already configured in `ConfiguredStudy`, and that
//! wrapper is where a study's own status can disappear. A unit test on
//! `CustomIndicator` alone would not catch it.
//!
//! Recalculation is driven by an actual chart frame (`Chart::show`), which is the
//! same path the application uses, instead of calling internal helpers.

use chrono::{DateTime, TimeDelta};
use Oracly_charts::model::{Bar, BarData, Instrument, SymbolType, Timeframe};
use Oracly_charts::studies::{Indicator, StudyStatus};
use Oracly_charts::widget::SeriesId;
use Oracly_charts::Chart;

fn bars(count: usize, step: f64) -> Vec<Bar> {
    (0..count)
        .map(|i| {
            let close = 100.0 + i as f64 * step;
            Bar::new(
                DateTime::UNIX_EPOCH + TimeDelta::minutes(i as i64),
                close - 1.0,
                close + 1.0,
                close - 2.0,
                close,
                10.0,
            )
        })
        .collect()
}

fn aapl() -> Instrument {
    Instrument::new("AAPL")
        .with_exchange("NASDAQ")
        .with_symbol_type(SymbolType::Stock)
        .with_tick_size(0.01)
}

/// Runs one real chart frame so pending recalculation happens.
fn draw(chart: &mut Chart, time: f64) {
    let ctx = egui::Context::default();
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(900.0, 700.0),
        )),
        time: Some(time),
        ..Default::default()
    };
    let output = ctx.run_ui(input, |ui| {
        egui::CentralPanel::default().show(ui, |ui| {
            chart.show(ui);
        });
    });
    output.drop_without_applying_deltas();
}

fn first_study(chart: &Chart) -> &dyn Indicator {
    chart.indicators().next().expect("the study is installed").1
}

const TICK_SCRIPT: &str = r#"//@version=6
indicator("Tick size", overlay = true)
plot(syminfo.mintick, title = "Tick")
"#;

const PERIOD_SCRIPT: &str = r#"//@version=6
indicator("Period", overlay = true)
plot(timeframe.multiplier, title = "Multiplier")
"#;

/// An installed study follows the chart's interval without running the script
/// again: `Chart::set_timeframe` notifies the study, which replays against the
/// new `timeframe.*`.
#[test]
fn an_interval_change_reaches_an_installed_study() {
    let mut chart = Chart::new(BarData::from_bars(bars(30, 1.0)));
    chart.set_timeframe(Timeframe::Min1);
    let context = opinerust::RunContext::from_chart(chart.instrument(), chart.timeframe());
    let built = opinerust::build_study_in_context(
        PERIOD_SCRIPT,
        chart.bars(SeriesId::MAIN),
        0,
        context,
        None,
    )
    .expect("builds");
    chart.add_indicator(Box::new(built.study));

    draw(&mut chart, 0.0);
    assert_eq!(
        first_study(&chart).values().value(0, 29),
        Some(1.0),
        "a 1m chart reports multiplier 1"
    );

    chart.set_timeframe(Timeframe::Min5);
    draw(&mut chart, 1.0);
    assert_eq!(
        first_study(&chart).values().value(0, 29),
        Some(5.0),
        "the interval change must reach the script instead of keeping 1m"
    );
}

/// The symbol reaches an installed study too, even with the interval unchanged.
#[test]
fn a_symbol_change_reaches_an_installed_study() {
    let mut chart = Chart::new(BarData::from_bars(bars(30, 1.0)));
    chart.set_instrument(Some(aapl()));
    let context = opinerust::RunContext::from_chart(chart.instrument(), chart.timeframe());
    let built = opinerust::build_study_in_context(
        TICK_SCRIPT,
        chart.bars(SeriesId::MAIN),
        0,
        context,
        None,
    )
    .expect("builds");
    chart.add_indicator(Box::new(built.study));

    draw(&mut chart, 0.0);
    assert_eq!(first_study(&chart).values().value(0, 29), Some(0.01));

    let other = Instrument::new("MSFT")
        .with_exchange("NASDAQ")
        .with_tick_size(0.0001);
    chart.set_instrument(Some(other));
    draw(&mut chart, 1.0);
    assert_eq!(
        first_study(&chart).values().value(0, 29),
        Some(0.0001),
        "the new symbol's tick size must replace the old one"
    );
}

/// The chart wraps a plain study in `ConfiguredStudy`. A failed recalculation
/// reported by the wrapped study must remain visible through that wrapper.
#[test]
fn a_wrapped_study_failure_is_visible_on_the_chart() {
    let mut chart = Chart::new(BarData::from_bars(bars(30, 1.0)));
    let built =
        opinerust::build_study(PERIOD_SCRIPT, chart.bars(SeriesId::MAIN), 0).expect("builds");
    let slot = std::sync::Arc::clone(&built.last_error);
    chart.add_indicator(Box::new(built.study));
    draw(&mut chart, 0.0);

    *slot.lock().unwrap() = Some("the script failed after the chart changed".to_owned());
    let status = first_study(&chart).study_status();
    assert!(
        matches!(status, Some(StudyStatus::DataError { .. })),
        "the wrapper must not hide the study's own error, got {status:?}"
    );
}

/// Reading the error repeatedly must not consume it: the panel and the chart
/// both observe the same slot.
#[test]
fn reading_a_failure_twice_still_reports_it() {
    let mut chart = Chart::new(BarData::from_bars(bars(30, 1.0)));
    let built =
        opinerust::build_study(PERIOD_SCRIPT, chart.bars(SeriesId::MAIN), 0).expect("builds");
    let slot = std::sync::Arc::clone(&built.last_error);
    chart.add_indicator(Box::new(built.study));
    draw(&mut chart, 0.0);

    *slot.lock().unwrap() = Some("boom".to_owned());
    for pass in 0..2 {
        let status = first_study(&chart).study_status();
        assert!(
            matches!(status, Some(StudyStatus::DataError { .. })),
            "pass {pass} must still see the failure, got {status:?}"
        );
    }
}

/// The forming-candle decision uses the candle span of the *current* data, not
/// the span measured when the study was built.
///
/// The study is built from 1-minute bars and then recalculated with 5-minute
/// bars, as the chart does after an interval change. Two minutes into the last
/// 5-minute candle is still forming; reusing the build-time 1-minute span would
/// call it confirmed and quietly change `barstate.isrealtime`.
#[test]
fn the_forming_candle_follows_the_current_interval() {
    const MINUTE: i64 = 60_000;
    let spaced = |step_minutes: i64| -> Vec<Bar> {
        (0..10)
            .map(|i| {
                let open = DateTime::UNIX_EPOCH + TimeDelta::minutes(i * step_minutes);
                Bar::new(open, 100.0, 101.0, 99.0, 100.5, 1.0)
            })
            .collect()
    };
    let minute_bars = spaced(1);
    let five_minute_bars = spaced(5);
    // 2 minutes into the last 5-minute candle, which is 2 minutes past a 1-minute one.
    let now = five_minute_bars[9].time.timestamp_millis() + 2 * MINUTE;

    let source = r#"//@version=6
indicator("state")
plot(barstate.isrealtime ? 1 : 0, title = "Realtime")
"#;
    let built = opinerust::build_study_in_context(
        source,
        &minute_bars,
        now,
        opinerust::RunContext::default(),
        Some(std::sync::Arc::new(move || now)),
    )
    .expect("builds");
    let mut study = built.study;
    // The chart's recalculation after switching to 5-minute bars.
    study.calculate(&five_minute_bars);
    assert_eq!(
        study.values().value(0, 9),
        Some(1.0),
        "2 minutes into a 5-minute candle is still forming, not confirmed"
    );
}
