//! End-to-end proof that a Smart-Money-Concepts style script runs.
//!
//! The LuxAlgo Library scripts this bridge is meant to run are not curve plots:
//! they draw order blocks, fair-value gaps, structure breaks and swing labels with
//! `box`/`line`/`label`, and many of them read a higher timeframe through
//! `request.security`. These tests pin that shape — a script with no `plot(...)` at
//! all, whose whole output is geometry — because that is the case the old bridge
//! refused outright.

use opinerust::{
    execute_with_context, execute_with_environment, ChartOnlyProvider, EngineBar, HostEnvironment,
    MemoryLibraries, RunContext,
};
use std::collections::HashMap;
use std::sync::Arc;

/// Twenty bars that oscillate around a rising trend: they contain swing highs and
/// swing lows for `ta.pivothigh`/`pivotlow` to find, plus one genuine gap up at bar
/// 10 so a fair-value gap exists by construction. A monotonic series has no pivots
/// and no gaps, which would let these assertions pass vacuously.
fn bars() -> Vec<EngineBar> {
    // An up/down zig-zag: +-6 around the trend line, so each extreme is a pivot.
    const ZIGZAG: [f64; 6] = [0.0, 6.0, 2.0, -4.0, 0.0, 5.0];
    (0..20)
        .map(|i| {
            let trend = 100.0 + i as f64;
            let wave = ZIGZAG[i % ZIGZAG.len()];
            // Bar 10 jumps clear of bar 8's high (a 5.0 wave), leaving a gap.
            let jump = if i >= 10 { 12.0 } else { 0.0 };
            let close = trend + wave + jump;
            EngineBar {
                forming: false,
                history: true,
                time_ms: 1_700_000_000_000 + i as i64 * 3_600_000,
                open: close - 1.5,
                high: close + 2.0,
                low: close - 2.5,
                close,
                volume: 10.0 + i as f64,
            }
        })
        .collect()
}

fn smc_script() -> &'static str {
    r#"//@version=5
indicator("SMC Lite", overlay = true)

// A fair-value gap between three consecutive bars, boxed and labelled — the
// shape every LuxAlgo structure indicator produces. Guarded on `bar_index[2]`
// because a lookback before the first bar is `na`, which no box can span.
gap_up = low > high[2] and not na(bar_index[2])
gap_down = high < low[2] and not na(bar_index[2])

var box last_box = na
if gap_up
    last_box := box.new(bar_index[2], low, bar_index, high[2], border_color = color.green, bgcolor = color.new(color.green, 80))
if gap_down
    last_box := box.new(bar_index[2], high, bar_index, low[2], border_color = color.red, bgcolor = color.new(color.red, 80))

// Structure: the last confirmed swing high, drawn as a level that runs right.
ph = ta.pivothigh(2, 2)
pl = ta.pivotlow(2, 2)
var line level = na
if not na(ph)
    level := line.new(bar_index[2], ph, bar_index + 5, ph, color = color.orange, style = line.style_dashed)
if not na(pl)
    level := line.new(bar_index[2], pl, bar_index + 5, pl, color = color.blue, style = line.style_dashed)

// A break-of-structure label.
bos = ta.crossover(close, ta.sma(close, 5))
if bos
    label.new(bar_index, high, "BOS", color = color.new(color.teal, 20), textcolor = color.white)
"#
}

#[test]
fn a_structure_script_reports_boxes_lines_and_labels() {
    let output = execute_with_context(
        smc_script(),
        &bars(),
        1_700_000_000_000,
        &RunContext::default(),
    )
    .expect("an SMC-shaped script must run");

    assert_eq!(output.info.title, "SMC Lite");
    assert_eq!(output.info.kind, "indicator");
    assert!(
        output.plots.is_empty(),
        "the whole output is geometry: {:?}",
        output.plots
    );
    assert!(!output.boxes.is_empty(), "fair-value gaps became boxes");
    assert!(!output.lines.is_empty(), "swing levels became lines");
    assert!(!output.labels.is_empty(), "structure breaks became labels");

    let boxed = &output.boxes[0];
    assert!(
        boxed.top.is_finite()
            && boxed.bottom.is_finite()
            && boxed.left.is_finite()
            && boxed.right.is_finite(),
        "a box has finite edges: {boxed:?}"
    );
    assert!(
        (boxed.top - boxed.bottom).abs() > 0.0,
        "and a non-zero extent: {boxed:?}"
    );
    assert!(boxed.fill_color.is_some(), "the gap keeps its fill");
    assert_eq!(output.labels[0].text, "BOS");
}

#[test]
fn a_higher_timeframe_request_reads_the_host_cache() {
    // The chart's own bars, served as the "daily" feed. A real host serves
    // whatever it has cached; the engine never fetches.
    let data = pine_core::Data::new(
        bars()
            .iter()
            .map(|bar| pine_core::Bar {
                open: bar.open,
                high: bar.high,
                low: bar.low,
                close: bar.close,
                volume: bar.volume,
                time: bar.time_ms,
                ..pine_core::Bar::default()
            })
            .collect(),
    );
    let environment = HostEnvironment::empty()
        .with_security(Arc::new(ChartOnlyProvider::new("BTCUSDT", "1D", data)));

    let source = r#"//@version=5
indicator("HTF Bias", overlay = true)
daily_close = request.security(syminfo.tickerid, "1D", close)
plot(daily_close, title = "Daily")
"#;
    let context = RunContext {
        ticker: "BTCUSDT".to_string(),
        tickerid: "BTCUSDT".to_string(),
        ..RunContext::default()
    };
    let output =
        execute_with_environment(source, &bars(), 0, &context, &HashMap::new(), &environment)
            .expect("the host answers the request");
    assert!(
        output.plots[0].values.iter().any(|value| value.is_some()),
        "the requested series fills in: {:?}",
        output.plots[0].values
    );
}

#[test]
fn an_imported_library_is_callable() {
    let environment =
        HostEnvironment::empty().with_libraries(Arc::new(MemoryLibraries::new().with_library(
            "oracly/Math/1",
            r#"//@version=5
library("Oracly Math")
// Midpoint of two prices.
mid(a, b) => (a + b) / 2
export mid
"#,
        )));

    let source = r#"//@version=5
indicator("Mid")
import oracly/Math/1 as lib
plot(lib.mid(high, low), title = "Mid")
"#;
    let output = execute_with_environment(
        source,
        &bars(),
        0,
        &RunContext::default(),
        &HashMap::new(),
        &environment,
    )
    .expect("the host library resolves");
    let first = output.plots[0].values[0].expect("a value");
    assert!(
        (first - (bars()[0].high + bars()[0].low) / 2.0).abs() < 1e-9,
        "the imported function computed the midpoint: {first}"
    );
}

#[test]
fn a_strategy_reports_its_ledger() {
    let source = r#"//@version=5
strategy("Cross", overlay = true, initial_capital = 10000)
if ta.crossover(close, ta.sma(close, 5))
    strategy.entry("Long", strategy.long)
if ta.crossunder(close, ta.sma(close, 5))
    strategy.close("Long")
plot(ta.sma(close, 5), title = "MA")
"#;
    let output =
        execute_with_context(source, &bars(), 0, &RunContext::default()).expect("a strategy runs");
    assert_eq!(output.info.kind, "strategy");
    assert_eq!(output.info.title, "Cross");
    assert!(
        output.trades.iter().any(|trade| !trade.exit),
        "an entry was recorded: {:?}",
        output.trades
    );
    assert!(
        output.trades.iter().all(|trade| trade.bar < bars().len()),
        "every marker lands on a real bar: {:?}",
        output.trades
    );
}

#[test]
fn smc_real_script_drawings_investigation() {
    use Oracly_charts::model::Bar as ChartBar;
    use Oracly_charts::studies::Indicator;

    let path = "/root/egui-charts/smc.txt";
    let Ok(source) = std::fs::read_to_string(path) else {
        return;
    };

    let bars: Vec<ChartBar> = if let Ok(csv_text) = std::fs::read_to_string("/tmp/btcusdt_1d.csv") {
        csv_text
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| {
                let parts: Vec<&str> = line.split(',').collect();
                let t: i64 = parts[0].parse().unwrap();
                let o: f64 = parts[1].parse().unwrap();
                let h: f64 = parts[2].parse().unwrap();
                let l: f64 = parts[3].parse().unwrap();
                let c: f64 = parts[4].parse().unwrap();
                let v: f64 = parts[5].parse().unwrap();
                ChartBar::new(
                    chrono::DateTime::from_timestamp_millis(t).unwrap(),
                    o,
                    h,
                    l,
                    c,
                    v,
                )
            })
            .collect()
    } else {
        (0..240)
            .map(|i| {
                let wave = ((i as f64) / 12.0).sin();
                let close = 60000.0 + 5000.0 * wave + (i as f64) * 30.0;
                ChartBar::new(
                    chrono::DateTime::from_timestamp_millis(1_700_000_000_000 + i * 86_400_000).unwrap(),
                    close - 500.0,
                    close + 1000.0,
                    close - 1000.0,
                    close,
                    100.0,
                )
            })
            .collect()
    };

    println!("Testing with {} bars...", bars.len());
    let now = bars.last().unwrap().time.timestamp_millis();
    let engine_bars: Vec<opinerust::EngineBar> = bars
        .iter()
        .map(|b| opinerust::EngineBar {
            forming: false,
            history: true,
            time_ms: b.time.timestamp_millis(),
            open: b.open,
            high: b.high,
            low: b.low,
            close: b.close,
            volume: b.volume,
        })
        .collect();
    let engine_out = opinerust::execute_with_environment(
        &source,
        &engine_bars,
        now,
        &opinerust::RunContext::default(),
        &std::collections::HashMap::new(),
        &opinerust::HostEnvironment::empty(),
    )
    .expect("engine executes");

    for diag in &engine_out.diagnostics {
        if diag.message.contains("DISP_STRUCT:") {
            println!("{}", diag.message);
        }
    }
    for (i, bx) in engine_out.boxes.iter().enumerate() {
        println!(
            "BOX #{i}: left={} right={} top={} bottom={} extend={} fill={:?}",
            bx.left, bx.right, bx.top, bx.bottom, bx.extend, bx.fill_color
        );
    }

    let built = opinerust::build_study(&source, &bars, now).expect("SMC builds");
    let drawings = built.study.drawings();
    println!("Total drawings count: {}", drawings.len());
    let mut rect_count = 0;
    let mut line_count = 0;
    let mut label_count = 0;
    for d in drawings {
        match d.tool_type {
            Oracly_charts::drawings::DrawingToolType::Rect => {
                rect_count += 1;
                println!(
                    "RECT: pts={:?} color={:?} fill={:?} extend_r={}",
                    d.chart_points, d.color, d.fill_color, d.extend_right
                );
            }
            Oracly_charts::drawings::DrawingToolType::TrendLine
            | Oracly_charts::drawings::DrawingToolType::HorizontalLine => {
                line_count += 1;
            }
            Oracly_charts::drawings::DrawingToolType::AnchoredText => {
                label_count += 1;
            }
            _ => {}
        }
    }
    println!(
        "Drawings breakdown: rects={}, lines={}, labels={}",
        rect_count, line_count, label_count
    );
    assert!(rect_count > 0, "Rect count should be > 0!");
}
