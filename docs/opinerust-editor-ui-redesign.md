# Tugas DeepSeek: redesign UI panel OpineRust Editor

**Status: implemented in the working tree (not committed); the automated suites and a real browser pass below record what was actually run.** Tugas ini khusus merapikan pengalaman editor OpineRust di demo Oracly. Baca dokumen ini seluruhnya sebelum mulai, lalu baca bagian UI dan kontrak Run pada [`pinecone-pine-editor-bridge.md`](pinecone-pine-editor-bridge.md) serta keterbatasan aktual dalam [`opinerust-status.md`](opinerust-status.md). Jika laporan historis bertentangan dengan source, periksa source dan uji perilaku sebenarnya.

## 1. Tujuan

Buat editor kanan yang terasa seperti **code editor yang menyatu dengan chart**, dengan hierarki visual sederhana: satu header ringkas, area kode yang dominan, dan Logs yang bisa dilipat. Vela adalah referensi **pola interaksi**, bukan keharusan menyalin tampilannya; ikuti bahasa desain, token, komponen, dan tema Oracly. Jangan menyalin atau mentransliterasi kode AGPL dari Vela-pinets.

Pengguna harus bisa menemukan panel, menulis script, menjalankannya, melihat hasil/error, dan kembali ke chart tanpa kehilangan draft. Redesign ini **tidak** berarti runtime siap produksi atau seluruh Pine Script® v6 didukung.

### Dalam cakupan

- Panel OpineRust di demo: akses rail/compact, header, resize, tutup, editor, Logs, diagnostik, dan keyboard.
- Konfigurasi atau pemisahan tampilan `StudyEditor` generik bila dibutuhkan, tanpa merusak host terminal atau API yang sudah ada.
- Tes otomatis, build WASM/demo, dan verifikasi visual/interaksi browser nyata.

### Di luar cakupan

- Implementasi bahasa Pine baru, perubahan semantik runtime, dukungan v6 penuh, `request.*`, strategy, drawings, atau perubahan schema study.
- File/document store, Open/Save palsu, tabs banyak, multi-document, command palette baru, autocomplete, IntelliSense, pencarian kode, atau layout manager baru.
- Worker/cancellation atau perombakan transaksi Run sebagai cara menyembunyikan masalah UI. Bila kalkulasi sinkron mengunci UI, **laporkan blocker**; jangan berpura-pura spinner berarti pekerjaan async.
- Commit, push, force-push, release, mengubah remote, atau menghapus perubahan lokal tanpa permintaan eksplisit user. **Jangan gunakan trailer `Co-authored-by` atau atribusi assistant dalam commit** jika user kemudian meminta commit.

## 2. Kondisi aktual yang harus diverifikasi lagi

Audit saat brief ini dibuat pada `main` HEAD `690001d7`; working tree bersih saat diperiksa. Kondisi dapat berubah sebelum implementasi: jalankan `git status --short`, `git diff`, dan periksa file untracked sebelum mengedit. Perlakukan semua perubahan lokal berikutnya sebagai milik user; jangan reset, bersihkan cache, atau timpa tanpa memahami diff.

- `examples/demo/controller.rs`: `SidePanel::OpineRust`, rail widget id `5`, menu compact pada viewport `< 600` logical px, `detail_panel`, dan `show_opinerust`. Panel kanan dipakai panel lain juga; jangan merusak watchlist, object tree, atau rail. `show_opinerust` sekarang mengembalikan label jika chart belum punya bar: pastikan header/close dan draft tetap dapat diakses dalam keadaan ini.
- `examples/demo/opinerust_panel.rs`: `show`, `header`, `logs_ui`, `prepare`, `install`, status dan history panel. Header, Run, lalu `StudyEditor` dirender sebagai bagian terpisah; `StudyEditor` sendiri masih menampilkan baris Title/status/unsaved. Logs saat ini diberi slot tetap `110` px bahkan ketika `CollapsingHeader` ditutup, dan hitungan error memakai pencarian teks `failed`.
- `src/ui/study_editor.rs`: editor generik dipakai juga oleh terminal. `is_modified()` membandingkan teks terhadap baseline **open/save**, bukan terhadap script yang terpasang; `apply_result()` tidak menyimpan file. `show()` memiliki Title, status, body, dan daftar diagnostik. Status sukses saat ini menyebut OpineRust di komponen generik; audit perilaku terminal sebelum mengubahnya.
- Di source `OpineRustPanel::show`, komentar mengatakan Ctrl/Cmd+Enter hanya untuk editor yang fokus, tetapi pemeriksaan yang terlihat hanya mencari event key dan modifier. **Jangan anggap pembatasan fokus sudah bekerja:** reproduksi dan uji melalui interaksi nyata.
- `describe`/`source_span` menghubungkan error engine ke byte span; ada kasus tanpa lokasi. Periksa bagaimana baris/kolom ditafsirkan runtime pada revisi yang dipin, termasuk perbedaan byte dan karakter, sebelum mengubah konversi.
- Tes dengan nama resize/undo/UI tidak selalu membuktikan semua interaksi namanya. Baca asersi aktual; tambahkan drag/klik/fokus dan observasi hasil, bukan hanya memanggil metode atau memeriksa widget ada.

### Workflow Codebase Memory

Ikuti `.cursor/rules/codebase-memory.mdc`, project `root-egui-charts`: sebelum eksplorasi gunakan `search_graph`, `trace_path`, `get_code_snippet`, atau `get_architecture`; sebelum mengedit **setiap fungsi**, jalankan `trace_path` inbound untuk mencari callers; sebelum mengutip file gunakan `check_index_coverage`. Periksa source dan diff aktual karena graph bisa stale. Saat brief dibuat indeks `2026-11-09T21:16:53Z` menandai beberapa file panel/controller/docs `metadata_changed`; jangan menganggap graph sebagai isi terbaru. Jika fungsi baru tak terindeks, laporkan gap dan cari callers pada source; reindex bila tersedia dan aman. Jangan mengklaim reindex terjadi bila tidak dilakukan.

## 3. Spesifikasi visual dan layout

Semua ukuran di bawah adalah **logical pixels egui**, target yang boleh disesuaikan token/spacing proyek. Jangan hardcode kombinasi ukuran yang menyebabkan overflow atau mengunci chart pada layar kecil. Ukur hasil sebenarnya di viewport yang tercantum pada §8.

### 3.1 Struktur dan ukuran

Panel terdiri dari tiga area vertikal:

1. **Header tetap**, sekitar `48–56` px tinggi, termasuk pemisah tipis.
2. **Editor** mengambil sisa tinggi yang tersedia setelah header dan Logs; editor minimal `80` px bila ruang memungkinkan.
3. **Logs** menempel di bawah panel; tinggi ringkasan sekitar `32–36` px. Saat dibuka, area isi awal sekitar `140` px ditambah ringkasan, dibatasi supaya editor tetap berguna. Bila viewport terlalu pendek, prioritaskan header dan scroll yang dapat dioperasikan; jangan gambar area negatif atau elemen di luar viewport.

Desktop: default lebar panel sekitar `500` px, praktis dalam rentang `320–800` px bila viewport mendukung, dan chart tetap terlihat. Panel bisa diubah lebar dengan drag tepi kiri; pointer/hover harus menunjukkan area resize, tanpa membuat isi editor ikut terseleksi. Batas maksimum menghormati viewport dan menyisakan ruang chart yang masuk akal; ukur perilaku, bukan hanya menyetel angka.

Viewport kecil `< 600` px: tetap bisa dibuka dari jalur compact yang sudah ada. Gunakan lebar yang muat pada viewport, termasuk rail/safe padding; pada layar sangat sempit panel boleh menutupi chart sementara bila perlu, tetapi user harus bisa menutupnya dan chart/draft tidak hilang. Header dan tombol tidak boleh terpotong atau memerlukan horizontal scroll panel. Tidak perlu menambah breakpoint global yang mengubah panel lain.

### 3.2 Header

Satu header konsisten dengan panel Oracly lain:

- Kiri: identitas **OpineRust** dan nama script yang relevan; kanan: tombol primer **Run** dan tombol ikon **Close** dengan accessible name `Close OpineRust editor`.
- Status ringkas ditampilkan sekali, dekat identitas, tidak mengulang banner panjang di body. Jika ruang kurang, pangkas nama dengan ellipsis dan tampilkan nama penuh pada hover/focus; tombol Run dan Close tetap terlihat dan fokus keyboard tetap masuk akal.
- Nama script terpasang berasal dari metadata hasil runtime `indicator(...)` yang benar-benar diterapkan. Sebelum Run berhasil, tampilkan `Draft` atau nama dokumen fallback yang jujur; **jangan parse teks draft dengan regex lalu menyajikannya sebagai metadata terverifikasi**. Tidak ada input Title terpisah khusus OpineRust: nama study berasal dari script saat runtime berhasil. Jangan mengubah title field/default host terminal.
- Tinggi target tombol `28–32` px, padding mengikuti token proyek; target pointer/touch cukup besar pada compact. Gunakan satu tombol primer, bukan dua tombol Apply/Run tersembunyi yang menjalankan jalur berbeda.
- Tooltip/deskripsi Run: `Run script · Ctrl+Enter` pada Windows/Linux dan `Run script · Cmd+Enter` pada macOS bila platform bisa diketahui; label generik `Ctrl/Cmd+Enter` boleh dipakai bila platform tidak diketahui.
- Tombol Close hanya menutup panel, tidak menghapus study atau draft, tidak menyamakan Run dengan Save. Tutup melalui `DemoApp`/mekanisme host yang benar; jangan hanya menyembunyikan isi panel atau merusak state active rail.
- Nonaktifkan Run dan tampilkan alasan singkat saat chart tidak memiliki bars atau prasyarat lain tidak tersedia. Saat pekerjaan **benar-benar** ongoing dan sistem dapat menerima frame, boleh tampilkan `Running…` dan cegah request kedua. Kalkulasi sinkron yang memblokir frame tidak boleh dipasarkan sebagai async/cancellable; jika status ini tidak dapat terlihat/diuji secara nyata, pertahankan perilaku jujur dan catat keterbatasannya.

### 3.3 Status dan model draft

Jangan gabungkan dua fakta berbeda: apakah draft sama dengan source yang terpasang dan apakah teks sudah disimpan di file. Demo tidak memiliki document store, jadi **jangan** tampilkan Open/Save aktif atau menyebut script telah disimpan. `Run`/`Applied` tidak memanggil `mark_saved` dan tidak boleh menghapus semantik `is_modified()` bagi terminal.

Status satu lokasi dengan prioritas deterministik:

- `Error`/`Stale`: kegagalan compile/install/recalculate terbaru yang masih relevan. Jika hasil lama tetap digambar, jelaskan singkat bahwa hasil sebelumnya dipertahankan dan dapat stale; jangan tampilkan sebagai output baru yang sukses.
- `Applied`: study script ini benar-benar dipasang dan draft sesuai source yang dipasang; baru setelah host konfirmasi.
- `Edited`: ada study terpasang, tetapi draft berbeda dari source terpasang; jelaskan bahwa chart masih memakai versi sebelumnya.
- `Draft`: belum ada study panel yang terpasang; termasuk setelah study dihapus/undo bila memang tidak ada yang dapat diidentifikasi.

Pilih satu prioritas saat beberapa kondisi berlaku dan tulis tes untuk konflik (mis. edited + recalculation error, failed Run setelah Applied, recovery sukses). Bedakan error kandidat dari status study lama; jangan menutupi kegagalan yang relevan dengan label sukses. Update/reconcile status setelah perubahan chart, undo/redo, study dihapus, panel ditutup/dibuka. Seluruh identitas dan watcher harus mengikuti study yang benar; jangan mengandalkan nama yang dapat sama. Jangan menampilkan `Unsaved changes` dua kali atau mengklaim ada file tersimpan.

### 3.4 Area editor

- Isi area editor dengan font monospace dari sistem font/theme proyek; ukuran target `13–14` logical px dan line height terbaca (`~1.4–1.6`), tanpa zoom yang memotong glyph. Latar, teks, selection, focus ring dan border mengambil token light/dark Oracly.
- Gutter kiri menampilkan nomor baris mulai dari `1`, selaras dengan baseline teks; lebar bertambah saat jumlah digit bertambah. Pemisah gutter tipis dan kontras rendah. Beri padding kode sekitar `8–12` px dan jarak gutter yang cukup; jangan membuat setiap baris menjadi widget input terpisah.
- Baris aktif boleh diberi highlight halus bila posisi cursor dan viewport dapat ditentukan benar. Highlight tidak boleh mengurangi kontras selection atau menyamarkan diagnostik.
- Kode panjang: jangan wrap diam-diam. Sediakan horizontal dan vertical scrolling yang hanya memengaruhi editor. Gutter dan kode harus tetap sinkron saat vertical scroll; horizontal scroll tidak menggeser gutter. Perhitungkan tab, font fallback, newline final, CRLF, baris kosong, baris sangat panjang, Unicode termasuk emoji/combining characters, dan perubahan DPI. Bila implementasi egui tidak bisa menjaga alignment/performance, laporkan blocker; jangan menggambar nomor baris yang salah.
- Pertahankan undo/redo teks bawaan editor, selection, paste dan clipboard sesuai kemampuan host. `Tab` menyisipkan indent dan `Shift+Tab` mengurangi indentasi untuk selection multi-baris; jangan merusak keyboard focus traversal saat fokus di luar editor. Definisikan ukuran indent (empat spasi sebagai default) dan uji awal/akhir selection, mixed tabs/spaces, CRLF, dan undo satu langkah. Jika constraint toolkit mencegah hal ini, dokumentasikan sebagai blocker, bukan menggantinya dengan perilaku berbeda diam-diam.
- Ctrl+Enter (Windows/Linux) / Cmd+Enter (macOS) menjalankan draft **hanya saat editor kode fokus**; tombol Run selalu bisa diaktifkan dengan pointer/keyboard ketika fokusnya di tombol. Shortcut tidak boleh ikut menambah newline, dieksekusi dua kali, atau memicu chart shortcut. Mengetik biasa dan Ctrl+A/C/V/Z di editor tidak boleh menggerakkan chart atau memicu action chart. Di luar editor, Ctrl+Enter tidak boleh menjalankan script secara diam-diam.
- Syntax highlighting token dasar adalah **opsional fase lanjutan**, bukan syarat menipu sebagai sudah selesai. Jika diimplementasikan, ukur biaya pada file besar, pertahankan selection/IME, warna tema dan lexer error; source mentah tidak boleh berubah. Jangan gunakan parser versi tertentu untuk mengubah isi draft. Autocomplete bukan cakupan.

### 3.5 Logs dan diagnostik

- Ringkasan bawah selalu terlihat: `Logs`, chevron buka/tutup, dan count error/warning yang berasal dari metadata severity yang benar. Tidak ada substring-matching seperti mencari `failed` untuk menentukan severity. Count nol tampil ringkas; jangan pakai warna sebagai satu-satunya sinyal.
- Saat collapsed, isi **tidak** memesan tinggi 110/140 px. Saat expanded, isi punya scroll vertikal sendiri, tinggi awal `~140` px, dibatasi tinggi yang tersedia. Preserve state expanded saat close/reopen panel; pastikan resize panel tidak menutupi kode/header. Periksa daftar banyak pesan tetap bisa discroll dan header ringkasan tetap bisa ditutup.
- Tampilkan baris pesan `Error`, `Warning`, `Info` dengan teks jelas dan lokasi `line:column` hanya bila benar-benar tersedia. Pesan runtime tanpa lokasi tidak memiliki link/lokasi palsu. Tekan/click diagnostik yang punya lokasi memilih span pada source aktual, scroll ke sana dan fokus editor; row harus bisa diakses keyboard dengan label/nama bermakna.
- Gunakan satu sumber diagnostik yang jelas untuk Logs dan navigasi: hindari error sama tampil dua kali sebagai banner, daftar editor dan log. Log non-diagnostik seperti Run/replace boleh ditampilkan sebagai info ringkas, bukan memenuhi panel setiap frame. Dedup recalculation error tanpa mengonsumsi slot status chart; recovery harus terlihat dan error lama tidak dianggap aktif selamanya.
- Lokasi terikat **revisi source** saat diagnosa diproduksi. Setelah draft berubah, jangan memilih byte span lama seolah akurat; hapus/tandai stale atau perbarui berdasarkan revisi, dengan kebijakan dan tes eksplisit. Konversi dan navigasi harus benar untuk UTF-8, CRLF, final line tanpa newline, kolom di tengah multibyte/out-of-range, dan error tanpa lokasi; penanganan clamp tidak boleh panic.

## 4. Kontrak host dan keselamatan regresi

- Panel adalah pilihan di dock kanan, bukan modal atau pengganti seluruh chart. Rail dan menu compact harus membuka panel yang sama, toggle dan close konsisten; switching ke panel lain tidak membuang draft ataupun study.
- Header, Logs, dan area editor jangan memodifikasi OHLCV, symbol/timeframe, konfigurasi study, atau panel lain. Jangan mengubah runtime atau pin dependency sebagai bagian redesign tanpa kebutuhan yang terbukti dan persetujuan.
- Pertahankan lifecycle `prepare → chart install → confirm` dan perilaku Run ulang/failed Run. Perubahan visual tidak boleh membuat `Applied` muncul ketika install gagal, membuat indikator duplikat, menguras `last_error`, atau menghapus hasil sebelumnya pada kegagalan. Reproduksi kondisi nyata; bila rollback/history belum memenuhi klaim, perbaiki dalam cakupan yang relevan atau laporkan blocker, **jangan klaim atomik hanya karena ada tes**.
- UI harus tetap fungsional bila chart kosong: draft terlihat dan bisa diedit, Run disabled disertai alasan, close tetap tersedia; ketika data tiba Run bisa kembali dipakai.
- `StudyEditor` juga digunakan terminal (`terminal/src/trading.rs`, periksa callers aktual): setelan khusus OpineRust harus opt-in/configurable atau dipisah, default terminal mempertahankan Title/Open/Save/Apply, status generik, aksi, fokus, dan diagnostics sesuai kontrak host terminal. Jangan menghapus `StudyEditorAction` atau mengubah semantik `mark_saved` demi demo.
- Clock replay, cancellation/performance runtime dan interoperabilitas penuh Pine v6 tetap keterbatasan terpisah. Jangan mengatakan masalah ini selesai karena panel terlihat baik.

## 5. Urutan implementasi yang diminta

1. **Baseline:** baca CBM + source aktual, `git status/diff`, kedua dokumen sumber, tes panel/editor/controller serta host terminal. Catat baseline layout dan issue nyata. Gunakan browser pada demo yang sedang berjalan **hanya setelah memastikan build/served bundle**, jangan hentikan atau ubah server user; pakai port dan dist terpisah. Catat viewport/theme dan screenshot yang benar-benar terbaca.
2. **Pisahkan view/state:** tentukan ownership title, installed source, dirty status, diagnostic severity/location/revision, logs expanded, fokus dan requested close. Hindari status ganda pada header dan `StudyEditor`; pertahankan API dan perilaku terminal. Pastikan penanganan no-bars tidak membuat panel/close hilang.
3. **Bangun layout header/editor/logs:** gunakan token/komponen yang sudah ada. Implementasikan resize/close yang benar dalam host dan sesuaikan akses compact. Pastikan ukuran responsive, scroll independen, serta gutter yang selaras dengan teks.
4. **Keyboard dan diagnosis:** terapkan pembatasan fokus dan shortcut platform, indentasi, scroll/reveal yang byte-accurate, severity terstruktur dan revisi diagnosa. Jangan memperkenalkan lokasi palsu.
5. **Integrasi Run:** uji first Run, replace, failed compile/install/recalculate, close/reopen, chart kosong dan perubahan chart. Pastikan UI menunjukkan state aktual, termasuk hasil lama yang stale.
6. **Verifikasi** seluruh gerbang §6–§8 dan perbarui `docs/opinerust-status.md` hanya dengan hasil yang benar-benar dibuktikan. Jangan menulis klaim browser atau kelengkapan v6 berdasarkan hasil historis.

## 6. Tes otomatis wajib

Buat regresi yang menguji perilaku, bukan hanya keberadaan label. Gunakan tes existing dan harness `egui`/`egui_kittest` bila sesuai; nama tes boleh berbeda asalkan asersinya mencakup hal berikut.

### Panel dan interaksi

- Desktop rail dan compact menu membuka panel; active state cocok; klik close/rail mengembalikan chart; draft dan study bertahan. Dengan chart kosong, editor/close tetap tersedia dan Run tidak menjalankan script.
- Simulasikan drag tepi kiri pada panel yang terbuka, verifikasi lebar panel berubah dan chart masih dapat digunakan; cek min/max dan resize kembali. Tes close/reopen draft **bukan** pengganti tes drag.
- Header hanya punya satu Run dan satu status utama; title panjang dipangkas tanpa menyembunyikan tombol, termasuk viewport sempit dan dark/light. Tidak ada field Title, Open/Save palsu atau duplikasi unsaved di host OpineRust.
- Logs collapsed benar-benar menghemat tinggi; expand → scroll banyak entri → collapse; counts severity akurat, error lama/recovery tidak hilang dari chart.

### Editor, keyboard, lokasi

- Teks panjang, `\n`, `\r\n`, Unicode multibyte/emoji/combining, selection lintas baris, insert/delete, scroll horizontal/vertical, dan perubahan jumlah digit gutter; verifikasi alignment terhadap glyph/viewport yang relevan.
- Fokus editor vs chart vs header: Ctrl/Cmd+Enter hanya sekali saat fokus editor, bukan saat chart fokus; tidak menyisipkan newline, tombol Run dengan keyboard juga berfungsi. Tab/Shift+Tab dan undo/redo teks, shortcut chart yang tidak boleh bocor; uji paste/IME sejauh toolkit memungkinkan.
- Klik dan keyboard activation pada diagnostik: posisi line/column yang benar, scroll dan selection byte boundaries untuk Unicode/CRLF/final line; invalid/out-of-range aman; tanpa lokasi tidak dapat diklik sebagai lokasi. Edit draft setelah error: span lama tidak menavigasi posisi salah.
- Unit-test severity terstruktur/count; kasus pesan mengandung `failed` tetapi bukan error dan error tanpa kata itu. Uji satu sumber diagnostik, bukan duplikasi error yang menghabiskan ruang.

### Lifecycle dan host lain

- A sukses → B sukses mengganti → C gagal mempertahankan B, termasuk label status, logs, id/kontrak identity, history dan draft. Failed install setelah preflight tidak memunculkan Applied. Uji error update, panel tertutup dan recovery sukses.
- Perubahan label/tampilan `StudyEditor` untuk OpineRust tidak mengubah default terminal: `Open`, `Save`, `Apply`, perubahan saved baseline, title, diagnostics dan aksi tetap bekerja. Tes host terminal yang relevan, bukan hanya unit test generik.
- Uji performa interaksi pada script representatif (mis. 1.000 dan 10.000 baris) serta garis sangat panjang; catat platform, waktu input/layout/scroll, penggunaan memori bila bisa diukur, dan batas yang dipakai. Jangan mengklaim performa atau editor responsif jika tes tidak dijalankan. Runtime Run sinkron adalah isu terpisah dari mengetik/render editor.

## 7. Perintah dan verifikasi build

Konfirmasi toolchain/dependencies dan kondisi repo lebih dulu. Contoh perintah, sesuaikan dengan workspace aktual dan jelaskan setiap kegagalan:

- `cargo fmt --all -- --check` di root; cek workspace demo/bridge juga bila memiliki manifest sendiri.
- `cargo test --lib -- study_editor`, serta tes terarah komponen dan controller yang diubah.
- `cargo test --manifest-path examples/demo/Cargo.toml --bins`.
- Tes workspace terminal yang relevan setelah memeriksa `terminal/Cargo.toml` dan feature yang tersedia.
- `cargo test --manifest-path opinerust/Cargo.toml`, termasuk fixture yang relevan jika lifecycle Run/diagnostik disentuh.
- `cargo clippy` untuk target dan fitur yang berubah; cantumkan warning yang tersisa, jangan sembunyikan pipeline exit. Jika output dipipe, gunakan `set -o pipefail` atau simpan exit code perintah pertama.
- Build demo WASM **setelah perubahan terakhir** dengan `trunk` pada dist terpisah; bandingkan SHA-256 file lokal dengan bytes yang benar-benar disajikan server. Catat command, cwd, exit, port, file, hash, browser/version, dan apakah runtime berasal dari pin fork yang diharapkan. Jangan mengubah remote atau menghentikan server user.

Tidak perlu menjalankan tes panjang yang tidak relevan tanpa alasan, tetapi jika suatu gerbang tidak dijalankan, tulis jelas sebagai belum diverifikasi. Jangan menyalin angka atau timestamp hasil tes sesi lama menjadi bukti baru.

## 8. Review visual dan browser nyata — wajib sebelum menyatakan selesai

Buka demo yang menyajikan **bundle terakhir yang sudah dibuktikan hash-nya**, di browser nyata. Capture yang hanya menampilkan background putih atau tidak bisa dibaca bukan bukti. Jika WebGL tidak tercapture dengan cara biasa, coba jalur capture yang didukung lingkungan; bila tetap tidak terbaca, laporkan visual sebagai blocker. Jangan menyimpulkan study terpasang hanya dari perbedaan jumlah piksel atau HTTP 200: baca label/log, nilai/plot dan interaksi yang benar-benar terlihat atau verifikasi dengan observasi/test seam yang andal; bedakan ini dari screenshot visual.

Minimal untuk light/dark:

- Desktop `1440×900`, intermediate `800×700` dan compact `360×640`; cek juga `320×360` untuk overflow. Catat ukuran **viewport aktual**, bukan hanya ukuran window.
- Buka dari rail/menu compact, resize dari kiri, close/reopen dan verifikasi teks yang terlihat tetap ada. Confirm chart/panel lain tidak rusak.
- Tulis/run v5 dua plot, Run ulang dengan hasil berubah, Run v6 subset yang telah ada fixture, lalu unsupported/error. Verifikasi chart, nilai/warna/gap sejauh script mendukungnya, nama/status, Logs, lokasi yang diklik, serta error tidak menghilangkan study lama. Jika satu langkah tidak dapat diverifikasi secara visual atau output nyata, catat blocker dan buktinya.
- Cek fokus keyboard, shortcut di dalam/luar editor, scroll kode panjang, line numbers saat scroll, tema/kontras, panel kosong, log collapsed/expanded dan tidak tertutup header/footer. Jangan menilai `Running…` dari screenshot yang dibuat-buat.
- Periksa console error/panic selama langkah. Sertakan screenshot yang terbaca dan jelaskan apa yang benar-benar dilihat. Jika screenshot memuat data sensitif, redaksi sebelum dibagikan.

## 9. Definition of done

Selesai hanya jika seluruh poin berikut terbukti:

1. Panel kanan tampil dengan satu header, editor dominan dengan gutter/scroll selaras, dan Logs yang benar-benar collapsible; desktop, compact, tema light/dark telah diperiksa.
2. Draft, nama dan status jujur; Run bukan Save; tidak ada Open/Save palsu atau Applied sebelum konfirmasi chart. No-bars, error, recovery, undo/redo, close/reopen dan lifecycle study tetap benar.
3. Navigasi diagnostik severity/lokasi/revisi akurat, keyboard/fokus dan interaksi editor berfungsi, dan terminal tetap memakai kontrak generik tanpa regresi.
4. Tes relevan dan build terakhir dilaporkan dengan exit code; browser nyata membuktikan interaksi/output dan screenshot terbaca. Keterbatasan visual, runtime, performance, private dependency dan dukungan v6 dinyatakan tanpa klaim berlebihan.
5. Tidak ada perubahan lokal user tertimpa dan tidak ada commit/push tanpa permintaan eksplisit. Tidak ada co-author trailer jika user meminta commit nantinya.

Jika salah satu gerbang terhambat oleh toolkit, runtime, akses private, browser atau keterbatasan arsitektur, status tetap **belum selesai**: jelaskan reproduksi, dampak, apa yang sudah dibuktikan, dan langkah tersisa.

## 10. Laporan akhir untuk user

Ringkas dalam bahasa Indonesia: apa yang berubah dan file utama; hasil tes dengan perintah/jumlah/exit; hasil browser dengan viewport/theme/hash/screenshot; keterbatasan dan langkah tersisa; serta status git (tanpa commit/push kecuali diminta). Jangan menyebut redesign selesai atau Pine v6 penuh apabila gerbang di atas belum terpenuhi.

## 11. Hasil implementasi (diperbarui)

Bagian ini mencatat apa yang benar-benar dikerjakan dan diukur. Jika bertentangan dengan kode, percayai kode dan jalankan ulang tesnya.

### 11.1 Perubahan

- `src/ui/study_editor.rs`: `set_chrome(false)` menyembunyikan Title, catatan modified dan banner status untuk host yang punya header sendiri; `set_line_numbers(true)` menggambar nomor baris per visual row dari galley (bukan dari asumsi line height), memakai clip rect teks sehingga ikut scroll vertikal dan tidak ikut scroll horizontal. `mark_applied`/`is_dirty_against` memisahkan "draft ≠ yang chart jalankan" dari "file belum disimpan"; `mark_saved` tetap satu-satunya yang membersihkan baseline file. Default host lain (terminal) tidak berubah: chrome tetap tampil.
- `src/ui/study_editor.rs`: `text_id(ui)` memberi id stabil berbasis `scope_id` supaya panel bisa memeriksa fokus.
- `examples/demo/opinerust_panel.rs`: satu header (identitas, status, Run, Close), `RunState` tunggal (`Draft`/`Applied`/`Edited`/`Failed`/`NoData`) dengan prioritas failure > installed, `last_failure` yang dibersihkan saat recalculation sukses, Logs dengan ringkasan satu baris yang benar-benar hemat tinggi (32 px collapsed, +140 px saat dibuka, dibatasi 60% panel), severity terstruktur (`LogEntry { severity, message }`) alih-alih mencocokkan substring `failed`, dan `show_with_close` yang mengembalikan permintaan close ke host. Error otomatis membuka Logs.
- `examples/demo/controller.rs`: `show_opinerust` menangani close (rail/menu active state ikut dibersihkan) dan tetap menggambar panel saat chart belum punya bars, sehingga draft dan tombol Close tetap dapat dipakai.

### 11.2 Yang diukur

| Perintah | Hasil |
| --- | --- |
| `cargo test --manifest-path examples/demo/Cargo.toml --bins` | 104 lulus, 0 gagal, 1 ignored |
| `cargo test --lib` | 1184 lulus, 0 gagal, 1 ignored |
| `cargo clippy --lib` | tanpa warning |
| `cargo fmt --all` | bersih |
| `trunk build --dist /tmp/opinerust-ui-dist` | sukses; `sha256 3656b346…a777ee`, hash sama dengan bytes yang disajikan |
| Chromium 1248 headless via CDP, light & dark, 1440×900 | dimuat tanpa exception; panel terbuka dari rail; Run (x≈1320,y≈58) mengubah area chart **29.974 px** dengan noise floor **0 px** |

Tes baru yang menutup perilaku redesign: `chrome_can_be_hidden_without_touching_the_file_baseline`, `the_dock_chrome_is_absent_and_the_generic_chrome_stays`, `line_numbers_follow_the_rows_they_belong_to`, `a_failure_shows_then_clears_when_the_study_recovers`, `collapsed_logs_cost_only_the_summary_row`, `the_header_close_button_asks_the_host_to_close`, `the_dock_fits_a_narrow_viewport`.

### 11.3 Keterbatasan yang masih terbuka

- **Tautan `docs/icon`/`Populate-state` pada panel compact belum diverifikasi lewat screenshot**: capture compact gagal menargetkan menu rail, jadi perilaku sempit dibuktikan lewat tes frame egui (`the_dock_fits_a_narrow_viewport`), bukan visual browser.
- **Pada jendela 360 px dock boleh menutupi chart** saat dibuka; tes membuktikan Close mengembalikan lebar chart. Upaya menahan lebar panel lewat `max_size` tidak berlaku karena egui memulihkan lebar hasil drag, dan memaksa reset lebar tersimpan dianggap bukan solusi yang layak.
- **Ctrl+Enter hanya saat fokus editor**: tes panel menargetkan area teks secara tepat; tes host memakai tombol Run karena klik pointer di dalam panel 1440 px tidak dapat ditargetkan secara andal oleh harness.
- **Belum dikerjakan dari brief**: syntax highlighting, indentasi multi-baris Shift+Tab khusus (perilaku bawaan `TextEdit` dipakai apa adanya), dan penandaan span diagnostik sebagai stale setelah draft berubah.
