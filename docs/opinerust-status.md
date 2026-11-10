# OpineRust: status implementasi dan matriks kapabilitas

Dokumen ini adalah **laporan eksekusi** untuk [brief editor](pinecone-pine-editor-bridge.md) dan
[roadmap runtime](opinerust-runtime-fork-v6-roadmap.md). Brief tetap menjadi spesifikasi; dokumen ini
mencatat apa yang benar-benar lulus, apa yang belum, dan bukti perintahnya.

**Status keseluruhan: prototipe; belum aman untuk keputusan trading.** Bridge sudah dipin ke
runtime fork pada revisi `3e4e6e40fc355c268743b867407a37e71201c0fd`. Rollback install, pemetaan
`input.*`, fixture v6, dan jalur browser sudah diuji pada sesi ini (lihat §4); status error pada
chart, konteks kalender/sesi, transaksi saat replay, dan review visual penuh masih terbuka.
Temuan source dan hasil uji di bawah dibedakan: baris yang ditandai **bukti sesi ini** dijalankan
ulang setelah perubahan terakhir, sisanya hasil historis.

## 1. Kondisi dua repo

| | Runtime | Bridge/Chart |
| --- | --- | --- |
| Lokasi | `/root/OpineRust` | `/root/egui-charts` |
| Origin | `younglord420/OpineRust` (private) | `younglord420/Oracly-Rust-Ugui` |
| Upstream | `ferranbt/pinecone` | — |
| Referensi saat audit | `3e4e6e4` (revisi yang dipin bridge) | `b10e7914` |

Runtime masih memakai nama display **OpineRust Runtime** dengan crate `pine-*` dipertahankan agar API
tidak pecah. `LICENSE` MPL-2.0 dan `NOTICE` baru mencatat sumber Pinecone dan commit awal
`7bb739d771f804ef6e0352bb85b517fcf610594e`.

## 2. Perubahan runtime (`/root/OpineRust`)

- **Blocker WASM diperbaiki.** `pine-interpreter` menarik `ahash` lewat workspace, dan `ahash`
  default memakai `runtime-rng` → `getrandom`, yang menolak `wasm32-unknown-unknown` tanpa `wasm_js`.
  Sekarang fitur dipilih per target: native tetap `runtime-rng`, wasm memakai `no-rng`
  (`Counter + address`, bukan untuk keamanan). Build core crate untuk WASM berhasil.
- **Tes WASM runtime nyata** di `tests/wasm_runtime.rs`: lexer, parser, sema, dan interpreter
  dijalankan di `wasm32-unknown-unknown` lewat `wasm-bindgen-test-runner` (Node). Mencakup v5/v6,
  error semantik, `na`, clock host, dan string/math tanpa OS RNG.
- **Dependensi tes dipisah per target.** `pine-data`/`pine-lang` (→ `ureq` → `ring` → `getrandom`)
  dipindah ke `cfg(not(wasm32))` supaya tidak meracuni build browser.
- README, `NOTICE`, dan README extension VS Code diperbarui. `up.sh` **dinonaktifkan** karena dulu
  mengunduh binary Pinecone seolah produk kita; sekarang mengarahkan build dari source.

## 3. Perubahan bridge (`/root/egui-charts`)

- **Status error lewat wrapper sudah diperbaiki.** `Chart::add_indicator` membungkus
  `CustomIndicator` dengan `ConfiguredStudy`, dan `ConfiguredStudy::study_status` dulu hanya
  mengembalikan status wrapper sehingga error recalculation study di dalamnya hilang. Sekarang
  wrapper mengembalikan `self.status.clone().or_else(|| self.study.study_status())`: status hasil
  hitung chart tetap menang, dan error milik study tampil saat chart tidak punya status sendiri.
  Panel juga berhenti memakai `.take()`; ia membaca slot dan mencatat pesan yang sudah pernah
  di-log lewat `logged_error`, jadi chart dan log bisa membaca kegagalan yang sama.
  Regresi: `opinerust/tests/context_change.rs` (`a_wrapped_study_failure_is_visible_on_the_chart`,
  `reading_a_failure_twice_still_reports_it`) lewat `Chart` yang benar-benar membungkus study.
- **`syminfo`/`timeframe` tidak lagi ditolak berdasarkan nama token**, tetapi penerimaan sintaks
  bukan bukti semua field benar. `Data::with_syminfo` meneruskan konteks saat Run; tes sebelumnya
  memeriksa nilai `syminfo.mintick = 0.01` dalam konteks statis, bukan pergantian chart end-to-end.
- **Konteks study terpasang sekarang bisa berubah.** `RunContext` disimpan di sel bersama yang
  dibaca closure recalculate setiap kali jalan. `CustomIndicator` mendapat hook generik
  `with_context_update`, dan chart sudah memanggil `set_instrument`, `set_timeframe`, serta
  `set_bar_clock` pada study terpasang saat metadata berubah; `Chart::set_timeframe` juga mengisi
  ulang `pending_from` sehingga replay dari bar pertama. Regresi:
  `an_interval_change_reaches_an_installed_study` (1m → 5m tanpa Run ulang) dan
  `a_symbol_change_reaches_an_installed_study` (tick size AAPL → MSFT) di
  `opinerust/tests/context_change.rs`, keduanya lewat frame chart nyata.
  Yang **belum** terbukti: kalender/sesi/DST, replay, metadata instrument hilang, dan field
  `syminfo.*` selain ticker/exchange/mintick.
- **Pemetaan interval sudah diperbaiki menjadi jujur.** `pine_timeframe` sekarang mengembalikan
  `Result`: `100ms`/`250ms`/`500ms` serta `Ticks`/`Volume`/`Range`/`Delta` **ditolak** dengan alasan
  karena Pinecone tidak punya unit sub-detik atau unit non-waktu, alih-alih melaporkan 100 detik atau
  menit palsu. Cast `u64`→`u32` memakai `try_from` dan melaporkan overflow. Interval tanpa
  representasi membuat `timeframe.*` tidak diisi dan `unsupported_interval` mencatat alasan.
  Tes di `opinerust/src/context.rs` memeriksa 1S/30S/1/15/60/240/1D/1W/1M, penolakan sub-detik dan
  non-waktu, custom 45S, serta chart tanpa interval.
- Nilai/warna menggunakan snapshot bersama, tetapi keselarasan saat error, clone/history, perubahan
  data dan konteks belum dibuktikan end-to-end. **Bar span tidak lagi ditebak dari jarak bar.**
  `Chart::bar_clock()` sekarang dipublikasikan ke study lewat `CustomIndicator::with_bar_clock` +
  `Indicator::set_bar_clock`, dan bridge memakai `BarClock::next_open_after` untuk menentukan kapan
  candle berakhir. Heuristik median gap hanya dipakai sebagai fallback bila host tidak pernah
  mengirim clock, dan itu dinyatakan di doc, bukan diperlakukan sama. Regresi:
  - `the_forming_candle_follows_the_current_interval` — span basi saat 1m → 5m.
  - `a_session_close_ends_the_last_candle` — candle 15:56 berakhir 16:00 (4 menit), bukan 16:01.
  - `a_monthly_candle_ends_at_the_next_month` — Januari 31 hari, Februari kabisat 29 hari.
  - `a_trade_based_interval_has_no_candle_span` — tick bars tidak menghasilkan span `0`.
  - `the_chart_session_clock_ends_the_candle_at_the_session_close` — clock sesi dari `Chart`
    sungguhan sampai ke `barstate`.
  DST sudah dicakup tes `BarClock` yang ada (beberapa zona transisi). Yang **belum**: replay cursor,
  data sparse, dan apakah jam host yang dipakai panel (`Utc::now`) sesuai saat replay aktif.
- **Run/replace belum terbukti atomik:** `try_add_indicator` menangani batas study, sementara
  `add_indicator` juga menghitung kandidat melalui API yang tidak mengembalikan error kalkulasi.
  Panel langsung menghapus study lama dan menyatakan `Applied` setelah `Ok(id)`, tanpa memeriksa
  keberhasilan kalkulasi saat install. Risiko ini diturunkan dari source dan perlu tes host nyata.

## 4. Hasil uji

### 4.1 Bukti sesi ini (dijalankan setelah perubahan terakhir)

Bridge (`/root/egui-charts/opinerust`):

| Perintah | Hasil |
| --- | --- |
| `cargo test --manifest-path opinerust/Cargo.toml` | 51 lib + 6 + 7 + 10 fixture v6 lulus, 0 gagal |
| `CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner cargo test --manifest-path opinerust/Cargo.toml --target wasm32-unknown-unknown --test wasm_execute` | 6 lulus di Node/WASM nyata (termasuk input dan konteks chart) |
| `cargo clippy --manifest-path opinerust/Cargo.toml --all-targets` | tanpa warning untuk `opinerust` |
| `cargo fmt --manifest-path opinerust/Cargo.toml` / `cargo fmt --all -- --check` | bersih |

Chart/demo:

| Perintah | Hasil |
| --- | --- |
| `cargo test --lib` | 1181 lulus, 0 gagal, 1 ignored |
| `cargo test --bins` di `examples/demo` | 100 lulus, 0 gagal, 1 ignored |
| `trunk build --dist /tmp/opinerust-verify-dist` | sukses (`dev`), bundle baru |
| `sha256sum` bundle lokal vs `curl` dari server | sama: `e1688bb4…ceab0ec` |

Browser nyata (Chromium headless via CDP, `Page.startScreencast` karena capture WebGL langsung
kosong):

- Demo dimuat tanpa error console/exception; satu canvas 1440×700, judul `Oracly | Live Chart`.
- Rail → panel OpineRust terbuka dan chart tetap terlihat.
- Klik tombol Run: chart berubah **28.684 px** (region chart) dan strip Logs **1.706 px**, sedangkan
  noise floor dua frame tanpa aksi adalah **0 px**. Jadi Run benar-benar memasang study.
- Ctrl+Enter sampai ke editor (baris baru muncul, 471 px), dan Ctrl+Enter menambah satu baris
  penuh `#10a368` di chart (garis plot SMA) plus perubahan panel.
- Aksesibilitas egui tidak diekspos ke DOM (`getFullAXTree` hanya `RootWebArea`), jadi klik
  diverifikasi lewat piksel, bukan accessibility tree.

### 4.2 Hasil historis

Ini hasil yang dilaporkan pada sesi sebelumnya, **bukan hasil yang dijalankan ulang saat audit
ini**. Suite native/WASM hijau tidak membuktikan status chart, konteks yang berubah, transaksi
atau visual browser.

Runtime (`/root/OpineRust`):

| Perintah | Hasil |
| --- | --- |
| `cargo fmt --all -- --check` | bersih |
| `cargo test --lib` | lulus semua (26+11+16+6+2+4+8+15+14+5+10+12 tes per crate) |
| `cargo test -p pine-integration-tests --test integration` | 1 lulus |
| `cargo build ... --target wasm32-unknown-unknown` (core crates) | sukses |
| `wasm-bindgen-test-runner` `--test wasm_runtime` | 4 lulus |
| `cargo clippy --all-targets --all-features --locked` | **gagal**: `approx_constant` pada `phi` di `pine-builtins` (lint bawaan, bukan akibat perubahan) |

Bridge (`/root/egui-charts`):

| Perintah | Hasil |
| --- | --- |
| `cargo test --manifest-path opinerust/Cargo.toml` | 29 + 7 lulus |
| `wasm-bindgen-test-runner --test wasm_execute` | 4 lulus (termasuk candle forming) |
| `cargo test --bins` di `examples/demo` | 97 lulus, 0 gagal, 1 ignored |
| `cargo test --lib -- custom studies:: widget::` | 398 lulus |
| `cargo clippy --manifest-path opinerust/Cargo.toml --all-targets` | bersih untuk bridge |
| `cargo fmt` bridge + chart | bersih |

## 5. Belum lulus / blocker

1. **Build bersih butuh kredensial repo private.** Bridge sudah dipin ke `rev = 3e4e6e4` dan
   `Cargo.lock` menunjuk revisi itu, tetapi repo `OpineRust` private: clean build/CI memerlukan
   akses yang sah. Pada lingkungan yang diuji, Cargo memerlukan `CARGO_NET_GIT_FETCH_WITH_CLI=true`
   untuk memakai konfigurasi SSH Git; kebutuhan ini bergantung pada konfigurasi autentikasi. Akses
   CI dan distribusi belum diverifikasi; jangan mengubah visibility atau membagikan token sembarangan.
2. **Clippy runtime gagal** karena lint `approx_constant` di `pine-builtins` (`phi`), masalah upstream
   pada toolchain 1.99.
3. **Review visual browser belum tuntas.** Capture WebGL headless mengembalikan area chart kosong,
   jadi warna/gap/panel tidak bisa dinilai dari screenshot. Tes widget/egui nyata lulus, tetapi itu
   bukan review visual.
4. **Live candle di UI browser belum diuji end-to-end**; hanya diuji numerik dan di WASM headless.
5. **`input.*` sudah dipetakan dan diuji, dengan batas yang jelas.** Token `input` tidak lagi
   diblokir; deklarasi `input.int/float/bool/string/color` dipetakan ke `StudyInput` lewat
   `opinerust/src/settings.rs`, dan edit settings masuk ke skrip lewat override by-title. Bukti:
   `input_declarations_run_and_reach_the_output`, `an_override_by_title_changes_the_run`,
   `an_unknown_override_title_is_ignored`, `an_edited_input_changes_the_next_recalculation`,
   `an_input_override_only_touches_its_own_declaration` (native) dan `inputs_run_and_override_on_wasm`.
   Batas yang masih ada: `input.source`/`session`/`time` **tidak** dipetakan (dilaporkan sebagai
   skipped, bukan ditebak), dan judul duplikat ditolak karena Pinecone me-resolve override by title.
   Jalur UI-nya juga sudah diuji: `a_declared_input_is_editable_and_reaches_the_calculation` membuka
   dialog settings sungguhan (kittest), menemukan widget `Length` dan checkbox `Show`, mengubah
   toggle, menekan Apply, lalu membuktikan nilai itu sampai ke kalkulasi study.
6. **Full replay sinkron** tanpa cancellation/generation guard; skrip berat bisa membekukan frame.
7. **Open/Save dokumen** sengaja disembunyikan karena host demo belum punya penyimpanan skrip.
8. **Run/replace dan undo/redo sudah diuji lewat handler host.** Panel tidak menyatakan `Applied`
   setelah `Ok(id)` saja: `install_failure` membaca status study kandidat sesudah
   `try_add_indicator`, dan bila kalkulasi pertama gagal panel menghapus kandidat, memanggil
   `abandon_user_action`, memulihkan limit, dan melaporkan `Install failed`.
   Regresi: `a_failed_first_calculation_keeps_the_previous_study`.
   Undo/redo: `undo_and_redo_of_a_replace_keep_the_panel_and_host_in_step` menjalankan A→B lewat
   Ctrl+Enter pada frame egui nyata, lalu `chart.undo()` + `sync_chart_history()`. Dua cacat nyata
   ditemukan dan diperbaiki di situ: (a) host tetap menunjuk `SeriesId` study yang sudah diganti,
   (b) panel lupa study yang ia pasang sendiri sehingga Run setelah undo akan **menduplikasi**.
   Panel sekarang menyimpan `installed_history` (id + judul + teks draf saat itu), mengadopsi
   kembali study yang dikembalikan undo, dan menandai draf sebagai edited bila teksnya tidak cocok
   dengan study yang diadopsi. Yang **belum** diuji: kegagalan saat replay history (bukan install)
   dan undo/redo lintas beberapa replace berantai.
9. **Konteks simbol/timeframe:** perubahan symbol/timeframe untuk study terpasang sudah diuji di
   native lewat `Chart`, dan mapping interval palsu sudah ditolak. Jalur konteks juga sudah diuji di
   WASM (`chart_context_reaches_the_script_on_wasm`), tetapi itu hanya membuktikan konteks **statis**
   sampai ke skrip di wasm, bukan pergantian konteks. Belum diuji: replay, kalender/DST, sesi,
   metadata instrument hilang, dan field `syminfo.*` di luar ticker/exchange/mintick.

## 6. Matriks kapabilitas

| Fitur | v5 | v6 | Bukti |
| --- | --- | --- | --- |
| `indicator()`, `plot()` line + warna statis | partial | partial | fixture plot/warna; identitas plot ambigu ditolak |
| Warna per bar, opacity `color.new` | partial | partial | snapshot bersama diuji; belum diuji visual penuh |
| `na`/warm-up gap | partial | partial | gap warm-up diuji termasuk `close[3]` di dalam loop |
| `ta.sma`, aritmetika, ternary | partial | partial | fixture numerik + garis SMA terlihat di browser |
| `barstate.*` (forming/realtime/confirmed) | partial | partial | span dari `BarClock` chart (sesi, bulanan, trade-based) diuji; replay & data sparse belum |
| `timenow` | partial | partial | host clock diuji native + WASM |
| `syminfo.*` / `timeframe.*` | partial | partial | konteks statis diuji native + WASM; field lain & kalender belum |
| `input.int/float/bool/string/color` | partial | partial | dipetakan ke `StudyInput`; override by-title diuji native + WASM |
| `switch`, UDT `type`, `method`, `enum` | — | **supported (fixture)** | `opinerust/tests/v6_fixtures.rs`, nilai harapan eksplisit |
| `var` (state antar bar), loop `for` + history | partial | partial | fixture numerik: running sum dan warm-up `close[3]` |
| `math.*`, `str.length` | partial | partial | fixture numerik |
| `plot` style/width/offset/display | unsupported | unsupported | ditolak |
| `fill`, `bgcolor`, `barcolor`, `plotshape*` | unsupported | unsupported | ditolak |
| `hline`, `label`, `box`, `table`, `line*` | unsupported | unsupported | ditolak dengan baris |
| `request.*`, `import`, `strategy`, `alert` | unsupported | unsupported | ditolak dengan baris |
| Seluruh Pine v6 | tidak diklaim | **partial** | hanya konstruk di atas yang punya fixture |

## 7. Lisensi dan atribusi

Runtime berasal dari Pinecone (MPL-2.0). `LICENSE` dan `NOTICE` menyebut sumber, commit awal, dan
kewajiban source saat distribusi. Repo private tidak otomatis memenuhi kewajiban distribusi.
Pine Script® adalah merek TradingView; OpineRust bukan implementasi resmi.
