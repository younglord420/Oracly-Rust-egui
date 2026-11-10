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

/// The chart's own session clock must reach `barstate` in an installed study.
///
/// The chart publishes `Chart::bar_clock`, which carries the exchange session, and
/// a study decides "still forming" from *that* clock. The candle here opens four
/// minutes before the close, so it lasts four minutes rather than the five of its
/// interval: a span guessed from bar spacing would keep it open one extra minute.
#[test]
fn the_chart_session_clock_ends_the_candle_at_the_session_close() {
    use chrono::{NaiveTime, Utc, Weekday};
    use Oracly_charts::model::{Session, SessionWindow, TimezoneId};

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
    let instrument = aapl().with_time_zone(zone).with_session(session);

    // 19:56 UTC is 15:56 at UTC-4: the last five-minute candle before a 16:00 close.
    let open = DateTime::parse_from_rfc3339("2024-06-03T19:56:00Z")
        .expect("timestamp")
        .with_timezone(&Utc);
    let minute = TimeDelta::minutes(1);
    let chart_bars: Vec<Bar> = (0..5)
        .map(|i| {
            let at = open - minute * (4 - i);
            Bar::new(at, 100.0, 101.0, 99.0, 100.5, 1.0)
        })
        .collect();

    let mut chart = Chart::new(BarData::from_bars(chart_bars.clone()));
    chart.set_timeframe(Timeframe::Min5);
    chart.set_instrument(Some(instrument));

    // The clock the chart publishes is the authority, and it ends the candle at the
    // close instead of one interval later.
    let clock = chart.bar_clock();
    assert_eq!(
        clock
            .next_open_after(open)
            .map(|next| (next - open).num_minutes()),
        Some(4),
        "the chart clock closes the 15:56 candle at 16:00"
    );

    let source = r#"//@version=6
indicator("state")
plot(barstate.isrealtime ? 1 : 0, title = "Realtime")
"#;
    let build_at = |now: DateTime<Utc>, bars: &[Bar]| {
        let now_ms = now.timestamp_millis();
        opinerust::build_study_in_context(
            source,
            bars,
            now_ms,
            opinerust::RunContext::from_chart(chart.instrument(), chart.timeframe()),
            Some(std::sync::Arc::new(move || now_ms)),
        )
        .expect("builds")
        .study
    };

    // Two minutes in: open either way.
    let mut inside = build_at(open + minute * 2, &chart_bars);
    inside.set_bar_clock(&clock);
    inside.calculate(&chart_bars);
    assert_eq!(
        inside.values().value(0, 4),
        Some(1.0),
        "two minutes in, the candle is open"
    );

    // One second after the close: confirmed. A five-minute span would still say open.
    let mut closed = build_at(open + minute * 5 + TimeDelta::seconds(1), &chart_bars);
    closed.set_bar_clock(&clock);
    closed.calculate(&chart_bars);
    assert_eq!(
        closed.values().value(0, 4),
        Some(0.0),
        "after the session close the candle is confirmed, not still forming"
    );
}
