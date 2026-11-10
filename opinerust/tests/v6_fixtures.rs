//! Pine Script v6 fixtures with hand-checked expected values.
//!
//! Each case pins one v6 construct and the exact series it must produce, so a
//! runtime upgrade that changes semantics fails here instead of silently drawing
//! something else. These are *our* oracle: they were derived from the script text
//! and the bars below, not copied from another implementation. Where TradingView
//! semantics would differ, the case is not here — an untested claim is worse than
//! a missing one.

use opinerust::{execute, EngineBar, EngineOutput};

/// Five rising closes: 10, 11, 12, 13, 14.
fn bars() -> Vec<EngineBar> {
    (0..5)
        .map(|i| EngineBar {
            forming: false,
            history: true,
            time_ms: i * 60_000,
            open: 10.0 + i as f64,
            high: 11.0 + i as f64,
            low: 9.0 + i as f64,
            close: 10.0 + i as f64,
            volume: 1.0,
        })
        .collect()
}

fn run(source: &str) -> EngineOutput {
    execute(source, &bars(), 0).expect("fixture must run")
}

fn values(output: &EngineOutput, plot: usize) -> Vec<Option<f64>> {
    output.plots[plot].values.clone()
}

#[test]
fn switch_expressions_pick_the_matching_branch() {
    let source = r#"//@version=6
indicator("switch")
x = switch
    close > 11 => 1
    => 0
plot(x)
"#;
    // close > 11 only from the third bar (12, 13, 14).
    assert_eq!(
        values(&run(source), 0),
        [Some(0.0), Some(0.0), Some(1.0), Some(1.0), Some(1.0)]
    );
}

#[test]
fn user_defined_types_hold_and_return_fields() {
    let source = r#"//@version=6
indicator("udt")
type Point
    float price
p = Point.new(close)
plot(p.price)
"#;
    assert_eq!(
        values(&run(source), 0),
        [Some(10.0), Some(11.0), Some(12.0), Some(13.0), Some(14.0)]
    );
}

#[test]
fn methods_on_user_defined_types_receive_their_instance() {
    let source = r#"//@version=6
indicator("method")
type Point
    float price
method price(Point self) => self.price
p = Point.new(close)
plot(p.price())
"#;
    assert_eq!(
        values(&run(source), 0),
        [Some(10.0), Some(11.0), Some(12.0), Some(13.0), Some(14.0)]
    );
}

#[test]
fn enums_declare_members_without_breaking_the_run() {
    let source = r#"//@version=6
indicator("enum")
enum Side
    up
    down
chosen = Side.up
plot(chosen == Side.up ? close : 0)
"#;
    assert_eq!(
        values(&run(source), 0),
        [Some(10.0), Some(11.0), Some(12.0), Some(13.0), Some(14.0)]
    );
}

#[test]
fn var_keeps_state_across_bars() {
    let source = r#"//@version=6
indicator("var")
var float total = 0.0
total += close
plot(total)
"#;
    // Running sum of 10, 11, 12, 13, 14.
    assert_eq!(
        values(&run(source), 0),
        [Some(10.0), Some(21.0), Some(33.0), Some(46.0), Some(60.0)]
    );
}

#[test]
fn history_references_inside_a_for_loop_warm_up_then_sum() {
    let source = r#"//@version=6
indicator("for")
sum = 0.0
for i = 0 to 3
    sum += close[i]
plot(sum)
"#;
    // Four bars are needed before `close[3]` exists.
    assert_eq!(
        values(&run(source), 0),
        [None, None, None, Some(46.0), Some(50.0)]
    );
}

#[test]
fn math_and_string_builtins_are_available_to_v6_scripts() {
    let math = r#"//@version=6
indicator("math")
plot(math.max(close, open))
"#;
    assert_eq!(
        values(&run(math), 0),
        [Some(10.0), Some(11.0), Some(12.0), Some(13.0), Some(14.0)]
    );

    let text = r#"//@version=6
indicator("string")
plot(str.length("abc"))
"#;
    assert_eq!(
        values(&run(text), 0),
        [Some(3.0), Some(3.0), Some(3.0), Some(3.0), Some(3.0)]
    );
}

#[test]
fn a_v5_script_is_not_silently_evaluated_as_v6() {
    let source = r#"//@version=5
indicator("v5")
plot(close)
"#;
    let output = run(source);
    assert_eq!(output.info.version, 5, "the declared version is reported");
    assert_eq!(
        values(&output, 0),
        [Some(10.0), Some(11.0), Some(12.0), Some(13.0), Some(14.0)]
    );
}

#[test]
fn a_script_without_an_annotation_reports_the_default_version() {
    // The bridge defaults to v5; that policy is part of the contract.
    let output = run("indicator(\"unversioned\")\nplot(close)\n");
    assert_eq!(output.info.version, 5);
}

#[test]
fn unsupported_v6_constructs_name_the_line() {
    for source in [
        "//@version=6\nindicator(\"x\")\nplot(close)\na = request.security(\"X\", \"D\", close)\n",
        "//@version=6\nindicator(\"x\")\nimport username/lib/1\nplot(close)\n",
        "//@version=6\nstrategy(\"x\")\nplot(close)\n",
    ] {
        match execute(source, &bars(), 0) {
            Err(opinerust::EngineError::Compile(diagnostics)) => {
                assert!(
                    diagnostics.iter().any(|d| d.line.is_some()),
                    "the rejection must point at a line: {source}"
                );
                assert!(
                    diagnostics
                        .iter()
                        .any(|d| d.message.contains("not supported")),
                    "the rejection must say why: {source}"
                );
            }
            other => panic!("{source} must be rejected before running, got {other:?}"),
        }
    }
}
