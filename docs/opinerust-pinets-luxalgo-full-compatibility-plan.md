# Blueprint & Roadmap Teknis: Pencapaian Kompatibilitas Penuh Pine Script® v5/v6 & LuxAlgo PineTS di OpineRust

**File Dokumen**: `docs/opinerust-pinets-luxalgo-full-compatibility-plan.md`  
**Target Pelaksana**: DeepSeek / Automated Coding Agent  
**Lingkup Workspace**:
- **Repo Oracly Charts & Bridge**: `/root/egui-charts` (CBM Project: `root-egui-charts`)
- **Repo Runtime Engine Fork**: `/root/OpineRust` (CBM Project: `root-opinerust`)  
**Status**: Spesifikasi Eksekusi Terinci, Standar Arsitektur Bersih, & Matriks Implementasi Bertahap

---

## 1. Latar Belakang & Visi Produk

### 1.1 Masalah Saat Ini
1. **Banyak Skrip LuxAlgo / Pine Script v5/v6 Tidak Jalan**:
   - Skrip-skrip modern dari [LuxAlgo Library](https://www.luxalgo.com/library/) (Smart Money Concepts/SMC, Order Blocks, Liquidity Sweeps, Fair Value Gaps/FVG, Volume Profiles, Percentile Rank, ARIMA) gagal dieksekusi di OpineRust.
   - Kegagalan disebabkan oleh pembatasan bridge saat ini yang hanya menyerap output kurva `plot()`, penolakan token `request.*` dan `import` pada tahap lexer, serta belum adanya jembatan visual untuk objek drawing (`box`, `line`, `label`, `table`).
2. **Indikator Oracly Masih Terkunci di Hardcoded Rust**:
   - Koleksi indikator teknikal di `src/studies/builtin/*.rs` (RSI, MACD, Bollinger Bands, Chaikin, dll.) saat ini dikompilasi statis di Rust binary.
   - User tidak dapat melihat, memodifikasi, atau mempelajari kode indikator tersebut.
   - Visi produk: Mengubah indikator teknikal standar menjadi skrip Pine Script (`.pine`) yang terdaftar di pustaka resmi Oracly, sehingga user bisa menekan tombol *"View / Edit Source"* langsung di OpineRust Studio Editor seperti di TradingView atau Vela.

### 1.2 Batasan Hukum & Lisensi (Clean-Room Boundary)
- **LuxAlgo Vela-pinets & PineTS**: Berlisensi **GNU AGPL-3.0**. Dilarang keras menyalin kode baris demi baris, mentransliterasi file TypeScript ke Rust, atau menyertakan file AGPL ke dalam repo.
- **OpineRust**: Berlisensi **MPL-2.0** (berakar dari Pinecone).
- **Oracly-charts**: Berlisensi **Apache-2.0 / MIT**.
- **Aturan Eksekusi DeepSeek**: Gunakan repositori Vela-pinets semata-mata sebagai **referensi perilaku sistem (*behavioral & architectural reference*)**. Seluruh implementasi di `/root/OpineRust` dan `/root/egui-charts` harus berupa kode Rust murni yang ditulis secara independen (*clean-room implementation*).

---

## 2. Arsitektur Integrasi & Batas Modul (Zero-Conflict Design)

```text
┌──────────────────────────────────────────────────────────────────────────────────┐
│                                 ORACLY DEMO APP                                  │
│             (Panel Editor OpineRust + Docking UI + Oracly Candlestick)           │
└────────────────────────────────────────┬─────────────────────────────────────────┘
                                         │
                 ┌───────────────────────┴───────────────────────┐
                 ▼                                               ▼
┌─────────────────────────────────┐             ┌──────────────────────────────────┐
│       Oracly-charts             │             │            opinerust             │
│   (Candlestick, Viewport, Panes,│             │    (Bridge Adapter & Studio)     │
│    Drawing Canvas, IndicatorUI) │             │  - Token / Syntax Validator      │
└────────────────┬────────────────┘             │  - Host Context (syminfo/tf)     │
                 │                              │  - Input Mapper (StudyInput)     │
                 │ Mengonsumsi                  │  - Drawing Converter             │
                 │ CustomIndicator              └────────────────┬─────────────────┘
                 │ (Values & Drawings)                           │
                 │                                               │ Memanggil Core
                 │                                               ▼
                 │                              ┌──────────────────────────────────┐
                 │                              │        /root/OpineRust           │
                 │                              │   (Core Rust Engine - Pinecone)  │
                 │                              │  - pine-core (Output Traits)     │
                 │                              │  - pine-builtins (box/line/ta)   │
                 └──────────────────────────────┤  - pine-interpreter / pine-sema  │
                                                │  - pine-broker (Strategy)        │
                                                └──────────────────────────────────┘
```

### Prinsip Harmoni Arsitektur:
1. **Kontrak `Indicator` Tetap Kebenaran Tunggal**:
   - `Oracly-charts` tidak boleh memiliki keterikatan langsung ke runtime compiler Pinecone.
   - Bridge `opinerust` membungkus output Pine ke dalam `CustomIndicator` yang mengimplementasikan trait `Indicator: Send + Sync`.
2. **Isolasi Clock & Eksekusi WASM**:
   - Seluruh built-in runtime dilarang membaca jam OS (`SystemTime::now()`) yang memicu panic di `wasm32-unknown-unknown`.
   - Host chart selalu menginjeksi parameter waktu eksplisit (`now_ms: i64`).
3. **Data Immutability & Replay Safety**:
   - Setiap kali terjadi penambahan bar, update forming candle, atau perubahan timeframe, `OpineRustStudy` menjalankan recalculate secara terisolasi tanpa merusak snapshot visual sebelumnya jika terjadi kegagalan kalkulasi.

---

## 3. Matriks Kesenjangan (Gap Analysis) & Status Implementasi

Diperbarui setelah Fase 1–3 dikerjakan dan diverifikasi (lihat §8 untuk bukti tes).

| Fitur Pine Script / LuxAlgo | Status | Keterangan |
| :--- | :--- | :--- |
| **Output Plots (`plot`, `plotshape`)** | ✅ Jalan | `plot` + warna per-bar. `plotshape`/`plotchar`/`plotarrow` jadi marker di samping garis, atau drawing bila skrip tidak punya plot (supaya garis tidak hilang). |
| **Drawing Primitives (`box.*`, `line.*`, `label.*`)** | ✅ Jalan | `DrawingOutputSink` menyimpan handle persisten; `drawings_from` memetakan ke drawing chart. `xloc.bar_time` diubah ke indeks bar terdekat. |
| **Table (`table.*`)** | ✅ Jalan | Sel, warna, jangkar `position.*`, `size.*`, dan `text.align_*` masuk `StudyTable`. Frame, border, dan `merge_cells` tidak disimpan runtime yang di-pin, jadi belum bisa digambar. |
| **Multi-Timeframe (`request.security`)** | ✅ Jalan via host | Token tidak lagi ditolak; `HostEnvironment`/`SecurityProvider` menyuplai bar. Tanpa host → `na`. `"D"`/`"W"`/`"M"` kini period valid (fix di `pine-core::Timeframe`). |
| **Pustaka Eksternal (`import ...`)** | ✅ Jalan via host | `LibrarySource` me-resolve path; tanpa resolver → diagnostik bernama path + baris. |
| **Strategi & Backtesting (`strategy.*`)** | ✅ Jalan | Broker runtime native; fill diekstrak jadi `TradeMarker`. Deklarasi `strategy()` kini merekam `Indicator` metadata (fix di `pine-builtins::strategy`). |
| **Manipulasi Array & Matrix** | ✅ Inti teruji | `array.from/sort/push/get/size/sum/avg/min/max/includes/indexof` dan `matrix.new/set/get/sum/transpose/trace/rows/columns` lulus di bridge. Builtin lain ada di runtime, belum satu-satu. |
| **Settings Dialog (`input.*`)** | ⚠️ Sebagian | int/float/bool/string/color/symbol jalan. `input.timeframe` jadi interval chart bila period-nya salah satu preset (`"15"`, `"1D"`, `"D"`); period lain tetap teks. `options=[...]` literal jadi dropdown dan override-nya string pilihan. `input.source` tetap dilewati karena runtime mengabaikan override-nya. |
| **`bar_index[n]` lookback** | ✅ Diperbaiki | Dulu skalar (lookback `na`, kotak SMC kosong). Kini `Value::Series`. |

---

## 4. Rincian Implementasi Spesifik (Fase demi Fase)

### FASE 1: Aktivasi Objek Gambar / Drawings (`box`, `line`, `label`, `linefill`)
*Tujuan: Memungkinkan skrip SMC LuxAlgo merender Order Blocks, Fair Value Gaps (FVG), Break of Structure (BOS), dan Change of Character (CHoCH).*

#### 1.1 Di Runtime Engine (`/root/OpineRust` - CBM Project: `root-opinerust`)
1. Telusuri trait yang sudah ada di `crates/pine-core/src/output.rs`:
   - `BoxOutput` (memiliki method `add_box`, `get_box`, `get_box_mut`, `delete_box`).
   - `LineOutput` (memiliki method `add_line`, `get_line`, `get_line_mut`, `delete_line`).
   - `LabelOutput` (memiliki method `add_label`, `get_label`, `get_label_mut`, `delete_label`).
2. Pastikan `DefaultPineOutput` di `crates/pine-core/src/output.rs` mengimplementasikan ketiga trait ini dan menyimpan state drawing dalam struktur yang dapat diekstraksi:
   ```rust
   #[derive(Clone, Debug, PartialEq)]
   pub struct BoxRecord {
       pub id: u64,
       pub left: i64,      // bar index atau time_ms
       pub right: i64,
       pub top: f64,
       pub bottom: f64,
       pub border_color: Color,
       pub bg_color: Color,
       pub border_width: u32,
   }
   ```
3. Pastikan eksekusi per-bar di `crates/pine-builtins/src/box/mod.rs` tidak menghapus box persisten yang dideklarasikan dengan kata kunci `var`.

#### 1.2 Di Bridge Adapter (`/root/egui-charts/opinerust` - CBM Project: `root-egui-charts`)
1. Di `opinerust/src/engine.rs`:
   - Tambahkan koleksi drawing ke dalam struct `EngineOutput`:
     ```rust
     pub struct EngineOutput {
         pub info: ScriptInfo,
         pub plots: Vec<PlotSeries>,
         pub boxes: Vec<BoxRecord>,
         pub lines: Vec<LineRecord>,
         pub labels: Vec<LabelRecord>,
         pub inputs: Vec<ScriptInput>,
         pub diagnostics: Vec<Diagnostic>,
     }
     ```
   - Ambil data drawing dari interpreter saat run selesai dan kemas ke `EngineOutput`.
2. Di `opinerust/src/study.rs`:
   - Hubungkan koleksi drawing tersebut ke dalam snapshot study.
   - Sediakan method atau adapter agar sistem render `oracly-charts` dapat membaca koordinat box/line/label dan merendernya pada pane chart yang bersesuaian.

---

### FASE 2: Host Data Resolver untuk `request.security` dan `import`
*Tujuan: Membuka dukungan analisa multi-timeframe dan modularisasi library tanpa melanggar batasan WASM.*

#### 2.1 Pola Host Cache Data Provider (`request.security`)
1. Di `opinerust/src/engine.rs`:
   - Hapus kata `"request"` dari daftar `forbidden` di fungsi `validate_tokens`.
2. Definisikan antarmuka provider data di `opinerust/src/context.rs`:
   ```rust
   pub trait SecurityDataProvider: Send + Sync {
       fn get_bars(&self, symbol: &str, timeframe: &str) -> Option<Vec<Bar>>;
   }
   ```
3. Ketika skrip memanggil `request.security(symbol, tf, expr)`:
   - Interpreter meminta slice bar dari `SecurityDataProvider` yang disediakan oleh chart host.
   - Chart host menggunakan cache bar yang sudah dimuat dalam memori. Jika data belum tersedia, kembalikan `Value::Na` tanpa membuat engine panic atau melakukan blocking network I/O.

#### 2.2 Pola Library Resolver (`import`)
1. Di `opinerust/src/engine.rs`:
   - Hapus kata `"import"` dan `"library"` dari `validate_tokens`.
2. Sediakan callback resolver pada `RunContext`:
   ```rust
   pub type LibraryResolver = Arc<dyn Fn(&str) -> Option<String> + Send + Sync>;
   ```
3. Parser melakukan pre-pass untuk me-resolve dependensi kode dari direktori pustaka lokal sebelum evaluasi AST utama dijalankan.

---

### FASE 3: Dukungan Strategi & Ekstraksi Order Fills (`strategy.*`)
*Tujuan: Mendukung skrip strategi LuxAlgo dan menampilkan titik entry/exit pada chart.*

1. Di `opinerust/src/engine.rs`:
   - Hapus kata `"strategy"` dari penolakan di `validate_tokens`.
   - Izinkan deklarasi root `strategy("Title", ...)` selain `indicator("Title", ...)`.
2. Integrasikan broker emulator dari `/root/OpineRust/crates/pine-broker`:
   - Ekstrak riwayat eksekusi order (`trades` / `orders`):
     ```rust
     pub struct TradeMarker {
         pub bar_index: usize,
         pub time_ms: i64,
         pub price: f64,
         pub side: OrderSide, // Long / Short
         pub comment: String,
     }
     ```
3. Di `oracly-charts`, gambar marker panah beli (hijau) dan jual (merah) pada koordinat bar yang sesuai.

---

### FASE 4: Migrasi Indikator Bawaan Oracly ke Format Pine Script

*Tujuan: Mengubah ratusan file Rust statis di `src/studies/builtin/` (~177 file) menjadi skrip terbuka berbasis Pine Script v5/v6 yang mudah dikelola, dapat diedit oleh user, dan membentuk katalog Oracly Library yang setara dengan TradingView dan LuxAlgo.*

#### 4.1 Pemisahan Arsitektur: Hybrid Library (Pine Script 85% vs High-Performance Native 15%)
Tidak semua studi diubah tanpa pertimbangan. DeepSeek harus membagi koleksi studi ke dalam dua kategori arsitektur yang tegas:

```text
┌─────────────────────────────────────────────────────────────┐
│                 ORACLY STUDY INDICATOR PICKER               │
└──────────────┬───────────────────────────────┬──────────────┘
               │                               │
               ▼                               ▼
┌──────────────────────────────┐ ┌────────────────────────────┐
│   PINE SCRIPT LIBRARY (85%)  │ │   HIGH-PERF NATIVE (15%)   │
│  - Indikator Teknikal Biasa  │ │  - Order Flow & Footprint  │
│  - Strategi Backtesting      │ │  - Deep Volume Profile     │
│  - SMC & Price Action        │ │  - Heatmap Liquidity DOM   │
│  - Bisa dibuka & diedit user │ │  - Tetap hardcoded Rust    │
│    di OpineRust Studio       │ │    untuk kecepatan GPU/CPU │
└──────────────────────────────┘ └────────────────────────────┘
```

1. **Kelompok Dimigrasi ke Pine Script (Katalog Terbuka & Diedit User - ~140+ Studi)**:
   - **Osilator & Momentum**: RSI, Stochastic, MACD, CCI, Williams %R, Ultimate Oscillator, MFI, Awesome Oscillator, Chaikin, ROC, DPO, TRIX, TSI, Elder Ray, Fisher Transform, KST, Mass Index, SMI, Vidya, Squeeze Momentum, KDJ, CMO, PPO, PVO.
   - **Trend & Moving Averages**: SMA, EMA, WMA, HMA, ALMA, KAMA, McGinley Dynamic, DEMA, TEMA, Supertrend, Parabolic SAR, Aroon, Ichimoku Cloud, ADX/DMI, Envelopes, Guppy MA, MA Cross, Moving Average Channel, Trend Intensity Index.
   - **Volatilitas & Channels**: Bollinger Bands, Keltner Channels, Donchian Channels, Chandelier Exit, ATR, Historical Volatility, Standard Error Bands, Acceleration Bands, Ulcer Index, Choppiness Index.
   - **Pola & Price Action**: Pivot Points (Standard, Fibonacci, Woodie, Camarilla), Fractals, Candlestick Patterns (Engulfing, Doji, Hammer, dll.), Heikin Ashi Smoothed.
2. **Kelompok Dipertahankan Tetap Native Rust (Kinerja Komputasi Berat - ~20+ Studi)**:
   - **Order Flow & Footprint**: `footprint_studies.rs`, `order_flow.rs`, `trade_profile.rs`, `trade_delta.rs`. (Memerlukan throughput jutaan tick/L2 per detik dan struktur data raw memory buffer).
   - **Deep Volume Profile**: `volume_profile.rs`, `options_volume_profile.rs`, `anchored_vwap_study.rs`. (Perlu komputasi paralel multi-thread native untuk agregasi jutaan bucket harga).
   - **DOM Liquidity Heatmap**: Memerlukan rendering direct GPU vertex buffer di egui.

#### 4.2 Rencana Struktur Direktori Library
Buat direktori pustaka standar di dalam workspace chart:
```text
opinerust/
└── library/
    ├── trend/
    │   ├── ema.pine
    │   ├── sma.pine
    │   ├── supertrend.pine
    │   ├── macd.pine
    │   ├── ichimoku.pine
    │   ├── adx.pine
    │   └── aroon.pine
    ├── oscillators/
    │   ├── rsi.pine
    │   ├── stochastic.pine
    │   ├── cci.pine
    │   ├── williams_r.pine
    │   ├── mfi.pine
    │   ├── awesome_oscillator.pine
    │   └── trix.pine
    ├── volatility/
    │   ├── bollinger_bands.pine
    │   ├── atr.pine
    │   ├── keltner.pine
    │   ├── donchian.pine
    │   └── chandelier_exit.pine
    ├── volume/
    │   ├── obv.pine
    │   ├── vwap.pine
    │   ├── cmf.pine
    │   └── pvo.pine
    └── price_action/
        ├── pivot_points.pine
        ├── fractals.pine
        └── candlestick_patterns.pine
```

#### 4.3 Alur Eksekusi & Integrasi UI Chart
1. **Daftarkan Indikator ke Dialog Chart**:
   - Di `IndicatorCategory`, daftarkan metadata indikator (Nama, Deskripsi, Kategori, Path Skrip).
2. **Tombol "View / Edit Source" (`{ }`)**:
   - Pada legend menu setiap indikator Pine di chart, tambahkan tombol aksi *"View / Edit Source"*.
   - Mengklik tombol ini otomatis membuka dock `OpineRustPanel` dan mengisi canvas teks editor dengan source code `.pine` indikator tersebut secara real-time.
   - User dapat memodifikasi parameter, logika kalkulasi, atau warna, lalu menekan tombol *"Run"* untuk langsung meng-apply hasil modifikasi ke chart sebagai custom indicator.
3. **Penyusutan Kode Native**:
   - Indikator matematika standar yang sudah berhasil dimigrasi ke `.pine` ditandai sebagai *deprecated* di Rust native untuk merampingkan ukuran bundle WASM dan waktu kompilasi secara bertahap.

---

## 5. Matriks Sinkronisasi Fitur Belum Lengkap (Dari `features.csv`)

Berdasarkan audit backlog pelacakan fitur di `/root/egui-charts/docs/progress/features.csv`, berikut daftar fitur, studi, dan indikator dengan status `missing` atau `partial` yang diselaraskan untuk diselesaikan melalui kombinasi runtime OpineRust (`.pine` / Pine Script v5/v6) dan integrasi chart `oracly-charts`:

### 5.1 Strategi Bawaan (Built-in Strategies - Status: `missing`)
Semua strategi built-in TradingView yang berstatus `missing` di `features.csv` **TIDAK PERLU** di-hardcode lagi di Rust native `src/strategies/`, melainkan langsung diimplementasikan sebagai skrip Pine Script (`strategy(...)`) yang berjalan di atas OpineRust:
1. **Trend & Channels**:
   - `Built-in Bollinger Bands strategy` & `Directed`
   - `Built-in Keltner Channels strategy`
   - `Built-in Price Channel strategy`
   - `Built-in channel breakout strategy`
   - `Built-in Supertrend strategy`
2. **Momentum & Crossovers**:
   - `Built-in MACD strategy`
   - `Built-in RSI strategy`
   - `Built-in Stochastic Slow strategy`
   - `Built-in Momentum strategy`
   - `Built-in price and moving average crossover strategy`
   - `Built-in two moving average crossover strategy`
   - `Built-in Technical Ratings strategy`
3. **Price Action & Candlestick Bar**:
   - `Built-in Consecutive Up/Down strategy`
   - `Built-in inside bar strategy`
   - `Built-in outside bar strategy`
   - `Built-in up and down bar strategy`
   - `Built-in volatility expansion close strategy`
   - `Built-in Pivot Extension strategy` & `Pivot Reversal strategy`
   - `Built-in Parabolic SAR strategy`
   - `Built-in Greedy strategy`

### 5.2 Indikator Teknis & Chart Patterns (Status: `missing` & `partial`)
1. **Chart & Pattern Recognition (Status: `missing`)**:
   - `All Chart Patterns`
   - `Chart Pattern Elliott Wave`
   - `Chart Pattern Falling Wedge`
   - `Chart Pattern Rising Wedge`
   - `Auto key levels`
   - *Solusi Eksekusi*: Diimplementasikan melalui skrip Pine Script menggunakan primitive `line.*` dan `box.*` yang baru di-bridge di Fase 1.
2. **Indikator Kuantitatif & Korelasi (Status: `missing`)**:
   - `Correlation Coefficient` & `Correlation - Log`
   - `Volatility Index`
   - *Solusi Eksekusi*: Skrip Pine Script menggunakan fungsi matematika statistik bawaan Pinecone (`ta.correlation`, `math.log`).
3. **Penyempurnaan Indikator Partial**:
   - `RSI Divergence Indicator` (membutuhkan plotting label/line divergence).
   - `Technical Ratings` (membutuhkan widget `table.*`).
   - `Volume Weighted Average Price` (VWAP) & `Anchored VWAP` (penyempurnaan multi-anchor session).
   - `Connors RSI`, `Woodies CCI`, `Trend Strength Index`.

### 5.3 Order Flow & Advanced Microstructure (Status: `missing` / `partial`)
Fitur-fitur order flow tingkat lanjut dari `features.csv` yang akan dijembatani:
- `Absorption detection` & `Iceberg detection`
- `Spoofing and pulled-liquidity detection`
- `Stop-run detection` (burst market orders)
- `Options gamma exposure suite`
- `Liquidity tracker` & `Passive wall detection`
- *Pendekatan*: Data micro-order level diumpankan melalui injeksi `request.footprint()` pada OpineRust bridge sehingga skrip analisis order flow tingkat lanjut dapat ditulis dengan Pine Script.

### 5.4 Datafeed & Save/Load Adapters
- `HTTP history and streaming adapter` (status: `missing`): Sediakan adapter HTTP/WebSocket standar untuk feed market live.
- `Save and restore the complete chart state` (status: `partial`): Pastikan state indikator OpineRust (script source, input values, pane assignment) terserialisasi penuh ke JSON layout.

---

## 6. Spesifikasi & Blueprint Kode Pine Script untuk Fitur Missing (Ready-to-Implement)

Bagian ini memberikan template dan standar implementasi kode Pine Script v5/v6 yang wajib dibuat oleh DeepSeek di direktori `opinerust/library/` untuk menuntaskan backlog fitur dari `features.csv`:

### 6.1 Struktur Penyimpanan & Bundling Pustaka
```text
opinerust/library/
├── strategies/
│   ├── bollinger_bands_strategy.pine
│   ├── macd_strategy.pine
│   ├── rsi_strategy.pine
│   ├── supertrend_strategy.pine
│   ├── keltner_strategy.pine
│   ├── channel_breakout_strategy.pine
│   ├── stochastic_slow_strategy.pine
│   ├── two_ma_crossover_strategy.pine
│   ├── inside_bar_strategy.pine
│   ├── consecutive_up_down_strategy.pine
│   ├── pivot_reversal_strategy.pine
│   └── parabolic_sar_strategy.pine
├── patterns/
│   ├── rsi_divergence.pine
│   ├── auto_key_levels.pine
│   ├── falling_wedge.pine
│   └── rising_wedge.pine
├── quantitative/
│   ├── correlation_coefficient.pine
│   ├── correlation_log.pine
│   └── volatility_index.pine
└── technical_ratings/
    └── technical_ratings.pine
```

### 6.2 Contoh Blueprint Kode Standar

#### A. `opinerust/library/strategies/bollinger_bands_strategy.pine`
```pinescript
//@version=5
strategy("Bollinger Bands Strategy", overlay=true, default_qty_type=strategy.percent_of_equity, default_qty_value=10)

length = input.int(20, "Length", minval=1)
mult = input.float(2.0, "StdDev Multiplier", minval=0.001, maxval=50)
src = input.source(close, "Source")

basis = ta.sma(src, length)
dev = mult * ta.stdev(src, length)
upper = basis + dev
lower = basis - dev

plot(basis, "Basis", color=color.orange)
p1 = plot(upper, "Upper", color=color.blue)
p2 = plot(lower, "Lower", color=color.blue)
fill(p1, p2, color=color.rgb(33, 150, 243, 90), title="Background")

if ta.crossover(src, lower)
    strategy.entry("BB_Long", strategy.long)

if ta.crossunder(src, upper)
    strategy.entry("BB_Short", strategy.short)
```

#### B. `opinerust/library/strategies/macd_strategy.pine`
```pinescript
//@version=5
strategy("MACD Strategy", overlay=false, default_qty_type=strategy.percent_of_equity, default_qty_value=10)

fast_length = input.int(12, "Fast Length")
slow_length = input.int(26, "Slow Length")
src = input.source(close, "Source")
signal_length = input.int(9, "Signal Smoothing")

fast_ma = ta.ema(src, fast_length)
slow_ma = ta.ema(src, slow_length)
macd = fast_ma - slow_ma
signal = ta.ema(macd, signal_length)
hist = macd - signal

plot(hist, title="Histogram", style=plot.style_columns, color=(hist >= 0 ? (hist[1] < hist ? color.teal : color.green) : (hist[1] < hist ? color.maroon : color.red)))
plot(macd, title="MACD", color=color.blue)
plot(signal, title="Signal", color=color.orange)

if ta.crossover(macd, signal)
    strategy.entry("MACD_Long", strategy.long)

if ta.crossunder(macd, signal)
    strategy.close("MACD_Long")
```

#### C. `opinerust/library/strategies/supertrend_strategy.pine`
```pinescript
//@version=5
strategy("Supertrend Strategy", overlay=true, default_qty_type=strategy.percent_of_equity, default_qty_value=10)

atr_period = input.int(10, "ATR Length")
factor = input.float(3.0, "Factor", step=0.1)

[supertrend, direction] = ta.supertrend(factor, atr_period)

body_middle = plot((open + close) / 2, display=display.none)
up_trend = plot(direction < 0 ? supertrend : na, "Up Trend", color=color.green, style=plot.style_linebr)
down_trend = plot(direction > 0 ? supertrend : na, "Down Trend", color=color.red, style=plot.style_linebr)

fill(body_middle, up_trend, color.new(color.green, 90), fillgaps=false)
fill(body_middle, down_trend, color.new(color.red, 90), fillgaps=false)

if ta.change(direction) < 0
    strategy.entry("ST_Long", strategy.long)
if ta.change(direction) > 0
    strategy.entry("ST_Short", strategy.short)
```

#### D. `opinerust/library/quantitative/correlation_coefficient.pine`
```pinescript
//@version=5
indicator("Correlation Coefficient", overlay=false)

length = input.int(20, "Lookback Length", minval=1)
source1 = input.source(close, "Source 1")
source2 = input.source(open, "Source 2")

corr = ta.correlation(source1, source2, length)

plot(corr, "Correlation", color=color.blue, linewidth=2)
hline(1.0, "Max", color=color.gray, linestyle=hline.style_dotted)
hline(0.0, "Zero", color=color.gray)
hline(-1.0, "Min", color=color.gray, linestyle=hline.style_dotted)
```

#### E. `opinerust/library/patterns/auto_key_levels.pine`
```pinescript
//@version=5
indicator("Auto Key Levels", overlay=true)

left_bars = input.int(10, "Left Bars", minval=1)
right_bars = input.int(10, "Right Bars", minval=1)

ph = ta.pivothigh(left_bars, right_bars)
pl = ta.pivotlow(left_bars, right_bars)

var box[] level_boxes = array.new_box()

if not na(ph)
    b = box.new(bar_index - right_bars, ph, bar_index + 20, ph, border_color=color.red, border_width=1, border_style=line.style_dashed, bgcolor=color.new(color.red, 90))
    array.push(level_boxes, b)

if not na(pl)
    b = box.new(bar_index - right_bars, pl, bar_index + 20, pl, border_color=color.green, border_width=1, border_style=line.style_dashed, bgcolor=color.new(color.green, 90))
    array.push(level_boxes, b)
```

---

## 7. Protokol Verifikasi, Build, & Panduan DeepSeek

### 7.1 Perintah Wajib & Prosedur Validasi
DeepSeek wajib menjalankan perintah pengujian secara bertahap dan memverifikasi exit code:

```bash
# 1. Pastikan kedua project CBM tersedia
codebase-memory-mcp cli list_projects

# 2. Cek integritas unit test bridge opinerust saat ini
cargo test --manifest-path opinerust/Cargo.toml

# 3. Cek integritas workspace OpineRust runtime fork
cargo test --manifest-path /root/OpineRust/Cargo.toml --lib

# 4. Verifikasi kompilasi target WebAssembly
cargo check --target wasm32-unknown-unknown --manifest-path opinerust/Cargo.toml

# 5. Build dan verifikasi demo chart
cargo check --manifest-path examples/demo/Cargo.toml
```

### 7.2 Standar Keselamatan Kerja
- **Jangan Commit / Push**: Jangan menjalankan perintah git commit, push, atau release kecuali diminta eksplisit oleh user.
- **Jangan Menghapus Perubahan Lokal**: Periksa `git status` sebelum melakukan modifikasi file.
- **Graph-First**: Gunakan MCP `project-0-egui-charts-codebase-memory-mcp` (atau CLI CBM) untuk meneliti `search_graph` dan `trace_path` pada project `root-opinerust` dan `root-egui-charts` sebelum mengubah fungsi core.
- **WASM Safety**: Dilarang memanggil dependensi OS thread/clock/socket di dalam crate yang dikompilasi untuk target `wasm32-unknown-unknown`.

---

## 8. Status Implementasi & Bukti Verifikasi

**Tanggal verifikasi**: 2026-11-11. Runtime ada di `98c09bb` dan bridge mem-pin revisi itu.

### 8.1 Yang sudah dikerjakan

**Runtime fork `/root/OpineRust`** (MPL-2.0):
- `crates/pine-core/src/timeframe.rs` — `FromStr` menerima shorthand `"D"`/`"W"`/`"M"` sebagai `1D`/`1W`/`1M`. Sebelumnya setiap `request.security(sym, "D", …)` membaca `na` karena parse gagal. Test: `a_bare_unit_suffix_means_one_of_that_unit`.
- `crates/pine-builtins/src/strategy/mod.rs` — `strategy(...)` kini merekam `Indicator` metadata (`title`, `shorttitle`, `overlay`, `format`, `precision`) sehingga skrip strategi tidak tampil sebagai studi tanpa judul. Bound `MetadataOutput` ditambahkan pada `register`.

**Bridge `/root/egui-charts/opinerust`**:
- `src/output.rs` (baru) — `DrawingOutputSink`: implementasi penuh trait output dengan penyimpanan handle drawing yang persisten (`boxes`, `lines`, `labels`, `tables`, `linefills`, `polylines`). Trait `BoxOutput`/`LineOutput`/`LabelOutput` di `pine-core` menyembunyikan map-nya, jadi sink ini yang membuat host bisa membacanya.
- `src/host.rs` (baru) — `HostEnvironment`, `SecurityProvider`, `LibrarySource`, `ChartOnlyProvider`, `MemoryLibraries`, plus adapter `InterpreterFeed`/`InterpreterLibraries` (dengan cache per `(simbol, timeframe)` + stamping `syminfo`). Tidak ada I/O jaringan: semua data datang dari host.
- `src/engine.rs` — `execute_with_environment`; drawing & trade diekstrak ke `EngineOutput` (tipe baru `ScriptBox`, `ScriptLine`, `ScriptLabel`, `ScriptTable`, `ScriptCell`, `TradeMarker`); `validate_tokens` tidak lagi menolak `request`/`import`/`library`/`strategy`; `declaration_of` membaca deklarasi dari AST sehingga `strategy()` dapat judul/overlay walau runtime belum merekamnya; `bar_index` dipublikasikan sebagai `Value::Series` agar `bar_index[n]` bekerja.
- `src/study.rs` — `build_study_with_environment`; `drawings_from` memetakan box/line/label ke `Oracly_charts::drawings::Drawing`; `Snapshot` menerbitkan nilai + warna + geometri dari run yang sama; studi gambar-saja (tanpa `plot`) kini valid; kegagalan kalkulasi menyimpan geometri terakhir yang baik.
- `src/settings.rs` — `input.timeframe` yang period-nya preset chart jadi interval; period lain tetap teks. `input.symbol` jadi simbol. `options=[...]` literal jadi dropdown dan override-nya string pilihan, bukan indeks.
- `src/study.rs` — tabel Pine membawa jangkar `position.*`, warna, `size.*`, dan perataan `text.align_*`.
- `src/engine.rs` — tes `array_and_matrix_builtins_reduce_to_plots` mengunci inti `array.*` dan `matrix.*`.
- `src/ui/panel.rs` — `prepare_with_environment`.
- `src/lib.rs` — ekspor tipe & trait baru.
- `tests/structure_scripts.rs` (baru) — 4 tes end-to-end untuk skrip berbentuk LuxAlgo SMC (gap boxes + swing lines + BOS label), `request.security` lewat host, `import` pustaka, dan ledger strategi.

**Chart `/root/egui-charts`**:
- `src/studies/custom.rs` — `CustomIndicator::with_drawings(initial, source)` + field `drawings` + implementasi `Indicator::drawings()`. Geometri dibangun ulang pada setiap `calculate`, jadi kegagalan kalkulasi tidak menyisakan kotak dari run sebelumnya.
- `src/studies/table.rs` — `StudyTable` punya `anchor`, `background`, dan `cell_styles` (warna, `text_size`, `halign`, `valign`). `TableAnchor::from_pine` memetakan sembilan sudut plus `top`/`middle`/`bottom`. Tabel bawaan tetap di kanan bawah karena field baru memakai default.

### 8.2 Perintah verifikasi & hasil

| Perintah | Hasil |
| :--- | :--- |
| `cd /root/OpineRust && cargo test --workspace` | **139 passed, 0 failed** |
| `cd /root/OpineRust && cargo fmt --check` | bersih |
| `cd /root/egui-charts/opinerust && cargo test` | **155 passed, 0 failed** (unit + integrasi; `wasm_execute` di-skip di native). Termasuk ukuran sel, array, dan matrix. |
| `cd /root/egui-charts && cargo test --lib --features ui table::` | **5 passed** (jangkar, ukuran font, perataan, tabel yang meluap). |
| `cd /root/egui-charts/opinerust && cargo fmt --check` | bersih untuk semua file yang diubah; `ui/mod.rs` & `ui/study_editor.rs` sudah tidak terformat sebelum pekerjaan ini (pre-existing, tidak disentuh) |
| `cd /root/egui-charts/opinerust && cargo clippy --lib` | 1 warning, sudah ada sebelumnya (`study_editor.rs`), bukan dari perubahan ini |
| `cd /root/egui-charts/opinerust && CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner cargo test --target wasm32-unknown-unknown --test wasm_execute` | **6 passed, 0 failed** |
| `cd /root/egui-charts/opinerust && cargo check --target wasm32-unknown-unknown --lib` | sukses |
| `cd /root/egui-charts && cargo test --lib --features ui` | **1180 passed, 0 failed, 1 ignored** |
| `cd /root/egui-charts && cargo fmt --check` | bersih |
| `cd /root/egui-charts && cargo check --manifest-path examples/demo/Cargo.toml` | sukses |
| `cd /root/egui-charts && cargo check --manifest-path terminal/Cargo.toml` | sukses |

### 8.3 Yang BELUM terverifikasi / blocker

1. **`input.source`** tetap dilewati: runtime mengembalikan series itu sendiri dan tidak membaca override. Daftar `options` yang dihitung (bukan literal) juga tidak jadi dropdown.
2. **`DrawingOutputSink`** hanya menyimpan handle akhir run; mutasi per-bar tidak punya riwayat.
3. **Belum ada uji browser visual** (screenshot demo) pada pekerjaan ini. Frame, border, dan `table.merge_cells` dibuang oleh runtime (`frame_color`/`border_width` tidak masuk `Table`).
