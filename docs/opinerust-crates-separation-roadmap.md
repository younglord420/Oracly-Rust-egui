# Rencana Kerja & Roadmap: Pemisahan Modular Crate `oracly-charts` dan `opinerust` untuk Publikasi Crates.io

**File**: `docs/opinerust-crates-separation-roadmap.md`  
**Target Pelaksana**: DeepSeek / Automated Agent  
**Tujuan Utama**: Memisahkan kode inti chart (`oracly-charts`) agar 100% mandiri dan independen, serta memindahkan seluruh komponen Pine Script Studio Editor (`StudyEditor`, `PineReference`, `OpineRustPanel`) ke dalam crate add-on terpisah (`opinerust`) agar kedua crate siap dipublikasikan secara terpisah ke [crates.io](https://crates.io/).

---

## 1. Latar Belakang & Analisis Arsitektur

### Masalah Arsitektur Saat Ini
1. **`Oracly-charts` (Root Crate)**:
   - Di `src/ui/mod.rs`, terdaftar modul `study_editor` dan `pine_reference`.
   - Developer yang hanya ingin menggunakan candlestick chart, drawing tools, atau indikator teknikal bawaan ikut terbebani kompilasi editor teks Pine Script, lexer token warna, dan referensi built-in TradingView.
2. **`opinerust` (Folder `opinerust/`)**:
   - Berfungsi sebagai bridge engine Pinecone ke `Oracly-charts`.
   - Belum menyediakan UI editor mandiri karena panel editornya (`OpineRustPanel`) masih diletakkan di dalam `examples/demo/opinerust_panel.rs`.
3. **`examples/demo` (Aplikasi Demo)**:
   - Mengambil `egui-charts` dan `opinerust`, lalu membungkusnya ke dalam demo live chart dengan dock panel editor.

### Target Arsitektur Baru
```text
┌─────────────────────────────────────────────────────────────┐
│                    examples/demo (App)                      │
│     (Mengonsumsi oracly-charts + opinerust sebagai UI dock) │
└──────────────┬───────────────────────────────┬──────────────┘
               │                               │
               ▼                               ▼
┌──────────────────────────────┐ ┌────────────────────────────┐
│      opinerust (Crate)       │ │    oracly-charts (Crate)   │
│  - Pinecone Runtime Bridge   │ │  - Candlestick & Volume    │
│  - OpineRustPanel (UI Dock)  │─┼─>│  - Technical Studies    │
│  - StudyEditor (Canvas UI)   │ │  - Drawing Tools           │
│  - PineReference (LSP Docs)  │ │  - Order Flow & Footprint  │
└──────────────────────────────┘ └────────────────────────────┘
```

---

## 2. Rincian Langkah Refactoring (Step-by-Step)

### Langkah 1: Pindahkan Komponen Editor ke Crate `opinerust`
1. Pindahkan file dari root repo ke dalam `opinerust/src/ui/`:
   - `src/ui/study_editor.rs` $\rightarrow$ `opinerust/src/ui/study_editor.rs`
   - `src/ui/pine_reference.rs` $\rightarrow$ `opinerust/src/ui/pine_reference.rs`
   - `examples/demo/opinerust_panel.rs` $\rightarrow$ `opinerust/src/ui/panel.rs`
2. Buat file `opinerust/src/ui/mod.rs`:
   ```rust
   pub mod panel;
   pub mod pine_reference;
   pub mod study_editor;

   pub use panel::{InstallStudy, OpineRustPanel};
   pub use study_editor::{StudyEditor, StudyEditorAction};
   ```
3. Di `opinerust/src/lib.rs`, ekspos modul UI:
   ```rust
   pub mod ui;
   pub use ui::{InstallStudy, OpineRustPanel, StudyEditor, StudyEditorAction};
   ```

### Langkah 2: Bersihkan Root Crate `Oracly-charts`
1. Di `src/ui/mod.rs`:
   - Hapus baris `pub mod pine_reference;`
   - Hapus baris `pub mod study_editor;`
2. Hapus file lama `src/ui/study_editor.rs` dan `src/ui/pine_reference.rs` dari root repo.
3. Pastikan `Oracly-charts` tidak lagi memiliki referensi atau dependensi terhadap sintaks dan dokumentasi Pine Script.

### Langkah 3: Konfigurasi Metadata `opinerust/Cargo.toml`
Lengkapi metadata untuk persiapan publish ke crates.io:
```toml
[package]
name = "opinerust"
version = "0.1.0"
edition = "2021"
license = "MPL-2.0"
description = "OpineRust: Pine Script v5/v6 runtime bridge and modern studio editor dock for Oracly charts"
repository = "https://github.com/younglord420/Oracly-Rust-egui.git"
homepage = "https://oracl.io"
keywords = ["trading", "pinescript", "charts", "egui", "finance"]
categories = ["visualization", "gui", "finance"]
readme = "README.md"

[dependencies]
Oracly-charts = { version = "0.3.0", path = "..", default-features = false, features = [
    "widget",
    "studies",
    "icons",
    "ui"
] }
egui = { version = "0.37.0", default-features = false }
chrono = { version = "0.4.43", features = ["serde"] }

# Pinecone Core Fork crates
pine-core = { git = "ssh://git@github.com/younglord420/OpineRust.git", rev = "25f9e41ce8de6f8167ef9c1b0c661a46f274d71f" }
pine-lexer = { git = "ssh://git@github.com/younglord420/OpineRust.git", rev = "25f9e41ce8de6f8167ef9c1b0c661a46f274d71f" }
pine-parser = { git = "ssh://git@github.com/younglord420/OpineRust.git", rev = "25f9e41ce8de6f8167ef9c1b0c661a46f274d71f" }
pine-interpreter = { git = "ssh://git@github.com/younglord420/OpineRust.git", rev = "25f9e41ce8de6f8167ef9c1b0c661a46f274d71f" }
pine-builtins = { git = "ssh://git@github.com/younglord420/OpineRust.git", rev = "25f9e41ce8de6f8167ef9c1b0c661a46f274d71f" }
pine-sema = { git = "ssh://git@github.com/younglord420/OpineRust.git", rev = "25f9e41ce8de6f8167ef9c1b0c661a46f274d71f" }
pine-diagnostics = { git = "ssh://git@github.com/younglord420/OpineRust.git", rev = "25f9e41ce8de6f8167ef9c1b0c661a46f274d71f" }

[target.'cfg(target_arch = "wasm32")'.dev-dependencies]
wasm-bindgen-test = "=0.3.80"

[workspace]

[lib]
name = "opinerust"
path = "src/lib.rs"
```

### Langkah 4: Sesuaikan Demo App (`examples/demo`)
1. Di `examples/demo/main.rs` dan `examples/demo/controller.rs`:
   - Ganti `mod opinerust_panel;` dan `use crate::opinerust_panel::OpineRustPanel;` menjadi `use opinerust::OpineRustPanel;`.
2. Hapus file `examples/demo/opinerust_panel.rs` (karena sudah menjadi bagian dari crate library `opinerust`).

---

## 3. Cara Penggunaan oleh End-User Setelah Dipublikasikan

### Skenario A: User Hanya Butuh Chart Biasa
User cukup menambahkan satu baris di `Cargo.toml`:
```toml
[dependencies]
oracly-charts = "0.3"
```
```rust
use egui_charts::Chart;

let mut chart = Chart::new(bar_data);
chart.draw(ui);
```

### Skenario B: User Ingin Menambahkan Pine Script Studio
User menambahkan crate add-on `opinerust`:
```toml
[dependencies]
oracly-charts = "0.3"
opinerust = "0.1"
```
```rust
use egui_charts::Chart;
use opinerust::OpineRustPanel;

let mut panel = OpineRustPanel::default();
let (installed, closed) = panel.show_with_close(ui, &bars, now_ms);
if let Some(study) = installed {
    chart.add_indicator(study.study);
}
```

---

## 4. Checklist Verifikasi & Pengujian (Quality Gate)

Sebelum dinyatakan selesai oleh pelaksana (DeepSeek):
- [ ] **Kompilasi Core Library**:
  ```bash
  cargo check --lib
  ```
  Harus lulus tanpa error atau warning modul hilang.
- [ ] **Unit Tests Core Library**:
  ```bash
  cargo test --lib
  ```
  Seluruh suite tes internal chart harus lulus.
- [ ] **Kompilasi & Tes OpineRust**:
  ```bash
  cargo test -p opinerust
  ```
  Harus lulus semua (termasuk tes UI studio, editor, dan v6 fixtures).
- [ ] **Demo WASM Rebuild**:
  `trunk serve --release --port 8081` harus berhasil build dan reload tanpa runtime panic.
- [ ] **Git Commits**:
  Tidak boleh menyertakan `Co-authored-by` trailer sama sekali.

---

## 5. Catatan Pelaksanaan (jangan ubah, ini rekam jejak)

Pemisahan ini **sudah dieksekusi sekali**; bagian ini mencatat apa yang dikerjakan
dan apa yang harus dijaga bila diulang.

### Yang dipindahkan
| Asal | Tujuan |
| --- | --- |
| `src/ui/study_editor.rs` | `opinerust/src/ui/study_editor.rs` |
| `src/ui/pine_reference.rs` | `opinerust/src/ui/pine_reference.rs` |
| `examples/demo/opinerust_panel.rs` | `opinerust/src/ui/panel.rs` |

### Perubahan yang ikut wajib
1. `src/ui/mod.rs` tidak lagi mendeklarasikan `study_editor` maupun `pine_reference`.
2. `opinerust/src/ui/mod.rs` mengekspor `panel`, `pine_reference`, `study_editor`
   serta `OpineRustPanel`, `InstallStudy`, `StudyEditor`, `StudyEditorAction`.
3. Seluruh path `crate::ui_kit::*`, `crate::theme::*`, `crate::localization` di file
   yang dipindah berganti menjadi `Oracly_charts::*`; `crate::ui::pine_reference`
   menjadi `crate::ui::pine_reference` milik crate baru.
4. `opinerust/Cargo.toml` menambah fitur `studio` (default aktif) dan fitur `ui`
   pada dependency `Oracly-charts`.
5. **`opinerust` butuh `egui` dengan `default_fonts` sebagai `dev-dependencies`.**
   Contohnya: tes `line_numbers_follow_the_rows_they_belong_to` menggambar frame
   nyata dan membaca posisi gutter; tanpa metrik font semua lebar nol sehingga
   tes gagal (`number 1 is clipped away`) lalu proses abort saat drop.
6. `opinerust` crate ini masih `edition = "2021"`, jadi **tidak boleh** memakai
   let-chains (`if cond && let Some(x) = ...`). Tulis bersarang.
7. `examples/demo` memakai `opinerust::OpineRustPanel` langsung; modul
   `opinerust_panel` dihapus dari `main.rs`.
8. Crate `terminal/` ikut bergantung pada `opinerust` untuk `StudyEditor`;
   crate chart **tidak** boleh memuat UI editor.

### Hasil verifikasi pada pelaksanaan pertama
- `cargo test --lib` (chart crate): **1179 lulus, 1 ignored** — tidak ada regresi
  dari pemindahan.
- `cargo test` di `opinerust`: **117 lulus** (94 lib + suite integrasi).
- `cargo check` di `examples/demo` dan `terminal/`: lulus.

### Yang belum
- `oracly-charts` belum dipublikasikan ke crates.io, dan `opinerust` masih
  mengunci runtime lewat `rev` Git privat (bukan crate `pine-*` dari registry).
  Publish ke crates.io menuntut runtime tersedia di registry dulu, atau bundle
  source runtime seperti keputusan lisensi berikutnya.

