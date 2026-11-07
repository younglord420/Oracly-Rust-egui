# Study legend rows and pane controls

Status: partly implemented. A hidden study keeps its pane and its eye. The legend no
longer collapses the pane. Plot values have no swatch. The row shows Move and Add alert when
`ChartMenus` is running: Move opens the "Move to" targets, and Add alert returns the same
alert the study menu builds. Still open: hiding values while a row is open, tests 5.2.2–5.2.6
and 5.2.8, the full check list, and the screenshots. The magnet test in section 5.1 stays as
it was: that study is an overlay on the price pane, so hiding it leaves the magnet on the main bar.

This document fixes how a study's legend row and its pane behave when the user hides a study,
collapses a pane, moves a study or adds an alert. It follows the behaviour of a mature charting
product, described in section 2 as rules and sizes, so no outside reference is needed.

Four problems in the running demo:

1. **The eye removes the study.** Hiding a study in an indicator pane makes the pane, its legend
   and its eye disappear. The user cannot bring the study back from the chart.
2. **The legend collapses the pane.** The whole legend row of a pane is a "Collapse pane" button.
   Hovering the study name shows that tooltip, and a click folds the pane.
3. **A coloured block sits in the values.** "RSI 14 ▮ 57.05": a plot colour that fails contrast is
   drawn as a tall swatch before its value.
4. **No Move or Alert in the row.** Both exist only inside the right-click menu.

Read the whole document first. Every change needs the test listed with it.

---

## 1. Files

| Area | File and place |
|---|---|
| Study actions | `src/chart/renderers/legend/mod.rs`, `enum StudyAction` (~36) |
| Overlay rows on the price pane | `src/chart/renderers/legend/mod.rs`, study loop in `show_with_colors` (~428–616), collapse chevron (~617–660) |
| Row buttons | `src/chart/renderers/legend/mod.rs`, `study_controls` (~802–920), `legend_row_revealed` (~786) |
| Pane legend | `src/widget/indicator_pane.rs`: early return on hidden (~584), controls and legend (~842–870), `legend_job` (~1454), `legend_controls_rect` (~1529), `draw_legend` (~1569) |
| Pane members and header toggle | `src/widget/panes.rs`: entries filter (~528–545), header "Collapse pane" toggle (~808–850), `pending_pane_collapse` (~1220) |
| Pane visibility | `src/widget/pane_layout.rs`, `pane_shown` (~162) |
| Pane buttons | `src/widget/pane_layout.rs`, `pane_maximize_button` (~812), `pane_actions` "..." menu (~852–990) |
| Applying study actions | `src/widget/crosshair.rs`, `apply_study_action` (~107) |
| Study menu, Move to, Add alert | `src/ui/context_menu/study.rs`: `StudyContextMenu::open` builds the alert (~145–175), "Move to" entries (~277–290), `apply` (~374) |
| Menu host | `src/ui/context_menu/chart_menus/mod.rs`, `ChartMenus::show` (~160–297) |
| Readable colours | `src/theme/style.rs`, `readable_color` (~18) |
| Icons | `src/icons/mod.rs`: `EYE`, `EYE_HIDE`, `SETTINGS`, `MOVE_PANE`, `ALERT_ADD`, `TRASH`, `CHEVRON_UP`, `CHEVRON_DOWN`, `MAXIMIZE` |
| Demo host | `examples/demo/controller.rs`: settings request (~3043), study menu actions (~3231) |

---

## 2. Target behaviour

### 2.1 Legend row

One row per study, the same in the price pane (overlays) and in indicator panes.

**Idle row** (pointer elsewhere, not selected):

- Title, inputs, then values, on one line.
- Background: `theme.chart.bg` at the configured backdrop opacity (`indicator_backdrop_opacity`),
  radius 4.
- No border, no buttons.

**Open row** (title hovered, row focused, or row selected by touch or click):

- Background: `theme.chart.bg`, fully opaque. Border: 1 px `theme.ui.border`, radius 4.
- The **values step aside**: they are not drawn while the row is open. The buttons take their
  place right after the title and inputs. This keeps the row short and stops buttons covering
  numbers.
- Buttons, in this order: **eye, settings, move, alert, remove**.

**Hidden study** (eye off):

- Title and inputs at half opacity (`gamma_multiply(0.5)`). No values.
- The eye button shows `EYE_HIDE` and stays visible even when the row is idle, so the study can
  always be shown again without hovering.

**Clicks on the row** (not on a button):

| Input | Result |
|---|---|
| Left click | Select the study (the row stays open) |
| Double click | Open the study's settings |
| Right click | Open the study menu at the pointer (unchanged) |
| Middle click | Remove the study |
| Keyboard, row focused, Shift+F10 | Open the study menu below the row |

A click on the row never collapses the pane.

**Sizes:**

| Item | Size |
|---|---|
| Button hit target | 18 × 18, radius 3 |
| Icon | 16 × 16, centred in the target |
| Gap between buttons, and title to first button | 6 |
| Row padding | 2 vertical, 6 horizontal |
| Row height | at least 22 (button plus padding) |
| Gap title to values | 6 |
| Gap between values | 5 |
| Title | strong (bold) |

Button hover: `theme.ui.btn_bg_hover` fill inside the target. Icon tint `theme.ui.text_secondary`,
`theme.ui.text` on hover. Tooltips through `l10n::text`: "Hide", "Show", "Settings",
"Move to pane", "Add alert", "Remove".

**Values:**

- Each value is drawn in its plot colour, with no swatch.
- A plot colour that fails contrast 4.5 against `theme.chart.bg` is moved towards `theme.ui.text`
  with `theme::readable_color(color.to_opaque(), theme.ui.text, &[theme.chart.bg], 4.5)`. The
  hue stays recognisable and the number stays readable.
- A missing value is `scales::NO_VALUE` in `theme.ui.text_muted`.
- Remove the swatch code (`swatch.background = color` in `legend_job`) and the
  `contrast >= 4.5 ? plot : colors.text` choice in the overlay loop. Both rows use
  `readable_color`.

### 2.2 Move button

`MOVE_PANE` icon. A click opens a small menu under the button with exactly the targets of the
study menu's "Move to" submenu: "New pane above", "New pane below", each other pane by title, and
"Main pane" when the study is not already on it. Choosing one moves the study as one undoable
user action, the same as the study menu.

### 2.3 Alert button

`ALERT_ADD` icon. A click creates the same alert as the study menu's "Add alert on {name} at
{value}…" at the bar the legend shows (the hovered bar, else the latest), and hands it to the host
as `ChartContextAction::Study { id, action: StudyContextMenuAction::AddAlert(alert) }`. The host
already saves that action (`examples/demo/controller.rs` ~3231).

The button is shown only when:

- the chart has a symbol, and
- the study has a value at that bar, and
- the host has not hidden `Control::Alerts` (`ChartMenus::set_control_visible`).

### 2.4 Pane controls

Collapsing, moving and maximizing a pane belong to a **pane button cluster**, not to the legend.

- Place: top-right of the pane's data area, just left of the price axis, 4 from the pane top.
- Shown while the pointer is inside the pane (respect `config.pane_buttons`:
  `Always`, `OnHover`, `Never`).
- Background: one pill, `theme.chart.bg` at 0.9 opacity, 1 px `theme.ui.border`, radius
  `theme.radius.md`, padding 2, gap 2. Buttons 20 × 20, icon 12–14.
- Buttons for an indicator pane, in order:
  1. **Move pane up** (`CHEVRON_UP`), disabled at the top (opacity 0.3).
  2. **Move pane down** (`CHEVRON_DOWN`), disabled at the bottom.
  3. **Collapse pane / Expand pane**.
  4. **Maximize pane / Restore panes** (`MAXIMIZE`), only when there is more than one pane.
  5. The existing **"..."** menu (merge with pane above or below, remove pane, move a member).
- The price pane gets only Maximize, as today.
- A **collapsed** pane keeps its Expand button visible without hover, drawn as a selected chip
  (`theme.ui.accent` fill or the selected-button colours of the theme), so the pane can always be
  opened again. A **maximized** pane keeps its Restore button visible the same way.
- A collapsed pane is a strip about 26 high. It shows only its first study's legend row, centred
  in the strip.

Reuse the existing actions in `pane_actions` (move up, move down, collapse) and
`pane_maximize_button`. Only their placement and presentation change.

---

## 3. Changes by file

### 3.1 Hidden studies keep their pane (`panes.rs`, `pane_layout.rs`, `indicator_pane.rs`)

1. `panes.rs` ~528–545: keep a hidden study in `entries`. Do not filter on `is_visible()`.
2. `pane_layout.rs` `pane_shown`: a pane is shown when it is listed and has any member, hidden or
   not. Use `pane_member(id)` without `is_series_visible(id)` for studies. Host bar series keep
   their current rule.
3. `indicator_pane.rs` ~584: do not return `None` for a hidden study. Draw the pane background,
   the grid, the legend (dimmed, see 2.1) and the controls, and return the `PaneInteraction`.
   Skip only the plots, fills, markers, value tags and crosshair snapping. The other two early
   returns (~355, ~445) are plot-only paths and keep their return.
4. A hidden study still does not count for auto-scale, magnet snapping or drawing hit tests. The
   scale of a pane whose only study is hidden keeps its last range.

### 3.2 Remove the legend-as-collapse toggle (`panes.rs`)

Delete the `header` rect and the `toggle` response (~808–850), with its "Collapse pane" /
"Expand pane" hover text and its accessibility node. Delete `pending_pane_collapse` if nothing
else sets it. Collapsing is done by the pane cluster (2.4).

### 3.3 Row buttons (`legend/mod.rs`)

1. `StudyAction`: add `Move { index: usize, pos: Pos2 }` and `AddAlert(usize)`. Keep `Menu` for the
   right click and Shift+F10.
2. `study_controls`: five buttons in the order of 2.1, sizes of 2.1. Return their responses so the
   callers map them to `ToggleVisibility`, `Settings`, `Move`, `AddAlert`, `Remove`. Remove the
   "More actions" button.
3. A new parameter tells `study_controls` which optional buttons exist (move, alert), so the
   capsule width is `n × 18 + (n − 1) × 6` and never leaves an empty slot.
4. A hidden study draws its eye even when the row is not revealed.
5. While the row is open, the overlay loop and `legend_job` skip the values (2.1).
6. Double click on the row returns `Settings`; middle click returns `Remove`; Shift+F10 on the
   focused row returns `Menu` at the row's left-bottom corner.
7. The overlay collapse chevron (fold every legend row) stays, outside the last row's capsule.

### 3.4 Pane legend (`indicator_pane.rs`)

Same row as the overlay: one rect from the legend's left edge to the end of the buttons, the
fill and border of 2.1, values hidden while open. `legend_controls_rect` places the buttons right
after the title and inputs, not after the values.

### 3.5 Move and alert requests (`crosshair.rs`, `chart_menus/mod.rs`, `study.rs`, `widget/mod.rs`)

1. `widget/mod.rs`: add `pub(crate) study_menu_request: Option<StudyMenuRequest>` to `Chart`, where
   `StudyMenuRequest` is `Move { id: SeriesId, pos: Pos2 }` or `Alert { id: SeriesId, bar: usize }`.
   Add `pub(crate) legend_menus: LegendMenus { moves: bool, alerts: bool }`, default both `false`.
2. `crosshair.rs` `apply_study_action`: `Move` sets `study_menu_request = Move { id, pos }`;
   `AddAlert` sets `Alert { id, bar }` with the legend's bar (hovered, else the latest).
3. `study.rs`: move the alert construction out of `StudyContextMenu::open` into
   `pub(crate) fn study_alert(chart, id, bar) -> Option<(Alert, String)>`, used by both the menu
   and the button. Add `StudyContextMenu::open_moves(chart, id)` whose `build_menu_items` returns
   only the "Move to" targets (no submenu, no other entries).
4. `chart_menus/mod.rs` `ChartMenus::show`:
   - At the start of each call set `chart.legend_menus = LegendMenus { moves: true, alerts:
     !self.study.hidden.contains(&Control::Alerts) }`. A chart without `ChartMenus` shows no Move
     or Alert button.
   - Take `chart.study_menu_request`. For `Move`, open the moves-only study menu at `pos`; the
     chosen `MoveTo` is applied through the existing `study_context_menu::apply` in one user
     action. For `Alert`, build the alert with `study_alert` and return
     `ChartContextAction::Study { id, action: AddAlert(alert) }` without opening a menu.
5. The legend shows the Move button when `legend_menus.moves`, the Alert button when
   `legend_menus.alerts` and `study_alert` would return `Some`.

### 3.6 Pane cluster (`pane_layout.rs`)

Build the cluster of 2.4 from `pane_actions` and `pane_maximize_button`: one pill, buttons in the
order of 2.4, the "..." menu last. Keep the existing action numbers and undo grouping. A collapsed
or maximized pane keeps its state button visible without hover.

---

## 4. Rules

- Every visible string through `l10n::text` or `l10n::format_text`.
- Colours, sizes and radii from the theme (`theme.ui`, `theme.chart`, `theme.radius`,
  `theme.spacing`) and `DESIGN_TOKENS`. No new hard-coded `Color32` in UI code.
- Do not rename studies, input ids, `PaneTarget` or public actions. Adding variants is fine; mark
  new public enums `#[non_exhaustive]` like their neighbours.
- Saved layouts keep loading: visibility is already saved per study; no new saved fields.
- The scan below must print nothing (the quotes split the two words on purpose):
  `git diff | grep -iE "^\+.*(trading""view|v""ela)" | grep -v WaveLabel`
- Do not weaken, skip or delete tests. Change an expectation only where this document changes the
  behaviour on purpose, and list it in the report.

---

## 5. Tests

### 5.1 Expectations that change on purpose

| Test | File | Old expectation | New expectation |
|---|---|---|---|
| `magnet_snaps_to_study_values_on_their_own_scale` | `src/widget/price_series.rs` ~1624 | after hiding the study, the pane is gone and the point maps to the main pane | the pane stays; the point is still in the study pane but snaps to nothing |
| `merged_panes_share_scales_and_studies_can_return_to_the_main_pane` | `src/widget/panes.rs` ~1989 | hiding both studies makes the price pane taller than 500 | the panes stay; the price pane keeps its height |
| `legend_values_identify_each_plot_without_low_contrast_text` | `src/widget/indicator_pane.rs` ~1836 | a low-contrast plot gets a swatch in its exact colour | the value is drawn in `readable_color(plot)`; no section has a background colour |
| `study_controls_reveal_on_touch_and_dismiss_outside` | `src/chart/renderers/legend/mod.rs` ~1268 | labels include "More actions" | labels are the five buttons of 2.1; "More actions" is gone |
| `quote_controls_and_collapse_belong_to_each_legend` | `src/chart/renderers/legend/mod.rs` | stroked-rect count and keyboard order with the old controls | the same checks with the new buttons |
| `dialogs_expose_titles_and_label_their_fields` and other accessibility tests | `src/ui/accessibility_tests.rs` | any "Collapse pane" legend node | the pane cluster's buttons carry those labels |

`drawings_use_the_owning_study_pane_scale` (`panes.rs` ~1581, "hidden panes have no drawing
hits") must keep passing unchanged: a hidden study's drawings still do not hit.

### 5.2 New tests

1. **`hidden_study_keeps_its_pane_and_eye`** (`panes.rs`): a chart with RSI in its own pane. Hide
   it. After a frame the pane is still rendered with the same rect, its legend text is painted at
   half opacity, the RSI line is not painted, and an `EYE_HIDE` button exists without any pointer
   over the pane. Click it: RSI is visible again.
2. **`legend_click_never_collapses_the_pane`** (`panes.rs`): click the RSI title. The pane is not
   collapsed and RSI is selected. Double click: `take_indicator_settings_request()` returns RSI.
3. **`pane_cluster_collapses_and_expands`** (`pane_layout.rs`): hover the RSI pane, click the
   collapse button. The pane becomes the collapsed strip, its expand button stays painted after
   the pointer leaves, and a click on it restores the pane. Move up and move down are disabled at
   the ends.
4. **`open_row_hides_values_and_shows_five_buttons`** (`legend/mod.rs`): an overlay SMA with
   `ChartMenus` attached. Idle: the value text is painted, no buttons. Hovered: no value text, the
   buttons "Hide", "Settings", "Move to pane", "Add alert", "Remove" in that order, each 18 × 18,
   6 apart, inside one bordered rect.
5. **`move_button_offers_the_study_menu_targets`** (`chart_menus/tests.rs`): click Move on the
   SMA. The menu lists exactly the "Move to" entries of the study menu. Choose "New pane below":
   the SMA is in a new pane and one undo puts it back.
6. **`alert_button_returns_the_study_alert`** (`chart_menus/tests.rs`): click Alert on the SMA.
   `ChartMenus::show` returns `Study { id, action: AddAlert(alert) }` equal to the alert of the
   study menu at the same bar. With `Control::Alerts` hidden, the button is not painted.
7. **`legend_values_have_no_swatch`** (`indicator_pane.rs` and `legend/mod.rs`): a study with a
   low-contrast plot colour. No text section has a non-transparent background, and the value
   colour has contrast ≥ 4.5 with `theme.chart.bg`.
8. **`hidden_overlay_keeps_its_eye`** (`legend/mod.rs`): hide an overlay SMA. Without hover the
   row shows the dimmed title and the `EYE_HIDE` button only.

---

## 6. Work order

1. 3.1 hidden studies keep their pane, with tests 5.2.1 and the first two rows of 5.1.
2. 3.2 remove the legend toggle, and 3.6 the pane cluster, with tests 5.2.2 and 5.2.3.
3. 3.3 and 3.4 rows and buttons, with tests 5.2.4, 5.2.7, 5.2.8 and the legend rows of 5.1.
4. 3.5 move and alert, with tests 5.2.5 and 5.2.6. Wire the demo if it needs anything beyond the
   existing `AddAlert` handling.
5. Full checks, each must exit 0:
   - `cargo fmt --check`
   - `cargo clippy --all-features --all-targets -- -D warnings`
   - `cargo clippy --no-default-features --all-targets -- -D warnings`
   - `cargo test --lib` (the whole suite)
   - `cd examples/demo && cargo test`
   - `cd terminal && cargo check --tests` (restore `terminal/Cargo.lock` if only the check
     changed it)
6. Rebuild the release demo (`trunk serve --release --address 127.0.0.1 --port 8081` in
   `examples/demo`), wait for "success" with no warnings, and take screenshots of: an idle and a
   hovered overlay row, a hidden RSI pane, the pane cluster on hover, a collapsed pane, and the
   Move menu.
7. Set Status at the top to "implemented" only when every check passes and the screenshots match
   section 2. Otherwise "partly implemented", with the list of what is open.
