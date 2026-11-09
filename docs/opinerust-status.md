# OpineRust: status implementasi dan matriks kapabilitas

Dokumen ini adalah **laporan eksekusi** untuk [brief editor](pinecone-pine-editor-bridge.md) dan
[roadmap runtime](opinerust-runtime-fork-v6-roadmap.md). Brief tetap menjadi spesifikasi; dokumen ini
mencatat apa yang benar-benar lulus, apa yang belum, dan bukti perintahnya.

**Status keseluruhan: prototipe; belum aman untuk keputusan trading.** Bridge sudah dipin ke
runtime fork pada revisi `3e4e6e40fc355c268743b867407a37e71201c0fd`, tetapi status error pada
chart, perubahan konteks, keamanan Run/replace, dan verifikasi browser belum terbukti benar.
Temuan source dan hasil uji di bawah dibedakan: audit dokumen ini **tidak menjalankan ulang tes**.

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
  data dan konteks belum dibuktikan end-to-end. `barstate` diturunkan dari jam host dan perkiraan
  jarak bar. **Bar span sekarang dihitung ulang per recalculation**, bukan memakai span saat build:
  sebelumnya chart yang beralih 1m → 5m tetap menilai candle terakhir dengan satu menit, sehingga
  candle yang masih forming dilaporkan confirmed. Regresi:
  `the_forming_candle_follows_the_current_interval` (gagal `Some(0.0)` dengan span basi, lulus
  `Some(1.0)` dengan span per run). Kalender/sesi/DST, monthly bar, replay, dan data sparse masih
  belum terbukti.
- **Run/replace belum terbukti atomik:** `try_add_indicator` menangani batas study, sementara
  `add_indicator` juga menghitung kandidat melalui API yang tidak mengembalikan error kalkulasi.
  Panel langsung menghapus study lama dan menyatakan `Applied` setelah `Ok(id)`, tanpa memeriksa
  keberhasilan kalkulasi saat install. Risiko ini diturunkan dari source dan perlu tes host nyata.

## 4. Hasil uji terdahulu (historis)

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
5. **`input.*` masih ditolak.** Upstream menyediakannya untuk v5/v6, tetapi mapping ke `StudyInput`
   belum diimplementasikan; sengaja belum dibuka agar tidak memberi status sukses palsu.
6. **Full replay sinkron** tanpa cancellation/generation guard; skrip berat bisa membekukan frame.
7. **Open/Save dokumen** sengaja disembunyikan karena host demo belum punya penyimpanan skrip.
8. **Run/replace belum terbukti atomik.** Status error lewat `ConfiguredStudy` dan pembacaan slot
   tanpa `take()` sudah diperbaiki dan diuji (§3), tetapi kegagalan kalkulasi **saat** install,
   pane/settings/identity yang harus bertahan, serta undo/redo replace belum direproduksi lewat
   handler host. `add_indicator` masih menghitung lewat API infallible dan panel masih menyatakan
   `Applied` setelah `Ok(id)`; jangan klaim atomic sebelum skenario gagal-sesudah-preflight diuji.
9. **Konteks simbol/timeframe:** perubahan symbol/timeframe untuk study terpasang sudah diuji di
   native lewat `Chart`, dan mapping interval palsu sudah ditolak. Belum diuji: replay, kalender/DST,
   sesi, metadata instrument hilang, field `syminfo.*` di luar ticker/exchange/mintick, dan target
   WASM untuk perubahan konteks.

## 6. Matriks kapabilitas

| Fitur | v5 | v6 | Catatan |
| --- | --- | --- | --- |
| `indicator()`, `plot()` line + warna statis | partial | partial | contoh terbatas diuji; identitas plot ambigu ditolak |
| Warna per bar, opacity `color.new` | partial | partial | snapshot diuji terbatas; sinkronisasi chart saat error belum terbukti |
| `na`/warm-up gap | partial | partial | contoh warm-up diuji; kasus gap/chart lain perlu diverifikasi |
| `ta.sma`, aritmetika, ternary | partial | partial | hanya fixture tertentu; bukan seluruh semantik |
| `barstate.*` (forming/realtime/confirmed) | partial / belum tervalidasi | partial / belum tervalidasi | estimasi jam dan jarak bar; live/replay/sesi belum terbukti |
| `timenow` | partial | partial | host clock diuji pada fixture terbatas; perilaku live/replay perlu validasi |
| `syminfo.*` / `timeframe.*` | partial / berisiko salah | partial / berisiko salah | konteks statis dan mapping interval salah; tidak klaim semua field benar |
| `input.*` | unsupported | unsupported | belum dipetakan ke `StudyInput` |
| `plot` style/width/offset/display | unsupported | unsupported | ditolak |
| `fill`, `bgcolor`, `barcolor`, `plotshape*` | unsupported | unsupported | ditolak |
| `hline`, `label`, `box`, `table`, `line*` | unsupported | unsupported | ditolak |
| `request.*`, `import`, `strategy`, `alert` | unsupported | unsupported | ditolak |
| Seluruh Pine v6 | tidak diklaim | **partial** | hanya subset di atas yang diuji |

## 7. Lisensi dan atribusi

Runtime berasal dari Pinecone (MPL-2.0). `LICENSE` dan `NOTICE` menyebut sumber, commit awal, dan
kewajiban source saat distribusi. Repo private tidak otomatis memenuhi kewajiban distribusi.
Pine Script® adalah merek TradingView; OpineRust bukan implementasi resmi.
