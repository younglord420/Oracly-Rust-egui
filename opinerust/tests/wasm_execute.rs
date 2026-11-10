#![cfg(target_arch = "wasm32")]

//! Real browser/WASM execution of the public bridge API.
//!
//! Run with:
//! `CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
//!  cargo test --target wasm32-unknown-unknown --test wasm_execute`
//!
//! These call `execute` and `build_study` exactly as the chart does, so a
//! `SystemTime::now()` on any phase (lex, parse, sema, run) fails here.

use opinerust::{
    build_study, build_study_in_context, execute, execute_with_inputs, EngineBar, EngineError,
    RunContext,
};
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

/// `input.*` must run on wasm and the settings schema must survive the trip, so a
/// browser user can edit a script's inputs without a native-only path.
#[wasm_bindgen_test]
fn inputs_run_and_override_on_wasm() {
    let source = r#"//@version=5
indicator("Inputs", overlay = true)
len = input.int(3, "Length", minval=1, maxval=50)
plot(close * 0 + len, title = "Len")
"#;
    let output = execute(source, &bars(), 0).expect("input.int must run on wasm");
    assert_eq!(output.inputs.len(), 1);
    assert_eq!(output.inputs[0].title, "Length");
    assert_eq!(output.plots[0].values[2], Some(3.0));

    let mut overrides = std::collections::HashMap::new();
    overrides.insert("Length".to_owned(), pine_core::InputValue::Int(9));
    let overridden = execute_with_inputs(source, &bars(), 0, &RunContext::default(), &overrides)
        .expect("override runs on wasm");
    assert_eq!(overridden.plots[0].values[2], Some(9.0));

    // The chart-facing path: an edited setting must reach the script's input.
    let chart_bars: Vec<ChartBar> = (0..5)
        .map(|i| {
            ChartBar::new(
                chrono::DateTime::UNIX_EPOCH + chrono::TimeDelta::minutes(i as i64),
                10.0,
                11.0,
                9.0,
                10.0 + i as f64,
                1.0,
            )
        })
        .collect();
    let built = build_study(source, &chart_bars, 0).expect("study builds on wasm");
    let id = built.study.inputs()[0].id.clone();
    let mut study = built.study;
    study.calculate(&chart_bars);
    assert_eq!(study.values().value(0, 4), Some(3.0));
    study
        .set_input(&id, Oracly_charts::studies::InputValue::Length(12))
        .expect("the schema accepts it");
    study.calculate(&chart_bars);
    assert_eq!(
        study.values().value(0, 4),
        Some(12.0),
        "an edited input changes the wasm calculation"
    );
}

/// Chart metadata must reach the script on wasm, through the same
/// `syminfo`/`timeframe` path a native host uses.
#[wasm_bindgen_test]
fn chart_context_reaches_the_script_on_wasm() {
    let chart_bars: Vec<ChartBar> = (0..3)
        .map(|i| {
            ChartBar::new(
                chrono::DateTime::UNIX_EPOCH + chrono::TimeDelta::minutes(i as i64),
                10.0,
                11.0,
                9.0,
                10.0,
                1.0,
            )
        })
        .collect();
    let source = r#"//@version=5
indicator("Context", overlay = true)
plot(syminfo.mintick * 0 + 1, title = "Tick")
"#;
    let context = RunContext::from_chart(None, Some(Oracly_charts::model::Timeframe::Min5));
    let built = build_study_in_context(source, &chart_bars, 0, context, None)
        .expect("context study builds on wasm");
    let mut study = built.study;
    study.calculate(&chart_bars);
    assert_eq!(
        study.values().value(0, 2),
        Some(1.0),
        "the script ran with chart context on wasm"
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
