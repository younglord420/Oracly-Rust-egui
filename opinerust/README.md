# OpineRust

Pine Script® v5/v6 studies on [Oracly](https://github.com/younglord420/Oracly-Rust-egui) charts,
powered by the [Pinecone](https://github.com/ferranbt/pinecone) Rust runtime (MPL-2.0).

Two independent things live here, and a host may take either:

- the **engine**: compiles and runs a Pine Script into charts values, with no UI;
- the **studio**: a code editor dock with syntax highlighting, line numbers, hover
  documentation, inline colour pickers, a context menu, a formatter and a pinned
  Logs drawer.

## Install

```toml
[dependencies]
Oracly-charts = "0.3"
opinerust = "0.1"
```

## Engine only

```rust
use opinerust::{build_study, EngineBar};
# fn bars() -> Vec<EngineBar> { Vec::new() }
let built = build_study("//@version=6\nindicator(\"SMA\")\nplot(ta.sma(close, 20))\n", &[], 0)?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Studio dock

```rust
use opinerust::OpineRustPanel;

let mut panel = OpineRustPanel::default();
let (installed, closed) = panel.show_with_close(ui, bars, now_ms);
if let Some(study) = installed {
    chart.add_indicator(study.study);
}
```

Turn the dock off for a headless build:

```toml
opinerust = { version = "0.1", default-features = false }
```

## Unsupported today

Strategies (`strategy(...)`), `import` of libraries, and cross-symbol or
cross-timeframe requests (`request.*`) are rejected with a message that names the
line, rather than silently dropped. Drawings (`box`, `line`, `label`, `table`) are
accepted and kept, but the chart paints plots only.

## Licence

MPL-2.0. The runtime's origin and attribution are recorded in `NOTICE`.
