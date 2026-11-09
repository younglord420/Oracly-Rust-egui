# OpineRust: status implementasi dan matriks kapabilitas

Dokumen ini adalah **laporan eksekusi** untuk [brief editor](pinecone-pine-editor-bridge.md) dan
[roadmap runtime](opinerust-runtime-fork-v6-roadmap.md). Brief tetap menjadi spesifikasi; dokumen ini
mencatat apa yang benar-benar lulus, apa yang belum, dan bukti perintahnya.

**Status keseluruhan: prototipe.** Integrasi ke runtime fork belum selesai karena perubahan runtime
masih lokal dan belum ada di origin, dan review visual browser masih terbatas.

## 1. Kondisi dua repo

| | Runtime | Bridge/Chart |
| --- | --- | --- |
| Lokasi | `/root/OpineRust` | `/root/egui-charts` |
| Origin | `younglord420/OpineRust` (private) | `younglord420/Oracly-Rust-Ugui` |
| Upstream | `ferranbt/pinecone` | — |
| HEAD | `7bb739d7` + perubahan lokal belum dikomit | perubahan lokal belum dikomit |

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

- **Status error sampai ke chart.** `CustomIndicator` kini menyimpan status dan punya
  `with_status_source`; `study_status()` milik study tidak lagi tertimpa `None` oleh pass status
  chart ketika chart tidak punya alasan sendiri. Recalculate gagal → `StudyStatus::DataError`.
- **`syminfo`/`timeframe` tidak lagi ditolak.** Sebelumnya bridge memblokir nama token tersebut
  padahal runtime mendaftarkannya. Sekarang diizinkan.
- **Metadata chart benar-benar dipasok.** `RunContext::from_chart(instrument, timeframe)` memetakan
  `Instrument` dan `Timeframe` Oracly ke `SymInfo`/`Timeframe` Pinecone. `execute_with_context` dan
  `build_study_in_context` membawa konteks ke sema **dan** runtime.
- **Bug nyata ditemukan lewat tes:** `Data::new` mengisi `syminfo` kosong, sehingga `syminfo.mintick`
  membaca `0.0` meski sema melihat nilai benar. Diperbaiki dengan `.with_syminfo(context.syminfo())`.
  Tes membuktikan `0.01` dari chart sampai ke skrip.
- Nilai/warna per bar tetap satu snapshot bersama (sudah ada sebelumnya), dan `build_study_with_clock`
  memberi jam host untuk `barstate` live.

## 4. Perintah dan hasil aktual

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

1. **Integrasi pin ke runtime fork belum selesai.** Bridge masih memakai `pine-* = "=0.2.6"` dari
   registry. Perubahan runtime belum dikomit/dipush (roadmap melarang tanpa izin), jadi `rev` git
   belum bisa menunjuk ke kode baru. Mekanisme pin sudah diuji berhasil dengan
   `CARGO_NET_GIT_FETCH_WITH_CLI=true` terhadap commit `7bb739d7`; repo private butuh kredensial.
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

## 6. Matriks kapabilitas

| Fitur | v5 | v6 | Catatan |
| --- | --- | --- | --- |
| `indicator()`, `plot()` line + warna statis | supported | supported | urutan deklarasi dipertahankan |
| Warna per bar, opacity `color.new` | supported | supported | snapshot sama dengan nilai |
| `na`/warm-up gap | supported | supported | `None` per bar |
| `ta.sma`, aritmetika, ternary | supported | supported | subset teruji native+WASM |
| `barstate.*` (forming/realtime/confirmed) | supported | supported | dari flag + jam host |
| `timenow` | supported | supported | jam host, aman WASM |
| `syminfo.*` / `timeframe.*` | supported | supported | dari chart; field tak diketahui tetap kosong |
| `input.*` | unsupported | unsupported | belum dipetakan ke `StudyInput` |
| `plot` style/width/offset/display | unsupported | unsupported | ditolak |
| `fill`, `bgcolor`, `barcolor`, `plotshape*` | unsupported | unsupported | ditolak |
| `hline`, `label`, `box`, `table`, `line*` | unsupported | unsupported | ditolak |
| `request.*`, `import`, `strategy`, `alert` | unsupported | unsupported | ditolak |
| Seluruh Pine v6 | tidak diklaim | **partial** | hanya subset di atas yang diuji |

## 7. Lisensi dan atribusi

Runtime berasal dari Pinecone (MPL-2.0). `LICENSE` dan `NOTICE` menyebut sumber, commit awal, dan
kewajiban source saat distribusi. Repo private tidak otomatis memenuhi kewajiban distribusi.
Tidak ada kode AGPL Vela-pinets yang disalin. Pine Script® adalah merek TradingView; OpineRust bukan
implementasi resmi.
