//! Regression tests for the status an OpineRust study reports.
//!
//! The chart recomputes a study through `Indicator::calculate`, which returns no
//! result. A recalculation can fail (an engine error, or an output shape that no
//! longer matches its declaration) while the study still holds a good previous
//! snapshot. The bridge must publish that failure as the study's status, because
//! otherwise a frozen line looks exactly like a live one.
//!
//! Producing such a failure from bar data alone is not something the current
//! engine allows on purpose, so these tests drive the status plumbing directly:
//! the same `Arc<Mutex<Option<String>>>` the recalculation writes is the value
//! handed to the study.

use chrono::{DateTime, TimeDelta};
use Oracly_charts::model::Bar;
use Oracly_charts::studies::{Indicator, StudyStatus};

fn bars(count: usize) -> Vec<Bar> {
    (0..count)
        .map(|i| {
            let close = 100.0 + i as f64;
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

const SCRIPT: &str = r#"//@version=5
indicator("Status", overlay = true)
plot(close, title = "C")
"#;

#[test]
fn a_healthy_recalculation_reports_no_status() {
    let mut study = opinerust::build_study(SCRIPT, &bars(10), 0)
        .expect("builds")
        .study;
    study.calculate(&bars(10));
    assert_eq!(study.study_status(), None);
}

#[test]
fn a_failed_recalculation_reports_a_data_error() {
    let built = opinerust::build_study(SCRIPT, &bars(10), 0).expect("builds");
    // This is the slot the recalculation writes its failure into.
    let slot = std::sync::Arc::clone(&built.last_error);
    let mut study = built.study;
    study.calculate(&bars(10));
    assert_eq!(study.study_status(), None, "healthy run says nothing");

    *slot.lock().unwrap() = Some("the script failed after the symbol changed".to_owned());
    assert_eq!(
        study.study_status(),
        Some(StudyStatus::DataError {
            message: "the script failed after the symbol changed".to_owned(),
        }),
        "a stale line must not look live"
    );
}

/// The chart's own data-status pass writes `None` when the study still has
/// values to draw. That must not erase a study-owned failure.
#[test]
fn the_chart_status_pass_does_not_hide_a_study_failure() {
    let built = opinerust::build_study(SCRIPT, &bars(10), 0).expect("builds");
    let slot = std::sync::Arc::clone(&built.last_error);
    let mut study = built.study;
    study.calculate(&bars(10));
    *slot.lock().unwrap() = Some("boom".to_owned());

    study.set_study_status(None);
    assert!(
        matches!(study.study_status(), Some(StudyStatus::DataError { .. })),
        "the chart computing no status must not hide the study's own error"
    );
}

/// A status the chart computed itself still wins: it knows about hidden
/// intervals and missing data, which the study cannot see.
#[test]
fn a_chart_computed_status_takes_precedence() {
    let built = opinerust::build_study(SCRIPT, &bars(10), 0).expect("builds");
    let slot = std::sync::Arc::clone(&built.last_error);
    let mut study = built.study;
    study.calculate(&bars(10));
    *slot.lock().unwrap() = Some("boom".to_owned());

    study.set_study_status(Some(StudyStatus::HiddenOnInterval));
    assert_eq!(
        study.study_status(),
        Some(StudyStatus::HiddenOnInterval),
        "the chart's own reason is more specific and wins"
    );
}

/// `syminfo` and `timeframe` are ordinary Pine namespaces the runtime already
/// registers. Rejecting scripts that read them is a bridge bug, not a language
/// limitation, and it hid every v6 script that uses `syminfo.ticker`.
#[test]
fn metadata_namespaces_are_not_rejected() {
    for source in [
        "//@version=5\nindicator(\"x\")\nplot(syminfo.mintick)\n",
        "//@version=6\nindicator(\"x\")\nplot(timeframe.multiplier)\n",
        "//@version=6\nindicator(\"x\")\nplot(syminfo.tickerid == \"\" ? 0 : 1)\n",
    ] {
        if let Err(error) = opinerust::build_study(source, &bars(10), 0) {
            panic!("{source}: {error:?}");
        }
    }
}

/// A context built from real chart metadata must reach the script, not just be
/// accepted by the parser.
#[test]
fn syminfo_and_timeframe_report_the_chart_values() {
    use Oracly_charts::model::{Instrument, SymbolType, Timeframe};
    let instrument = Instrument::new("AAPL")
        .with_exchange("NASDAQ")
        .with_description("Apple Inc.")
        .with_symbol_type(SymbolType::Stock)
        .with_tick_size(0.01);
    let context = opinerust::RunContext::from_chart(Some(&instrument), Some(Timeframe::Min5));
    let source = r#"//@version=6
indicator("meta")
plot(syminfo.mintick, title = "Tick")
"#;
    let mut study = opinerust::build_study_in_context(source, &bars(10), 0, context, None)
        .expect("builds")
        .study;
    study.calculate(&bars(10));
    let plotted = study.values().value(0, 9);
    assert_eq!(
        plotted,
        Some(0.01),
        "mintick must come from the chart, not the empty default"
    );

    let context = opinerust::RunContext::from_chart(Some(&instrument), Some(Timeframe::Min5));
    assert_eq!(context.ticker, "AAPL");
    assert_eq!(context.tickerid, "NASDAQ:AAPL");
    assert_eq!(context.description, "Apple Inc.");
    assert_eq!(context.multiplier, 5);
}

#[test]
fn a_default_context_reports_empty_metadata() {
    let context = opinerust::RunContext::default();
    assert!(context.ticker.is_empty(), "no instrument means no ticker");
    assert_eq!(context.multiplier, 0, "no interval means no multiplier");
    assert_eq!(context.timeframe(), None);
}
