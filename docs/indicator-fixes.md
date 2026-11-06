# Indicator calculation fixes, empty-study state and gradient follow-ups

Status: partly implemented. Section 2 formulas are in the code with their tests. The study
status API, legend text, empty-pane message and demo breadth bars are wired. Still open:
the settings-dialog restyle (section 4.3), `SpreadColumns`, the factory smoke test, the
release demo rebuild, and these failing tests — `staple_studies_match_published_definitions`
(SMI Ergodic rows still expect the old TSI seed), `visible_average_follows_the_visible_range`,
and `status_visibility_reaches_the_chart_legend`.

Three problems, found by an audit of every factory study that has no bar-by-bar reference test:

1. Some studies compute numbers that differ from the standard definition (section 2).
2. A study that is active but has no values looks switched off: bright title, "∅" values and an
   empty pane with a 0–100 axis, with no reason given (section 3).
3. `docs/indicator-gradients.md` was only partly implemented (section 4).

Read the whole document first. Do the work in the order of section 6. Every change needs the test
listed with it. Do not change studies this document marks **keep**.

Sources used, in this order of priority:

- The reference TypeScript catalogue at `/tmp/refrepo/src/core/native-indicators/classics/`
  (`math.ts`, `averages.ts`, `oscillators.ts`, `volume.ts`, `volatility.ts`, `bands.ts`). Where a study
  exists there, **match it exactly**, including seeds and warm-up. File and line numbers are given.
- Otherwise the original author's published definition, named in the row.

Studies already compared bar by bar in `src/studies/reference.rs` (RSI, MACD, Bollinger, ATR, ADX, CCI,
MFI, Stochastic RSI, Aroon, PSAR, …) are correct and are out of scope, except where named below.

Shared conventions of the reference (`math.ts`), needed by several rows:

- `ema(v, n)` and `rma(v, n)` are `recursive(v, n, alpha)` (`math.ts` 206–240): the **first value is
  the SMA of the first `n` finite inputs**, then `prev = alpha*x + (1-alpha)*prev`; a non-finite input
  resets the run and the next finite stretch re-seeds. `alpha = 2/(n+1)` for `ema`, `1/n` for `rma`.
  Our `builtin/smoothing.rs` `Smoothing::next(x, seed = n, alpha)` gives the same seed when called with
  `seed = n`; `seed = 1` means "start at the first value", which is **not** the reference seed.
- `atr(bars, n) = rma(trueRange, n)` (Wilder). Not an SMA of true range.
- `stdev` is the **population** standard deviation (`math.ts` 134–146).
- `cmo(v, n) = 100*(sumUp - sumDown)/(sumUp + sumDown)` over `n` one-bar changes, 0 when both are 0
  (`math.ts` 452–457).
- `linreg(v, n, offset)` evaluates the least-squares line at `x = n - 1 - offset` (`math.ts` 321–337).

---

## 1. Files

| Area | File |
|---|---|
| Studies | `src/studies/builtin/*.rs` (one file per row below) |
| Shared EMA seed helper | `src/studies/builtin/smoothing.rs` |
| Study trait | `src/studies/indicator_trait.rs` |
| Applied-study wrapper | `src/studies/settings.rs` (`ConfiguredStudy`) |
| Shading | `src/studies/shading.rs` |
| Study data plumbing | `src/widget/studies_data.rs` (`study_data_requests`, `set_study_data`, `study_data_error`) |
| Legend | `src/chart/renderers/legend/mod.rs` |
| Indicator pane | `src/widget/indicator_pane.rs` (autoscale fallback ~963) |
| Study renderer | `src/chart/renderers/indicator.rs` |
| Settings dialog | `src/ui/dialogs/indicator_dialog/settings.rs` |
| Chart settings helpers | `src/ui/dialogs/chart_settings/tabs/mod.rs` |
| Demo | `examples/demo/controller.rs`, `examples/demo/live.rs`, `examples/demo/binance/` |

---

## 2. Calculation fixes

Severity: **WRONG** = numbers differ from the source; **DEFAULT** = an input default or a missing
input; **EDGE** = an edge case (zero, gap, tiny period). For every row write a unit test in the study's
file with a short hand series and expected values computed from the source formula (write the
expected numbers in the test, do not compute them with the code under test).

When a default changes, saved layouts keep their saved input values (inputs are restored by id); only
new studies get the new default.

### 2.1 Moving averages

| Study (file) | Severity | Now | Fix (target = source) |
|---|---|---|---|
| McGinley Dynamic (`mcginley_dynamic.rs`, `update`) | WRONG | `divisor = 0.6 * period * ratio^4` | Ref `math.ts` 414–429: seed `md = first finite close`; then `ratio = md == 0 ? 0 : close/md`, `denom = period * ratio^4`, `md += denom == 0 ? 0 : (close - md)/denom`. Remove the `0.6`. Test: period 10, md 100, close 110 → `md = 100.683013455` (now 101.138). |
| ZLEMA (`vidya.rs`, `ZLEMA::update`) | WRONG + DEFAULT | `lag = (period-1)/2` (floor); before `lag` bars it feeds the raw close into the EMA; seeds the EMA with the mean of `period` inputs; default 14 | Ref `averages.ts` 217–222: `lag = round((period-1)/2)` (half away from zero, `f64::round`); `delagged[i] = 2*close[i] - close[i-lag]`, **undefined** (skip, do not feed the EMA) until `i >= lag`; `zlema = ema(delagged, period)` with the reference seed (`Smoothing::next(x, period, 2/(period+1))`); output from the first finite EMA. Default period **21**. Test: period 10 → lag 5; period 21 → lag 10; a hand series of 12 bars for period 4 (lag 2, seed = mean of delagged[2..=5]). |
| VIDYA (`vidya.rs`, `VIDYA`) | WRONG + DEFAULT | Scaler = `|up-down|/(up+down)` over `period` changes; no volatility mode; no `alpha <= 1` cap; per-bar scaler pushed into a `Vec` inside the stepper state (cloned with the state every bar) | Ref `averages.ts` 230–273. Inputs: Length (default **12**), Adaptive scaler choice `Momentum (|CMO|)` / `Volatility (StDev ratio)` (default Momentum), Scaler lookback (default **9**), Reference stdev length (default **30**, volatility mode only), keep "Adaptive gradient" (default on). `k = |cmo(close, scalerLen)|/100` or `stdev(close, scalerLen)/stdev(close, refLen)` (0 when the reference is 0); `alphaBase = 2/(len+1)`; on each finite close: if `prev` or `k` is not finite, `prev = close`, else `alpha = min(alphaBase*k, 1)`, `prev = alpha*close + (1-alpha)*prev`. Output `prev`. Store `k` per bar in a field `scales: Vec<f64>` indexed by bar (keep a running bar index in the stepper state, as `SchaffTrendCycle` does with `state.index`, and write `scales[index]`); do **not** keep a growing `Vec` inside the stepper state. `plot_color` blends `MUTED → line colour` by `min(k, 1)`. Test: both scaler modes on a 15-bar series. |
| KAMA (`kama.rs`) | WRONG | No value until `period` closes; first value = SMA of the first `period` closes | Ref `math.ts` 385–407: `direction = |close - close[i-period]|`, `volatility = sum(|change|, period)`. For a finite close where direction or volatility is not yet defined: `prev = prev finite ? prev : close`, output `prev`. Otherwise `er = volatility == 0 ? 0 : direction/volatility`, `sc = (er*(fastAlpha - slowAlpha) + slowAlpha)^2`, `prev = prev finite ? prev + sc*(close - prev) : close`. `fastAlpha = 2/(fast+1)`, `slowAlpha = 2/(slow+1)` from the existing inputs. Update the existing tests. |
| LSMA (`lsma.rs`) | DEFAULT + EDGE | Default period 20; period 1 allowed | Default **25** (ref `averages.ts` 325). Input minimum **2**. |
| Smoothed MA / SMMA (`smoothed_ma.rs`) | EDGE | A non-finite close returns `None` but keeps the running state | Ref `recursive`: on a non-finite close reset the state (count and value) so the next finite stretch re-seeds with the SMA of its first `period` closes. Test: gap in the middle. |
| Hamming MA (`ma_hamming.rs`) | EDGE | `period < 2` → all `None` | Ref `math.ts` 433–434: `period == 1` returns the close. |
| MA Channel (`ma_channel.rs`) | DEFAULT | Offset 1% | Default **2.5%** (ref `averages.ts` 306–315). |
| Chandelier Exit (`chandelier_exit.rs`) | DEFAULT (missing input) | No ratchet | Add input "Ratchet stops" (toggle, default off). Ref `bands.ts` 256–267: when on, `long[i] = basicLong finite ? (prevClose > prevLong ? max(basicLong, prevLong) : basicLong) : None`, `short[i] = basicShort finite ? (prevClose < prevShort ? min(basicShort, prevShort) : basicShort) : None`, where `prevLong`/`prevShort` are the previous outputs (or the basic value when the previous output is missing) and `prevClose` is the previous bar's close (the current close on bar 0). Also: ATR here must be Wilder (`rma`), check it. |
| Standard Error, Standard Error Bands (`standard_error.rs`, `standard_error_bands.rs`) | EDGE | `n == 2` gives SE 0 | Period minimum **3**; residual SE `sqrt(RSS/(n-2))` stays. |
| Anchored VWAP (`anchored_vwap_study.rs`) | EDGE | Cumulative volume 0 → outputs the typical price | Output `None` while cumulative volume is 0. Add a test that changing the anchor input recalculates from the new anchor (no stale sums). |
| ALMA, SWMA, Guppy, Acceleration Bands, VWAP bands, MA presets (`ma_combination.rs`), Visible Average Price, Higher Interval Candles | keep | — | Only fix the ALMA module doc formula to `s = period / sigma` (comment matches the code and `math.ts` 353–368). |
| Linear Regression Channel (`linear_regression_channel.rs`) | keep | Fits the last `period` bars only | This is the standard channel (drawn over the last window, not rolling). Keep it. |
| Majority Rule (`majority_rule.rs`) | keep | A `period` window counts `period-1` moves | No single standard; add a doc comment stating the count. |

### 2.2 Momentum

| Study (file) | Severity | Now | Fix |
|---|---|---|---|
| Schaff Trend Cycle (`schaff_trend.rs`) | WRONG + DEFAULT | Flat window → 50; smoothing starts at 0 (`state.smooth = [0, 0]`), so early values are pulled down; fixed factor 0.5; output gated by `count < macd_long + cycle` with a `pending` back-fill; final clamp | Ref `oscillators.ts` 644–676. `macd = ema(close, fast) - ema(close, slow)` (reference seed). `stochNorm(v)`: over the last `cycle` values, `pct = 100*(x - lo)/(hi - lo)` when `hi > lo`, otherwise **keep the previous `pct`** (undefined until the first non-flat window); output `pct`. `smoothStep(v)`: on the first finite input `s = x`, then `s = s + factor*(x - s)`. Result = `smoothStep(stochNorm(smoothStep(stochNorm(macd))))`. Add input "Smoothing factor" (0.01–1, default 0.5). Remove `pending`, the `count` gate and the clamp. Defaults 23/50/10 stay. |
| Squeeze Momentum (`squeeze_momentum.rs`) | WRONG | Keltner range = SMA of true range; momentum = linreg(close − SMA(HL2)) | Ref `oscillators.ts` 993–1060. `basis = sma(close, len)`, `dev = bbMult * stdev(close, len)`, `kcRange = kcMult * atr(len)` (Wilder), squeeze on when `dev < kcRange`, `anchor = ((highest(high, len) + lowest(low, len))/2 + basis)/2`, `momentum = linreg(close - anchor, len, 0)`. Defaults: length 20 for all three existing period inputs, BB mult 2, KC mult 1.5. Add a "Squeeze fire" marker (an `IndicatorMarker` on the zero line, coloured by the momentum side) on the first bar where squeeze goes from on to off, behind a toggle "Mark squeeze fires" (default on). |
| Pretty Good Oscillator (`pretty_good_oscillator.rs`) | WRONG | `(close − SMA(close, n)) / SMA(TR, n)` | Mark Johnson's definition: `(close − SMA(close, n)) / EMA(TR, n)` with the reference EMA seed. Test on a 20-bar series. |
| True Strength Index (`true_strength.rs`) | WRONG + DEFAULT | All EMAs start at the first value (`seed = 1`) and the output is hidden for `long + short + 1` bars; signal default 7 | Ref `oscillators.ts` 537–567: `numer = ema(ema(change, long), short)`, `denom = ema(ema(|change|, long), short)`, `tsi = 100*(denom == 0 ? 0 : numer/denom)`, `signal = ema(tsi, signal)`, all with the reference seed (`Smoothing::next(x, n, 2/(n+1))`). Remove the manual gate (the seeds produce the warm-up). Signal default **13**. |
| Connors RSI (`connors_rsi.rs`) | EDGE | Percent rank counts prior ROC values `< current` | Ref `math.ts` 517–536: count prior values `<= current` over the previous `rank_period` values; if any of those is not finite, no value. The streak RSI is already correct (RSI of the streak series). |
| Stochastic (`stochastic.rs`) | EDGE | Raw %K clamped to 0–100 before smoothing | Remove the clamp (ref `oscillators.ts` 133–135 does not clamp). |
| Stochastic RSI (`stochastic_rsi.rs`) | EDGE | Rows with %K but %D = NaN | Keep the numbers; emit %D as missing (`f64::NAN` already) but make sure the legend shows "∅" for %D only, not for the row. No formula change. |
| Elder Impulse (`elder_impulse.rs`) | EDGE | Colours start at `max(slow, ema) + signal` (bar 39 by default) | Start colouring at the first bar where both the EMA slope and the MACD histogram are defined, using the same MACD values as `MACD` (reuse its calculation instead of a second copy). |
| Qstick (`trend_intensity.rs`, `Qstick`) | DEFAULT | Period 8 | Default **14** (Chande). |
| Fisher Transform, Center of Gravity, KDJ, SMI (Blau), Momentum, VW-MACD, Rainbow, Trend Intensity Index, Trend Detection Index, Relative Vigor Index, ADX Breakout, Knoxville, Technical Ratings, Ultimate Momentum, Trend Strength Index, Double Stochastic | keep | — | Checked against the source: Fisher's `0.33 * 2(x-0.5)` equals the reference `0.66(x-0.5)`; COG divides by the sum of prices as Ehlers does. Ultimate Momentum and Trend Strength Index are their own studies (not Williams' Ultimate Oscillator or Blau's TSI); leave the names. |

### 2.3 Volume and volatility

| Study (file) | Severity | Now | Fix |
|---|---|---|---|
| Klinger Oscillator (`klinger.rs`, `update` ~103–162) | WRONG | Equal H+L+C keeps the previous trend; no value until the first up/down bar; force uses `|2*(dm/cm - 1)|` | Ref `volume.ts` 274–293: `trend = hlc > prevHlc ? 1 : -1` (prevHlc = hlc on bar 0, so bar 0 is −1); `dm = high - low`; `cm = (i > 0 && trend == prevTrend) ? cm + dm : prevDm + dm`; `force = volume finite ? (cm == 0 ? 0 : volume * |2*(dm/cm) - 1| * trend * 100) : NaN`; `kvo = ema(force, 34) - ema(force, 55)`; `signal = ema(kvo, 13)`; reference seeds. Two variants of the force term are in print; use the reference one so the study matches the rest of the catalogue. Test: 6 hand bars including a tie bar. |
| PVT Enhanced (`price_volume_trend_study.rs`) | WRONG + DEFAULT | Signal = EMA(9) started at 0 | Ref `volume.ts` 415–446: inputs "Signal type" SMA/EMA (default **SMA**), "Signal length" (default **21**); signal = `sma(pvt, n)` or `ema(pvt, n)` with the reference seed. Keep the study name (saved layouts use it). |
| Volume Price Trend (`volume_price_trend.rs`, `VolumePriceTrend`) and PVT Enhanced line | EDGE | Bar 0 = 0.0 | Ref `volume.ts` 430–436: bar 0 has no value; each later bar adds `prevClose == 0 ? 0 : (close - prevClose)/prevClose * volume`; the running sum starts at the first finite term (`math.ts` `cumSum` 539–552). |
| NVI, PVI (`volume_price_trend.rs`) | DEFAULT (missing inputs) | Base 1000 and signal 255 fixed | Add inputs "Base" (default 1000) and "Signal length" (default 255), as in ref `volume.ts` 365–367 and 394–396. |
| Volume Oscillator (`volume_oscillator.rs`) | EDGE | Slow EMA 0 → `None` | Output **0** (ref `volume.ts` 465). |
| Ease of Movement (`ease_of_movement.rs`) | EDGE | Zero-volume bars are skipped, so the SMA window shifts | Ref `volume.ts` 205–232: a zero-volume bar contributes raw `0` and counts in the window. |
| Historical Volatility (`historical_volatility.rs`) | DEFAULT | Length 20 | Default **10** (ref `volatility.ts` 55). Keep the sample (n−1) deviation and the "trading days" input. |
| OHLC Volatility / Garman–Klass (`ohlc_volatility.rs`) | DEFAULT (missing input) | Annualises with a fixed 252 | Add input "Periods per year" (default 252, like HV's trading days). |
| Zero Trend Vol (`zero_trend_vol.rs`) | DEFAULT (missing input) | Fixed 252 | Same input as Garman–Klass. |
| Market Facilitation Index (`market_facilitation.rs`) | EDGE | Comment says the divisor is "typically 1000000", default is 1 | Fix the comment to match the default 1. No formula change. |
| Volume Rate of Change (`vroc.rs`) | keep | Past volume 0 → no value | No standard rule; keep. |
| Accumulation Swing Index, Williams A/D, Volume, Up/Down Volume, Volume 24h, Volume profiles, TPO, Performance, 52 Week studies | keep | — | Match their documented definitions. |

---

## 3. Active but empty studies

### 3.1 Why they are empty (evidence)

1. **Missing host data.** These studies need data other than the chart's bars:
   - Advance/Decline Line, Advance/Decline Ratio, Cumulative Volume Index
     (`market_breadth.rs` `data_requests`, symbols `ADVANCING_ISSUES` / `DECLINING_ISSUES` …).
   - Up/Down Volume (a lower-timeframe series, `up_down_volume.rs` `data_requests`).
   - Volume profiles that use a finer interval (`volume_profile.rs` `data_requests`).
   - Every order-flow study (`OrderFlowStudy`, `order_flow_input`).
   - Ratio, Spread and Correlation Coefficient only when a symbol input is set.

   The demo never calls `Chart::set_study_data` and its feed does not serve those symbols, so the
   values stay missing.
2. **Not enough history.**
   - Pring's Special K starts at bar 724, and its signal at bar 922 (`special_k.rs` doc).
   - 52 Week studies need 364 days.
   - Seasonality needs several years.
   - Performance needs daily closes from weeks, months or a year back.
   - The NVI/PVI signal is a 255-bar EMA.

   The demo loads about 7 days of synthetic 1-minute bars, or 1000 Binance candles
   (`binance/types.rs` `HISTORY_LIMIT`).
3. **Nothing tells the user.** The legend keeps the title bright and prints `NO_VALUE` for every
   line (`legend/mod.rs` ~502–516). The pane falls back to a 0–100 scale (`indicator_pane.rs`
   ~963). Nothing says why the study is empty. A study hidden on the current interval also only
   looks dimmed (its title at `gamma_multiply(0.5)`), with no reason given.

The gradient shading is **not** a cause. When the shade cache is empty the line falls back to its
plot colour.

### 3.2 Study status API (new)

Add to `src/studies/indicator_trait.rs`:

```rust
/// Why an applied study shows no values, for the legend and the pane.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum StudyStatus {
    /// Waiting for a history or stream the host has not supplied yet (symbol, metric or "trades").
    WaitingForData { what: String },
    /// The host reported an error for a requested history.
    DataError { message: String },
    /// Needs more bars than are loaded.
    NeedsBars { needed: usize, loaded: usize },
    /// Needs a calendar span of history (e.g. 364 days) that is not loaded.
    NeedsHistory { days: u32 },
    /// Hidden on this interval by its visibility settings.
    HiddenOnInterval,
}
```

Trait method, default `None`:

```rust
/// Bars of history before the first value, when it is fixed by the inputs.
fn required_bars(&self) -> Option<usize> { None }
/// Calendar days of history before the first value, when the study needs a time span.
fn required_days(&self) -> Option<u32> { None }
```

Implement `required_bars` for every study whose warm-up is longer than 200 bars with default
inputs. At least Pring's Special K (725, and the signal later), NVI/PVI (1 for the index), and
Know Sure Thing / Coppock if over 200. Compute it from the inputs, not a constant.

Implement `required_days` for 52 Week High/Low (364), 52 Week High/Low Levels (364), Seasonality
(730, two full years), and Performance (365 for the 1Y column).

`Chart::study_status(id: SeriesId) -> Option<StudyStatus>` in `src/widget/studies_data.rs`. Check
in this order and return the first that applies:

1. `!study.is_visible()` because of `hidden_intervals` / `interval_range` (not the eye toggle)
   → `HiddenOnInterval`.
2. For each `study_data_requests()` entry of this study:
   - If `study_data_error(request)` is `Some` → `DataError` with the error's display text.
   - Else, if no history is stored for it → `WaitingForData`, with `what` set to the symbol, or the
     metric id, or `"{interval} bars"` for a timeframe-only request.
3. The study has an order-flow input and no trades or book updates have arrived → `WaitingForData`
   with `what` = `"trades"`.
4. The study has no finite value in any of its lines over the loaded bars:
   - `required_days` is set and the loaded span is shorter → `NeedsHistory`.
   - Else, if `required_bars` is set and is greater than the loaded bar count → `NeedsBars`.
5. Otherwise `None`.

Cache the result per study, and recompute it only when data, inputs or settings change. The legend
calls it every frame.

### 3.3 Legend

In `src/chart/renderers/legend/mod.rs`, for a study row with `Some(status)`:

- **Title:** keep full colour for every status except `HiddenOnInterval`. An active study must not
  look switched off.
- **Values:** replace the value texts with one status text in `theme.ui.text_secondary`, same font
  size, truncated with "…" to the remaining width:
  - `WaitingForData`: "Waiting for {what}"
  - `DataError`: "No data: {message}"
  - `NeedsBars`: "Needs {needed} bars ({loaded} loaded)"
  - `NeedsHistory`: "Needs {days} days of history"
  - `HiddenOnInterval`: "Hidden on this interval"
- **Hover:** the full text appears as a tooltip on the status text.
- **Localisation:** all strings go through `l10n::text`. Add the keys to the locale files the
  other legend strings use.

### 3.4 Pane

In `src/widget/indicator_pane.rs`, when a non-overlay study's pane has a status (or no finite
value in the visible range):

- Draw the same status text centred in the pane, in `theme.ui.text_secondary`.
- Do not draw price-axis tick labels for the fallback 0–100 range. Grid lines may stay.

The pane keeps its height, so adding data later does not move the layout.

### 3.5 Demo

In `examples/demo` (keep the library generic):

- When a study requests the breadth symbols (`ADVANCING_ISSUES`, `DECLINING_ISSUES`,
  `ADVANCING_VOLUME`, `DECLINING_VOLUME`), supply deterministic synthetic bars for them through
  `Chart::set_study_data`:
  - Use the chart's bar times.
  - Use positive values that move with the main series, e.g. advancing = 1500 + 500·sin(i/37),
    declining = 3000 − advancing.

  Use the same mechanism for any other non-chart symbol the demo cannot serve. Leave it unserved,
  so that the legend shows "Waiting for SYMBOL", which is the intended behaviour.
- Leave Up/Down Volume and the order-flow studies to the feed. If the feed does not deliver, the
  status text explains it.

No change to the terminal crate is required. It gets the status texts for free.

---

## 4. Gradient follow-ups (from `docs/indicator-gradients.md`)

Commit `d68d54c` did not finish these items. Do them, then set that document's status line to
"implemented".

### 4.1 Colours and levels

| Study | Missing | Fix |
|---|---|---|
| MACD | Line colours | Default colours: MACD line `palette::INFO`, Signal `palette::AMBER` (`macd.rs` constructor `colors`). |
| Stochastic | Line colours | %K `INFO`, %D `AMBER`. |
| Bollinger Bands | Line colours | Upper and lower `INFO`, basis `AMBER`. |
| Donchian Channels | Line colours | Upper `DOWN`, middle `MUTED`, lower `UP`. |
| SMI Ergodic Indicator | Signal colour, zero level | Signal `AMBER`. Zero level dashed at `transp(MUTED, 50)`. |
| Klinger | Signal colour | Signal `AMBER` (ref `volume.ts` 272). |
| Fisher Transform | Levels | Add pane levels +1.5 and −1.5 dashed, 0 dotted. `Spread` shading does not replace levels, so put them in `FisherTransform::pane()` and make `level_style` return `Dotted` for 0. |
| PPO, PVO | Histogram colours | Remove their `plot_color` that calls `macd::histogram_color`. Colour columns by the side of zero: `transp(side ink, 40)` while the histogram moves away from zero, `transp(side ink, 70)` while it moves back. Here side ink is `UP` above zero and `DOWN` below. Implement this as a new `ShadingKind::SpreadColumns { line }`, with a coverage test row. |
| ADX, DMI | Key level input | Add input "Key level" (default 25). `shading()` reads it, and the pane level follows it. |
| Ulcer Index | Threshold input, levels | Add input "Stress threshold" (default 5). `shading()` uses `Threshold(self.threshold)`. Pane levels: the threshold dashed in `levels`, and 0 dotted in `transp(MUTED, 50)`. |
| Schaff Trend Cycle | Midline | The pane has 75/25 dashed. Add 50 dotted. Ref colours the line `bull` above 50 and `bear` at or below 50; keep that. |

### 4.2 Colours that follow the line

BBW, Commodity Selection Index and Chaikin Volatility use `Color32::PLACEHOLDER` in `bull`/`bear`
to mean "use the line colour". The settings dialog then shows that placeholder as a real colour.
Replace this with an explicit field on `StudyShading`:

```rust
/// Which sides of a centred wash use the plot's own colour instead of `bull` / `bear`.
#[serde(default)]
pub line_ink: LineInk, // enum LineInk { #[default] Never, Bear, Both }
```

`BBW` and `CSI` use `Both`, and `ChaikinVolatility` uses `Bear`. Remove every `PLACEHOLDER` use
from `shading.rs` and the studies. The dialog hides the colour rows that `line_ink` replaces.

### 4.3 Settings dialog (spec section 5 of the gradient document, still open)

- **Shared helpers:** move `section_header`, `setting_row`, `styled_dropdown`, `styled_checkbox`
  and `line_style_preview` from `chart_settings/tabs/mod.rs` to `src/ui_kit/settings_rows.rs`.
  Use them in both dialogs. Chart settings must look the same, and its tests must pass.
- **Colour pickers:** replace every `color_edit_button_srgba` in the indicator dialog with
  `ui_kit::ColorPicker`:
  - Colour inputs on the Inputs tab.
  - The SHADING rows.

  Keep one `ColorPickerState` per row, keyed by input id or shading field.
- **Inputs tab:** use the section header "INPUTS" and one `setting_row` per input.
- **SHADING rows:** name the colour rows per kind, as in section 5.4 of the gradient document.
  Today only Bounded is named.
- **SHADING controls:**
  - "Gradient fill" only for kinds that have a wash.
  - A "Zones" transparency `DragValue` (0–100) for `Bounded`.
  - Level `DragValue`s for `BandOnly` and `Centered`, as well as `Bounded`.
- **Validation error:** `StudyShading::validate` returns English text. Show it with the shared
  `status_message` helper in the footer, through `l10n::text`, instead of a plain label.
- **Defaults button:** also resets the inputs to their defaults. Today it resets only the
  presentation.
- **Range dropdown:** disabled interval checkboxes get the hover text "Outside the selected
  range".

### 4.4 Coverage test

`every_factory_study_is_listed` in `src/studies/shading.rs` only checks whether `shading()` is
`Some`. Change it to the version in the gradient document's test 14:

- Map every name to the expected `ShadingKind` variant.
- Compare with `std::mem::discriminant`.
- Add the new `SpreadColumns` variant.
- Move it to `src/studies/factory/tests.rs`.
- Keep the rule: no `_ =>` arm.

---

## 5. Tests

Besides the per-row unit tests of section 2:

1. **Factory smoke test** (`src/studies/factory/tests.rs`). For every name in
   `IndicatorFactory::new().list()`:
   - Create the study and calculate it on a deterministic 1500-bar 1-minute series (the same
     generator as `reference.rs` fixtures if possible).
   - Then `update(data, data.len() - 1)` and `update(data, data.len() - 20)` after changing the
     last 20 bars.
   - Assert that incremental and full calculation give the same values (bit-for-bit or within
     1e-9 relative).
   - Assert that no value is ±infinity.
   - Assert that the study either has at least one finite value, or reports `required_bars`,
     `required_days` or data requests or an order-flow input that explain why not.

   No `_ =>` escape: list the studies expected to be empty on this fixture by name.
2. **`study_status`:**
   - Breadth study without data → `WaitingForData("ADVANCING_ISSUES")`.
   - After `set_study_data` → `None`.
   - Special K on 500 bars → `NeedsBars { needed: 725, loaded: 500 }`.
   - Study with `interval_range = Only(1h)` on 1m → `HiddenOnInterval`.
3. **Legend:**
   - A waiting study shows "Waiting for …" and its title keeps full colour.
   - A hidden-on-interval study shows "Hidden on this interval" with the dimmed title.
4. **Pane:** an empty pane draws the status text and no 0–100 tick labels.
5. **Gradient follow-ups:**
   - PPO/PVO column colours on a hand series.
   - Ulcer and ADX inputs move the level.
   - Fisher levels are drawn.
   - Saved settings with the old `PLACEHOLDER` colours still load. Treat a saved `PLACEHOLDER`
     as `line_ink` `Both` or `Bear`.
6. Update existing tests whose expected numbers change because of section 2. Recompute the
   expected values from the source formula, never from the new code.

---

## 6. Work order

1. **Section 2, table by table:** moving averages, momentum, then volume and volatility. One
   commit per table. Run `cargo test --lib` after each.
2. **Factory smoke test (5.1).** Fix every incremental mismatch it finds. If a mismatch is in a
   study this document marks keep, fix the incremental path only, not the formula.
3. **Status API, legend, pane (section 3), then the demo data (3.5).**
4. **Gradient follow-ups (section 4)**, then the stricter coverage test.
5. **Full checks:** `cargo fmt --check`, both clippy configurations
   (`--all-features --all-targets -- -D warnings` and `--no-default-features --all-targets -- -D
   warnings`), `cargo test --lib`, `cd examples/demo && cargo test`, and
   `cd terminal && cargo check --tests`.
6. **Release demo rebuild** (`cd examples/demo && trunk serve --release --address 127.0.0.1 --port
   8081`), then check by hand:
   - McGinley, ZLEMA, VIDYA and KAMA follow price without a large lag at the start.
   - Schaff starts without the early dip.
   - The Squeeze histogram and dots, with the fire cross.
   - Klinger on flat bars.
   - Breadth studies show data in the demo.
   - Special K on 1000 Binance bars shows the line but not yet the signal.
   - A study hidden by "Only 1h" on 1m says so in the legend.
   - The settings dialog for BBW has no magenta swatches.

## Constraints

- **Code style:** follow the existing style. Write comments only for constraints the code cannot
  show. Every visible string goes through `l10n::text`.
- **Product names:** no third-party product names in code, comments, tests, docs or commit
  messages. Before committing, this must print nothing:
  ```bash
  git diff | grep -iE "^\+.*(trading""view|v""ela)" | grep -v WaveLabel
  ```
- **Saved layouts:** do not rename studies. Saved layouts and templates refer to them by name
  and input id.
- **Commits:** no `Co-authored-by` trailer in commit messages.
