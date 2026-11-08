# Alerts on live data

Status: implemented.

The demo evaluates alerts through hidden copies of the chart, each fed by its own copy of the
synthetic data generator, and rebuilds every copy's whole history on every frame. Placing an alert
makes the demo lag, and alerts on BTCUSDT are evaluated on generated prices instead of the live
exchange data the chart shows. This document replaces that with evaluation on the data the chart
already has, once per data change.

Read the whole document first. Every change needs the test listed with it.

---

## 1. How it works today

| Step | Where | What happens |
|---|---|---|
| Every frame | `examples/demo/controller.rs` ~1627 | `DemoApp::ui` calls `self.monitor_alerts(&ctx)` |
| One monitor per alert and symbol | `examples/demo/alert_monitor.rs`, `monitor_alerts` (~340–452) | For each active alert and each of its symbols, an `AlertMonitor` is created once and kept in `self.alert_monitors` |
| A copy of the chart | `AlertMonitor::with_studies` (~60–145) | Builds a hidden `Chart`, copies the studies the alert reads, and forks the synthetic `LiveFeed` with `self.live.fork()` and `set_symbol(symbol)` |
| Every frame, per monitor | `AlertMonitor::refresh` (~181–300) | `live.advance(now)`, then `live.snapshot(...)` |
| The snapshot | `examples/demo/live.rs`, `LiveFeed::snapshot` (~163–205) | Clones the whole minute history, aggregates every generated trade (the generator starts 7 days back), and resamples everything to the chart's interval |
| Every frame | `monitor_alerts` (~430–451) | `engine.update(symbol, timeframe, bars, now, value)` for each symbol and interval |

The commit after this document's first draft already narrowed each monitor to the studies its
alert reads and made the studies update from the first changed bar. The snapshot rebuild and the
synthetic data remain.

### 1.1 Problems

1. **Cost per frame grows with history × alerts.** `LiveFeed::snapshot` rebuilds the full series
   from minutes and trades on every call, and every monitor calls it every frame. A 24/7 symbol on
   a 1 h chart has the most generated data, so BTCUSDT is the worst case.
2. **Wrong data for exchange symbols.** `is_binance_symbol` (`examples/demo/binance/types.rs` ~26)
   routes BTCUSDT to the live exchange feed for the chart, but the monitor forks the synthetic
   `LiveFeed` and calls `set_symbol("BTCUSDT")`, which generates its own prices. An alert on the
   BTCUSDT chart is tested against numbers the user never sees.
3. **Evaluation is per frame, not per data change.** `engine.update` and every monitor run on
   every frame, including frames caused only by pointer movement.
4. **Monitors request repaints** on the synthetic trade schedule
   (`ctx.request_repaint_after(delay)` ~423), even when the chart has no alert-relevant change.

---

## 2. Target behaviour

Alerts are a host concern. The chart only draws price-alert levels (`render_alert_levels`, already
in place). The host evaluates alerts on the data stream it already receives:

1. **Alerts on the chart's symbol and interval** are evaluated on `self.chart` itself:
   `self.chart.bars(SeriesId::MAIN)` and `self.chart.alert_value(source, bar)`. No `AlertMonitor`,
   no forked feed, no copied studies. This covers BTCUSDT on its live exchange data.
2. **Alerts on another symbol, another interval or a watchlist** keep an `AlertMonitor`, but:
   - it is only for symbols the synthetic generator serves (`!is_binance_symbol(symbol)`);
   - its snapshot is rebuilt only when its feed produced new trades or a new bar since the last
     refresh, not on every frame;
   - extending the series is incremental (section 3.3).
3. **Alerts on an exchange symbol at an interval the chart does not show** are not evaluated with
   generated data. They wait until the chart shows that symbol and interval. The demo shows a
   one-time notice when such an alert is saved (section 3.4).
4. **Evaluation runs when data changed:** a new or updated last bar, a new bar count, or a study
   value change at the last bar. A frame with only pointer movement does no alert work.
5. Expiry still advances on time: keep `engine.tick(now)` and the existing one-second repaint for
   alerts with an expiry (~353–358).

---

## 3. Changes by file

### 3.1 `examples/demo/alert_monitor.rs`, `monitor_alerts`

1. Split active alerts into:
   - **on-chart**: no watchlist, `alert.symbol == self.chart.symbol()`, and the alert's timeframe
     (or `None`) equals `self.chart.timeframe()`, and no replay is running;
   - **off-chart**: everything else.
2. On-chart alerts:
   - Keep a key of the chart's data, for example `(bars.len(), bars.last().cloned())` of
     `self.chart.bars(SeriesId::MAIN)`, in a new `DemoApp` field (for example
     `alert_chart_key: Option<(usize, Option<Bar>)>`).
   - When the key changed since the last call, call
     `engine.update(symbol, timeframe, self.chart.bars(SeriesId::MAIN), now, |source, bar|
     self.chart.alert_value(source, bar))` once for that symbol and interval.
   - Do not create or keep an `AlertMonitor` for these alerts. Remove existing monitors whose key
     is on-chart.
3. Off-chart alerts:
   - Skip symbols where `crate::binance::types::is_binance_symbol(symbol)` is true (see 3.4).
   - Keep `AlertMonitor` for the rest, with 3.2 and 3.3.
   - Call `engine.update` for a monitor's symbol and interval only when that monitor's `refresh`
     reports that its bars changed.
4. Request a repaint only from monitors whose feed is due (the delay `advance` returns), and never
   for on-chart alerts; the chart's own data already repaints.

### 3.2 `AlertMonitor::refresh`

1. Return whether the bars changed, in addition to the delay, for example
   `-> (std::time::Duration, bool)`.
2. Before calling `live.snapshot`, ask the feed whether anything changed since the last snapshot.
   Add to `LiveFeed` (`examples/demo/live.rs`) a cheap counter that `advance` bumps when it creates
   trades or crosses a second, for example `fn generation(&self) -> u64`. Store the last seen value
   in the monitor and skip the snapshot when it did not change.
3. Keep the incremental study update from the first changed bar, already in place.

### 3.3 `LiveFeed::snapshot` (`examples/demo/live.rs` ~163)

Make repeated snapshots cheap without changing their result:

1. Cache, per `(timeframe, extended_hours, clock settings)`, the resampled bars of the completed
   minute history, and the number of history minutes and trades they cover.
2. On the next call, aggregate only the trades after the cached ones, resample only the minutes
   from the last cached bar's open, and replace or append the tail bars.
3. Invalidate the cache when the symbol, the view (`set_view`), the dividend adjustment or the
   history changes.
4. The returned `Vec<Bar>` must equal what the current full rebuild returns. Test 5.2.4 checks
   this on many steps.

### 3.4 Exchange symbols (`examples/demo/alert_monitor.rs`, `examples/demo/panel_alerts.rs`)

1. Never fork the synthetic `LiveFeed` for a symbol where `is_binance_symbol` is true.
2. When an alert on such a symbol is saved with a timeframe that differs from the chart's, show
   one toast through `self.toasts` with this text through `l10n::text`:
   "This alert is checked while the chart shows its symbol and interval."
   Show it once per alert id per session.
3. Do not reject the alert; it is evaluated once the chart shows that symbol and interval.

### 3.5 What stays

- `AlertEngine` (`src/services/alert_engine.rs`) is unchanged. It already evaluates the newest bar
  and keeps run state per alert and symbol.
- `render_alert_levels` is unchanged.
- The study menu's "Add alert on {name} at {value}…" is unchanged. There is no alert button in the
  legend row.
- Replay keeps its current behaviour: alerts are not evaluated against replayed bars on the chart.

---

## 4. Rules

- Every visible string through `l10n::text` or `l10n::format_text`.
- No new hard-coded `Color32` in UI code.
- Saved alerts keep loading; no change to the stored alert format.
- The scan below must print nothing (the quotes split the two words on purpose):
  `git diff | grep -iE "^\+.*(trading""view|v""ela)" | grep -v WaveLabel`
- Do not weaken, skip or delete tests. Change an expectation only where this document changes the
  behaviour on purpose, and list it in the report.
- Do not commit or push unless asked. Commit messages carry no `Co-authored-by` trailer.

---

## 5. Tests

### 5.1 Existing tests that must keep passing

In `examples/demo` (`cargo test` there):

- `controller::alert_monitor::tests::replay_preserves_alert_pause_state`
- `controller::alert_monitor::tests::alerts_on_one_symbol_see_one_series`
- `controller::alert_monitor::tests::monitors_copy_only_the_studies_their_alert_reads`
- `controller::tests::live_alerts_survive_symbol_changes_replay_and_watchlist_membership`
- `controller::tests::alt_a_creates_a_host_alert_at_the_hovered_price_and_dragging_submits_its_new_level`
- `controller::tests::a_settled_demo_does_not_repaint_every_frame`

If `monitors_copy_only_the_studies_their_alert_reads` assumes a monitor for an on-chart alert,
change it to an off-chart symbol (another catalog symbol than the chart's) and say so in the report.

### 5.2 New tests (`examples/demo/alert_monitor.rs` unless noted)

1. **`on_chart_alerts_use_the_chart_and_no_monitor`**: a price alert and a study alert on the
   chart's symbol and interval. After several `monitor_alerts` calls, `alert_monitors` is empty,
   and a price crossing on the chart's own bars (push a bar with `self.chart.update`) fires the
   price alert once.
2. **`pointer_only_frames_do_no_alert_work`**: with an on-chart alert, run frames that only move
   the pointer. `engine.update` must not be called: wrap the evaluation in a counter field on
   `DemoApp` used only by tests, or assert that the stored data key does not change and the alert
   log stays the same.
3. **`exchange_symbols_get_no_synthetic_monitor`**: an alert on `BTCUSDT` at an interval other than
   the chart's. No monitor is created for it, a toast with the text of 3.4 appears once, and the
   alert stays `Active`.
4. **`incremental_snapshots_match_a_full_rebuild`** (`examples/demo/live.rs`): for a 24/7 symbol
   and for a session symbol, on `Min1`, `Hour1` and `Day1`, with and without extended hours: call
   `advance` in many small steps; after each step the cached `snapshot` equals a snapshot from a
   fresh `fork()` of the same feed (which has no cache).
5. **`off_chart_monitors_skip_unchanged_feeds`**: an alert on another catalog symbol. Two
   `refresh` calls at the same `now` report "unchanged" on the second call and leave the bars equal.

### 5.3 Frame cost

Add an `#[ignore]` test (or a temporary one removed before the report) that runs the demo at
1280×720 with BTCUSDT-like 24/7 data on `Hour1`, the studies GMMA, HMA, KAMA, LSMA, RSI, MACD and
Elder Ray, and 0, 5 and 15 study alerts, moving the pointer each frame. Report the average frame
time for each count before and after the change. After the change, 15 alerts must cost less than
1 ms more per frame than 0 alerts in a `--release` run.

---

## 6. Work order

1. 3.1 on-chart evaluation, with tests 5.2.1 and 5.2.2.
2. 3.4 exchange symbols, with test 5.2.3.
3. 3.2 and 3.3 cheap off-chart refresh, with tests 5.2.4 and 5.2.5.
4. 5.3 frame cost before and after.
5. Full checks, each must exit 0:
   - `cargo fmt --check`
   - `cargo clippy --all-features --all-targets -- -D warnings`
   - `cargo clippy --no-default-features --all-targets -- -D warnings`
   - `cargo test --lib` (the whole suite)
   - `cd examples/demo && cargo test`
   - `cd terminal && cargo check --tests` (restore `terminal/Cargo.lock` if only the check
     changed it)
6. Rebuild the release demo (`trunk serve --release --address 127.0.0.1 --port 8081` in
   `examples/demo`), wait for "success" with no warnings, and check in a browser: place three study
   alerts on BTCUSDT 1 h; moving the pointer over the chart stays smooth.
7. Set Status at the top to "implemented" only when every check passes and step 6 holds. Otherwise
   "partly implemented", with the list of what is open.
