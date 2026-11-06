# Chart settings dialog: top tabs, live preview, and bug fixes

Status: implemented.

Decisions already taken by the user (do not change them):

- Tabs move from the left sidebar to a **row of tabs at the top**, like the reference screenshot
  (icon + label per tab, active tab highlighted).
- Add an **Appearance preview** column on the right of the content.
- **Keep Ok / Cancel.** Changes are not applied to the real chart until Ok.
- **Keep the current tab set and order:** Symbol, Status line, Scales and lines, Canvas, Trading,
  Alerts, Events. No "Advanced" tab.
- **No new Animation group.** Keep "Zoom smoothing" as it is.

Files:

- Dialog: `src/ui/dialogs/chart_settings/mod.rs`
- Tabs: `src/ui/dialogs/chart_settings/tabs/{mod,symbol,scales,canvas,trading}.rs`
- Tab enum / config / actions: `src/ui/top_toolbar/components/settings_menu/{types,config,actions}.rs`
- Quick popup menu: `src/ui/top_toolbar/components/settings_menu/menu.rs`
- Settings state and apply: `src/widget/settings/{apply,data,types,mod}.rs`
- Host wiring: `examples/demo/controller.rs` (`open_chart_settings_dialog` ~line 1047,
  `apply_chart_settings` ~1356, dialog `show` match ~1280), `terminal/src/studies.rs` (quick menu).

Line numbers are from commit `6367e7c`; search by the quoted code if they drift.

---

## 1. Layout: tabs on top

### Current

`draw_content` (`chart_settings/mod.rs` ~lines 336–427) uses `SidebarLayout` with a 164 px
sidebar (`SIDEBAR_WIDTH`) of `SidebarTab`s, a vertical seam line, and the active tab's content in a
`ScrollArea`.

### Target

```
┌ Chart settings ───────────────────────────────────────────── × ┐
│ [Symbol] [Status line] [Scales and lines] [Canvas] [Trading]   │
│ [Alerts] [Events]                                              │  ← one row; wraps or scrolls
├───────────────────────────────────────┬────────────────────────┤
│ tab content (scrolls vertically)      │ APPEARANCE PREVIEW     │
│                                       │ (small live chart)     │
├───────────────────────────────────────┴────────────────────────┤
│ [Template ▾]                                  [Cancel]  [Ok]   │
└────────────────────────────────────────────────────────────────┘
```

1. Replace `SidebarLayout` with a top tab row. Reuse `ui_kit::tab_bar::TabBar`
   (`src/ui_kit/tab_bar/mod.rs`), and add icon support to it:
   - add `TabLabel::tab_icon(&self) -> Option<&'static Icon>` with a default `None`;
   - when present, paint the icon (16 px, same tint as the label) before the label;
   - implement it for `SettingsTab` using the existing `SettingsTab::icon()` (`types.rs` line 40).
   The active tab uses a filled rounded "pill" (like the screenshot), not only an underline. If
   `TabBar` already offers `.pills()`, use it and add the icon there.
2. The tab row is horizontally scrollable (or wraps with `.pills()`) on narrow windows; labels never
   truncate or overlap.
3. Remove `SIDEBAR_WIDTH`, the seam `vline` and the `SidebarLayout`/`SidebarTab` imports if nothing
   else uses them in this file.
4. **Switch tabs in the same frame.** Today the click writes `active` and the content still matches on
   `self.active_tab`, which is only updated after drawing (`self.active_tab = active;` ~line 384), so
   the old tab shows for one frame. Update `self.active_tab` before drawing the content.
5. **Fixed dialog height.** `show` uses 520 px for most tabs and 460 px for Alerts and Events
   (~lines 284–291), so the dialog jumps when switching tabs. Use one height for every tab (the
   content area scrolls); still honour `config.height` when it is set, and clamp to the window.
6. Default width grows to fit the preview: content ≥ 520 px + preview ≈ 280 px. `SettingsDialogConfig`
   default `width` (`config.rs` line 34, currently 752) becomes ~880 px; the dialog must still fit a
   320 px wide window (hide the preview below ~720 px available width, see section 2).

Tests to update: `changing_settings_tabs_stays_centered_and_fits_the_window` (mod.rs ~1095) looks for
40 px sidebar rows; rewrite it to check that every tab label is fully visible in the tab row, the
dialog keeps the same rect for every tab, and it fits 1440×900, 600×400 and 320×450.

---

## 2. Appearance preview column

Nothing like it exists yet. Build it as a small read-only `Chart` owned by the dialog.

1. Add to `SettingsDialog`: `preview: Option<crate::widget::Chart>` created lazily on first show, with
   ~60 synthetic bars (deterministic random walk around 100, 15m spacing) so it looks like a real
   chart. Symbol text "Sample data" in the header row, the real symbol name on the right if the host
   passed one (optional `set_preview_symbol(&str)`).
2. Every frame the dialog draws, if `self.settings` changed since the last apply (compare a clone or a
   generation counter), call `self.settings.apply_to_chart(&mut preview)`; ignore its error (the footer
   already reports invalid input).
3. Draw it in a fixed-width right column (≈280 px) titled "APPEARANCE PREVIEW", with the caption
   "Simulated prices · scroll to zoom, drag to pan" below it (localised with `l10n::text`).
4. The preview never touches the host chart, history, storage or the feed. It is not saved with
   templates.
5. Hide the column when the available width is below ~720 px.

Tests:

- Changing body colour in the Symbol tab changes the preview's `config().bullish_color` without
  touching any host chart and without `SettingsAction::Apply`.
- Narrow window: no preview column, content still fits.

---

## 3. Bugs

### 3.1 Precision "Default" is lost after Ok

`apply_to_chart` (`apply.rs` ~453–463) writes `self.precision.decimals()` **or, for Default, the
instrument's precision** into the price scale. `from_chart` (~179–188) then reads that explicit value
back and maps it to e.g. "2 decimals". Reopening the dialog shows "2 decimals" instead of "Default",
and a later symbol with 5 decimals keeps 2.

Fix: keep "follow the instrument" as state. Either store `None` in the scale for Default and let the
formatter fall back to the instrument at format time, or add a flag on the chart/price scale
(`precision_follows_instrument`). `from_chart` must return `PrecisionMode::Default` when following.

Test: instrument with 2 decimals, apply Default, `from_chart` → Default; switch instrument to 5
decimals → labels show 5.

### 3.2 "Dark theme" applies immediately and Cancel does not undo it

The Canvas tab checkbox sets `pending_dark_theme`, which `show` returns as
`SettingsAction::SetDarkTheme` right away (mod.rs ~330–332; demo ~1285). Cancel keeps the new theme.
The dialog also keeps the colours captured from the old theme in `self.settings`, so Ok after toggling
may write old-theme grid/text/background colours onto the chart (verify in the demo).

Fix (Ok/Cancel model is kept):

- Store the choice in the edited state (e.g. `ChartSettingsState::dark_theme: Option<bool>` or a dialog
  field) and only emit `SetDarkTheme` together with `Apply` on Ok; Cancel emits nothing.
- When the theme choice changes, re-resolve the colours that follow the theme (background, grid, scale
  text/lines, crosshair, watermark) to the new theme's defaults unless the user picked them explicitly
  in this session, and refresh the colour pickers (`update_color_pickers_from_settings`).
- The preview (section 2) shows the chosen theme.

Tests: toggle + Cancel → no `SetDarkTheme`; toggle + Ok → `SetDarkTheme(true)` and grid/text colours
equal `Theme::dark()` defaults.

### 3.3 Settings with no control

`from_chart`/`apply_to_chart` round-trip `scales_and_lines.show_bid_ask` (apply.rs ~126, ~378) and
`show_high_low` (~127, ~379), but no tab shows them. Add to Scales and lines → PRICE LABELS & LINES,
after "Previous close":

- "Bid and ask" checkbox → `show_bid_ask`
- "High and low" checkbox → `show_high_low`

### 3.4 "Buy/sell buttons" appears twice

`status_line.show_quote_boxes` is a checkbox in Status line (`symbol.rs` ~261–266) **and** Trading
(`trading.rs` ~36–41). Keep it only in Trading → GENERAL.

### 3.5 Values that cannot be picked again

- **Timezone**: the list (mod.rs ~100–119) has 10 zones. `from_chart` can produce `"Local"` and
  `"Exchange:<zone>"` (apply.rs ~141–146), which show in the dropdown but cannot be re-selected once
  changed. Add "Local" and the current value (if not in the list) as options, and offer
  "Exchange" when the chart has a session timezone.
- **Watermark**: the dropdown (canvas.rs ~197–201) lists `ReplayMode`, `Symbol`,
  `SymbolAndDescription` but not `Custom`; when `from_chart` returns `Custom` the user cannot get back
  to it. Show `Custom` only when it is the current value (no text editor is needed). Also rename the
  variant label: `WatermarkMode::ReplayMode` is displayed as "Hidden" (`types.rs` ~131); keep the
  label "Hidden" but add a doc comment that it means no watermark, or rename the variant if no
  serialised data depends on it (check `serde` usage first).

### 3.6 Grid colour written when its direction is hidden

`apply_to_chart` sets `config.grid_color = horizontal_color` even when the mode is None or Vertical
(apply.rs ~331). Only write `grid_color` when horizontal lines are shown, and `vertical_grid_color`
when vertical lines are shown, so hidden directions keep their colour.

### 3.7 Margins as free text

Top/bottom/right margins are text fields (canvas.rs ~273–313). Invalid text is silently ignored and
only the footer says "Enter valid margins". Replace with `egui::DragValue`s:

- top and bottom: 0–99 %, step 1, suffix " %", and keep `top + bottom < 100`;
- right: 0–500 bars, integer.

Remove `margin_top_str`, `margin_bottom_str`, `margin_right_str` and their parsing in `draw_content`
(mod.rs ~391–395) and in `update_color_pickers_from_settings`.

### 3.8 Misplaced control

"Analyst price targets" is the first row of the Symbol tab (symbol.rs ~21–26). Move it to Events →
EVENT VISIBILITY (it toggles chart event marks).

### 3.9 Quick settings popup (`SettingsMenu`)

Used by the terminal app (`terminal/src/studies.rs`) and `src/widget/series.rs`.

- "Reset Scale" (menu.rs ~236–238) only closes the menu. Make it request a scale reset: add
  `scale_reset_requested` with `take_scale_reset_request()` like `take_scale_settings_request`, and
  have the hosts call the chart's price-scale reset.
- "Auto Scale" (~249–257) sets `PriceScaleMode::Normal`; that is a mode, not auto scaling. Rename the
  item to "Regular" (mode) and add a separate "Auto scale" checkbox wired like the dialog's
  `scales_and_lines.auto_scale`.
- The chart type picked in the menu (`settings.chart_type`) is never applied by `apply_to_config`.
  Either apply it in the hosts (`chart.set_chart_type`) or remove the section.
- The popup position is hard-coded `btn.min − (200, 400)` (~162) and the click-outside rect uses a
  fixed size (~331–338), so it can leave the window and close while clicking inside it. Use
  `egui::Popup`/`Area` with `constrain(true)` anchored to the button, and use the area's real rect for
  click-outside.

---

## 4. Keep

- Template menu (Save as, Update current, list, Delete, Reset to defaults) and `Control::Templates`
  hiding.
- Sound preview on Trading/Alerts (`SettingsAction::PreviewSound`) without applying settings.
- `validate()` checks and the footer error message.
- Everything already tested in `chart_settings/mod.rs` tests (`grid_colour_pickers_follow_the_visible_directions`,
  `scale_controls_show_small_ratios_and_keep_modes_exclusive`, sound preview, symbol controls by chart
  type, templates) — update only what the layout change affects.

---

## 5. Tests to add

1. Tab row: clicking "Canvas" shows Canvas content in the **same** frame; all seven tab labels are
   fully visible at 1440×900 and reachable at 320×450.
2. Same dialog rect for every tab (no height jump).
3. Preview updates from the edited state without `Apply` and is absent on narrow windows.
4. Precision Default round-trips (3.1).
5. Dark theme + Cancel emits nothing; + Ok emits `SetDarkTheme` with theme-following colours (3.2).
6. Bid/ask and High/low checkboxes change `config.show_bid_ask` / `show_high_low` after Ok (3.3).
7. "Buy/sell buttons" appears once (3.4).
8. Timezone "Local" and an `Exchange:` value can be selected again (3.5).
9. Grid mode Vertical keeps the previous horizontal `grid_color` (3.6).
10. Margin drag values reject top+bottom ≥ 100 via `validate()` (3.7).
11. Quick menu: Reset Scale sets the request flag; Auto scale and Regular are distinct (3.9).

---

## Constraints

- Follow the existing code style; comments only for constraints the code cannot show; all visible
  strings through `l10n::text`.
- No third-party product names in code, comments, tests, docs or commit messages. Before committing:
  ```bash
  git diff | grep -iE "^\+.*(trading""view|v""ela)" | grep -v WaveLabel
  ```
  must print nothing.
- Run `cargo fmt --check`, `cargo clippy --all-features --all-targets -- -D warnings`,
  `cargo clippy --no-default-features --all-targets -- -D warnings`, `cargo test --lib`, the demo tests
  (`cd examples/demo && cargo test`) and `cd terminal && cargo check --tests`.
- Rebuild the demo in release (`cd examples/demo && trunk serve --release --address 127.0.0.1 --port 8081`)
  and check by hand: tabs on top with icons, no flicker or size jump when switching, preview follows
  edits, Cancel restores everything including the theme, Precision Default survives reopening,
  bid/ask and high/low toggles work, margins use number fields, quick menu Reset Scale works.
