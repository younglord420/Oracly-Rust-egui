# Tugas DeepSeek: panel OpineRust mengikuti referensi dan audit Pine Script® v6

**Status: rencana implementasi, belum dikerjakan.** User meminta panel OpineRust yang lebih matang seperti screenshot referensi, log error yang tidak bertumpuk, dan peningkatan kompatibilitas script Pine Script v6 yang nyata. Baca dokumen ini seluruhnya sebelum mulai. Ini melanjutkan, bukan menghapus, kontrak di [`opinerust-editor-ui-redesign.md`](opinerust-editor-ui-redesign.md), [`pinecone-pine-editor-bridge.md`](pinecone-pine-editor-bridge.md), dan [`opinerust-runtime-fork-v6-roadmap.md`](opinerust-runtime-fork-v6-roadmap.md).

Screenshot referensi yang user kirim di percakapan menunjukkan editor kode gelap dengan **nama script dan Run di header, gutter nomor baris, syntax highlighting, minimap di kanan, scrollbar, serta strip Logs di bawah**. Itu acuan hierarki dan perilaku; gunakan warna, typography, icon, komponen, dan token **Oracly** sendiri. Screenshot bukan bukti bahwa semua kontrolnya harus ditiru atau bahwa implementasi referensi bebas lisensi untuk disalin. Jangan menyalin atau mentransliterasi kode AGPL, termasuk Vela-pinets.

## 1. Instruksi penting dan keselamatan

- Kerjakan berurutan: **P0 perbaiki log yang bertumpuk → P1 rapikan editor sesuai referensi → P2 audit/perbaiki dukungan v6 berdasarkan fixture nyata → P3 verifikasi**. Jangan menutupi bug log dengan sekadar mengecilkan font atau menyembunyikan semua error.
- **Dua repo:** chart/bridge `/root/egui-charts`; runtime fork `/root/OpineRust`. Bridge menggunakan crate `pine-*` dari Git fork pada revisi yang dipin manifest/lockfile, bukan otomatis memakai perubahan lokal runtime. Sebelum bekerja cek HEAD, `git status --short`, branch, remote dan diff pada **masing-masing** repo. Perlakukan semua perubahan lokal sebagai milik user; jangan reset, `cargo clean`, hapus cache, timpa atau membuang perubahan tanpa izin.
- Saat brief ini ditulis, `egui-charts` HEAD `b0cc767c` dengan perubahan lokal di `examples/demo/controller.rs`, `examples/demo/opinerust_panel.rs`, `src/ui/study_editor.rs`; `/root/OpineRust` HEAD `3e4e6e4` dan tidak menunjukkan perubahan tracked. Ini **snapshot**, bukan instruksi untuk menimpa atau mereset. Periksa keadaan terbaru.
- Jangan commit, push, force-push, release, mengubah remote atau pengaturan repo tanpa permintaan eksplisit. Jika nanti diminta commit, **jangan menambahkan `Co-authored-by` atau atribusi assistant**. Jangan menaruh token/kredensial repo private dalam source, log, contoh, atau screenshot.
- **Port user: `8081`.** Gunakan demo pada port itu untuk verifikasi. Jangan membuka server tambahan, mematikan/restart server user, mengganti dist atau menjalankan proses latar baru tanpa izin. Pertama cek PID, cwd, command/profile, bundle yang dilayani dan kecocokannya dengan source. Bila server tidak menyajikan build terbaru, minta izin untuk memperbarui/restart **server 8081 yang sama**, jelaskan dampaknya, lalu bandingkan hash file yang benar-benar dilayani. HTTP 200 saja bukan bukti UI/runtime.
- Ikuti `.cursor/rules/codebase-memory.mdc`: graph-first di project `root-egui-charts`, `trace_path` inbound **sebelum edit setiap fungsi**, `check_index_coverage` sebelum mengutip file. Graph saat audit `2026-11-09T21:16:53Z` sudah stale untuk panel/editor/engine; verifikasi source dan diff aktual. Jika simbol baru tidak ada di graph, catat gap dan cari callers di source. Repo `/root/OpineRust` bukan otomatis bagian dari graph itu; cek project terindeksnya sebelum mengklaim CBM mencakupnya.
- Dokumentasi lama dapat berisi klaim historis terlalu kuat; khususnya jangan menyebut UI/v6 selesai hanya karena tes widget, perubahan piksel, atau marker `//@version=6`.

## 2. Baseline yang harus direproduksi

### 2.1 Bug yang terlihat pada screenshot user

Pada script panjang, banyak error `box`, `line`, dan `bgcolor` muncul di bawah kode lalu teks di bagian bawah **tumpang-tindih** dengan area Logs. Hasil harus tetap mempertahankan draft dan indikator terakhir yang valid. Reproduksi pada ukuran panel/viewport mendekati screenshot, dengan **lebih dari enam diagnostik** dan pesan panjang. Catat screenshot sebelum dan sesudah yang dapat dibaca.

Source saat audit menunjukkan dua jalur yang perlu dibuktikan, bukan ditebak:

- `StudyEditor::show` menggambar `diagnostics_list` setelah editor; `diagnostics_height` hanya menghitung hingga enam baris, sementara daftar dapat menggambar lebih banyak. `OpineRustPanel` juga memiliki panel `logs_ui`. Periksa ukuran/clip/scroll dan kapan masing-masing berisi data: jangan otomatis menyatakan satu penyebab tunggal sebelum mereproduksi.
- `validate_tokens` di `opinerust/src/engine.rs` menolak nama `box`, `line`, `bgcolor`, `request`, dan lain-lain per token. Banyak kemunculan dapat menghasilkan banyak diagnostik; token-filter bisa salah mengenai identifier/property yang kebetulan bernama sama. Reproduksi kasus asli dan negative tests untuk string, komentar, identifier, member access dan alias sebelum mengganti perilaku.

### 2.2 Script v6 nyata

Screenshot contoh lain memperlihatkan script `Crypto FNG Index - Filosofi` bertanda `//@version=6`, berisi banyak `input.*`, tooltip, dan di bagian Logs terlihat pesan tentang session/`UTC` yang tidak lengkap. **Jangan simpulkan penyebab dari screenshot saja**: baris kode pemicunya tidak terlihat lengkap. Minta user memberikan source script utuh atau fixture minimal yang boleh digunakan bila source belum tersedia; jangan mengetik ulang/menebak bagian tersembunyi. Jangan mengekspor atau mempublikasikan script user tanpa izin. Jika source belum ada, lanjutkan UI dan buat fixture sintetis sendiri, tandai diagnosis script user sebagai **belum terverifikasi**.

Script pada screenshot pertama menggunakan `box`/`line` untuk zona. Periksa source utuh dan kebutuhan runtime/host sebelum menyebut semua kasusnya unsupported: bridge saat ini memang membatasi beberapa nama, tetapi jangan menyamakan keterbatasan drawings dengan seluruh semantik v6. Fixture `opinerust/tests/v6_fixtures.rs` hanya membuktikan **contoh terbatas**, bukan parity TradingView; sebagian merupakan oracle buatan sendiri dan perlu divalidasi terhadap referensi resmi.

## 3. P0 — Logs dan diagnostik: satu alur, tanpa overlap

### 3.1 Layout dan state

- Bagi dock menjadi header tetap, editor yang mengisi ruang sisa, dan Logs di bawah. Header Logs (`Logs`, chevron, count) tetap terlihat. Collapsed hanya memakan satu baris; expanded punya body scroll terpisah dengan tinggi awal target `140–180` logical px, dibatasi viewport agar editor tetap dapat digunakan. Jangan gambar di luar clip masing-masing area; no overlap ketika pesan 1, 6, 20, 100 atau pesan panjang.
- `StudyEditor` juga dipakai terminal. Untuk host OpineRust, tampilkan diagnostik di **satu tempat utama: Logs**. Jadikan daftar diagnostik inline di editor opsional/host-configurable atau desain lain yang setara; default terminal dan kontrak `StudyEditorAction`, `mark_saved`, title, open/save/apply tetap bekerja. Jangan diam-diam menghapus diagnostik dari host lain.
- Simpan `Severity`, message, **lokasi opsional** (baris/kolom dan/atau span byte) dan identitas/revisi source secara terstruktur, bukan parsing teks `failed`. Error runtime tanpa lokasi tetap dapat dibaca, namun tidak punya lokasi palsu atau aksi navigasi.
- Count severity bermakna: jelaskan apakah angka adalah **jumlah occurrence** atau **jumlah kelompok**, dan jangan mencampurnya. Pesan `Info` berisi kata `failed` tidak otomatis error; error tanpa kata itu tetap error. Jangan konsumsi (`take`) status error milik chart hanya untuk deduplikasi log.
- Setelah Run gagal, buka Logs secara otomatis dan tampilkan ringkasan error. Sukses/recovery memperbarui status aktif secara jujur; sejarah log lama boleh tetap tersedia tetapi tidak boleh membuat header terus menunjukkan error aktif. Log/info tetap bounded atau dikelola agar script dengan ribuan error tidak membekukan UI.

### 3.2 Kelompok error dan navigasi

- Kelompokkan **diagnostik yang benar-benar setara** dari run yang sama berdasarkan kode/kategori dan pesan normalisasi yang aman; contoh: satu baris `` `line` belum didukung · 4 lokasi ``. Jangan menggabungkan kesalahan berbeda hanya karena mengandung kata yang sama. Kalau runtime belum memiliki error code stabil, tentukan kunci terstruktur yang dapat diuji; pertahankan seluruh lokasi asli, severity, dan detail agar tidak kehilangan informasi.
- Klik/keyboard activation membuka daftar lokasi kelompok; pilih salah satu untuk memindahkan fokus, selection dan scroll editor ke span yang tepat. Diagnostik tanpa lokasi tidak menawarkan aksi navigasi. Jangan memotong daftar lokasi tanpa memberi cara mengakses sisanya.
- Ikat lokasi ke source/revisi yang menghasilkan diagnostik. Jika draft berubah setelah Run, tandai lokasi stale atau bersihkan diagnostik aktif dengan kebijakan eksplisit; jangan memilih offset lama seolah masih benar. Tetap aman saat panel ditutup/dibuka dan saat chart recalculation gagal.
- Konversi lokasi dengan benar untuk UTF-8 (termasuk emoji/combining), CRLF, tabs, EOF dan input parser dengan/ tanpa kolom; uji basis 0/1 dan satuan byte/karakter yang diberikan runtime. Clamp hanya pada character boundary; jangan membuat lokasi palsu jika tidak ada bukti.

## 4. P1 — Desain panel dan editor berdasarkan referensi

### 4.1 Header dan integrasi Oracly

- Dock kanan terintegrasi rail/menu compact; resize dari tepi kiri, close/reopen mempertahankan draft dan study. Jangan merusak panel lain. Saat chart kosong, editor dan Close tetap tersedia; Run disabled dengan penjelasan.
- Header satu baris `OpineRust` / nama script yang benar-benar terpasang, status ringkas, tombol **Run**, Close. Gunakan komponen/header, ikon, jarak dan theme tokens Oracly yang sudah ada; jangan menambah border form besar seperti screenshot panel lama. Pastikan title panjang terpotong rapi, actions tetap terjangkau di viewport sempit.
- Bedakan `Draft`, `Edited`, `Applied` dan `Error/Stale`; `Applied` hanya setelah install sukses. Nama script terpasang berasal dari metadata run, bukan hasil tebakan regex dari draft. Menutup panel tidak menghapus indikator. `Run` bukan `Save`, dan Open/Save tidak ditampilkan aktif jika document store belum ada.
- Screenshot acuan menunjukkan ikon copy/save/history/maximize; itu **referensi visual, bukan izin membuat tombol palsu**. Hanya tampilkan kontrol bila aksi benar-benar ada, punya tooltip/accessibility dan tes; jangan mengubah persistence atau user data secara diam-diam.

### 4.2 Code surface, gutter dan warna

- Code surface mendominasi panel, terasa menyatu dengan dock: tanpa border input bulat tebal, dengan padding dan pemisah halus dari theme. Nomor baris terlihat mulai `1`, rata kanan dan kontras cukup di light/dark; lebar mengikuti digit terbesar.
- Nomor baris menyatakan **baris logis**, bukan setiap baris hasil word-wrap. Gutter tetap pada sisi kiri saat horizontal scroll, dan tepat sejajar saat vertical scroll, resize, CRLF, newline terakhir, blank lines, font fallback, emoji/combining, DPI dan file panjang. Jangan menyimpulkan nomor benar dari jumlah shape: tes posisi, clip dan scroll aktual.
- Syntax highlighting selama mengetik memakai warna serasi Oracly, dengan perlakuan berbeda untuk keyword, identifier/builtin, fungsi, angka, string, komentar dan operator bila relevan. Periksa kontras actual light/dark termasuk selection/focus, bukan sekadar menghitung piksel berwarna. Teks dan offset sumber **tidak boleh berubah** akibat highlighting; tangani script belum lengkap, UTF-8 dan CRLF tanpa panic. Ukur performa script besar; jangan mengunci UI pada setiap frame.
- Vertical dan horizontal scroll berjalan independen dari chart/Logs. Text selection, copy/paste, IME dan undo/redo editor tetap berfungsi. `Ctrl+Enter`/`Cmd+Enter` menjalankan sekali hanya bila kode fokus; typing dan shortcut editor tidak menggerakkan chart. `Tab`/`Shift+Tab` multi-line: ikuti kontrak pada brief editor sebelumnya atau catat gap yang belum terselesaikan, jangan klaim selesai tanpa tes.

### 4.3 Minimap kanan

User secara khusus memberikan referensi editor dengan minimap: implementasikan **minimap yang nyata**, bukan screenshot statis atau scrollbar tambahan yang menyamar.

- Strip kanan target `72–100` logical px di panel lebar, menampilkan representasi seluruh dokumen yang disederhanakan, warna mengikuti token source bila layak, dengan viewport indicator yang kontras dan scrollbar/posisi sinkron.
- Scroll editor menggerakkan indikator; klik/drag minimap membawa viewport editor ke posisi dokumen yang dipilih. Posisi awal/akhir dan dokumen pendek harus benar. Minimap tidak mengubah teks, selection, kursor atau area chart; akses pointer dan keyboard/focus diberi label/tooltip yang jelas.
- Jangan menggandakan seluruh `TextEdit` interaktif atau melakukan render mahal semua glyph tiap frame. Pakai representasi/caching yang aman untuk 1.000 dan 10.000 baris; uji scrolling dan pembaruan saat edit. Bila biaya/arsitektur `egui` tidak memungkinkan, laporkan blocker dan bukti performa, bukan menyebut fitur selesai.
- Di dock sempit prioritaskan ruang kode: minimap boleh disembunyikan di bawah lebar efektif yang ditentukan dan diuji, lalu kembali tanpa mereset scroll saat panel dilebarkan. Tentukan breakpoint berdasarkan ruang kode yang tersedia, bukan viewport global semata.

## 5. P2 — audit dan peningkatan kompatibilitas v6

### 5.1 Proses: fixture → oracle → implementasi

1. Periksa versi sumber yang benar-benar dipin di `opinerust/Cargo.toml`, kedua lockfile, `cargo metadata`/`cargo tree`, dan status repo fork. Catat source/rev aktual dan apakah perubahan runtime sudah tersedia untuk bridge. Tidak boleh berpura-pura kode lokal `/root/OpineRust` otomatis masuk dependency Git.
2. Audit contoh konkret dari script user bila source diberikan. Identifikasi setiap error dan fase: lexer/parser, sema, runtime, mapping data, input, atau renderer chart. Untuk error session/UTC, buat reproduksi minimal setelah melihat **pemanggilan lengkap** dan bandingkan semantik signature dengan dokumentasi resmi Pine; jangan sekadar menerima string `UTC` sebagai session atau meredam error.
3. Buat matriks per fitur dengan script pendek, input bar/timestamp/timezone, expected output atau error, sumber oracle resmi/version, hasil v5/v6 native + WASM, batasan dan status `supported`, `partial`, `unsupported`, `unverified`. Fixture internal bukan oracle parity otomatis. Uji script tanpa annotation sesuai kebijakan saat ini dan jangan diam-diam menurunkan v6 ke v5.
4. Prioritaskan fitur yang benar-benar dibutuhkan script user, lalu pisahkan yang bisa dikerjakan di runtime dari yang membutuhkan host. Setiap fix harus punya tes positif **numerik/semantik**, negatif, regresi v5 dan WASM yang relevan. Jangan membuka token/namespace hanya untuk membuat kompilasi hijau jika output salah atau hilang.

### 5.2 Inputs dan metadata

Audit dengan fixture deklarasi, default, overrides, constraints, options, group, tooltip, title kosong/duplikat untuk `input.int`, `float`, `bool`, `string`, `color`, `source`, `session`, `time` sesuai kebutuhan script. Saat ini pemetaan bridge dan override by-title memiliki batas; periksa source aktual termasuk nilai negatif, konversi warna dan penanganan unsupported sebelum mengklaim input lengkap. User harus menerima pesan jelas bila field tak bisa dipetakan; jangan menyembunyikan input yang mengubah hasil seolah berhasil. `syminfo`, `timeframe`, chart session/timezone, live/replay dan `timenow` harus berasal dari host yang benar atau dinyatakan unavailable.

### 5.3 Drawings dan fitur yang perlu host

Untuk script zona berbasis `box`/`line`, jangan sekadar menghapus blokir. Audit kontrak Pine dan inventaris runtime fork + chart:

- Objekt lifecycle `new`, setter/getter, `delete`, reference/collection, mutasi antarbar, batas jumlah dan perilaku saat reload, simbol/timeframe, replay, edit input, Run gagal dan undo/redo.
- Pemetaan `xloc` (bar index vs waktu), `yloc`, offset, extend, clipping, colors/transparency, line style/width, border, overlay/pane, z-order dan koordinat chart yang valid. Untuk `bgcolor`/fill/label/table/plotshape, audit kebutuhan renderer masing-masing; jangan anggap dukungan box/line otomatis menyelesaikan semuanya.
- Pilih boundary yang menjaga `Indicator: Send`, ownership, identitas, keselamatan memory dan sinkronisasi output dengan bar/script/context revision. Ukur jumlah objek dan biaya. Jangan mengizinkan loop tak terbatas membekukan UI; bila belum ada pembatasan/cancellation yang dapat dibuktikan, dokumentasikan blocker.
- Uji scene minimal yang dibuat, diubah, dihapus dan direplay dengan expected geometry/style; hasil Run gagal tetap mempertahankan study/drawing sebelumnya; cek urutan/gap/clipping pada chart nyata dan WASM. Jika sebuah jenis objek belum bisa dipetakan dengan benar, tetap tolak dengan diagnostik spesifik dan lokasi.

Jika perlu mengubah fork runtime, buat perubahan lokal + fixture di repo itu; uji melalui seam lokal hanya sebagai eksperimen yang jelas (jangan tinggalkan absolute path dependency di manifest). Integrasi permanen memerlukan commit/revisi fork yang tersedia pada origin **setelah user menyetujui publikasi**. Bila belum ada izin commit/push runtime, catat blocker dan langkah integrasi; jangan melakukan push sendiri untuk memaksa pin. Pertahankan MPL-2.0, copyright, NOTICE dan attribution Pinecone. Jangan klaim seluruh Pine Script v6 kompatibel hanya karena beberapa contoh berjalan.

## 6. Tes dan definisi penerimaan

### 6.1 Tes otomatis

- **Log overlap:** generate 0, 1, 6, 20 dan 100 diagnostik, sebagian berulang, pesan panjang dan severity berbeda; ukur `Rect`/clip editor, summary dan setiap log row pada panel terbuka/tertutup. Tidak ada row yang tumpang-tindih atau bocor ke editor; bisa scroll ke entri pertama/terakhir dan collapse kembali. Uji count, grouping, lokasi setiap occurrence, no-location dan revisi source stale.
- **Host/regresi:** Run A sukses → B sukses mengganti → C gagal (parse, unsupported, dan kegagalan install setelah preflight) mempertahankan B/history/draft/status. Error recalculation saat panel tertutup tetap tampak pada chart; saat recovery status pulih. Uji close/reopen, chart kosong, symbol/timeframe switch, undo/redo, rail/compact, resize dan panel lain.
- **Editor:** gutter terlihat pada kedua tema dan setiap nomor berada dalam clip yang benar; alignment pada scroll horizontal/vertical, wrap policy, CRLF, Unicode dan final newline. Syntax palette tidak merusak source, string/comment/incomplete tokens; generic terminal tetap menggunakan default lama. Uji clipboard/selection, Ctrl/Cmd+Enter fokus/non-fokus, tidak newline/Run ganda, Tab dan Shift+Tab.
- **Minimap:** perubahan source memperbarui tampilan, viewport indicator sinkron di awal/tengah/akhir, click/drag menavigasi dan tetap benar setelah resize/scroll; dokumen pendek/kosong/panjang dan viewport sempit. Tes performa terukur, bukan sekadar fungsi mengembalikan `Ok`.
- **v6:** fixture script user yang tersedia + fixture sintetis per fitur; expected numerik/error/lokasi, v5 regression, native + `wasm-bindgen-test` untuk bridge/runtime path yang benar, dan host chart untuk fitur visual. `cargo check` atau pengakuan marker versi bukan eksekusi WASM.

### 6.2 Verifikasi visual/browser

Di **8081**, pastikan dulu bundle terbaru benar-benar disajikan; hitung SHA-256 dari file lokal dan bytes yang diambil dari server, catat profil release/dev dan revision. Jangan mendirikan server lain atau mengubah/mematikan server user tanpa izin. Buka panel melalui rail dan compact menu, jalankan interaksi nyata. Periksa viewport aktual desktop `1440×900`, intermediate `800×700`, compact `360×640` dan `320×360`, light/dark. Capture **gambar yang benar-benar dapat dibaca**: sebelum/sesudah, source panjang dengan minimap dan scroll, banyak error, lokasi diklik, chart tetap memegang study lama. Screenshot kosong, angka perubahan piksel atau HTTP 200 bukan bukti visual/fungsional. Jika capture WebGL tidak terbaca atau tool browser tidak tersedia, laporkan sebagai blocker; jangan mengarang apa yang terlihat.

Uji script v5 dua plot, v6 subset yang punya oracle, unsupported, dan script user bila tersedia; bandingkan nilai/warna/gap dan geometri objek yang memang didukung. Catat console panic/error, frame rate/responsiveness yang diukur dan limitasi. Simpan bukti sesuai kebijakan repo; jangan memasukkan source atau data sensitif user ke screenshot tanpa izin.

### 6.3 Build dan hasil akhir

Laporkan perintah yang benar-benar dijalankan, cwd, exit code, jumlah passed/failed/ignored, target native/Node/browser dan tool/version. Contoh: `cargo fmt --all -- --check`, tes `study_editor`, panel/controller demo, terminal host, `opinerust` + tes runtime yang diubah, dan clippy relevan; build/test target yang memang disentuh. Untuk perintah ber-pipe, aktifkan `set -o pipefail` atau periksa status tiap proses. Jangan mengulangi angka tes lama sebagai hasil baru.

**Definition of done:** log tidak overlap dan dapat dinavigasi, editor/minimap sesuai kontrak dan referensi Oracly, interaksi/tema/viewport lulus review visual nyata, script yang diuji memiliki hasil benar atau penolakan spesifik tanpa output salah, terminal dan lifecycle chart tidak regresi. Jika source script user, oracle, browser, akses repo private, feature host, performa atau izin publikasi runtime belum ada, nyatakan **belum selesai**, tulis blocker dan hasil parsial. Jangan menyebut v6 penuh, support drawings, atau responsif jika belum dibuktikan.

## 7. Laporan untuk user

Jelaskan singkat: perbaikan log/UI dan screenshot yang benar-benar terbaca; fitur v6/objek yang benar-benar diuji versus yang masih ditolak atau belum diverifikasi; hasil tes dan build aktual; sumber/revisi runtime yang dipakai; blocker (termasuk source script belum tersedia atau runtime belum dipublikasi); status perubahan kedua repo. Konfirmasi tidak ada commit/push tanpa instruksi dan tidak ada co-author bila nanti user meminta commit.
