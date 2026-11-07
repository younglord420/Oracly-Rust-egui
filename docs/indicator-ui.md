# Indicator UI: picker categories, search field, settings dialog and legend rows

Status: partly implemented. Wide plot rows use fixed column widths (name 150, width 80,
pattern 120, type 140); narrow windows still wrap. The search clear control is the icon plus
a small padding. The picker scroll bar stays visible, and a pane legend shows the study status
instead of values. `cargo test --lib` passes (1168), both clippy runs pass, `examples/demo`
tests pass (69), and `terminal` `cargo check --tests` passes. The release demo on port 8081
rebuilt with no warnings, and the picker opens with the category sidebar. Still open: the rest
of tests 7.2–7.5, settings-tab screenshots, and the OVERLAY badges drawing into the side panel
on a narrow window.

This document covers four areas of the indicator UI:

1. **Indicator picker:** the built-in studies are reorganised into clear categories, and the picker
   lets the user browse them (section 2).
2. **Search field:** the field at the top of the picker is misaligned (section 3).
3. **Settings dialog:** the dialog of an applied study is restyled to match Chart settings, and its
   bugs are fixed (section 4).
4. **Legend rows:** the overlay rows on the price pane and the rows of indicator panes behave
   differently and have small bugs (section 5).

Section 6 lists the bugs seen in the running demo, with screenshots. Section 7 lists the tests and
section 8 the work order.

Read the whole document first. Every change needs the test listed with it. Do not change
calculations; `docs/indicator-fixes.md` covers those.

---

## 1. Files

| Area | File |
|---|---|
| Factory and categories | `src/studies/factory/mod.rs` (`register_builtin`, ~92–366) |
| Factory tests | `src/studies/factory/tests.rs` |
| Picker | `src/ui/dialogs/indicator_dialog/dialog.rs` |
| Picker tabs and categories | `src/ui/dialogs/indicator_dialog/types.rs` |
| Picker rows data | `src/ui/dialogs/indicator_dialog/data.rs` |
| Picker size and colours | `src/ui/dialogs/indicator_dialog/config.rs` |
| Search field | `src/ui_kit/filter/mod.rs` (`search_field`, ~144–215) |
| Sidebar | `src/ui_kit/sidebar_layout/mod.rs` |
| Settings dialog | `src/ui/dialogs/indicator_dialog/settings.rs` |
| Chart settings helpers to share | `src/ui/dialogs/chart_settings/tabs/mod.rs`, `chart_settings/tabs/canvas.rs` (`line_style_preview` ~391) |
| New shared helpers | `src/ui_kit/settings_rows.rs` (new), exported from `src/ui_kit/mod.rs` |
| Colour picker | `src/ui_kit/color_picker.rs` (`ColorPicker`, `ColorPickerState`) |
| Footer message | `src/ui_kit/status_message/mod.rs` (`status_message`) |
| Shading model | `src/studies/shading.rs` (`ShadingKind`, `StudyShading`) |
| Overlay legend rows | `src/chart/renderers/legend/mod.rs` (`show_with_colors` ~427–562, `study_controls` ~694–831, `Layout` ~833–957) |
| Pane legend row | `src/widget/indicator_pane.rs` (`legend_job` ~1454, `legend_controls_rect` ~1507, `draw_legend` ~1541, call site ~842–868) |
| Demo | `examples/demo/controller.rs` |

---

## 2. Indicator picker: categories

### 2.1 Current state

- `register_builtin` puts every built-in into one of 13 category strings. Several are wrong or too
  broad:
  - "Momentum" holds 50 studies: bounded oscillators, unbounded momentum, volume oscillators
    (`VolumeOscillator`, `PercentageVolumeOscillator`, `EaseOfMovement`, `ForceIndex`),
    divergences and `TechnicalRatings`.
  - "Support and resistance" holds the pattern detectors `RisingWedge`, `FallingWedge`,
    `ElliottWave` and `Reversal`.
  - "Statistics" holds `Seasonality`, `LunarPhases` and `Performance`.
  - Stops (`SuperTrend`, `ParabolicSAR`, `ChandelierExit`, …) are split between "Trend" and
    "Volatility".
- The picker never shows these categories. `IndicatorDialog::destination` (`dialog.rs` ~409) maps
  every built-in except Profiles and Patterns to `(BuiltIns, Indicators)`, so the "Built-ins" page
  is one alphabetical list of about 190 rows. The category string is only used for search.
- Only the picker reads the category (`data.rs` ~76, `dialog.rs` ~171, ~411, ~468). Saved layouts
  store the study **name**, not its category, so renaming categories is safe. Do not rename
  studies.

### 2.2 New categories

Add one `pub const` per category on `IndicatorFactory` and a list in display order:

```rust
pub const MOVING_AVERAGES: &'static str = "Moving averages";
pub const BANDS: &'static str = "Bands and channels";
pub const TREND: &'static str = "Trend";
pub const STOPS: &'static str = "Stops and trailing";
pub const MOMENTUM: &'static str = "Momentum";
pub const OSCILLATORS: &'static str = "Oscillators";
pub const VOLATILITY: &'static str = "Volatility";
pub const VOLUME: &'static str = "Volume";
pub const AVERAGE_PRICE: &'static str = "VWAP and average price";
pub const LEVELS: &'static str = "Pivots and levels";
pub const STATISTICS: &'static str = "Statistics and regression";
pub const PRICE: &'static str = "Price and sessions";
pub const BREADTH: &'static str = "Market breadth";
pub const ORDER_FLOW: &'static str = "Order flow";

/// The built-in categories of the Indicators page, in the order the picker lists them.
pub const INDICATOR_CATEGORIES: [&'static str; 14] = [ /* the 14 above, in this order */ ];
```

`PROFILES`, `PATTERNS`, `PERSONAL`, `COMMUNITY` and `CUSTOM` stay as they are. Use the constants
in `register_builtins!` instead of string literals (the macro takes `$category:literal`; change it
to `$category:expr`).

Assign every type exactly as below. The type names are the ones in the `register_builtins!`
list; the loop-registered families are named at the end.

| Category | Types |
|---|---|
| Moving averages | `SMA`, `EMA`, `WMA`, `DEMA`, `TEMA`, `HMA`, `ALMA`, `VWMA`, `LSMA`, `SWMA`, `KAMA`, `SmoothedMA`, `GuppyMA`, `McGinleyDynamic`, `VIDYA`, `ZLEMA`, `HammingMA`, `MultipleMA`, `MARibbon`, `EMACross`, `MACross`, `MixedMACross`, `TripleMA` |
| Bands and channels | `BollingerBands`, `KeltnerChannels`, `DonchianChannels`, `Envelopes`, `MAChannel`, `AccelerationBands`, `StandardErrorBands`, `LinearRegressionChannel`, `Median` |
| Trend | `ADX`, `DirectionalMovement`, `AdxBreakout`, `Aroon`, `AroonOscillator`, `VortexIndicator`, `IchimokuCloud`, `WilliamsAlligator`, `SqueezeMomentum`, `TrendIntensityIndex`, `TrendDetectionIndex`, `Qstick`, `ChoppinessIndex`, `ChopZone`, `ElderImpulseSystem` |
| Stops and trailing | `SuperTrend`, `ParabolicSAR`, `ChandelierExit`, `ChandeKrollStop`, `VolatilityStop` |
| Momentum | `MACD`, `VolumeWeightedMACD`, `Momentum`, `RateOfChange`, `AwesomeOscillator`, `AcceleratorOscillator`, `TRIX`, `CoppockCurve`, `KnowSureThing`, `PriceOscillator`, `PriceMomentumOscillator`, `PringsSpecialK`, `ElderRay`, `BullBearPower`, `DetrendedPriceOscillator`, `BalanceOfPower`, `TechnicalRatings` |
| Oscillators | `RSI`, `Stochastic`, `StochasticRSI`, `WilliamsR`, `CCI`, `WoodiesCCI`, `ConnorsRSI`, `KDJ`, `DoubleStochastic`, `StochasticMomentumIndex`, `SMIErgodicIndicator`, `SMIErgodicOscillator`, `TrueStrengthIndex`, `UltimateOscillator`, `FisherTransform`, `RelativeVigorIndex`, `ChandeMomentumOscillator`, `RainbowOscillator`, `UltimateMomentum`, `SchaffTrendCycle`, `CenterOfGravity`, `PrettyGoodOscillator`, `RsiDivergence`, `KnoxvilleDivergence` |
| Volatility | `ATR`, `TrueRange`, `StandardDeviation`, `HistoricalVolatility`, `ChaikinVolatility`, `BollingerBandsWidth`, `PercentB`, `BBTrend`, `OhlcVolatility`, `ParkinsonVol`, `ZeroTrendVol`, `AverageDailyRange`, `UlcerIndex`, `RelativeVolatilityIndex`, `CommoditySelectionIndex`, `MassIndex` |
| Volume | `Volume`, `UpDownVolume`, `NetVolume`, `Volume24h`, `RelativeVolumeAtTime`, `VolumeRateOfChange`, `VolumeOscillator`, `PercentageVolumeOscillator`, `OnBalanceVolume`, `AccumulationDistribution`, `WilliamsAD`, `ChaikinMoneyFlow`, `ChaikinOscillator`, `MoneyFlowIndex`, `KlingerOscillator`, `ForceIndex`, `EaseOfMovement`, `VolumePriceTrend`, `PriceVolumeTrendStudy`, `NegativeVolumeIndex`, `PositiveVolumeIndex`, `MarketFacilitationIndex` |
| VWAP and average price | `VolumeWeightedAvgPrice`, `AnchoredVWAP`, `VwapAutoAnchored`, `TimeWeightedAvgPrice`, `VisibleAveragePrice` |
| Pivots and levels | `PivotPoints`, `PivotPointsHighLow`, `IntradayPivotPoints`, `ProjectedPivotPoints`, `GhostPivots`, `MissedPivots`, `KeyLevels`, `WilliamsFractals`, `ZigZag`, `Week52HighLowLevels`, `Gaps`, and the `AutoDrawing` loop (`#[cfg(feature = "drawings")]`, ~355–365) |
| Statistics and regression | `LinearRegression`, `LinearRegressionSlope`, `StandardError`, `TrendStrengthIndex`, `RankCorrelationIndex`, `RciRibbon`, `Correlation`, `Ratio`, `Spread`, `MajorityRule`, `AccumulationSwingIndex` |
| Price and sessions | `HigherIntervalCandles`, `BollingerBars`, `TypicalPrice`, `MedianPrice`, `WeightedClose`, `OHLC4`, `TradingSessions`, `Week52HighLow`, `Performance`, `Seasonality`, `LunarPhases` |
| Market breadth | `AdvanceDeclineRatio`, `AdvanceDeclineLine`, `CumulativeVolumeIndex`, `AdvanceDeclineRatioBars` |
| Order flow | the `OrderFlowStudy` loop (~305–344), unchanged |
| `PROFILES` (Profiles page) | `VolProfileVisible`, `VolProfileFixed`, `VolProfileSession`, `VolProfilePeriodic`, `VolProfileAnchored`, `VolProfileAutoAnchored`, `TimePriceOpportunity` (unchanged) |
| `PATTERNS` (Patterns page) | `CandlestickPatterns`, the `ChartPatterns` loop, **and** `RisingWedge`, `FallingWedge`, `ElliottWave`, `Reversal` (moved from "Support and resistance") |

Rules:

- Every registered name ends up in exactly one category. The test in 7.1 enforces it.
- If a type in `register_builtins!` is missing from this table, stop and report it. Do not guess.
- The category names are English keys. Every place that shows one passes it through `l10n::text`.

### 2.3 Picker layout

Keep the public types `IndicatorTab` and `IndicatorCategory` and the public methods of
`IndicatorDialog` (`open`, `open_tab`, `open_for_chart`, `with_factory`, …): hosts and the demo
use them.

Add a private field `sel_group: Option<&'static str>` to `IndicatorDialog`: `None` means "All
indicators". It only applies while `sel_category == BuiltIns` and `active_tab == Indicators`.

**Wide layout (dialog content ≥ 600 px):** the sidebar replaces the pill tabs.

```
 PERSONAL
   ☆ Favorites
   </> My scripts            (only when the host registers PERSONAL studies)
 BUILT-IN
   ▦ All indicators        190
   ∿ Moving averages        23
   ▭ Bands and channels      9
   ↗ Trend                  15
   ⤓ Stops and trailing      5
   ⚡ Momentum               17
   〰 Oscillators            24
   ≋ Volatility             16
   ▮ Volume                 22
   ≈ VWAP and average price  5
   ═ Pivots and levels      12+
   σ Statistics and regr.   11
   ¤ Price and sessions     11
   ⚖ Market breadth          4
   ⇅ Order flow              9
   ▤ Profiles                7
   ◇ Patterns               n
   … Host categories (CUSTOM, "Metrics") after these, only when present
 COMMUNITY
   👥 Community             (only when present)
```

- **Sidebar items:**
  - Selecting "All indicators" sets `(BuiltIns, Indicators, None)`.
  - Selecting a category sets `(BuiltIns, Indicators, Some(name))`.
  - Selecting "Profiles" or "Patterns" sets `(BuiltIns, Profiles | Patterns, None)`.
  - `open_tab(tab)` selects the matching sidebar item.
- **Counts:** right-aligned, in `theme.ui.text_secondary` at `theme.text.small`. Count the studies
  that `accepts_source` lets through, so the count matches what the list shows.
- **Hidden items:** an item with no study (count 0) is not shown.
- **Scrolling:** the sidebar scrolls vertically (`ScrollArea::vertical().auto_shrink([false, false])`)
  because 17+ rows do not fit at 640 px.
- **Icons:** use only icons that already exist in `crate::icons::icons`. Pick one per category and
  list your choice in the PR. If no fitting icon exists, use `INDICATORS`. Do not add new SVGs.
- **`SidebarTab` count:** it has no count today. Add `SidebarTab::count(usize)`, drawn
  right-aligned inside the tab.
- **Pill tabs:** remove the `TabBar` of `IndicatorTab` from the wide layout (`draw_tabs`).

**Narrow layout (< 600 px):** one `ComboBox` labelled "Category" with the same entries in the same
order, including "All indicators", every category, Profiles and Patterns. Remove the pill tabs
there too.

**List:**

- **"All indicators" without a search:** rows grouped by category in `INDICATOR_CATEGORIES`
  order, then host categories. Within a group, sort by display name, favourites first.
  - Each group starts with a header row: the category name in
    `RichText::new(..).size(theme.text.small).strong().color(theme.ui.text_secondary)`, upper
    case.
  - Header rows are 28 px high, not focusable, and not a `ListBoxOption`.
- **A selected category:** only its rows, no header.
- **A search:** covers every built-in category and both pages, as today. Results are grouped
  with the same header rows.
  - Each header also names the page when it is Profiles or Patterns.
  - Arrow-down from the search field still focuses the first **study** row (skip headers).
- **Favorites, My scripts, Community:** unchanged, no headers.
- **Empty state:** unchanged texts, plus a category with no match for the search: "No indicators
  in {category} match \"{query}\"" via `l10n::format_text`.

**Row:** keep the current row (star, name, badges, focus ring). Fix these:

- **Category label:** while a search is active, show the category as muted text right of the name
  and left of the badges, so the user sees where a result belongs. Use `theme.text.small`,
  `config.muted_color`, truncated with `…`.
- **Name centring:** the name is centred with `rect.center().y - name.size().y / 2.0`, but the star
  icon uses `star_rect.center()`. Both must use the row's centre. Check the screenshots in section 6.
- **Badges:** they must not overlap a long name. `draw_item_text` already truncates at `right`;
  keep that, and check it at 600 px.

---

## 3. Search field

### 3.1 Cause

`IndicatorDialog::draw_content` (`dialog.rs` ~325–331) shows the field inside a scope that sets
`interact_size.y = 40` and `override_font_id = theme.text.heading`. `search_field`
(`ui_kit/filter/mod.rs` ~144–215) builds a single-line `TextEdit` with
`min_size(0, interact_size.y)` and `Margin { left, right, ..Margin::symmetric(0, 2) }`.

A single-line `TextEdit` lays its text out from the top inner margin. The text therefore sits 2 px
below the top of a 40 px field, while the magnifier and the clear button are drawn at
`response.rect.center().y`. The text, the placeholder and the icons are not on one line.

- **Margins are `i8`:** `margin()` casts `inset + icon + spacing` to `i8`. Large themes overflow
  silently.
- **Clear button:** it sits in `rect.expand(4.0)` with `ButtonSize::SM`. The hit area can be larger
  than the right margin and overlap the last typed characters.
- **Field height:** the field takes the dialog's heading font, but the filter in other panels
  (watchlist, object tree) uses the body font. The fields have different heights in different
  places.

### 3.2 Fix

In `search_field`:

1. **Centre the text vertically.** Measure the text row height with the font the field uses:
   `ui.painter().layout_no_wrap("Ag".into(), font_id.clone(), Color32::WHITE).size().y`. The font
   is `ui.style().override_font_id` if set, otherwise the `TextStyle::Body` font. Then compute
   `let pad_y = ((ui.spacing().interact_size.y - row) / 2.0).max(2.0)`. Use `pad_y` as the top and
   bottom margin (`Margin { top: pad_y, bottom: pad_y, .. }`) and drop the `min_size` height. The
   field height is then `row + 2 * pad_y`, and the text is centred like the icons. Use
   `egui::Margin` in `i8` only after clamping with `.round().clamp(0.0, 127.0) as i8`.
2. **Icons:** draw both on the text row's centre, which is `response.rect.center().y` once
   step 1 holds.
3. **Clear button:**
   - Size it to `icon_size + 2 * spacing.xs`, centred at `right - inset - icon_size / 2`.
   - Reserve the right margin as `inset + icon_size + spacing.sm` whenever `show_clear` is true,
     even while the query is empty, so text does not shift when the button appears.
4. **Frame:** keep `expansion = 0` for hovered and active. Draw the focus outline with the same
   radius as the field (`theme.radius.md`), so the focused field does not grow or jump.
5. **Placeholder colour:** use `theme.ui.text_muted`.

In `IndicatorDialog::draw_content`, keep the 40 px height and the heading font. Step 1 makes them
line up.

Check every other caller of `FilterWidget` and `filter_input` (`rg "FilterWidget|filter_input"
src`). They must look unchanged or better, and their tests must pass.

---

## 4. Settings dialog

### 4.1 Current state and bugs (`settings.rs`)

1. **Title:** `self.title = format!("{} settings", study.name())` (~78) is not translated. Use
   `l10n::format_text(ctx, "{name} settings", &[("name", name)])` when drawing, not when opening.
2. **Inputs tab:**
   - It is a bare `FormGrid` with no section header.
   - A small "Reset inputs" button sits above it (~171–179).
   - Colour inputs use egui's native `color_edit_button_srgba` (~278–280).
   - Dropdowns are plain `ComboBox`es, not the Chart settings style.
3. **Style tab:**
   - Each plot is two `horizontal_wrapped` rows (~352–446), so the controls wrap differently per
     study and per width.
   - The width `DragValue` has no `speed` (~371–374).
   - The pattern names come from `line_style_name` (~618–626), not the previews Chart settings
     shows.
4. **SHADING** (`shading_rows`, ~573–616):
   - The header is a bold label instead of a section header.
   - The colour rows use the native colour button.
   - The rows are named only for `Bounded` ("Oversold"/"Overbought") and "Bullish"/"Bearish"
     otherwise. They are wrong for `Intensity` (only `accent` matters), `FlatChannel` (only `band`)
     and `MomentumColumns` (four colours, two missing).
   - The level `DragValue`s exist only for `Bounded`, and their prefixes "Overbought " and
     "Oversold " are not translated.
   - "Gradient fill" shows even for kinds that have no wash.
   - The "Zones" transparency (`zone_t`) is not editable.
5. **Validation:** the error (~531–540) is a plain `ui.label` above the footer. It should use
   `status_message(ui, MessageKind::Error, ..)` inside the footer.
6. **Defaults button** (~545–553): resets plots, shading, precision and visibility but **not the
   inputs**.
7. **Visibility tab:**
   - Checkboxes outside the selected range are disabled but have no hover text.
   - The section has no header.
8. **Theme:** `show(ctx, _theme)` ignores its theme. Every colour and text size must come from it.

### 4.2 Shared helpers

Move these from `src/ui/dialogs/chart_settings/tabs/mod.rs` and `tabs/canvas.rs` to a new
`src/ui_kit/settings_rows.rs`, `pub` and exported from `ui_kit/mod.rs`:

| Helper | New signature |
|---|---|
| `draw_section_header` | `pub fn section_header(ui: &mut Ui, title: &str, theme: &Theme)` |
| `draw_setting_row_with_controls` | `pub fn setting_row(ui: &mut Ui, label: &str, label_width: f32, theme: &Theme, add: impl FnOnce(&mut Ui))` |
| `styled_dropdown` | `pub fn styled_dropdown<R>(ui, id, width, text_color, selected, add)` (same body) |
| `styled_checkbox` | `pub fn styled_checkbox(ui, checked, label, text_color) -> bool` |
| `line_style_preview` | `pub fn line_style_preview(style: LineStyle) -> &'static str` |
| new | `pub fn label_width(ui: &Ui, theme: &Theme, labels: impl IntoIterator<Item = impl AsRef<str>>, max: f32) -> f32`: the widest translated label in `theme.text.title`, capped at `max` |

`SettingsDialog::draw_setting_row_with_controls` passes `&mut SettingsDialog` to its closure. Keep
a thin private wrapper there that calls `setting_row`, so the Chart settings tabs do not change.
**Chart settings must look exactly as before**, and its tests (`rg "mod tests"
src/ui/dialogs/chart_settings`) must pass unchanged.

### 4.3 Layout

- **Modal:** width 620, height 560 for every tab (as now), `top(120.0)`, clamped to the window.
  `dialog_follows_the_window_when_it_shrinks` must keep passing.
- **Tabs:** `TabBar::new(&[Inputs, Style, Visibility], &mut tab).pills()`. The icons already exist
  in `TabLabel::tab_icon`.
- **Scrolling:** content in `ScrollArea::vertical().auto_shrink([false, false])` with
  `max_height(available - footer)`.
- **Label column:** `label_width` of the labels of the current tab, capped at 40% of the content
  width.

**Inputs tab**

```
INPUTS
Length              [ 14          ]
Source              [ Close      ⌄]
Smoothing           [ EMA        ⌄]
Show signal         [x]
Line colour         [■]
CALCULATION                          ← only when the study has these inputs
Symbol              [ Chart symbol ]
Timeframe           [ Chart interval ⌄] [ 60 s ]
Wait for close      [x]
```

- **Rows:** `section_header("INPUTS")`, then one `setting_row` per input. The inputs
  `calculation_symbol`, `calculation_timeframe` and `wait_for_close` go under
  `section_header("CALCULATION")`.
- **Controls per input kind:**

  | Kind | Control |
  |---|---|
  | `Length`, `Integer` | `DragValue` with the range, width 120 |
  | `Number` | `DragValue` with `speed(0.1)` and the range, width 120 |
  | `Choice`, `Source`, `Timeframe` | `styled_dropdown`, width 200. Keep the current entries, the Custom timeframe entry and its seconds field |
  | `Toggle` | `styled_checkbox` with an empty label |
  | `Color` | `ColorPicker` with one `ColorPickerState` per colour input, kept in `input_pickers: HashMap<String, ColorPickerState>` and keyed by input id. Sync `sel_color` from the value before drawing and back after, as the plot pickers do |
  | `Symbol`, `Text` | `TextEdit::singleline`, width 200, hint through `l10n::text` |
  | other | "This input is unavailable." (as now) |

- **Existing rules to keep:**
  - A study-source input hides `calculation_symbol`, `calculation_timeframe` and `wait_for_close`
    and resets them to default.
  - The empty-inputs message stays.
- **Remove** the "Reset inputs" button; Defaults (4.5) does it.

**Style tab**

```
PLOTS
[x] RSI              [■]  [ 2.0 px ]  [ ──── Solid  ⌄]  [ Line      ⌄]
[x] Upper            [■]  [ 1.0 px ]  [ - - Dashed  ⌄]  [ Line      ⌄]
SHADING                                 ← only when settings.shading is Some
Gradient fill        [x]                ← only for kinds with a wash (table 4.4)
Overbought           [■]   level [ 70.0 ]
Oversold             [■]   level [ 30.0 ]
Band fill            [■]
Levels               [■]
Zones                [ 90 ] %           ← Bounded only, zone_t 0–100
PRECISION
Decimals             [x] Automatic  [ 2 ]
```

- **PLOTS:** one row per plot in an `egui::Grid` with five columns, so the columns line up across
  plots:
  1. A checkbox with the plot name. The name is truncated with `…` at the label width and has the
     full name as hover text.
  2. The existing plot `ColorPicker` with `well_size(18.0)`.
  3. Width `DragValue`: `speed(0.1)`, `range(0.5..=10.0)`, suffix " px", width 72.
  4. Pattern `styled_dropdown`, labelled with `line_style_preview`, listing `LineStyle::all()`.
  5. Plot type `styled_dropdown`. Keep the incompatibility rules: the disabled entries with their
     reason as hover text, and the saved choice stays listed.

  At content width < 520 px, put columns 3–5 on a second row under the name, aligned with column 2.
- **SHADING:** see 4.4.
- **PRECISION:** a `setting_row` "Decimals" with the Automatic checkbox and the `DragValue`
  `range(0..=12)`, suffix " decimals".

**Visibility tab**

```
INTERVALS
Range               [ Current and above ⌄]
Show this study on these intervals:
[x] 1m  [x] 3m  [x] 5m  [ ] 15m  …
```

- **Header and range:** `section_header("INTERVALS")`, then a `setting_row` "Range" with the
  existing four choices in a `styled_dropdown`.
- **Checkboxes:** in a wrapped `Grid` of fixed 72 px cells, so they line up.
- **Disabled checkboxes:** add
  `.on_disabled_hover_text(l10n::text(ctx, "Outside the selected range"))`.

**Footer**

```
[Defaults]   ⚠ Overbought must be above oversold          [Cancel] [Apply]
```

- `DialogFooter::new("Apply").secondary("Cancel").primary_enabled(valid).show_with_left(..)`.
- The left side has the Defaults button, then the validation message, if any, drawn with
  `status_message(ui, MessageKind::Error, &self.error)`.
- Remove the separate `ui.label(&self.error)`.

### 4.4 SHADING rows per kind

Draw `section_header("SHADING")` and then, for `edit.settings.shading.kind`:

| Kind | Gradient fill row | Colour rows (field) | Level rows |
|---|---|---|---|
| `Bounded` | yes if `midpoint_wash` | Overbought (`bear`), Oversold (`bull`), Band fill (`band`, only if `band_fill`), Levels (`levels`) | Overbought, Oversold `DragValue` (`speed 0.1`); Zones `zone_t` 0–100 |
| `BandOnly` | no | Band fill (`band`), Levels (`levels`) | Upper, Lower |
| `Centered` | yes | Bullish (`bull`), Bearish (`bear`), Levels (`levels`) | Baseline; each `extra_levels` entry as "Level {n}" |
| `Regime`, `Spread` | yes | Bullish (`bull`), Bearish (`bear`) | — (`Spread`: Fill `fill_t` 0–100) |
| `Channel` | yes | Bullish (`bull`), Bearish (`bear`) | — |
| `FlatChannel` | no | Fill (`band`) | Fill `fill_t` 0–100 |
| `MomentumColumns` | no | Grow above (`bull`), Fall above (`faded_bull`), Grow below (`bear`), Fall below (`faded_bear`) | — |
| `RisingColumns` | no | Rising (`bull`), Falling (`bear`) | — |
| `Intensity` | yes | Accent (`accent`) | `Threshold(x)`: Threshold; `Rolling(n)`: Lookback |
| `PriceSide`, `Slope` | yes | Bullish (`bull`), Bearish (`bear`) | — |
| `AdxInk` | no | +DI (`bull`), −DI (`bear`) | Key level |

- **Study-specific names:** the gradient document (section 5.4) gives these.
  - Relative Volatility Index: "Overbought" is `bull`.
  - Choppiness: "Choppy" is `bear` and "Trending" is `bull`.
  - Chaikin Volatility: only "Expansion" (`bull`).

  Read these from a new trait method `fn shading_labels(&self) -> Option<[&'static str; 2]>` on
  `Indicator`, defaulting to `None`. `ConfiguredStudy` forwards it. Store the result in the dialog
  on `open`.
- **Colour pickers:** every colour row is a `setting_row` with a `ColorPicker`. Keep one
  `ColorPickerState` per field in `shade_pickers: [ColorPickerState; 8]`, indexed by field
  (`bull`, `bear`, `faded_bull`, `faded_bear`, `accent`, `band`, `levels`, and one spare). Sync
  them the same way as the plot pickers.
- **Translation:** all labels and suffixes through `l10n::text`.
- **New variant:** if `docs/indicator-fixes.md` 4.1 has added a `SpreadColumns` kind by then, give
  it the rows of `MomentumColumns`.

### 4.5 Defaults

Clicking Defaults stages, without applying:

- `self.inputs[i].value = self.inputs[i].default.clone()` for every input
- `edit.settings = defaults` (plots, precision, shading, `hidden_intervals = []`,
  `interval_range = All`)
- every picker state's `sel_color` set from the new values

Apply sends it. Cancel or Escape discards it. `escape_discards_staged_inputs_and_presentation`
must keep passing.

---

## 5. Legend rows

Two code paths draw a study's legend row:

- **Overlays** on the price pane: `SeriesLegend::show_with_colors` in `legend/mod.rs`, ~427–562.
- **Indicator panes:** `IndicatorPane::draw_legend` and `legend_job` in `indicator_pane.rs`,
  ~1454–1573.

They disagree on several points. Make both follow the rules below, and put the shared parts in one
function in `legend/mod.rs` (for example `study_value_texts(study, bar, precision, theme) ->
Vec<(String, Color32)>`), used by both.

| Rule | Overlay today | Pane today | Required |
|---|---|---|---|
| Value colour | `study.colors()[i].to_opaque()` | `plot_style(i).color`, swatch when contrast < 4.5 | `plot_style(i).color` made opaque. When contrast with `theme.chart.bg` < 4.5, a swatch plus text in `theme.ui.text`, as the pane does now. The user's Style colour must show in both |
| Missing value | `NO_VALUE` text | value skipped | `NO_VALUE` in `theme.ui.text_muted`, so the columns do not shift while the crosshair moves |
| Inputs text colour | `theme.ui.text_muted`; Volume uses `Color32::from_gray(112)` (~466) | `text_color` (full) | `theme.ui.text_muted` everywhere. Remove `from_gray(112)` |
| Hidden study (eye off) | title at `gamma_multiply(0.5)`, no values | full colour, values shown | title and inputs at `gamma_multiply(0.5)`, no values, in both |
| Study status (`study_status()`) | replaces values with the status text | only the centred pane message, not in the row | the status text replaces the values in both, in `theme.ui.text_secondary` |
| Price formatting | price studies use the chart precision | always `format_value` | both: `value_format() == Price` uses the chart precision, others `format_value` |
| Font | `theme.text.ohlc` | `theme.text.ohlc` | keep |

Overlay row bugs:

1. **Reserved controls width:** `layout.reserve_space(Vec2::new(80.0, 20.0))` (~533) reserves the
   controls even while they are hidden. When the reservation does not fit, it starts a new row, so
   an overlay with long values gets an empty second line.
   - Reserve the controls only while the row is revealed (hovered, focused or touch-selected).
   - Otherwise reserve 0.
   - Keep the hit rect for hover detection: the row rect from `row_start` to the end of the
     values.
2. **Controls backdrop:** the controls are drawn over the chart without the row backdrop. Paint
   `theme.chart.bg` under the controls when revealed. `study_controls` does it; check it in the
   screenshots in section 6.
3. **Collapse button position:** the collapse chevron (`layout.reserve(29×21)`, ~565) follows the
   last row. It must sit on the line of the last overlay row, not on a new line by itself. If it
   does not fit there, it goes on its own line, left-aligned with the rows.

Pane row bug:

1. `legend_controls_rect` places the controls at
   `min(text_end, right - 80 or 120)` (~1523–1525). A long legend reaches past that point, so the
   controls overlap the text. Clip the legend text at `controls.left() - spacing.sm` with `…`.
   `draw_legend` gets a `max_width` and sets `job.wrap.max_width`, `max_rows = 1` and
   `overflow_character = Some('…')`.

Keep `study_controls_reveal_on_touch_and_dismiss_outside`,
`legend_values_identify_each_plot_without_low_contrast_text` and
`status_visibility_reaches_the_chart_legend` passing. Update their expectations only where this
section changes the behaviour on purpose, and say so in the PR.

---

## 6. Findings in the running demo

Taken from the release demo at `http://127.0.0.1:8081` **before** this working tree (the build
still had the old picker and the top-aligned search field). The browser saved the shots under
`C:\Users\YOUNGL~1\AppData\Local\Temp\cursor\screenshots\tmp\ui-audit\`, not `/tmp/ui-audit/`.
Captured: `01-chart.png`, `02-picker.png`, `03-search.png`, `04-search-typed.png`,
`05-chart-with-studies.png`, `06-legend-hover.png`. Settings tabs and the 800 px window were not
captured. Repeat them after the release rebuild.

1. **Search text sits high** (`03-search.png`, `04-search-typed.png`). Field centre is about y=59
   in the 3× crop; the magnifier is there; the placeholder and the typed "rsi" centre near y=45.
   Section 3. The clear button is centred while the text is not (`04`).
2. **Tabs vanish while searching** (`02` vs `04`). The Indicators / Profiles / Patterns pills
   disappear and the list jumps up. The new sidebar (2.3) replaces those pills; a search must not
   move the sidebar.
3. **Last picker row is clipped** (`02`). "Arnaud Legoux Moving Avg" is cut by the dialog bottom.
   The list `ScrollArea` must include bottom padding so the last row can scroll fully into view.
4. **OVERLAY badge drifts** between the full list and a filtered list (about 11 CSS px). Keep the
   badge's right edge on `rect.right() - theme.spacing.lg` in both cases (2.3 row).
5. **Legend text has no readable backdrop** (`05`, `06`). The Bollinger band crosses "BB" and
   "Vol"; Stochastic peaks rise into the values. `study_backdrop` is 0.6 of the chart background,
   which is not enough where a line crosses the text. Raise the study-row backdrop toward opaque
   (`theme.chart.bg` at least 0.92) in both the overlay rows and the pane legend (section 5).
6. **Hover controls are see-through** (`06`). The eye, gear, trash and more icons sit after the
   values, but the band shows through the box. The controls rect must fill with `theme.chart.bg`
   before the icons (section 5).
7. **Raw input text** (`05`, `06`). Bollinger shows "20 Simple 2". RSI Divergence shows
   "14 5 5 60 5 true false true false". Format booleans as "on"/"off" and keep each input next to
   its label, or show only the numeric inputs. This is in addition to section 5.
8. **Swatches differ by pane** (`05`). RSI glues a swatch to the value ("RSI 14▮66.22") while MACD,
   Stochastic, BB and EMA only colour the number. Section 5's contrast rule is the one rule for
   both rows: a swatch only when the plot colour fails contrast, with a space before the value.
9. **"Bear" is clipped** (`05`, `06`) at the top of the RSI Divergence pane and collides with the
   legend. Keep marker labels inside the pane, below the legend row. Not covered elsewhere.
10. **Price-scale tags overlap** (`05`). The BB middle tag is drawn above the EMA tag. Out of
    scope for this document unless a one-line order fix is obvious; otherwise list it in the report.

Not bugs: the dialog stays undimmed on purpose (`dim_background(false)`), and adding a study leaves
the picker open. "/" opens symbol search; the Indicators toolbar button opens this dialog.

---

## 7. Tests

### 7.1 Factory categories (`src/studies/factory/tests.rs`)

1. `every_study_has_a_listed_category`: for every name in `IndicatorFactory::new().list()` with
   all features, `category(name)` is one of `INDICATOR_CATEGORIES`, `PROFILES` or `PATTERNS`.
2. `categories_match_the_table`: a hand-written table of at least two names per category from 2.2,
   asserting `category(name) == Some(CONST)`. Include `RisingWedge`, `ElliottWave`, `SuperTrend`,
   `VolumeOscillator`, `Seasonality` and `VisibleAveragePrice`, the moved ones.
3. No `_ =>` arm in either test.

### 7.2 Picker (`dialog.rs` tests)

1. **`all_indicators_groups_by_category_in_order`:** with no search, the drawn
   `ListBoxOption` rows (egui_kittest `query_all_by_role`) come in `INDICATOR_CATEGORIES` order,
   and a header label precedes each group.
2. **`selecting_a_category_lists_only_it`:** for every category, every listed row has that
   category and the count equals the rows.
3. **`profiles_and_patterns_are_sidebar_items`:**
   - `open_tab(Profiles)` selects the Profiles sidebar item and lists only `PROFILES`.
   - The same for Patterns.
4. **`search_covers_all_categories_and_skips_headers`:** query "vol" lists rows from Volume,
   Volatility and Profiles. Arrow-down from the field focuses the first study row.
5. **Existing test:** update `each_tab_and_category_lists_only_its_own_indicators` for the new
   `sel_group` (add cases), without removing a case.
6. **`narrow_picker_uses_a_category_combo`:** at 480 px wide there is a "Category" combo and no
   sidebar.

### 7.3 Search field (`ui_kit/filter/mod.rs` tests)

1. **`text_and_icons_share_a_centre_line`:** with `interact_size.y` 40 and the heading font, the
   text galley's centre is within 1 px of the field rect's centre.
   - The magnifier's centre is also within 1 px of it.
   - Read the shapes from `ctx.run_ui` output.
2. **`clear_button_does_not_cover_text`:** with a long query, the clear button rect does not
   intersect the text clip rect.
3. **Existing test:** `escape_clears_only_the_focused_filter` passes.

### 7.4 Settings dialog (`settings.rs` tests)

1. **`defaults_reset_inputs_and_presentation`:** change an input, a plot colour, precision and a
   hidden interval, click Defaults through egui_kittest, Apply. The `StudyEdit` has the default
   inputs and settings.
2. **`shading_rows_follow_the_kind`:** for RSI (`Bounded`), MACD (`MomentumColumns`), Bollinger
   (`FlatChannel`), Historical Volatility (`Intensity`) and ADX (`AdxInk`), the Style tab shows
   exactly the labels of table 4.4. Query them by label.
3. **`colour_inputs_use_the_kit_picker`:** a study with a `Color` input shows a `ColorPicker` well
   and no native colour button.
4. **`intervals_outside_the_range_are_disabled_with_a_reason`:** with `Only(1h)` on 1m, the 15m
   checkbox is disabled and its disabled hover text is "Outside the selected range".
5. **`invalid_shading_disables_apply`:** overbought below oversold, Apply disabled, the error text
   shown in the footer.
6. **Existing tests:** `dialog_follows_the_window_when_it_shrinks` and
   `escape_discards_staged_inputs_and_presentation` pass.

### 7.5 Legend (`legend/mod.rs` and `indicator_pane.rs` tests)

1. **`overlay_and_pane_rows_use_the_style_colour`:** set plot 0's colour through
   `apply_settings`. The overlay row (EMA) and the pane row (RSI) paint the value in that colour.
2. **`missing_values_keep_their_slot`:** with the crosshair on a warm-up bar, both rows show
   `NO_VALUE` for each visible plot.
3. **`hidden_study_dims_in_both_rows`:** eye off, title colour alpha is half, no value texts.
4. **`overlay_row_does_not_wrap_for_hidden_controls`:** a long overlay row fits on one line
   without hover, and gets the controls on hover.
5. **`pane_legend_is_clipped_before_its_controls`:** a long pane legend's text rect ends before the
   controls rect.

---

## 8. Work order

1. **Categories (2.2)** and test 7.1. Run `cargo test --lib`.
2. **Search field (3)** and tests 7.3.
3. **Picker (2.3)** and tests 7.2.
4. **Shared helpers (4.2)**, with Chart settings unchanged. Run its tests.
5. **Settings dialog (4.1, 4.3–4.5)** and tests 7.4.
6. **Legend rows (5)** and tests 7.5.
7. **Section 6 findings** that are not covered yet.
8. **Full checks:**
   - `cargo fmt --check`
   - `cargo clippy --all-features --all-targets -- -D warnings`
   - `cargo clippy --no-default-features --all-targets -- -D warnings`
   - `cargo test --lib`
   - `cd examples/demo && cargo test`
   - `cd terminal && cargo check --tests` (restore `terminal/Cargo.lock` with `git checkout` if
     only that command changed it)

   Do not pipe cargo into `grep` or `head`.
9. **Release demo:**
   - Wait until `trunk serve --release --address 127.0.0.1 --port 8081` prints `success` with no
     warnings.
   - Repeat every screenshot of section 6, light and dark theme, at 1440×900 and 800×700.
   - Attach them to the PR next to the before shots.

## Constraints

- **Code style:** follow the existing style. Write comments only for constraints the code cannot
  show.
- **Visible strings:** every one goes through `l10n::text` or `l10n::format_text`. That includes
  category names, section headers, row labels, suffixes and hover texts.
- **Theme:** colours and text sizes come from the theme (`theme.ui.*`, `theme.text.*`,
  `theme.spacing.*`, `theme.radius.*`). No new hard-coded `Color32` in UI code.
- **Saved layouts:** do not rename studies or input ids. Saved layouts and templates refer to them.
- **Public API:** keep the public items of `indicator_dialog` (`IndicatorDialog`, `IndicatorTab`,
  `IndicatorCategory`, `IndicatorInfo`, `IndicatorSettingsDialog`, `StudyEdit`). Adding items is
  fine.
- **Tests:** do not weaken, skip or delete existing tests to make them pass. Update an expectation
  only where this document changes the behaviour, and name it in the PR.
- **Product names:** no third-party product names in code, comments, tests, docs or commit
  messages. Before committing, this must print nothing:
  ```bash
  git diff | grep -iE "^\+.*(trading""view|v""ela)" | grep -v WaveLabel
  ```
- **Commits:** do not commit or push unless asked. Commit messages have no `Co-authored-by`
  trailer.
