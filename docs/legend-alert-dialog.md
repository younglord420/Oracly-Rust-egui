# Legend alert button, more-actions button and the alert dialog

Status: partly implemented. Code, tests and clippy are done. The release demo has not been rebuilt in a browser yet.

The study legend row lost two buttons: the bell (Add alert) and the three dots (More actions).
Study alerts also skip the dialog: "Add alert on {name} at {value}…" saves the alert at once.
This document brings both buttons back and sends every study alert through the Create Alert
dialog, as drawing and series alerts already do.

Read the whole document first. Every change needs the test listed with it.

---

## 1. Today

| What | Where | State |
|---|---|---|
| Legend buttons | `src/chart/renderers/legend/mod.rs`, `study_controls` (~827–983) | Eye, Settings, Move (when the host allows it), Trash. No bell, no three dots |
| Button list | same, ~879–883 | `kinds = [0, 1, (2), 4]`; 4 draws `TRASH` |
| Study menu | right click on the row (`row_response.secondary_clicked()`, ~841) or Shift+F10 | Works, includes "Add alert on {name} at {value}…" |
| Study alert built | `src/ui/context_menu/study.rs`, `study_alert` (~378) | Crossing at the study value of the bar, source = study output (or Volume) |
| Host handles study alert | `examples/demo/controller.rs` ~3244 | `StudyContextMenuAction::AddAlert(alert) => self.alerts.save(*alert)`: saved without a dialog |
| Host handles drawing alert | same, ~3332–3335 | `list_alert_sources()` then `alert_dialog.edit(*alert)`: dialog opens prefilled |
| Host handles series alert | same, ~3305 | `open_alert(None)`: dialog opens |
| Dialog | `src/ui/dialogs/alert_dialog/dialog.rs` | `edit(alert)` with `alert.id == None` shows "Create Alert" (~300–304); saving happens only on `AlertDialogAction::Create` (`controller.rs` ~1368–1369) |

History:

- Commit `6ddb316` replaced the three dots with Move and Alert.
- Commit `e9f512c` removed the bell again, because alerts made the demo lag.

That lag is fixed. Alerts on the symbol and interval on screen are evaluated on the chart's own
bars, only when the bars change (`docs/alerts-live-data.md`). The bell can return.

The code for the bell is in `git show 6ddb316`, which helps for reference. Do not revert
`e9f512c`: it also holds the alert-evaluation fix.

---

## 2. Target behaviour

### 2.1 Row buttons

The revealed row shows, left to right:

1. **Eye**: Hide / Show (unchanged; a hidden study keeps its eye visible without hover).
2. **Settings** (unchanged).
3. **Move to pane**, only when the host allows moves (unchanged).
4. **Add alert**: `icons::ALERT_ADD` (`src/icons/mod.rs` ~993). Tooltip "Add alert".
5. **Remove**: `TRASH` (unchanged).
6. **More actions**: `icons::UI_MORE_HORIZONTAL` (`src/icons/mod.rs` ~534). Tooltip
   "More actions".

Rules:

- **Add alert** shows only when all of these hold:
  - the chart has a symbol;
  - the study has a finite value at the bar the legend shows (the hovered bar, else the latest
    one) in a visible plot;
  - the host has not hidden `Control::Alerts`;
  - context menus are on.

  This is the same rule `6ddb316` used.
- **More actions** always shows while the row is revealed, unless context menus are hidden
  (`Control::ContextMenus`). A click opens the study menu with its top-left corner at the
  button's bottom-left, like before `6ddb316`. Right click and Shift+F10 keep opening the menu
  at the pointer.
- Every label goes through `l10n::text`. Button size, gap, hover fill and icon tint follow the
  existing buttons: 18 wide, 24 pitch, `theme.ui.btn_bg_hover`, `text_secondary`/`text`.
- The capsule width is `n × 18 + (n − 1) × 6` for the `n` buttons actually shown. It never leaves
  an empty slot and never covers the pane's top-right cluster.

### 2.2 Clicking the bell or "Add alert on …" opens the dialog

Both open the **Create Alert** dialog prefilled with:

- symbol = chart symbol;
- interval = chart interval (`alert.timeframe = Some(self.state.timeframe)`);
- source = the study output, or Volume for a volume study, as `study_alert` builds it;
- condition = Crossing at the study value of the bar;
- the dialog's source list = `self.chart.alert_sources()` (through `list_alert_sources()`).

The alert is saved only when the user presses Create. Cancel, Esc or closing the dialog saves
nothing.

Unchanged:

- Alt+A on the chart (`ContextMenuAction::AddAlert`, `quick_price_alert`, `controller.rs` ~3285)
  stays a one-step price alert without a dialog.
- Drawing and series alert paths stay as they are.

---

## 3. Changes by file

### 3.1 `src/chart/renderers/legend/mod.rs`

1. `StudyAction`: add `AddAlert(usize)` back. Keep `Menu { index, pos }` for the three dots, the
   right click and Shift+F10.
2. `Legend`: add `pub(crate) study_alerts: bool` back (default `false`).
3. `study_controls(ui, row, controls, id, visible, moves, alerts)`:
   - Build `kinds` as `[0, 1]`, then `2` if `moves`, `3` if `alerts`, then `4` (remove) and
     `5` (more).
   - Return `([Response; 3], Option<Response> /* move */, Option<Response> /* alert */,
     Option<Pos2> /* menu */)`.
   - `menu` is the right-click position, or `more.rect.left_bottom()` when the three dots were
     clicked.
   - Icons: 3 → `ALERT_ADD`, 5 → `UI_MORE_HORIZONTAL`. Without the `icons` feature, the text
     glyphs are `"A"` and `"•••"`.
   - Keep the focus range in sync with the button count: `(0..n)` in `study_controls` and in
     `legend_row_revealed`.
4. Overlay loop (~568–630):
   - `alert` = `self.study_alerts && any visible plot has a finite value at self.bar_index`.
   - `button_count = 4 + usize::from(self.study_moves) + usize::from(alert)`, for eye, settings,
     remove and more, plus the optional ones.
   - The bell returns `StudyAction::AddAlert(index)`.

### 3.2 `src/widget/indicator_pane.rs`

- Put back `legend_alerts: bool` and `PaneInteraction::alert_response`.
- Compute `alert` with the same rule as 3.1 step 4, using `self.bar_index`.
- `buttons = 4 + usize::from(self.legend_moves) + usize::from(self.legend_alerts)`, in the
  controls-width code around `~1551`.

### 3.3 `src/widget/panes.rs`

- Set `panel.legend_alerts = self.legend_menus.alerts && !self.symbol.is_empty()`.
- Destructure `alert_response`, include it in `control_clicked`, and map a click to
  `StudyAction::AddAlert(idx)`.

### 3.4 `src/widget/mod.rs`

- `LegendMenus { moves: bool, alerts: bool }`.
- `StudyMenuRequest::Alert { id: SeriesId, bar: usize }`.
- Doc comments name Move and Alert again.

### 3.5 `src/widget/crosshair.rs` (~105–150)

- Add `StudyAction::AddAlert(index)` to the index match.
- On `AddAlert`, set `self.study_menu_request = Some(StudyMenuRequest::Alert { id, bar })`.
  `bar` is the bar the legend shows: the hovered bar, else the last one.
- Treat it like `Menu`/`Move` in the final match (no chart change).

### 3.6 `src/ui/context_menu/chart_menus/mod.rs` (~160–180)

- `chart.legend_menus = LegendMenus { moves: menus_on, alerts: menus_on &&
  !self.study.hidden.contains(&Control::Alerts) }`.
- Handle `StudyMenuRequest::Alert { id, bar }`. If `study_alert(chart, id, bar)` returns
  `Some((alert, _))`, return
  `ChartContextAction::Study { id, action: StudyContextMenuAction::AddAlert(Box::new(alert)) }`.
  The bell and the menu entry produce the same action. See `git show 6ddb316` for the original
  lines.

### 3.7 `examples/demo/controller.rs` ~3244

Replace the direct save with the dialog:

```rust
StudyContextMenuAction::AddAlert(alert) => {
    let mut alert = *alert;
    alert.timeframe = Some(self.state.timeframe);
    self.list_alert_sources();
    self.alert_dialog.edit(alert);
}
```

Keep this host-side. The library still only returns the action, and other hosts may save
directly if they want to.

---

## 4. Rules

- Every visible string through `l10n::text` or `l10n::format_text`.
- No new hard-coded `Color32` in UI code; use `theme.*`.
- No change to the stored alert format.
- The scan below must print nothing (the quotes split the two words on purpose):
  `git diff | grep -iE "^\+.*(trading""view|v""ela)" | grep -v WaveLabel`
- Do not weaken, skip or delete tests. Change an expectation only where this document changes the
  behaviour on purpose, and list it in the report.
- Do not commit or push unless asked. Commit messages carry no `Co-authored-by` trailer.

---

## 5. Tests

### 5.1 Existing tests to update

- `chart::renderers::labels::legend::tests::study_controls_reveal_on_touch_and_dismiss_outside`
  (`src/chart/renderers/legend/mod.rs` ~1335):
  - the labels include "Add alert" (when enabled) and "More actions";
  - the keyboard check walks every button;
  - Enter on "More actions" returns the menu corner at that button's bottom-left.
- Any test that calls `study_controls` with the old arity.
- Demo tests that expect a study alert to be saved at once, if any. Search for
  `StudyContextMenuAction::AddAlert` and `alerts.alerts().len()` in `examples/demo`. They now
  expect the dialog open and no saved alert until Create.

### 5.2 New tests

1. **`legend_shows_bell_and_more_actions`** (`src/chart/renderers/legend/mod.rs`): a revealed row
   with moves and alerts enabled paints six buttons in the order of 2.1. With alerts disabled,
   or with no value at the bar, there are five buttons and no empty slot. The capsule width
   matches `n × 18 + (n − 1) × 6`.
2. **`more_actions_opens_the_study_menu_under_the_button`** (`src/ui/context_menu/chart_menus/tests.rs`):
   clicking the three dots opens the study menu. Its corner is the button's bottom-left, and it
   contains "Add alert on …".
3. **`bell_returns_the_same_alert_as_the_menu_entry`** (`src/ui/context_menu/chart_menus/tests.rs`):
   on a chart with a symbol and an SMA, the bell returns
   `ChartContextAction::Study { action: StudyContextMenuAction::AddAlert(alert) }`. The alert
   equals `study_alert(chart, id, bar)` for the shown bar. With `Control::Alerts` hidden, there
   is no bell.
4. **`study_alerts_open_the_create_dialog`** (`examples/demo/controller.rs`):
   - `handle_study_context(.., StudyContextMenuAction::AddAlert(alert))` opens `alert_dialog`;
   - the dialog title is "Create Alert", the source is the study output, the condition is
     Crossing at the study value, and the interval is the chart's;
   - `alerts.alerts()` is unchanged.

   Pressing Create saves exactly one alert. A second run that cancels saves none.
5. Hidden study: the eye is still clickable without hover, and the bell is not shown, because a
   hidden study's values are not offered.

### 5.3 Must keep passing

- `controller::alert_monitor::tests::*`, especially `pointer_only_frames_do_no_alert_work`.
- `controller::tests::alt_a_creates_a_host_alert_at_the_hovered_price_and_dragging_submits_its_new_level`
- `controller::tests::study_menu_host_actions_keep_sources_and_clipboard_settings`
- `widget::panes::tests::panes_fit_and_share_the_lowest_time_axis` (legend must not overlap the
  pane cluster: `controls_left = chart_rect.right() - 60.0`).
- `chart::renderers::labels::legend::tests::quote_controls_and_collapse_belong_to_each_legend`
- The release frame test, which must stay under 1 ms extra for 15 alerts:
  `cd examples/demo && cargo test --release -- --ignored fifteen_study --nocapture`

---

## 6. Work order

1. 3.4, 3.1 and 3.2 (buttons and their wiring types), with tests 5.1 and 5.2.1.
2. 3.3, 3.5 and 3.6 (requests to actions), with tests 5.2.2, 5.2.3 and 5.2.5.
3. 3.7 (dialog in the demo), with test 5.2.4.
4. Full checks, each must exit 0:
   - `cargo fmt --check` (root) and `cd examples/demo && cargo fmt --check`
   - `cargo clippy --all-features --all-targets -- -D warnings`
   - `cargo clippy --no-default-features --all-targets -- -D warnings`
   - `cargo test --lib`
   - `cd examples/demo && cargo test`
   - `cd examples/demo && cargo clippy --all-targets -- -D warnings`. This already fails before
     this work, on `feeds.rs` ~107 (`sort_by` → `sort_by_key` with `Reverse`), `feeds.rs` ~196
     (`n % 2 == 0` → `n.is_multiple_of(2)`) and `live.rs` ~468 (give the
     `Mutex<Option<(Vec<String>, Events<(String, Quote)>)>>` a `type` alias). Fix those three so
     the command passes.
   - `cd terminal && cargo check --tests` (restore `terminal/Cargo.lock` if only the check
     changed it).
5. Rebuild the release demo (`trunk serve --release --address 127.0.0.1 --port 8081` in
   `examples/demo`) and wait for "success" with no warnings. Then check in a browser on BTCUSDT
   1 h with a moving average:
   - hover the legend row and see Eye, Settings, Move, Bell, Trash, three dots;
   - click the bell: the Create Alert dialog opens prefilled; Cancel saves nothing; Create saves
     one alert;
   - the right-click entry "Add alert on …" opens the same dialog;
   - the three dots open the study menu under the button;
   - with three alerts placed, moving the pointer stays smooth.
6. Set Status at the top to "implemented" only when every check passes and step 5 holds.
   Otherwise set it to "partly implemented", with the list of what is open.
