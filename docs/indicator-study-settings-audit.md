# Audit katalog: dialog pengaturan study

Artefak ini dihasilkan dari kode, bukan ditulis tangan. Regenerasi dengan:

```bash
cargo test --lib catalogue_audit_matrix -- --ignored --nocapture
```

Sumber: `IndicatorFactory::new().list()` dikombinasikan dengan `ConfiguredStudy::inputs`,
`line_names`, `shading`, dan `is_overlay` dari commit kerja saat ini.

- Total nama terdaftar: **201**
- Overlay: **86**, pane: **115**
- Kategori: **16**
- Study tanpa input study: **16** (wrapper lintas-chart tetap ada dan tidak ditampilkan)
- Sebaran shading: Centered 35, Bounded 9, Slope 7, BandOnly 6, Intensity 5,
  Channel 4, PriceSide 4, Spread 3, RisingColumns 3, MomentumColumns 3,
  AdxInk 2, FlatChannel 2, Regime 2, tanpa shading 116.

Kolom: nama terdaftar | kategori | penempatan | input study yang tampil (setelah
menyaring wrapper `plot_offset`, `calculation_symbol`, `calculation_timeframe`,
`wait_for_close`) | field wrapper yang tidak ditampilkan | nama plot | jenis shading.

| Study | Kategori | Penempatan | Input tampil | Wrapper disembunyikan | Plot | Shading |
|---|---|---|---|---|---|---|
| %B | Volatility | pane | `period`, `multiplier`, `source` | `plot_offset`, `calculation_symbol`, `calculation_timeframe`, `wait_for_close` | `%B(20,2)` | Centered |
| 24-hour Volume | Volume | pane |  | `calculation_timeframe`, `wait_for_close` | `24-hour Volume` | none |
| 52W HL% | Price and sessions | pane |  | `calculation_timeframe`, `wait_for_close` | `52W HL%` | none |
| 52W High/Low | Pivots and levels | overlay |  | `calculation_timeframe`, `wait_for_close` | `52W High`, `52W Low` | none |
| A/D | Volume | pane |  | `calculation_timeframe`, `wait_for_close` | `A/D Line` | Slope |
| ADX | Trend | pane | `period` | `calculation_timeframe`, `wait_for_close` | `ADX(14)`, `+DI`, `-DI` | AdxInk |
| ADX Breakout | Trend | pane | `period`, `threshold`, `range` | `calculation_timeframe`, `wait_for_close` | `ADX`, `Upward Breakout`, `Downward Breakout` | none |
| ALMA | Moving averages | overlay | `period`, `offset`, `sigma`, `source` | `plot_offset`, `calculation_symbol`, `calculation_timeframe`, `wait_for_close` | `ALMA(9, 0.85, 6)` | none |
| AO | Momentum | pane | `fast_period`, `slow_period`, `source` | `plot_offset`, `calculation_symbol`, `calculation_timeframe`, `wait_for_close` | `AO(5, 34)` | RisingColumns |
| ASI | Statistics and regression | pane | `limit_move` | `calculation_timeframe`, `wait_for_close` | `ASI` | Slope |
| ATR | Volatility | pane | `period` | `calculation_timeframe`, `wait_for_close` | `ATR(14)` | none |
| AVWAP | VWAP and average price | overlay | `anchor_bar`, `source` | `plot_offset`, `calculation_symbol`, `calculation_timeframe`, `wait_for_close` | `AVWAP(0)` | PriceSide |
| AccBands | Bands and channels | overlay | `period`, `factor` | `calculation_timeframe`, `wait_for_close` | `Upper(20)`, `Middle`, `Lower(20)` | Channel |
| Accelerator Oscillator | Momentum | pane | `source` | `plot_offset`, `calculation_symbol`, `calculation_timeframe`, `wait_for_close` | `AC` | RisingColumns |
| Advance/Decline Line | Market breadth | pane | `advancing_symbol`, `declining_symbol`, `timeframe` | `wait_for_close` | `Advance/Decline Line` | Slope |
| Advance/Decline Ratio | Market breadth | pane | `advancing_symbol`, `declining_symbol`, `timeframe` | `wait_for_close` | `Advance/Decline Ratio` | none |
| Advance/Decline Ratio (Bars) | Market breadth | pane | `period` | `calculation_timeframe`, `wait_for_close` | `A/D Ratio(9)` | none |
| Aggressor Rate | Order flow | pane | `seconds`, `instant` |  | `Bar orders`, `Rolling / second`, `Instant / second` | none |
| All Candlestick Patterns | Patterns | overlay | `trend`, `pattern_type`, `patterns 0`, `patterns 1`, `patterns 2`, `patterns 3`, `patterns 4`, `patterns 5`, `patterns 6`, `patterns 7`, `patterns 8`, `patterns 9`, `patterns 10`, `patterns 11`, `patterns 12`, `patterns 13`, `patterns 14`, `patterns 15`, `patterns 16`, `patterns 17`, `patterns 18`, `patterns 19`, `patterns 20`, `patterns 21`, `patterns 22`, `patterns 23`, `patterns 24`, `patterns 25`, `patterns 26`, `patterns 27`, `patterns 28`, `patterns 29`, `patterns 30`, `patterns 31`, `patterns 32`, `patterns 33`, `patterns 34`, `patterns 35`, `patterns 36`, `patterns 37`, `patterns 38`, `patterns 39`, `patterns 40`, `patterns 41`, `patterns 42`, `patterns 43` | `calculation_timeframe`, `wait_for_close` | `Bullish`, `Bearish`, `Neutral` | none |
| Aroon | Trend | pane | `period` | `calculation_timeframe`, `wait_for_close` | `Aroon Up(14)`, `Aroon Down(14)` | none |
| Aroon Oscillator | Trend | pane | `period` | `calculation_timeframe`, `wait_for_close` | `Aroon Osc(14)` | Centered |
| Auto Fibonacci Extension | Pivots and levels | overlay | `deviation_percent`, `depth` | `calculation_timeframe`, `wait_for_close` | `Auto Fibonacci Extension` | none |
| Auto Fibonacci Retracement | Pivots and levels | overlay | `deviation_percent`, `depth` | `calculation_timeframe`, `wait_for_close` | `Auto Fibonacci Retracement` | none |
| Auto Parallel Channel | Pivots and levels | overlay | `deviation_percent`, `depth` | `calculation_timeframe`, `wait_for_close` | `Auto Parallel Channel` | none |
| Auto Pitchfork | Pivots and levels | overlay | `deviation_percent`, `depth` | `calculation_timeframe`, `wait_for_close` | `Auto Pitchfork` | none |
| Auto Trend Line | Pivots and levels | overlay | `deviation_percent`, `depth` | `calculation_timeframe`, `wait_for_close` | `Auto Trend Line` | none |
| Average Daily Range | Volatility | pane | `period` | `calculation_timeframe`, `wait_for_close` | `ADR(14)` | none |
| BB | Bands and channels | overlay | `period`, `basis`, `std_dev`, `source` | `plot_offset`, `calculation_symbol`, `calculation_timeframe`, `wait_for_close` | `BB Upper(20, 2)`, `BB Middle(20, 2)`, `BB Lower(20, 2)` | FlatChannel |
| BBTrend | Volatility | pane | `short_length`, `long_length`, `multiplier`, `source` | `plot_offset`, `calculation_symbol`, `calculation_timeframe`, `wait_for_close` | `BBTrend(20, 50, 2)` | none |
| BBW | Volatility | pane | `period`, `std_dev`, `expansion`, `contraction`, `source` | `plot_offset`, `calculation_symbol`, `calculation_timeframe`, `wait_for_close` | `BBW(20, 2)`, `Highest Expansion(125)`, `Lowest Contraction(125)` | Centered |
| BoP | Momentum | pane | `smooth_period` | `calculation_timeframe`, `wait_for_close` | `BoP(14)` | Centered |
| Bollinger Bars | Price and sessions | overlay |  | `calculation_timeframe`, `wait_for_close` | `Wicks`, `Up`, `Down` | none |
| Book Size | Order flow | pane | `levels`, `distance`, `min_size`, `min_age`, `max_age` |  | `Bid size`, `Ask size`, `Difference` | none |
| Bull Bear Power | Momentum | pane | `period` | `calculation_timeframe`, `wait_for_close` | `BBP(13)` | none |
| CCI | Oscillators | pane | `period`, `source` | `plot_offset`, `calculation_symbol`, `calculation_timeframe`, `wait_for_close` | `CCI(20)` | Centered |
| CHOP | Trend | pane | `period` | `calculation_timeframe`, `wait_for_close` | `CHOP(14)` | Bounded |
| CMF | Volume | pane | `period` | `calculation_timeframe`, `wait_for_close` | `CMF(20)` | Centered |
| CMO | Oscillators | pane | `period`, `source` | `plot_offset`, `calculation_symbol`, `calculation_timeframe`, `wait_for_close` | `CMO(14)` | Centered |
| COG | Oscillators | pane | `period`, `source` | `plot_offset`, `calculation_symbol`, `calculation_timeframe`, `wait_for_close` | `COG(10)`, `Signal` | Spread |
| CSI | Volatility | pane | `period`, `point_value`, `margin`, `commission` | `calculation_timeframe`, `wait_for_close` | `CSI(14)` | Centered |
| CV | Volatility | pane | `ema_period`, `roc_period` | `calculation_timeframe`, `wait_for_close` | `CV(10,10)` | Centered |
| Chaikin Oscillator | Volume | pane | `fast_length`, `slow_length` | `calculation_timeframe`, `wait_for_close` | `Chaikin Osc(3, 10)` | Centered |
| Chande Kroll Stop | Stops and trailing | overlay | `atr_length`, `multiplier`, `stop_length` | `calculation_timeframe`, `wait_for_close` | `Stop Short`, `Stop Long` | none |
| Chandelier | Stops and trailing | overlay | `period`, `multiplier`, `ratchet` | `calculation_timeframe`, `wait_for_close` | `Long Exit(22,3)`, `Short Exit(22,3)` | none |
| Chop Zone | Trend | pane |  | `calculation_timeframe`, `wait_for_close` | `Chop Zone` | none |
| Connors RSI | Oscillators | pane | `rsi_period`, `streak_period`, `rank_period`, `source` | `plot_offset`, `calculation_symbol`, `calculation_timeframe`, `wait_for_close` | `CRSI(3,2,100)` | Bounded |
| Continuation Chart Patterns | Patterns | overlay | `strength`, `min_length`, `max_length`, `tolerance`, `touches`, `pole`, `patterns0`, `patterns1`, `patterns2`, `patterns3`, `patterns4`, `patterns5`, `patterns6`, `patterns7` | `calculation_timeframe`, `wait_for_close` | `Bullish`, `Bearish` | none |
| Coppock | Momentum | pane | `long_roc`, `short_roc`, `wma_period`, `source` | `plot_offset`, `calculation_symbol`, `calculation_timeframe`, `wait_for_close` | `Coppock(14,11,10)` | Centered |
| Correlation Coefficient | Statistics and regression | pane | `symbol_0`, `source_0`, `symbol_1`, `source_1`, `length`, `log_returns` | `calculation_timeframe`, `wait_for_close` | `Correlation Coefficient` | Centered |
| Cumulative Volume Index | Market breadth | pane | `advancing_symbol`, `declining_symbol`, `timeframe` | `wait_for_close` | `Cumulative Volume Index` | Slope |
| DEMA | Moving averages | overlay | `period`, `source` | `plot_offset`, `calculation_symbol`, `calculation_timeframe`, `wait_for_close` | `DEMA(20)` | none |
| DMI | Trend | pane | `period` | `calculation_timeframe`, `wait_for_close` | `+DI(14)`, `-DI(14)`, `ADX(14)` | AdxInk |
| DPO | Momentum | pane | `period`, `centered`, `source` | `plot_offset`, `calculation_symbol`, `calculation_timeframe`, `wait_for_close` | `DPO(21)` | Centered |
| DS | Oscillators | pane | `period`, `smooth` | `calculation_timeframe`, `wait_for_close` | `DS(10,3)` | BandOnly |
| Donchian | Bands and channels | overlay | `period` | `calculation_timeframe`, `wait_for_close` | `Middle(20)`, `Upper`, `Lower` | FlatChannel |
| EMA | Moving averages | overlay | `period`, `source` | `plot_offset`, `calculation_symbol`, `calculation_timeframe`, `wait_for_close` | `EMA(12)` | none |
| EMA Cross | Moving averages | overlay | `period_0`, `kind_0`, `source_0`, `period_1`, `kind_1`, `source_1` | `calculation_timeframe`, `wait_for_close` | `Fast EMA(12)`, `Slow EMA(26)` | none |
| Elder Ray | Momentum | pane | `period` | `calculation_timeframe`, `wait_for_close` | `Bull(13)`, `Bear(13)` | none |
| Elliott Wave | Patterns | overlay | `left`, `right`, `history` | `calculation_timeframe`, `wait_for_close` | `Developing Anchor`, `Confirmed Bullish Impulse`, `Confirmed Bearish Impulse`, `Wave` | none |
| Envelopes | Bands and channels | overlay | `period`, `percent`, `ma_type`, `source` | `plot_offset`, `calculation_symbol`, `calculation_timeframe`, `wait_for_close` | `Upper(+2.5%)`, `SMA(20)`, `Lower(-2.5%)` | Channel |
| EoM | Volume | pane | `period`, `divisor` | `calculation_timeframe`, `wait_for_close` | `EoM(14)` | Centered |
| Falling Wedge | Patterns | overlay | `left`, `right`, `touches`, `convergence`, `max_bars`, `history` | `calculation_timeframe`, `wait_for_close` | `Upper Line`, `Lower Line`, `Confirmed Breakout` | none |
| Fisher Transform | Oscillators | pane | `period`, `source` | `plot_offset`, `calculation_symbol`, `calculation_timeframe`, `wait_for_close` | `Fisher(9)`, `Signal` | Spread |
| Force Index | Volume | pane | `period`, `source` | `plot_offset`, `calculation_symbol`, `calculation_timeframe`, `wait_for_close` | `Force(13)` | Centered |
| GK Vol | Volatility | pane | `period`, `periods_per_year` | `calculation_timeframe`, `wait_for_close` | `GK Vol(20)` | Intensity |
| GMMA | Moving averages | overlay | `source` | `plot_offset`, `calculation_symbol`, `calculation_timeframe`, `wait_for_close` | `EMA(3)`, `EMA(5)`, `EMA(8)`, `EMA(10)`, `EMA(12)`, `EMA(15)`, `EMA(30)`, `EMA(35)`, `EMA(40)`, `EMA(45)`, `EMA(50)`, `EMA(60)` | none |
| Gaps | Pivots and levels | overlay | `close_partially`, `max_gaps`, `deviation`, `limit_trail`, `trail_length` | `calculation_timeframe`, `wait_for_close` | `Up gaps`, `Down gaps` | none |
| HMA | Moving averages | overlay | `period`, `source` | `plot_offset`, `calculation_symbol`, `calculation_timeframe`, `wait_for_close` | `HMA(20)` | none |
| HV | Volatility | pane | `period`, `trading_days`, `source` | `plot_offset`, `calculation_symbol`, `calculation_timeframe`, `wait_for_close` | `HV(10)` | Intensity |
| Hamming MA | Moving averages | overlay | `period`, `source` | `plot_offset`, `calculation_symbol`, `calculation_timeframe`, `wait_for_close` | `Hamming(20)` | none |
| Higher Interval Candles | Price and sessions | overlay | `interval`, `extended_hours` | `calculation_timeframe`, `wait_for_close` | `Up`, `Down` | none |
| Ichimoku | Trend | overlay | `tenkan_period`, `kijun_period`, `senkou_b_period`, `displacement` | `calculation_timeframe`, `wait_for_close` | `Tenkan(9)`, `Kijun(26)`, `Senkou A`, `Senkou B(52)`, `Chikou` | none |
| Impulse | Trend | pane | `ema_period`, `macd_fast`, `macd_slow`, `macd_signal`, `source` | `plot_offset`, `calculation_symbol`, `calculation_timeframe`, `wait_for_close` | `Impulse` | none |
| Intraday Pivot Points | Pivots and levels | overlay | `method`, `minutes` | `calculation_timeframe`, `wait_for_close` | `P`, `R1`, `S1`, `R2`, `S2`, `R3`, `S3`, `R4`, `S4`, `R5`, `S5` | none |
| KAMA | Moving averages | overlay | `period`, `fast_period`, `slow_period`, `source` | `plot_offset`, `calculation_symbol`, `calculation_timeframe`, `wait_for_close` | `KAMA(10, 2, 30)` | none |
| KDJ | Oscillators | pane | `period`, `k_smooth`, `d_smooth` | `calculation_timeframe`, `wait_for_close` | `K(9)`, `D(3)`, `J` | BandOnly |
| KST | Momentum | pane | `roc_periods 0`, `roc_periods 1`, `roc_periods 2`, `roc_periods 3`, `sma_periods 0`, `sma_periods 1`, `sma_periods 2`, `sma_periods 3`, `signal_period`, `source` | `plot_offset`, `calculation_symbol`, `calculation_timeframe`, `wait_for_close` | `KST`, `Signal(9)` | Centered |
| Keltner | Bands and channels | overlay | `ema_period`, `atr_period`, `multiplier` | `calculation_timeframe`, `wait_for_close` | `EMA(20)`, `Upper(2x ATR)`, `Lower(2x ATR)` | Channel |
| Key Levels | Pivots and levels | overlay | `left`, `right`, `min_touches`, `tolerance` | `calculation_timeframe`, `wait_for_close` | `Resistance`, `Support` | none |
| Klinger | Volume | pane | `short_period`, `long_period`, `signal_period` | `calculation_timeframe`, `wait_for_close` | `KVO(34,55)`, `Signal(13)` | Centered |
| Knoxville Divergence | Oscillators | pane | `momentum_period`, `rsi_period`, `left`, `right`, `lookback`, `oversold`, `overbought` | `calculation_timeframe`, `wait_for_close` | `Momentum`, `Bullish Divergence`, `Bearish Divergence` | none |
| LSMA | Moving averages | overlay | `period`, `offset`, `source` | `plot_offset`, `calculation_symbol`, `calculation_timeframe`, `wait_for_close` | `LSMA(25)` | none |
| Levels Executed | Order flow | pane | `seconds`, `reversal`, `min`, `max` |  | `Ask levels`, `Bid levels`, `Total levels` | none |
| Levels Traded Through | Order flow | pane | `seconds` |  | `Bid levels`, `Ask levels`, `Total levels` | none |
| LinReg | Statistics and regression | overlay | `period`, `source` | `plot_offset`, `calculation_symbol`, `calculation_timeframe`, `wait_for_close` | `LinReg(14)` | none |
| LinReg Channel | Bands and channels | overlay | `period`, `multiplier`, `extend`, `source` | `plot_offset`, `calculation_symbol`, `calculation_timeframe`, `wait_for_close` | `LinReg Upper(20, 2)`, `LinReg Middle(20)`, `LinReg Lower(20, 2)` | none |
| LinReg Slope | Statistics and regression | pane | `period`, `source` | `plot_offset`, `calculation_symbol`, `calculation_timeframe`, `wait_for_close` | `Slope(14)` | Centered |
| Lunar Phases | Price and sessions | overlay |  | `calculation_timeframe`, `wait_for_close` | `New Moon`, `Full Moon` | none |
| MA Channel | Bands and channels | overlay | `period`, `offset_percent`, `source` | `plot_offset`, `calculation_symbol`, `calculation_timeframe`, `wait_for_close` | `Upper(+2.5%)`, `MA(20)`, `Lower(-2.5%)` | Channel |
| MA Cross | Moving averages | overlay | `period_0`, `kind_0`, `source_0`, `period_1`, `kind_1`, `source_1` | `calculation_timeframe`, `wait_for_close` | `Fast SMA(9)`, `Slow SMA(21)` | none |
| MA Ribbon | Moving averages | overlay | `period_0`, `kind_0`, `source_0`, `period_1`, `kind_1`, `source_1`, `period_2`, `kind_2`, `source_2`, `period_3`, `kind_3`, `source_3`, `period_4`, `kind_4`, `source_4`, `period_5`, `kind_5`, `source_5`, `period_6`, `kind_6`, `source_6`, `period_7`, `kind_7`, `source_7`, `period_8`, `kind_8`, `source_8`, `period_9`, `kind_9`, `source_9`, `period_10`, `kind_10`, `source_10`, `period_11`, `kind_11`, `source_11` | `calculation_timeframe`, `wait_for_close` | `EMA(5)`, `EMA(10)`, `EMA(15)`, `EMA(20)`, `EMA(25)`, `EMA(30)`, `EMA(35)`, `EMA(40)`, `EMA(45)`, `EMA(50)`, `EMA(55)`, `EMA(60)` | none |
| MACD | Momentum | pane | `fast_period`, `slow_period`, `signal_period`, `source` | `plot_offset`, `calculation_symbol`, `calculation_timeframe`, `wait_for_close` | `MACD(12,26,9)`, `Signal`, `Histogram` | MomentumColumns |
| MD | Moving averages | overlay | `period`, `source` | `plot_offset`, `calculation_symbol`, `calculation_timeframe`, `wait_for_close` | `MD(14)` | none |
| MFI | Volume | pane | `period` | `calculation_timeframe`, `wait_for_close` | `MFI(14)` | Bounded |
| MFI BW | Volume | pane | `divisor` | `calculation_timeframe`, `wait_for_close` | `MFI` | none |
| MP | Price and sessions | overlay |  | `calculation_timeframe`, `wait_for_close` | `MP` | none |
| MajRule | Statistics and regression | pane | `period`, `source` | `plot_offset`, `calculation_symbol`, `calculation_timeframe`, `wait_for_close` | `MajRule(10)` | none |
| Mass Index | Volatility | pane | `ema_period`, `sum_period` | `calculation_timeframe`, `wait_for_close` | `Mass(9,25)` | BandOnly |
| Median | Bands and channels | overlay | `length`, `atr_length`, `multiplier` | `calculation_timeframe`, `wait_for_close` | `Median`, `Upper Band`, `Lower Band`, `Median EMA` | none |
| Missed Pivot Levels | Pivots and levels | overlay | `left`, `right`, `tolerance`, `max_levels` | `calculation_timeframe`, `wait_for_close` | `Missed high`, `Missed low` | none |
| Momentum | Momentum | pane | `period`, `source` | `plot_offset`, `calculation_symbol`, `calculation_timeframe`, `wait_for_close` | `MOM(10)` | Centered |
| Multi MA | Moving averages | overlay | `period_0`, `kind_0`, `source_0`, `period_1`, `kind_1`, `source_1`, `period_2`, `kind_2`, `source_2`, `period_3`, `kind_3`, `source_3` | `calculation_timeframe`, `wait_for_close` | `SMA(20)`, `SMA(50)`, `SMA(100)`, `SMA(200)` | none |
| Multi-Range Delta | Order flow | pane | `min0`, `max0`, `min1`, `max1`, `min2`, `max2` |  | `Delta 1`, `Delta 2`, `Delta 3` | none |
| NVI | Volume | pane | `base`, `signal_length`, `source` | `plot_offset`, `calculation_symbol`, `calculation_timeframe`, `wait_for_close` | `NVI`, `Signal` | Regime |
| Net Volume | Volume | pane | `source` | `plot_offset`, `calculation_symbol`, `calculation_timeframe`, `wait_for_close` | `Net Volume` | none |
| OBV | Volume | pane | `source` | `plot_offset`, `calculation_symbol`, `calculation_timeframe`, `wait_for_close` | `OBV` | none |
| OHLC4 | Price and sessions | overlay |  | `calculation_timeframe`, `wait_for_close` | `OHLC4` | none |
| PGO | Oscillators | pane | `period` | `calculation_timeframe`, `wait_for_close` | `PGO(14)` | Centered |
| PPO | Momentum | pane | `short_period`, `long_period`, `signal_period`, `oscillator_average`, `signal_average`, `source` | `plot_offset`, `calculation_symbol`, `calculation_timeframe`, `wait_for_close` | `PPO(12,26)`, `Signal(9)`, `Histogram` | Centered |
| PSAR | Stops and trailing | overlay | `initial_af`, `af_step`, `max_af` | `calculation_timeframe`, `wait_for_close` | `Up Trend`, `Down Trend` | none |
| PVI | Volume | pane | `base`, `signal_length`, `source` | `plot_offset`, `calculation_symbol`, `calculation_timeframe`, `wait_for_close` | `PVI`, `Signal` | Regime |
| PVO | Volume | pane | `short_period`, `long_period`, `signal_period`, `oscillator_average`, `signal_average` | `calculation_timeframe`, `wait_for_close` | `PVO(12,26)`, `Signal(9)`, `Histogram` | Centered |
| PVT Enhanced | Volume | pane | `signal_type`, `signal_period`, `source` | `plot_offset`, `calculation_symbol`, `calculation_timeframe`, `wait_for_close` | `PVT(21)`, `Signal` | Slope |
| Parkinson Vol | Volatility | pane | `period` | `calculation_timeframe`, `wait_for_close` | `Parkinson Vol(20)` | Intensity |
| Performance | Price and sessions | overlay |  | `calculation_timeframe`, `wait_for_close` | `1W`, `1M`, `3M`, `6M`, `YTD`, `1Y` | none |
| Pivot Points High Low | Pivots and levels | overlay | `high_left`, `high_right`, `low_left`, `low_right` | `calculation_timeframe`, `wait_for_close` | `Pivot High`, `Pivot Low` | none |
| Pivot Points Standard | Pivots and levels | overlay | `method`, `timeframe`, `pivots_back` | `calculation_timeframe`, `wait_for_close` | `P`, `R1`, `S1`, `R2`, `S2`, `R3`, `S3`, `R4`, `S4`, `R5`, `S5` | none |
| Price Momentum Oscillator | Momentum | pane | `first_length`, `second_length`, `signal_length`, `source` | `plot_offset`, `calculation_symbol`, `calculation_timeframe`, `wait_for_close` | `PMO(35,20)`, `Signal(10)` | Centered |
| Pring's Special K | Momentum | pane | `signal_lengths 0`, `signal_lengths 1`, `source` | `plot_offset`, `calculation_symbol`, `calculation_timeframe`, `wait_for_close` | `Special K`, `Signal(100,100)` | Centered |
| Projected Pivot Points | Pivots and levels | overlay | `method`, `timeframe`, `pivots_back` | `calculation_timeframe`, `wait_for_close` | `P`, `R1`, `S1`, `R2`, `S2`, `R3`, `S3`, `R4`, `S4`, `R5`, `S5` | none |
| Qstick | Trend | pane | `period` | `calculation_timeframe`, `wait_for_close` | `Qstick(14)` | Centered |
| Quoted Spread | Order flow | pane |  |  | `Last (ticks)`, `Mean (ticks)`, `Maximum (ticks)` | none |
| RCI Ribbon | Statistics and regression | pane | `lengths 0`, `lengths 1`, `lengths 2`, `source` | `plot_offset`, `calculation_symbol`, `calculation_timeframe`, `wait_for_close` | `Short RCI(10)`, `Middle RCI(30)`, `Long RCI(50)` | none |
| ROC | Momentum | pane | `period`, `source` | `plot_offset`, `calculation_symbol`, `calculation_timeframe`, `wait_for_close` | `ROC(14)` | Centered |
| RSI | Oscillators | pane | `period`, `source` | `plot_offset`, `calculation_symbol`, `calculation_timeframe`, `wait_for_close` | `RSI(14)` | Bounded |
| RSI Divergence Indicator | Oscillators | pane | `period`, `right`, `left`, `max_range`, `min_range`, `bullish`, `hidden_bullish`, `bearish`, `hidden_bearish` | `calculation_timeframe`, `wait_for_close` | `RSI`, `Regular Bullish`, `Hidden Bullish`, `Regular Bearish`, `Hidden Bearish`, `Regular Bullish Label`, `Hidden Bullish Label`, `Regular Bearish Label`, `Hidden Bearish Label` | Bounded |
| RVI | Oscillators | pane | `period` | `calculation_timeframe`, `wait_for_close` | `RVI(10)`, `Signal` | none |
| Rainbow | Oscillators | pane | `levels`, `base_period`, `source` | `plot_offset`, `calculation_symbol`, `calculation_timeframe`, `wait_for_close` | `Rainbow(10,2)` | none |
| RankCorr | Statistics and regression | pane | `period`, `source` | `plot_offset`, `calculation_symbol`, `calculation_timeframe`, `wait_for_close` | `RankCorr(14)` | Centered |
| Ratio | Statistics and regression | pane | `symbol_0`, `source_0`, `symbol_1`, `source_1` | `calculation_timeframe`, `wait_for_close` | `Ratio` | none |
| Relative Volatility Index | Volatility | pane | `length`, `source` | `plot_offset`, `calculation_symbol`, `calculation_timeframe`, `wait_for_close` | `RVI(10)` | Bounded |
| Relative Volume at Time | Volume | pane | `anchor`, `length`, `mode` | `calculation_timeframe`, `wait_for_close` | `RVOL(10)` | none |
| Reversal | Patterns | overlay | `lookback` | `calculation_timeframe`, `wait_for_close` | `Upward Reversal`, `Downward Reversal` | none |
| Reversal Chart Patterns | Patterns | overlay | `strength`, `min_length`, `max_length`, `tolerance`, `depth`, `handle`, `patterns0`, `patterns1`, `patterns2`, `patterns3`, `patterns4`, `patterns5`, `patterns6`, `patterns7` | `calculation_timeframe`, `wait_for_close` | `Bullish`, `Bearish` | none |
| Rising Wedge | Patterns | overlay | `left`, `right`, `touches`, `convergence`, `max_bars`, `history` | `calculation_timeframe`, `wait_for_close` | `Upper Line`, `Lower Line`, `Confirmed Breakout` | none |
| Rolling Volume and Delta | Order flow | pane | `bars`, `histogram` |  | `Volume sum`, `Delta sum` | none |
| SMA | Moving averages | overlay | `period`, `source` | `plot_offset`, `calculation_symbol`, `calculation_timeframe`, `wait_for_close` | `SMA(20)` | none |
| SMA/EMA Cross | Moving averages | overlay | `period_0`, `kind_0`, `source_0`, `period_1`, `kind_1`, `source_1` | `calculation_timeframe`, `wait_for_close` | `SMA(9)`, `EMA(21)` | none |
| SMI | Oscillators | pane | `period`, `smooth1`, `smooth2`, `signal_period` | `calculation_timeframe`, `wait_for_close` | `SMI(10,3,3)`, `Signal(10)` | BandOnly |
| SMI Ergodic Indicator | Oscillators | pane | `long_length`, `short_length`, `signal_length`, `source` | `plot_offset`, `calculation_symbol`, `calculation_timeframe`, `wait_for_close` | `SMI(20,5)`, `Signal(5)` | Spread |
| SMI Ergodic Oscillator | Oscillators | pane | `long_length`, `short_length`, `signal_length`, `source` | `plot_offset`, `calculation_symbol`, `calculation_timeframe`, `wait_for_close` | `SMI(20,5) - Signal(5)` | RisingColumns |
| SMMA | Moving averages | overlay | `period`, `source` | `plot_offset`, `calculation_symbol`, `calculation_timeframe`, `wait_for_close` | `SMMA(14)` | none |
| STC | Oscillators | pane | `macd_short`, `macd_long`, `cycle`, `factor`, `source` | `plot_offset`, `calculation_symbol`, `calculation_timeframe`, `wait_for_close` | `STC(23,50,10)` | Centered |
| SWMA | Moving averages | overlay | `source` | `plot_offset`, `calculation_symbol`, `calculation_timeframe`, `wait_for_close` | `SWMA` | none |
| Seasonality | Price and sessions | overlay | `start_year`, `source` | `plot_offset`, `calculation_symbol`, `calculation_timeframe`, `wait_for_close` | `Expected`, `Previous close`, `Average`, `Standard deviation`, `Months` | none |
| Size-Filtered CVD | Order flow | pane | `min`, `max` |  | `CVD` | none |
| Spread | Statistics and regression | pane | `symbol_0`, `source_0`, `symbol_1`, `source_1` | `calculation_timeframe`, `wait_for_close` | `Spread` | none |
| Squeeze | Trend | pane | `bb_period`, `bb_mult`, `kc_period`, `kc_mult`, `momentum_period`, `mark_fires` | `calculation_timeframe`, `wait_for_close` | `Momentum`, `Squeeze` | MomentumColumns |
| StdDev | Volatility | pane | `period`, `source` | `plot_offset`, `calculation_symbol`, `calculation_timeframe`, `wait_for_close` | `StdDev(20)` | none |
| StdErr | Statistics and regression | pane | `period`, `source` | `plot_offset`, `calculation_symbol`, `calculation_timeframe`, `wait_for_close` | `StdErr(20)` | none |
| StdErrBands | Bands and channels | overlay | `period`, `multiplier`, `source` | `plot_offset`, `calculation_symbol`, `calculation_timeframe`, `wait_for_close` | `SE Upper(20, 2)`, `SE Middle(20)`, `SE Lower(20, 2)` | none |
| StochRSI | Oscillators | pane | `rsi_period`, `stoch_period`, `smooth_k`, `smooth_d`, `source` | `plot_offset`, `calculation_symbol`, `calculation_timeframe`, `wait_for_close` | `%K(14, 14, 3)`, `%D(3)` | BandOnly |
| Stochastic | Oscillators | pane | `k_period`, `k_smooth`, `d_period` | `calculation_timeframe`, `wait_for_close` | `%K(14,3,3)`, `%D` | BandOnly |
| SuperTrend | Stops and trailing | overlay | `atr_period`, `multiplier` | `calculation_timeframe`, `wait_for_close` | `Up Trend`, `Down Trend` | none |
| TDI | Trend | pane | `period` | `calculation_timeframe`, `wait_for_close` | `TDI(20)` | Centered |
| TEMA | Moving averages | overlay | `period`, `source` | `plot_offset`, `calculation_symbol`, `calculation_timeframe`, `wait_for_close` | `TEMA(20)` | none |
| TII | Trend | pane | `period`, `source` | `plot_offset`, `calculation_symbol`, `calculation_timeframe`, `wait_for_close` | `TII(30)` | Bounded |
| TP | Price and sessions | overlay |  | `calculation_timeframe`, `wait_for_close` | `TP` | none |
| TR | Volatility | pane |  | `calculation_timeframe`, `wait_for_close` | `TR` | none |
| TRIX | Momentum | pane | `period`, `signal_period`, `source` | `plot_offset`, `calculation_symbol`, `calculation_timeframe`, `wait_for_close` | `TRIX(15)`, `Signal(9)` | Centered |
| TSI | Oscillators | pane | `long_period`, `short_period`, `signal_period`, `source` | `plot_offset`, `calculation_symbol`, `calculation_timeframe`, `wait_for_close` | `TSI(25,13)`, `Signal(13)` | Centered |
| Technical Ratings | Momentum | pane | `fast`, `medium`, `slow`, `simple`, `exponential`, `rsi`, `rsi_period`, `stochastic`, `stochastic_period`, `cci`, `cci_period`, `macd`, `macd_fast`, `macd_slow`, `macd_signal`, `momentum`, `momentum_period` | `calculation_timeframe`, `wait_for_close` | `Moving Averages`, `Oscillators`, `Summary`, `Moving Averages Coverage`, `Oscillators Coverage`, `Summary Coverage` | none |
| Time Price Opportunity | Profiles | overlay | `period`, `row_ticks`, `value_area`, `split_sessions`, `single_prints`, `display`, `sessions` | `calculation_timeframe`, `wait_for_close` |  | none |
| Time Weighted Average Price | VWAP and average price | overlay | `anchor`, `source` | `plot_offset`, `calculation_symbol`, `calculation_timeframe`, `wait_for_close` | `TWAP` | PriceSide |
| Trade Rate | Order flow | pane | `volume`, `seconds`, `instant` |  | `Bar total`, `Rolling / second`, `Instant / second` | none |
| Trading Sessions | Price and sessions | overlay | `show 0`, `start 0`, `end 0`, `show 1`, `start 1`, `end 1`, `show 2`, `start 2`, `end 2` | `calculation_timeframe`, `wait_for_close` | `Tokyo`, `London`, `New York` | none |
| Trend Strength Index | Statistics and regression | pane | `period`, `source` | `plot_offset`, `calculation_symbol`, `calculation_timeframe`, `wait_for_close` | `Trend Strength(14)` | Centered |
| Triple MA | Moving averages | overlay | `period_0`, `kind_0`, `source_0`, `period_1`, `kind_1`, `source_1`, `period_2`, `kind_2`, `source_2` | `calculation_timeframe`, `wait_for_close` | `SMA(9)`, `SMA(21)`, `SMA(50)` | none |
| UO | Oscillators | pane | `period1`, `period2`, `period3` | `calculation_timeframe`, `wait_for_close` | `UO(7, 14, 28)` | Bounded |
| Ulcer Index | Volatility | pane | `period`, `source` | `plot_offset`, `calculation_symbol`, `calculation_timeframe`, `wait_for_close` | `Ulcer Index(14)` | Intensity |
| UltMom | Oscillators | pane | `short_period`, `medium_period`, `long_period`, `source` | `plot_offset`, `calculation_symbol`, `calculation_timeframe`, `wait_for_close` | `UltMom(5,10,20)` | Centered |
| Up/Down Volume | Volume | pane |  |  | `Up Volume`, `Down Volume`, `Delta` | none |
| VIDYA | Moving averages | overlay | `period`, `scaler`, `scaler_len`, `ref_len`, `adaptive`, `source` | `plot_offset`, `calculation_symbol`, `calculation_timeframe`, `wait_for_close` | `VIDYA(12)` | none |
| VPT | Volume | pane | `source` | `plot_offset`, `calculation_symbol`, `calculation_timeframe`, `wait_for_close` | `VPT` | Slope |
| VROC | Volume | pane | `period` | `calculation_timeframe`, `wait_for_close` | `VROC(14)` | Centered |
| VW-MACD | Momentum | pane | `fast_period`, `slow_period`, `signal_period`, `source` | `plot_offset`, `calculation_symbol`, `calculation_timeframe`, `wait_for_close` | `VW-MACD(12,26,9)`, `Signal`, `Histogram` | MomentumColumns |
| VWAP | VWAP and average price | overlay | `anchor`, `percentage_bands`, `band_count`, `event_time_zone`, `event_count`, `source` | `plot_offset`, `calculation_symbol`, `calculation_timeframe`, `wait_for_close` | `VWAP` | PriceSide |
| VWAP Auto Anchored | VWAP and average price | overlay | `anchor`, `length`, `source` | `calculation_timeframe`, `wait_for_close` | `VWAP` | PriceSide |
| VWMA | Moving averages | overlay | `period`, `source` | `plot_offset`, `calculation_symbol`, `calculation_timeframe`, `wait_for_close` | `VWMA(20)` | none |
| Visible Average Price | VWAP and average price | overlay | `source` | `plot_offset`, `calculation_symbol`, `calculation_timeframe`, `wait_for_close` | `Avg Price` | none |
| Vol Profile Anchored | Profiles | overlay | `row_count`, `value_area`, `row_size`, `display`, `source_interval`, `feed_source`, `start_time` | `calculation_timeframe`, `wait_for_close` |  | none |
| Vol Profile Auto Anchored | Profiles | overlay | `row_count`, `value_area`, `row_size`, `display`, `source_interval`, `feed_source`, `anchor`, `length` | `calculation_timeframe`, `wait_for_close` |  | none |
| Vol Profile FR | Profiles | overlay | `row_count`, `value_area`, `row_size`, `display`, `source_interval`, `feed_source`, `start_time`, `end_time` | `calculation_timeframe`, `wait_for_close` |  | none |
| Vol Profile Periodic | Profiles | overlay | `row_count`, `value_area`, `row_size`, `display`, `source_interval`, `feed_source`, `period` | `calculation_timeframe`, `wait_for_close` |  | none |
| Vol Profile Session | Profiles | overlay | `row_count`, `value_area`, `row_size`, `display`, `source_interval`, `feed_source`, `adaptive_rows` | `calculation_timeframe`, `wait_for_close` |  | none |
| Vol Profile VR | Profiles | overlay | `row_count`, `value_area`, `row_size`, `display`, `source_interval`, `feed_source` | `calculation_timeframe`, `wait_for_close` |  | none |
| Volatility Stop | Stops and trailing | overlay | `length`, `multiplier` | `calculation_timeframe`, `wait_for_close` | `Volatility Stop` | none |
| Volume | Volume | overlay | `period`, `average`, `show_average`, `color_rule`, `overlay` | `calculation_timeframe`, `wait_for_close` | `Up volume`, `Down volume`, `SMA 9` | none |
| Volume Osc | Volume | pane | `short_period`, `long_period` | `calculation_timeframe`, `wait_for_close` | `VolOsc(5,10)` | Centered |
| Vortex | Trend | pane | `period` | `calculation_timeframe`, `wait_for_close` | `+VI(14)`, `-VI(14)` | none |
| WAD | Volume | pane |  | `calculation_timeframe`, `wait_for_close` | `WAD` | Slope |
| WC | Price and sessions | overlay |  | `calculation_timeframe`, `wait_for_close` | `WC` | none |
| WMA | Moving averages | overlay | `period`, `source` | `plot_offset`, `calculation_symbol`, `calculation_timeframe`, `wait_for_close` | `WMA(20)` | none |
| Williams %R | Oscillators | pane | `period` | `calculation_timeframe`, `wait_for_close` | `%R(14)` | Bounded |
| Williams Alligator | Trend | overlay | `jaw_length`, `teeth_length`, `lips_length`, `jaw_offset`, `teeth_offset`, `lips_offset`, `source` | `plot_offset`, `calculation_symbol`, `calculation_timeframe`, `wait_for_close` | `Jaw`, `Teeth`, `Lips` | none |
| Williams Fractals | Pivots and levels | overlay | `periods` | `calculation_timeframe`, `wait_for_close` | `Up Fractal`, `Down Fractal` | none |
| Woodies CCI | Oscillators | pane | `turbo_length`, `cci_length`, `down_color`, `transition_color`, `neutral_color`, `source` | `plot_offset`, `calculation_symbol`, `calculation_timeframe`, `wait_for_close` | `Histogram(14)`, `CCI Turbo(6)`, `CCI(14)` | none |
| ZLEMA | Moving averages | overlay | `period`, `source` | `plot_offset`, `calculation_symbol`, `calculation_timeframe`, `wait_for_close` | `ZLEMA(21)` | none |
| Zero-Trend Vol | Volatility | pane | `period`, `periods_per_year`, `source` | `plot_offset`, `calculation_symbol`, `calculation_timeframe`, `wait_for_close` | `Zero-Trend Vol(20)` | Intensity |
| ZigZag | Pivots and levels | overlay | `deviation_percent`, `depth`, `mode` | `calculation_timeframe`, `wait_for_close` | `ZigZag(5.0%, 10)` | none |
| Ziv Ghost Pivots | Pivots and levels | overlay | `period` | `calculation_timeframe`, `wait_for_close` | `P`, `R1`, `S1`, `R2`, `S2` | none |
