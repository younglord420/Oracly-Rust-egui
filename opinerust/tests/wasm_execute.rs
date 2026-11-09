#![cfg(target_arch = "wasm32")]

//! Real browser/WASM execution of the public bridge API.
//!
//! Run with:
//! `CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
//!  cargo test --target wasm32-unknown-unknown --test wasm_execute`
//!
//! These call `execute` and `build_study` exactly as the chart does, so a
//! `SystemTime::now()` on any phase (lex, parse, sema, run) fails here.

use opinerust::{build_study, execute, EngineBar, EngineError};
use wasm_bindgen_test::wasm_bindgen_test;
use Oracly_charts::model::Bar as ChartBar;
use Oracly_charts::studies::Indicator;

fn bars() -> Vec<EngineBar> {
    (0..3)
        .map(|i| EngineBar {
            forming: false,
            history: true,
            time_ms: i * 60_000,
            open: 9.0 + i as f64,
            high: 11.0 + i as f64,
            low: 8.0 + i as f64,
            close: 10.0 + i as f64,
            volume: 100.0,
        })
        .collect()
}

#[wasm_bindgen_test]
fn public_execution_runs_v5_v6_and_errors() {
    for version in [5, 6] {
        let source = format!(
            "//@version={version}\nindicator(\"Wasm\")\nplot(close, title=\"Close\")\nplot(timenow, title=\"Now\")\n"
        );
        let result = execute(&source, &bars(), 123456).unwrap();
        assert_eq!(result.info.version, version);
        assert_eq!(result.plots[0].values, [Some(10.0), Some(11.0), Some(12.0)]);
        assert_eq!(result.plots[1].values, [Some(123456.0); 3]);
        let flags = execute(
            "indicator(\"flags\")\nplot(barstate.isfirst ? 1 : 0)\n",
            &bars(),
            0,
        )
        .unwrap();
        assert_eq!(flags.plots[0].values, [Some(1.0), Some(0.0), Some(0.0)]);
    }
    assert!(matches!(
        execute(
            "indicator(\"x\")\nplot(ta.no_such_builtin(close))",
            &bars(),
            0
        ),
        Err(EngineError::Compile(_))
    ));
}

/// The sema pass must not read the wall clock either; it builds the same
/// `per_bar` values as the run.
#[wasm_bindgen_test]
fn the_study_path_compiles_and_recalculates_off_the_wall_clock() {
    let chart_bars: Vec<ChartBar> = (0..30)
        .map(|i| {
            let close = 100.0 + (i as f64).sin();
            ChartBar::new(
                chrono::DateTime::UNIX_EPOCH + chrono::TimeDelta::minutes(i as i64),
                close - 1.0,
                close + 1.0,
                close - 2.0,
                close,
                10.0,
            )
        })
        .collect();
    let source = r#"//@version=5
indicator("Two plots", overlay = true)
fast = ta.sma(close, 3)
slow = ta.sma(close, 10)
plot(fast, title = "Fast", color = color.orange)
plot(slow, title = "Slow", color = close > open ? color.green : color.red)
"#;
    let built = build_study(source, &chart_bars, 1_700_000_000_000).expect("builds on wasm");
    assert_eq!(built.study.name(), "Two plots");
    assert_eq!(built.study.line_count(), 2);
    let mut study = built.study;
    study.calculate(&chart_bars);
    assert_eq!(study.values().len(), 30);
    assert!(study.values().value(0, 0).is_none(), "warm-up is a gap");
    assert!(
        study.values().value(1, 5).is_none(),
        "the slow line warms up"
    );

    // Appending a candle recomputes values and colours together.
    let mut extended = chart_bars.clone();
    extended.push(ChartBar::new(
        chrono::DateTime::UNIX_EPOCH + chrono::TimeDelta::minutes(30),
        100.0,
        100.0,
        90.0,
        90.0,
        1.0,
    ));
    study.calculate(&extended);
    assert_eq!(study.values().len(), 31);
    assert_eq!(study.plot_color(1, 30), Some(egui::Color32::RED));
}

/// Unsupported constructs are rejected before any chart mutation, on wasm too.
/// A host clock inside the last candle's window must mark it realtime on wasm,
/// which is the path the live chart depends on.
#[wasm_bindgen_test]
fn a_forming_candle_is_realtime_on_wasm() {
    let mut live = bars();
    live[2].forming = true;
    live[2].history = false;
    let source = r#"//@version=5
indicator("state")
plot(barstate.isrealtime ? 1 : 0, title = "Realtime")
plot(barstate.isconfirmed ? 1 : 0, title = "Confirmed")
"#;
    let output = execute(source, &live, 0).expect("runs on wasm");
    assert_eq!(output.plots[0].values[2], Some(1.0), "last bar is realtime");
    assert_eq!(
        output.plots[1].values[2],
        Some(0.0),
        "the forming bar is open"
    );
    assert_eq!(
        output.plots[1].values[1],
        Some(1.0),
        "earlier bars are closed"
    );
}

#[wasm_bindgen_test]
fn unsupported_constructs_are_rejected_on_wasm() {
    for source in [
        "//@version=5\nstrategy(\"s\")\n",
        "//@version=5\nindicator(\"x\")\na = request.security(\"X\", \"D\", close)\n",
    ] {
        assert!(build_study(source, &[], 0).is_err(), "{source}");
        match execute(source, &bars(), 0) {
            Err(EngineError::Compile(diagnostics)) => {
                assert!(diagnostics.iter().any(|d| d.line.is_some()), "{source}");
            }
            other => panic!("{source} must fail before running, got {other:?}"),
        }
    }
}
