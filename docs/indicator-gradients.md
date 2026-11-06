# Indicator gradients, indicator settings dialog and legend controls

Status: implemented.

Decisions already taken by the user (do not change them):

- Every indicator gets the gradient look of the reference repo (`/tmp/refrepo`, TypeScript), using
  the same patterns, colours and transparency numbers listed below.
- Studies the reference does not have also get a gradient, chosen by their kind (bounded, centred on
  a baseline, channel …). Every study is listed in section 4 with what it gets. A study whose
  drawing does not suit a gradient (profiles, pivots, patterns, order flow …) is listed as
  **unchanged** with the reason.
- The indicator settings dialog is restyled to match our Chart settings dialog, and its bugs are fixed
  (section 5).
- "Dialog toolbar" means the small hide / settings / remove / more buttons on an indicator's legend
  row on the chart (section 6).

Read the whole document before starting. Do the work in the order of section 8.

Files you will touch (search by the quoted code if line numbers drift; numbers are from `a3cdef0`):

| Area | File |
|---|---|
| Indicator trait | `src/studies/indicator_trait.rs` |
| Applied-study wrapper, `StudySettings`, `PlotStyle` | `src/studies/settings.rs` (`ConfiguredStudy`) |
| Study colour constants | `src/studies/palette.rs` |
| Bands between plots | `src/studies/plots.rs` (`StudyFill`) |
| New module | `src/studies/shading.rs` (create) |
| Study renderer | `src/chart/renderers/indicator.rs` (`render_study`, `paint_fills`, `paint_band`, `draw_line`) |
| Pane levels and level fill | `src/widget/indicator_pane.rs` (~lines 130–145, 975–1030) |
| Settings dialog | `src/ui/dialogs/indicator_dialog/settings.rs` |
| Chart settings row helpers to share | `src/ui/dialogs/chart_settings/tabs/mod.rs` |
| Legend row buttons | `src/chart/renderers/legend/mod.rs` (`study_controls`, ~654–786) |
| Every study | `src/studies/builtin/*.rs` |

---

## 1. How the reference draws it (copy these rules exactly)

Source: `/tmp/refrepo/src/core/native-indicators/classics/shared.ts`, `define.ts`,
`/tmp/refrepo/src/core/model/scene.ts`, `/tmp/refrepo/src/renderers/native/backend/Canvas2dBackend.ts`
(`drawFill`, ~line 739).

### 1.1 Colour helpers

- `transp(color, t)`: same RGB, alpha = `round((100 - t) / 100 * 255)`. `t = 0` is opaque,
  `t = 100` is invisible. All transparency numbers in this document are in this 0–100 form.
- `blend(value, low, high, from, to)`: `t = clamp((value - low) / (high - low), 0, 1)` (0 when
  `high == low`); each RGB channel is `round(from + (to - from) * t)`; alpha of `from`. A non-finite
  value returns `from`. This is a plain sRGB byte lerp. Do not convert to linear light.
- `ramp(value, low, mid, high, lowInk, midInk, highInk)`: `blend(value, low, mid, lowInk, midInk)`
  when `value <= mid`, else `blend(value, mid, high, midInk, highInk)`.

### 1.2 Palette (reference names → our constants)

| Reference | Hex | Ours |
|---|---|---|
| BULLISH | `#089981` | `palette::UP` (already 8,153,129) |
| BEARISH | `#f23645` | `palette::DOWN` (already 242,54,69) |
| NEUTRAL | `#787b86` | `palette::MUTED` (already 120,123,134) |
| INFO | `#5b9cf6` | **add** `palette::INFO = Color32::from_rgb(91, 156, 246)` |
| WARNING | `#ff9800` | **add** `palette::AMBER = Color32::from_rgb(255, 152, 0)` |

### 1.3 The gradient fill

A fill between two series stores, per bar, an optional `GradientStop { top_value, bottom_value,
top_color, bottom_color }`. The span from bar `i` to bar `i + 1` is a quad between the two series and
is coloured by the stop of the **left** bar `i` (never `i + 1`). Inside the quad the colour is a
vertical linear gradient: `top_color` at `y(top_value)`, `bottom_color` at `y(bottom_value)`,
clamped beyond both ends.

The common stop is the **baseline wash** (`baselineGradient`): for a value `x`, a baseline `b`, an
ink `c`, `value_t` (default 50) and `base_t` (default 100):

```
top_value    = max(x, b)
bottom_value = min(x, b)
top_color    = transp(c, x > b ? value_t : base_t)
bottom_color = transp(c, x > b ? base_t : value_t)
```

So the fill is strongest at the line and fully transparent where it meets the baseline.

### 1.4 The shapes the reference reuses

| Pattern | Reference helper | What it draws |
|---|---|---|
| **Bounded** | `boundedOscillator` | Line coloured by `ramp(x, oversold, mid, overbought, oversoldInk, lineInk, overboughtInk)` where `mid = (ob + os) / 2`. Zone from overbought level to scale top filled `transp(overboughtInk, zone_t)`, zone from oversold level to scale bottom filled `transp(oversoldInk, zone_t)`, `zone_t` default **92**. Optional band fill between the two levels (flat colour). Optional midpoint wash: baseline wash between the line and `mid`, ink = the line's ramp colour of that bar. Levels: overbought and oversold dashed, optional midline dotted, all in the levels colour. |
| **Band only** | stochastic (`stochasticOutput`) | Lines keep their own colours, only a flat band between the two levels (`transp(INFO, 90)`), levels dashed. No ramp, no zones. |
| **Centred** | `centeredOscillator` | Line coloured bull when `x > baseline`, bear when `x < baseline`, line colour when equal. Baseline wash with ink = that side's colour, `value_t` default **50**. Zero/baseline level dashed (some studies dotted, see table). Optional extra levels. |
| **Regime** | `regimeIndex` (volume.ts ~54) | Line coloured bull/bear by its side of its **signal line** (not zero); wash between the line and the signal line. |
| **Spread** | Fisher, SMI Ergodic | Line coloured bull/bear by its side of its signal line; flat per-bar fill between the line and the signal in `transp(ink, t)` (Fisher 80, SMI 75). No gradient. |
| **Channel** | `directionalChannel` | Basis coloured bull when close > basis, bear when close < basis, neutral when equal. Upper line `transp(bear, 30)`, lower line `transp(bull, 30)`. Fill between upper and lower is a gradient from `transp(bear, 90)` at the upper line to `transp(bull, 90)` at the lower line. |
| **Flat channel** | Bollinger, Donchian | Flat fill between upper and lower (`transp(INFO, 92)` for Bollinger, `transp(NEUTRAL, 92)` for Donchian). |
| **Momentum columns** | `momentumColumnColors` (MACD) | Histogram columns: above zero and rising = growAbove, above zero and falling = fallAbove, below zero and rising = growBelow, below zero and falling = fallBelow. Rising means `x > previous` (previous = `x` on the first bar or after a gap). |
| **Rising/falling columns** | Awesome Oscillator, SMI Ergodic oscillator | Column = rising colour when `x > previous`, else falling colour. |
| **Intensity** | Historical Volatility, Ulcer Index | Line colour = `blend` from a calm colour to an accent; baseline wash to zero using that colour with its own `value_t`. HV: range = lowest/highest of the value over the last **100** bars, calm NEUTRAL, accent WARNING, `value_t` **75**, width 2. Ulcer: range = 0 … threshold (default 5), calm NEUTRAL, accent BEARISH, `value_t` 50, threshold level dashed, zero level dotted at `transp(NEUTRAL, 50)`. |
| **Line by price side** | VWAP | Line bull when `close >= line`, else bear. Band lines: upper `transp(bear, 40)`, lower `transp(bull, 40)`, fill between band pairs `transp(bandsInk, 95)`. |
| **ADX ink** | ADX | ADX line NEUTRAL while `adx <= keyLevel` (25); above it the colour of the dominant DI (+DI > −DI → bull, else bear). +DI `transp(bull, 30)`, −DI `transp(bear, 30)`, ADX width 2, key level dashed. |
| **Slope** | PVT, Accumulation/Distribution | Line bull while rising from the previous bar, bear while falling, neutral when flat. Width 2. |

---

## 2. Engine (new code)

### 2.1 `src/studies/shading.rs` (new, `pub mod shading` in `src/studies/mod.rs`, re-export the public types)

```rust
/// How a study's main line, wash and zones are coloured. Declared once per study type.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum ShadingKind {
    Bounded { line: usize, overbought: f64, oversold: f64, top: f64, bottom: f64,
              midline: Option<f64>, band_fill: bool, midpoint_wash: bool, zone_t: u8 },
    BandOnly { upper: f64, lower: f64 },
    Centered { line: usize, baseline: f64, extra_levels: Vec<f64>, baseline_dotted: bool },
    Regime { line: usize, signal: usize },
    Spread { line: usize, signal: usize, fill_t: u8 },
    Channel { upper: usize, lower: usize, basis: Option<usize> },
    FlatChannel { upper: usize, lower: usize, fill_t: u8 },
    MomentumColumns { line: usize, faded_t: u8 },
    RisingColumns { line: usize },
    Intensity { line: usize, range: IntensityRange },
    PriceSide { line: usize },
    AdxInk { adx: usize, plus: usize, minus: usize, key_level: f64 },
    Slope { line: usize },
}

#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum IntensityRange { Rolling(usize), Threshold(f64) }

/// Colours and switches the user can edit. Defaults come from the study.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct StudyShading {
    pub kind: ShadingKind,
    pub gradient_fill: bool,     // the "Gradient fill" switch; true by default
    pub bull: Color32,           // also "Oversold"/"Trending"/"Rising"/"Grow above" by kind
    pub bear: Color32,           // also "Overbought"/"Choppy"/"Falling"/"Fall below" by kind
    pub faded_bull: Color32,     // MomentumColumns "Fall above"
    pub faded_bear: Color32,     // MomentumColumns "Grow below"
    pub accent: Color32,         // Intensity accent
    pub band: Color32,           // band / zone-between-levels fill
    pub levels: Color32,         // level line colour
    pub value_t: u8,             // wash strength at the line (50 by default)
}
```

Put the colour helpers of section 1.1 here as `pub(crate) fn transp`, `blend`, `ramp`, plus
`pub(crate) fn baseline_stop(x, b, ink, value_t, base_t) -> Option<GradientStop>` and
`pub struct GradientStop { pub top_value: f64, pub bottom_value: f64, pub top_color: Color32,
pub bottom_color: Color32 }`. Unit-test each helper against hand values (e.g. `transp(UP, 90).a() ==
26`, `blend(5.0, 0.0, 10.0, black, white) == rgb(128,128,128)`).

The names in the dialog depend on the kind (section 5.4); the struct keeps generic field names so
one type serves every study.

### 2.2 Trait and wrapper

- `Indicator::shading(&self) -> Option<StudyShading>`: default `None`. Each study in section 4 that
  gets a pattern overrides it and returns its defaults.
- `StudySettings` gains `#[serde(default)] pub shading: Option<StudyShading>`. `Indicator::settings`
  (default impl, `indicator_trait.rs` ~229) fills it from `self.shading()`. Saved settings without
  the field keep loading (serde default) and then use the study's defaults.
- `ConfiguredStudy`:
  - `shading()` returns `self.settings.shading.clone().or_else(|| self.study.shading())`.
  - Keeps a per-bar colour cache `shading_colors: Vec<Option<Color32>>` (one per source bar, for the
    shaded line) and `shading_stops: Vec<Option<GradientStop>>`. Rebuild them in `update` from the
    `from` index (incremental, like the values), and fully in `apply_settings`/`restore_settings`.
    `Intensity::Rolling(100)` needs a rolling min/max — use a monotonic deque so it stays O(n).
  - `plot_color(line, bar)`: first `self.study.plot_color(line, bar)` (studies that already colour
    their own bars keep doing so, e.g. MACD columns until migrated); else the shading cache when
    `line` is the shaded line; else `None`. The existing renderer already calls `plot_color`, so lines
    need no renderer change. The cache is indexed by the **study's own** bar index; `plot_color`
    already maps aligned bars before calling into the study — keep that order.
  - `pane()`: when shading is `Bounded`, `BandOnly` or `Centered`, replace `levels` with the shading
    levels (overbought, oversold, midline / upper, lower / baseline and extra levels) in
    `shading.levels` colour, and set `fill_between_levels = false` (the new renderer draws the band).
  - `level_style(value)`: midline and a `baseline_dotted` baseline → `Dotted`; others `Dashed`.
  - `apply_settings` validates shading: finite levels, `overbought > oversold` for `Bounded`
    (swap is not allowed — return `Err("overbought must be above oversold")`), transparencies 0–100.
- `Indicator::default_settings(&self) -> StudySettings`: default `self.settings()`. `ConfiguredStudy`
  stores the settings it had when it was constructed (`ConfiguredStudy::new`) and returns those, with
  `shading` reset to `self.study.shading()`. Used by the dialog's Defaults button.

### 2.3 Renderer

In `render_study` (`src/chart/renderers/indicator.rs` ~364), before `paint_fills`, call a new
`paint_shading(&painter, indicator, range, price_scale, coords, context.rect)` that draws, in this
order:

1. **Zones** (`Bounded`): rect from `y(overbought)` to `y(top)` in `transp(bear, zone_t)` and from
   `y(oversold)` to `y(bottom)` in `transp(bull, zone_t)`, full pane width, clipped to the pane.
2. **Band between levels** (`Bounded` with `band_fill`, `BandOnly`): rect between the two levels in
   `shading.band`.
3. **Washes** (`Bounded` with `midpoint_wash`, `Centered`, `Regime`, `Intensity`, and %B/BBW): only
   when `gradient_fill` is true. For each visible span `i → i+1` where both ends of both series are
   finite: four vertices `(x_i, y(a_i))`, `(x_{i+1}, y(a_{i+1}))`, `(x_{i+1}, y(b_{i+1}))`,
   `(x_i, y(b_i))`; each vertex gets the colour of the bar-`i` stop evaluated at that vertex's `y`
   (clamped to the stop's top/bottom). When the line crosses the other boundary inside the span, split
   at the crossing exactly as `paint_band` already does: the left piece uses the stop of bar `i`, the
   right piece the stop of bar `i + 1`. (The reference does not split; we already split flat bands
   and keeping the colour on the correct side of zero is required.)
   Build **one `egui::Mesh` per fill per frame** — never one shape per bar. Do not antialias (shared
   edges would show seams; see the comment in `paint_band`).
4. **Spread / Channel / FlatChannel / PriceSide band pairs**: flat or two-colour fills through the
   same mesh path (`Channel` uses a stop with `top_value = upper`, `bottom_value = lower`).

The price scale must not change because of shading: zones and anchors never feed autoscale.
Inverted price scales: compute stops in value space (as above) and only map to `y` at the end, so
"above" keeps its meaning (same rule as the comment in `paint_fills`).

`IndicatorLineStyle::fill` (used by `ma_combination`) stays as it is.

### 2.4 Pane level fill

`indicator_pane.rs` ~980 paints a flat rect between the outer levels when `fill_between_zones`.
Leave that path for studies without shading. Studies with shading set `fill_between_levels = false`
via `ConfiguredStudy::pane`, so the old rect is not drawn twice.

---

## 3. Defaults every pattern uses

Unless section 4 says otherwise:

- `gradient_fill = true`, `value_t = 50`, `zone_t = 92`.
- `bull = UP`, `bear = DOWN`, line (neutral) colour = the plot's own colour, `levels = MUTED`.
- `Bounded`: `bull` is the **oversold** colour and `bear` the **overbought** colour (reference RSI);
  `band = transp(MUTED, 90)`; line width 2 where the table says so.
- `MomentumColumns`: `bull = UP` (grow above), `faded_bull = transp(UP, 50)` (fall above),
  `faded_bear = transp(DOWN, 50)` (grow below), `bear = DOWN` (fall below).

Line widths: when the table says "width 2", change the study's default `line_style(line).width` /
`PlotStyle::width` to 2.0. Existing saved settings keep their saved width.

---

## 4. Every study and what it gets

Line references use the study's `line_names()` order (index 0 first). Where a study has no level or
input the table needs, add it as a study input with the given default (use `study_inputs!` like the
other inputs) so the user can edit it, and keep it in sync with `shading()`.

### Moving averages (overlay)

| Study | Pattern | Details |
|---|---|---|
| SMA, EMA, WMA, DEMA, TEMA, HMA, ALMA, VWMA, LSMA, SWMA, KAMA, SmoothedMA, McGinleyDynamic, ZLEMA, HammingMA | unchanged | Reference draws a plain line. |
| VIDYA | Adaptive fade | Line colour = `blend(min(scaler, 1), 0, 1, MUTED, lineColour)` where `scaler` is the study's own adaptive factor at that bar (expose it from `vidya.rs` and return it through `plot_color`). Add input "Adaptive gradient" (default on). |
| GuppyMA, MultipleMA, EMACross, MACross, MixedMACross, TripleMA | unchanged | Several plain lines; markers already distinguish them. |
| MARibbon (`ma_combination`) | unchanged | Already has its own fill (`IndicatorLineStyle::fill`). |
| MAChannel | Channel | upper = "Upper", lower = "Lower", basis = "MA". |

### Momentum

| Study | Pattern | Details |
|---|---|---|
| RSI | Bounded | 70/30, top 100, bottom 0, midline 50, band fill, no midpoint wash, width 2. Replaces the current `fill_between_levels`. |
| RsiDivergence | Bounded | Same as RSI. Divergence lines and labels unchanged. |
| MACD | MomentumColumns | Line = histogram. MACD line `palette::INFO`, signal `palette::AMBER`, zero level dashed MUTED. Replace the study's current `plot_color` histogram logic with the shading (remove the study override so the cache is used). |
| VolumeWeightedMACD | MomentumColumns | Same as MACD. |
| Stochastic | BandOnly | 80/20, band `transp(INFO, 90)`. %K `INFO`, %D `AMBER`. |
| StochasticRSI | BandOnly | 80/20, as Stochastic. |
| KDJ | BandOnly | 80/20. |
| DoubleStochastic | BandOnly | 80/20. |
| StochasticMomentumIndex (SMI) | BandOnly | +40/−40. |
| WilliamsR | Bounded | overbought −20, oversold −80, top 0, bottom −100, no midline, no band fill. Add the levels (it has none today). |
| CCI | Centered | baseline 0 dotted, extra levels +100/−100 dashed. Add the levels. |
| Momentum | Centered | baseline 0. |
| RateOfChange | Centered | baseline 0. |
| TRIX | Centered | baseline 0. |
| TrueStrengthIndex | Centered | line = "TSI"; signal keeps its colour. |
| KnowSureThing | Centered | line = "KST". |
| CoppockCurve | Centered | `value_t = 60`. |
| DetrendedPriceOscillator | Centered | baseline 0 (keep existing zero level). |
| PrettyGoodOscillator | Centered | baseline 0. |
| ChandeMomentumOscillator | Centered + zones | baseline 0 dotted, levels +50/−50 dashed, zones +50…+100 in `transp(bear, 92)` and −50…−100 in `transp(bull, 92)` (draw them with the Bounded zone code: add `zones: Option<(f64, f64, f64, f64)>` to `Centered` if needed). |
| SchaffTrendCycle | Centered | baseline **50**, line neutral = bear (reference), levels 75/25 dashed, width 2. |
| UltimateOscillator | Bounded | 70/30, `zone_t = 91`, no band fill. |
| ConnorsRSI | Bounded | 90/10, midpoint wash. |
| RelativeVigorIndex | unchanged | Reference draws plain lines. |
| CenterOfGravity | Spread | line = COG, signal = its trigger, `fill_t = 80`. |
| RainbowOscillator | unchanged | Own multi-band drawing. |
| UltimateMomentum | Centered | baseline 0. |
| PriceOscillator (PPO) | Centered + columns | line = "PPO", signal `AMBER`. Histogram (if present) columns: `transp(side ink, 40)` while the spread widens, `transp(side ink, 70)` while it narrows (reference ppo ~239). |
| PercentageVolumeOscillator (PVO) | Centered + columns | Same as PPO, `value_t = 60`. |
| VolumeOscillator | Centered | `value_t = 60`. |
| BalanceOfPower | Centered | `value_t = 60`. |
| EaseOfMovement | Centered | baseline 0. |
| AwesomeOscillator | RisingColumns | rising `UP`, falling `DOWN`, zero level dashed. Keep the existing `plot_color` if it already does exactly this; otherwise move it to shading. |
| AcceleratorOscillator | RisingColumns | As AO. |
| AroonOscillator | Centered | baseline 0, range ±100 kept. |
| BullBearPower | unchanged (colours only) | Columns are coloured by sign through its existing `plot_color` (`x >= 0` bull, else bear). Keep that logic; only set the colours to `UP`/`DOWN`. |
| AdvanceDeclineRatioBars | unchanged | Breadth bars, no baseline meaning. |
| PriceMomentumOscillator | Centered | line = PMO, keep zero level. |
| PringsSpecialK | Centered | baseline 0. |
| SMIErgodicIndicator | Spread | `fill_t = 75`, grow `UP`, fall `DOWN`, signal `AMBER`, zero level dashed `transp(MUTED, 50)`. |
| SMIErgodicOscillator | RisingColumns | grow `UP`, fall `DOWN`. |
| WoodiesCCI | unchanged | Keeps its trend-coloured histogram; only levels colour → MUTED. |
| ElderRay | unchanged | Reference: plain bull/bear columns. Align colours to UP/DOWN. |
| FisherTransform | Spread | `fill_t = 80`, line width 2, levels +1.5/−1.5 dashed and 0 dotted (add them). |
| MassIndex | BandOnly | upper 27, lower 26.5, band `transp(AMBER, 90)`. |
| Aroon | unchanged | Reference: plain Up/Down lines. |
| RSI-based ribbons (`RciRibbon`), KnoxvilleDivergence, TechnicalRatings | unchanged | Own drawing / markers. |

### Trend

| Study | Pattern | Details |
|---|---|---|
| ADX | AdxInk | key level 25 (add as input "Key level"), +DI and −DI at `transp(…, 30)`, ADX width 2. |
| DirectionalMovement | AdxInk | Same; if the study has no ADX line, only colour +DI/−DI. |
| AdxBreakout | unchanged | Already has its threshold level. |
| SuperTrend, IchimokuCloud, ParabolicSAR, WilliamsAlligator, ChopZone, VolatilityStop, ChandeKrollStop, ElderImpulseSystem, VortexIndicator | unchanged | Own colouring or plain lines in the reference. |
| SqueezeMomentum | MomentumColumns | `faded_t = 60` (reference TTM: `transp(bull, 60)` fall above, `transp(bear, 60)` grow below). Squeeze dots unchanged. |
| TrendIntensityIndex | Bounded | 80/20, top 100, bottom 0, band fill. |
| TrendDetectionIndex, Qstick | Centered | baseline 0. |
| ChoppinessIndex | Bounded | upper 61.8 (bear = "Choppy"), lower 38.2 (bull = "Trending"), band fill, midpoint wash. |

### Volatility

| Study | Pattern | Details |
|---|---|---|
| BollingerBands | FlatChannel | `fill_t = 92` with `INFO`; bands `INFO`, basis `AMBER`. It already has a `StudyFill`; replace that by the shading. |
| BollingerBandsWidth (BBW) | Centered (no colour change) | Wash from BBW to 0 in the line colour, `value_t = 50`, no bull/bear (bull = bear = line colour). |
| ATR, TrueRange, StandardDeviation, AverageDailyRange | unchanged | Reference: plain line. |
| KeltnerChannels | Channel | upper/lower/basis by name ("Upper…", "Lower…", "EMA…"). Replace its current `StudyFill`. |
| DonchianChannels | FlatChannel | `fill_t = 92`, `MUTED`; upper `DOWN`, middle `MUTED`, lower `UP`. |
| AccelerationBands | Channel | basis = "Middle". |
| Envelopes | Channel | basis = the MA line. |
| ChandelierExit | unchanged | Stop line. |
| HistoricalVolatility | Intensity | `Rolling(100)`, accent `AMBER`, `value_t = 75`, width 2. |
| OhlcVolatility, ParkinsonVol, ZeroTrendVol | Intensity | Same as HistoricalVolatility. |
| ChaikinVolatility | Centered | bull = `AMBER` ("Expansion"), bear = line colour. |
| PercentB | Centered | baseline **0.5** dotted `transp(MUTED, 50)`, levels 1 and 0 dashed. |
| CommoditySelectionIndex | Centered (no colour change) | Wash to 0 in the line colour. |
| BBTrend | unchanged | Already coloured columns. |
| UlcerIndex | Intensity | `Threshold(5)` (add input "Stress threshold"), accent `DOWN`. |
| RelativeVolatilityIndex | Bounded | 80/20, **bull = overbought** and bear = oversold for this study (reference swaps them: high RVI is bullish volatility), width 2. Replaces `fill_between_levels`. |
| Median | unchanged | Already has its fill. |

### Volume

| Study | Pattern | Details |
|---|---|---|
| Volume, UpDownVolume, Volume24h, RelativeVolumeAtTime, MarketFacilitationIndex, NetVolume | unchanged | Bar-coloured columns already. |
| OnBalanceVolume | unchanged | Reference: plain line. |
| VolumeWeightedAvgPrice (VWAP) | PriceSide | Band lines `transp(bear/bull, 40)`, band pair fill `transp(MUTED, 95)`. |
| AnchoredVWAP, VwapAutoAnchored, TimeWeightedAvgPrice | PriceSide | Line only. |
| MoneyFlowIndex | Bounded | 80/20, `zone_t = 91`, band `transp(MUTED, 94)`. |
| AccumulationDistribution | Slope | width 2. |
| ChaikinMoneyFlow | Centered | baseline 0. |
| VolumeRateOfChange | Centered | baseline 0. |
| KlingerOscillator | Centered | line = "KVO", `value_t = 60`, width stays 1, signal `AMBER`. |
| ForceIndex | Centered | baseline 0, `value_t = 60`, width 2. |
| VolumePriceTrend, PriceVolumeTrendStudy, WilliamsAD | Slope | width 2. |
| NegativeVolumeIndex, PositiveVolumeIndex | Regime | line = NVI/PVI, signal = "Signal", width 2. |
| ChaikinOscillator | Centered | baseline 0 (keep its zero level). |

### Profiles, support/resistance, price, market structure, breadth, patterns, order flow

| Study | Pattern | Reason |
|---|---|---|
| VolProfileVisible, VolProfileFixed, VolProfileSession, VolProfilePeriodic, VolProfileAnchored, VolProfileAutoAnchored, TimePriceOpportunity, Seasonality, LunarPhases, Performance | unchanged | Not a value-over-time line. |
| PivotPoints, PivotPointsHighLow, IntradayPivotPoints, ProjectedPivotPoints, MissedPivots, KeyLevels, GhostPivots, Reversal, ElliottWave, RisingWedge, FallingWedge, WilliamsFractals, ZigZag | unchanged | Levels, drawings or markers. |
| HigherIntervalCandles, BollingerBars, TypicalPrice, MedianPrice, WeightedClose, OHLC4, VisibleAveragePrice | unchanged | Price replicas. |
| Week52HighLow, Week52HighLowLevels, TradingSessions, Gaps | unchanged | Levels/zones of their own. |
| AdvanceDeclineRatio, Ratio, Spread, RciRibbon, MajorityRule, StandardError, StandardErrorBands, LinearRegression, LinearRegressionChannel | unchanged | No baseline meaning or own drawing. |
| AdvanceDeclineLine, CumulativeVolumeIndex, AccumulationSwingIndex | Slope | Cumulative lines. |
| LinearRegressionSlope | Centered | baseline 0, `gradient_fill = false` (reference only colours the line), width 2. |
| TrendStrengthIndex, Correlation, RankCorrelationIndex | Centered | baseline 0. |
| CandlestickPatterns | unchanged | Markers. |
| Continuation Chart Patterns, Reversal Chart Patterns (`ChartPatterns`, one entry per `PatternFamily::ALL`) | unchanged | Pattern drawings. |
| Auto Trend Line, Auto Parallel Channel, Auto Fibonacci Retracement, Auto Fibonacci Extension, Auto Pitchfork (`AutoDrawing`, one entry per `AutoDrawingKind::ALL`, `drawings` feature) | unchanged | Drawings. |
| Quoted Spread, Book Size, Trade Rate, Aggressor Rate, Levels Executed, Multi-Range Delta, Rolling Volume and Delta, Size-Filtered CVD, Levels Traded Through (`OrderFlowStudy`, one entry per `OrderFlowKind`) | unchanged | Host order-flow streams. |
| Footprint studies, `trade_delta`, `trade_profile`, `cvd`, `market_breadth` (types in `src/studies/builtin` that are not in the factory) | unchanged | Footprints or host streams; they return `None` from `shading()`. |
| Studies a host adds with `IndicatorFactory::register` / `register_in`, and `CustomIndicator` | unchanged | Default `shading()` is `None`; hosts may override it. |

### 4.1 Coverage rule (mandatory)

The tables above cover **every** name in `IndicatorFactory::new().list()` (185 types from the
`register_builtins!` list plus the families registered in loops after it). Doing only the studies
whose names you remember is not acceptable.

1. Before writing code, print `IndicatorFactory::new().list()` (with all features) and tick every name
   against these tables. Display names differ from type names (e.g. `RSI` is `RSI`, `KeltnerChannels`
   is `"Keltner"`); match through the type each constructor builds.
2. Implement **every** row whose pattern is not "unchanged". Do not skip a row because it looks
   similar to another one; each study needs its own `shading()` override.
3. If a registered name is not in the tables, stop and list it in the PR description; do not guess a
   pattern.
4. Test 14 (section 7) enforces this: it iterates `IndicatorFactory::new().list()` at runtime, so the
   loop-registered families are included, and fails for any name that has no entry in the expected
   table.

---

## 5. Indicator settings dialog

File: `src/ui/dialogs/indicator_dialog/settings.rs`.

### 5.1 Bugs

1. **Visibility tab cannot show a study again.** The study context menu writes
   `StudySettings::interval_range` (`src/ui/context_menu/study.rs` ~387), e.g. `Only(1h)`.
   `ConfiguredStudy::is_visible` (`settings.rs` ~901–906) hides the study when the interval is outside
   that range, but the dialog only shows `hidden_intervals`. Ticking 15m in the dialog does nothing.
   Fix: show a "Range" dropdown above the checkboxes with All / Only current / Current and above /
   Current and below (same four as the context menu, same mapping), and grey out (disable) the
   checkboxes of intervals the range excludes with the hover text "Outside the selected range".
   Ticking a disabled one is impossible, so the two settings never disagree.
2. **Line pattern names are raw debug text** ("LargeDashed", "SparseDotted"): `format!("{:?}")`
   at ~348–369. Use `LineStyle::all()` and the same `line_style_preview` labels as Chart settings
   (`chart_settings/tabs/canvas.rs` `line_style_preview`), moved to the shared helpers (5.2).
3. **"Reset inputs" only resets inputs** and sits as a small button above the form. Replace it with a
   footer **Defaults** button (5.3) that resets inputs, plot styles, precision, shading and
   visibility to `Indicator::default_settings()` and the inputs' `default` values.
4. `show(ctx, _theme)` ignores the theme. Use it for every colour and text size (5.2).
5. **Width is applied without a bound**: `DragValue` for width has no `speed`; set `.speed(0.1)` and
   keep `0.5..=10.0`. Precision drag keeps `0..=12`.
6. **Colour buttons are egui's native picker** (`color_edit_button_srgba`) in Inputs (Color inputs)
   and Style. Use `ui_kit::ColorPicker` with one `ColorPickerState` per row (keyed by input id /
   plot index / shading field), like Chart settings.
7. Overbought must stay above oversold (shading validation, 2.2): show the error in the footer with
   `status_message` like Chart settings and disable Apply.

### 5.2 Match the Chart settings look

Move these private helpers from `src/ui/dialogs/chart_settings/tabs/mod.rs` into a new shared module
`src/ui_kit/settings_rows.rs` (export from `ui_kit/mod.rs`) and use them from both dialogs:
`section_header(ui, title, theme)`, `setting_row(ui, label, label_width, theme, |ui| …)`,
`styled_dropdown`, `styled_checkbox`, `line_style_preview`. Chart settings must look exactly as before
(its tests must still pass).

Layout of the indicator dialog:

- `Modal` width 620 (keep), **fixed height 560 for every tab** (already), clamped to the window.
- Tab row: `TabBar::new(&[Inputs, Style, Visibility], &mut tab).pills()` with icons via
  `TabLabel::tab_icon`: Inputs `icons::SETTINGS_NUT`, Style `icons::PAINTBRUSH`, Visibility
  `icons::EYE`. Update `self.tab` before drawing the content (same frame switch, like Chart settings).
- Content in a vertical `ScrollArea` with `auto_shrink([false, false])`, top-down layout.
- Inputs tab: section header "INPUTS"; one `setting_row` per input (label column = widest label of
  this study, capped at 40% of the width); controls: `DragValue`, `styled_dropdown` (choices, source,
  timeframe), `styled_checkbox` (toggle), `ColorPicker` (colour), `TextEdit` (symbol/text).
  Keep the existing rules: study-source inputs hide `calculation_symbol`, `calculation_timeframe`,
  `wait_for_close`; custom timeframe seconds field.
- Style tab:
  - section "PLOTS": per plot one row: visibility checkbox + name, `ColorPicker`, width `DragValue`,
    pattern dropdown (with preview labels), plot type dropdown (keep the incompatibility rules and the
    disabled-with-reason entries).
  - section "SHADING" (only when `settings.shading` is `Some`): "Gradient fill" checkbox (only for
    kinds that have a wash), the colour rows named per kind (5.4), and for `Bounded` / `BandOnly`
    `Centered` the level `DragValue`s (overbought/oversold, upper/lower, extra levels), plus
    "Zones" transparency `DragValue` 0–100 for `Bounded`.
  - section "PRECISION": the existing Automatic checkbox and decimals.
- Visibility tab: section "INTERVALS": the Range dropdown (bug 1), then the interval checkboxes in a
  wrapped grid.
- Footer: `DialogFooter::new("Apply").secondary("Cancel").primary_enabled(valid).show_with_left(ui,
  |ui| { error message or Defaults button })`.

### 5.3 Defaults button

Left side of the footer. Click → stage the defaults (inputs, plots, precision, shading,
`hidden_intervals = []`, `interval_range = All`). Nothing is applied until Apply; Cancel/Escape still
discards everything.

### 5.4 Colour row names per kind

| Kind | Rows (in order) |
|---|---|
| Bounded | Overbought (`bear`), Oversold (`bull`), Band fill (`band`), Levels (`levels`) — RVI and Choppiness use their own names: RVI "Overbought" = `bull`; Choppiness "Choppy" = `bear`, "Trending" = `bull` |
| BandOnly | Band fill, Levels |
| Centered | Bullish, Bearish, Levels (ChaikinVolatility: "Expansion" only) |
| Regime, Spread | Bullish, Bearish |
| Channel | Bullish, Bearish |
| FlatChannel | Fill |
| MomentumColumns | Grow above, Fall above, Grow below, Fall below |
| RisingColumns | Rising, Falling |
| Intensity | Accent |
| PriceSide, Slope | Bullish, Bearish |
| AdxInk | +DI / bullish, −DI / bearish, Key level (DragValue) |

All labels through `l10n::text`.

---

## 6. Legend row controls

File: `src/chart/renderers/legend/mod.rs`, `study_controls` (~654–786). They work; align the look
only:

- Buttons 20 px, icons 14 px (keep). Hover fill `theme.ui.btn_bg_hover` (keep); icon tint
  `theme.ui.text_secondary`, **`theme.ui.text` while hovered** (today it stays secondary).
- A hidden study's row text and swatches draw at `gamma_multiply(0.5)` (check the row painter; add it
  if missing) and the first button shows `EYE_HIDE` (already).
- Settings button opens the restyled dialog (unchanged wiring).
- Keep `study_controls_reveal_on_touch_and_dismiss_outside` passing.

---

## 7. Tests to add

Put them next to the code they test.

1. `shading.rs`: `transp`, `blend`, `ramp`, `baseline_stop` hand values (above, below, equal to the
   baseline, non-finite).
2. Bounded RSI on a rising then falling series: `plot_color` at RSI 70+ equals `DOWN`, at 30− equals
   `UP`, at 50 equals the line colour, between blends.
3. Centered CCI: a frame with values above and below zero paints a mesh whose vertices at the line
   have alpha ≈ 127 (t 50) and at the zero line alpha 0; vertices above zero are `UP`, below `DOWN`;
   a span crossing zero is split at the crossing.
4. Gradient fill off → no wash mesh, line colours unchanged.
5. Bounded zones: RSI pane paints a rect from y(70) to y(100) in `transp(DOWN, 92)` and one from
   y(30) to y(0) in `transp(UP, 92)`; the old flat between-levels rect is not painted twice.
6. Channel Keltner: basis colour follows close vs basis; fill vertices at the upper line are
   `transp(DOWN, 90)`, at the lower line `transp(UP, 90)`.
7. MACD columns: four-tone rule on a hand series.
8. Intensity HV: colour at the 100-bar high is `AMBER`, at the low `MUTED`.
9. Saved settings without `shading` load and use the study defaults; edited shading round-trips
   through `snapshot_settings`/`restore_settings` and serde.
10. Dialog: Style tab shows SHADING rows for RSI and none for SMA; changing Overbought to below
    Oversold disables Apply with the error; Defaults restores inputs, plots and shading; Cancel
    discards.
11. Dialog Visibility: with `interval_range = Only(1h)` the 15m checkbox is disabled; choosing Range
    All re-enables it and Apply makes the study visible on 15m.
12. Dialog pattern dropdown shows the preview labels, never "LargeDashed".
13. Mesh count: a study with a wash adds one mesh shape per fill per frame regardless of bar count.
14. Coverage test in `src/studies/factory/tests.rs`: copy the section 4 tables into a
    `match name { … }` that maps each registered name to its expected `ShadingKind` variant (or to
    "unchanged"), with **no** `_ =>` arm that accepts unknown names (an unknown name panics with the
    name). Iterate `IndicatorFactory::new().list()`, create each study, wrap it in `ConfiguredStudy`,
    and assert that `shading()` is `Some` with that variant, or `None` for unchanged ones. Run it with
    default features and with `--all-features` (auto drawings appear only with `drawings`).
15. Update existing tests that assert old colours/fills (RSI level fill, Bollinger/Keltner fills,
    MACD histogram colours, `dialog_follows_the_window_when_it_shrinks`,
    `escape_discards_staged_inputs_and_presentation`).

---

## 8. Work order

1. `palette::INFO`, `palette::AMBER`; `shading.rs` with helpers and tests.
2. Trait method, `StudySettings::shading`, `ConfiguredStudy` cache, `pane()`/`level_style()`
   overrides, `default_settings()`.
3. Renderer `paint_shading` (zones, band, washes, channels) with one mesh per fill.
4. Write the coverage test (test 14) first so it fails, then go through the studies table by table
   (Momentum, Volatility, Volume, Trend, Moving averages, Statistics and the rest) until it passes.
   Remove per-study `fills()`/`plot_color()`/`fill_between_levels` that the shading replaces.
5. Shared `ui_kit/settings_rows.rs`, then the dialog rewrite and bug fixes.
6. Legend control tweaks.
7. Tests, then the checks below.

## Constraints

- Follow the existing code style; comments only for constraints the code cannot show; every visible
  string through `l10n::text`.
- No third-party product names in code, comments, tests, docs or commit messages. Before committing:
  ```bash
  git diff | grep -iE "^\+.*(trading""view|v""ela)" | grep -v WaveLabel
  ```
  must print nothing.
- Performance: no per-bar allocation in the paint path beyond the one mesh per fill; caches are
  incremental from `update(from)`.
- Run `cargo fmt --check`, `cargo clippy --all-features --all-targets -- -D warnings`,
  `cargo clippy --no-default-features --all-targets -- -D warnings`, `cargo test --lib`,
  `cd examples/demo && cargo test`, `cd terminal && cargo check --tests`.
- Rebuild the demo in release (`cd examples/demo && trunk serve --release --address 127.0.0.1 --port 8081`)
  and check by hand: RSI/MFI zones and ramp, CCI/TRIX washes fading to zero, Keltner two-colour
  channel, MACD four-tone histogram, HV intensity, the restyled dialog in light and dark theme, the
  Visibility range fix, Defaults/Cancel behaviour, legend icon hover.
