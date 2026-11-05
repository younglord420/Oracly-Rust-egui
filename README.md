# Oracly Chart

Interactive financial charts for [egui](https://github.com/emilk/egui), with 201 indicators.
It also provides 111 drawing tools and supports 21 chart types plus order flow.

| | |
|---|---|
| Crate | `Oracly-charts` |
| Docs | [oracl.io](https://oracl.io) |
| Repository | [younglord420/Oracly-Rust-Ugui](https://github.com/younglord420/Oracly-Rust-Ugui) |
| License | MIT OR Apache-2.0 |

## Requirements

- Rust **1.95+** (edition 2024)
- For the desktop demo: a normal egui/`eframe` stack
- For WASM demos: [Trunk](https://trunkrs.dev/)

## Quick start

```toml
[dependencies]
Oracly-charts = "0.3"
```

Default features turn on the application UI (`ui`), locale tables, and time zones. For a headless engine (alerts, detectors, backtests, study math) disable defaults and pick the services you need:

```toml
Oracly-charts = { version = "0.3", default-features = false, features = ["studies", "alert-engine", "strategies"] }
```

Build and test locally:

```bash
cargo fmt --all
cargo clippy --all-features --all-targets -- -D warnings
cargo test --all-features --lib
```

Run the native demo (from `examples/demo`):

```bash
cargo run --manifest-path examples/demo/Cargo.toml
```

## Feature flags

| Feature | Role |
|---|---|
| `widget` | egui `Chart` / `ChartBuilder`, theme, rendering |
| `drawings` | Drawing tools and annotation storage |
| `studies` | Built-in studies (headless-capable) |
| `icons` | Embedded SVG icons |
| `ui` | Toolbars, panels, dialogs, `ui_kit` (default) |
| `alert-engine` | Headless alert evaluation |
| `detectors` | Headless order-flow detectors |
| `strategies` | Headless backtest runner |
| `locales` / `timezones` | Locale labels and IANA zones (default) |
| `serde` | Serialize themes and related config |
| `native-dialogs` | Desktop file dialogs via `rfd` |
| `desktop-webview` | Example wry/tao shell for WASM embedding notes |

The headless layers (`model`, `scales`, `data`, `services`, `studies`, `strategies`, `tokens`) do not depend on egui. Keep that boundary when extending the crate.

## Layout

```
src/chart/      chart viewport, series, overlays
src/model/      bars, timescale, domain types
src/scales/     price and time scales
src/data/       feeds, order flow, inbox
src/studies/    indicators
src/drawings/   drawing tools
src/ui/         application chrome
src/ui_kit/     reusable widgets
src/theme/      light/dark presets
src/tokens/     design tokens
examples/demo/  full terminal-style demo
```

## Licence

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE)), or
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option. See [THIRD-PARTY-NOTICES.md](THIRD-PARTY-NOTICES.md) for third-party assets.
