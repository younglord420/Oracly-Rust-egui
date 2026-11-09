# Tugas DeepSeek: rebranding runtime OpineRust, integrasi aman, dan roadmap Pine Script® v6

**Status: rencana kerja; runtime dan bridge belum dinyatakan production-ready.** Baca dokumen ini seluruhnya sebelum mulai. Tugas ini melibatkan **dua repository terpisah** dan melanjutkan pekerjaan editor/bridge dalam [`pinecone-pine-editor-bridge.md`](pinecone-pine-editor-bridge.md); bukan pengganti spesifikasi atau alasan untuk menandai milestone editor selesai.

## 1. Tujuan dan batas tugas

Kembangkan **OpineRust** sebagai runtime Rust milik Oracly yang berasal dari [Pinecone](https://github.com/ferranbt/pinecone), dan hubungkan bridge Oracly ke sumber runtime kita secara reproducible. Sesudah fondasi aman, tingkatkan kompatibilitas Pine Script® v6 **bertahap, per fitur dan berdasarkan fixture**, sambil mempertahankan kompatibilitas v5 yang sudah benar-benar diuji.

Prioritas wajib:

1. Audit keadaan kedua repo dan buktikan baseline upstream.
2. Rebranding produk dan dokumentasi **tanpa menghilangkan asal, lisensi, atau merusak API**.
3. Perbaiki kebenaran runtime/data serta error propagation dan transaksi Run/replace di Oracly.
4. Hubungkan bridge ke commit runtime kita yang spesifik; buktikan dependency graph dan pengujian sesungguhnya.
5. Tambah fitur v6 secara bertahap, aman dan teruji; fitur yang belum didukung ditolak jelas.
6. Verifikasi demo browser nyata dan laporkan kapabilitas serta blocker dengan jujur.

**Jangan melakukan commit, push, force-push, membuat release, menerbitkan crate, atau mengubah visibility/setting GitHub kecuali user meminta secara eksplisit.** Walaupun branch `main` upstream sudah pernah dikirim, itu bukan izin untuk push pekerjaan baru. Jangan hapus/rollback perubahan lokal user, jangan jalankan `cargo clean`, jangan hentikan server atau proses user tanpa kebutuhan dan persetujuan.

### Dua repo dan identitasnya

- **Runtime terpisah:** `/root/OpineRust`, origin [`younglord420/OpineRust`](https://github.com/younglord420/OpineRust), saat brief dibuat **private**, bukan fork resmi GitHub. `upstream` mengarah ke `ferranbt/pinecone`. Clone bukan shallow; branch awal memiliki riwayat upstream dan HEAD `7bb739d771f804ef6e0352bb85b517fcf610594e` ketika diperiksa. Cek lagi; jangan anggap remote tidak berubah.
- **Chart dan bridge:** `/root/egui-charts`, repo Oracly. `opinerust/` di dalamnya adalah **adapter/bridge**, bukan clone runtime. `examples/demo/` adalah workspace demo tersendiri. Jangan menyalin seluruh runtime ke dalam repo chart atau mengubah remote Oracly.
- Runtime dan bridge adalah **dua deliverable**, bukan bukti bahwa keduanya sudah tersambung. Bridge saat brief dibuat masih memakai crate registry `pine-* = 0.2.6` exact pin, bukan source Git repo OpineRust.
- Karena repo runtime **private**, konsumsi git dependency oleh developer lain, CI, dan build publik memerlukan autentikasi yang benar. Jangan menaruh token, SSH private key, URL berkredensial, atau `.cargo/config` rahasia ke repo, lockfile, log atau contoh. Jangan ubah repo menjadi public tanpa izin. Bila akses CI/build publik belum tersedia, jelaskan blocker dan jalur akses, jangan diam-diam kembali ke registry lalu mengklaim sudah memakai fork.

## 2. Cara bekerja dan sumber kebenaran

### 2.1 Keselamatan working tree

Sebelum membaca detail atau mengedit, catat pada **masing-masing** repo: `pwd`, `git status --short`, branch, `git remote -v`, HEAD, `git diff --stat` dan diff relevan. Perhatikan file untracked tidak terlihat di `git diff`. Oracly saat brief disusun memiliki modifikasi lokal dan file untracked, termasuk bridge, panel, dokumen dan `.vscode/`; perlakukan semuanya sebagai pekerjaan user. Jangan gunakan `git reset --hard`, `git checkout --`, `git clean`, atau menimpa file untuk menyederhanakan pekerjaan. Periksa instruksi `AGENTS.md`/rules yang berlaku di kedua repo.

Ikuti `.cursor/rules/codebase-memory.mdc` di repo Oracly untuk project `root-egui-charts`: **graph-first** (`search_graph`, `trace_path`, `get_architecture` atau `get_code_snippet`) sebelum eksplorasi source; `trace_path` arah inbound sebelum mengedit setiap fungsi; `check_index_coverage` sebelum mengutip file. Graph adalah petunjuk, bukan kebenaran: konfirmasi source, status dan diff langsung. Jika file baru tidak terindeks, laporkan gap, baca source dan cari caller dengan alat source; reindex hanya bila tersedia, aman, dan sesuai instruksi lingkungan. Jangan berpura-pura graph meliputi repo `/root/OpineRust`: cek apakah ada project indeks terpisah; bila tidak, catat dan telusuri source/caller secara langsung. Jangan membuat ulang indeks sebagai pengganti membaca source aktual.

### 2.2 Dokumen sebelumnya bisa usang atau saling bertentangan

Baca [`pinecone-pine-editor-bridge.md`](pinecone-pine-editor-bridge.md) dan [`opinerust-status.md`](opinerust-status.md) seluruhnya. Mereka merekam konteks dan hasil lama, tetapi **bukan bukti kondisi saat ini**:

- Brief lama menyebut beberapa cacat implementasi awal (raw text filter, snapshot warna lama, `Applied` di `prepare`); source saat ini sudah berubah. Reproduksi perilaku aktual sebelum mengatakan cacat masih ada atau sudah selesai.
- Laporan status lama memuat klaim yang terlalu kuat/tidak konsisten: misalnya menyebut `input.*` ditolak tetapi bagian lain mengatakan berjalan memakai default; `syminfo`/`timeframe` juga tercantum di daftar token yang ditolak. Jangan mengadopsi salah satu klaim tanpa tes aktual.
- Klaim bahwa `barstate` live sudah benar tidak cukup dibuktikan oleh clock + jarak median bar. `bar_span_ms` dihitung saat build, bukan otomatis mencerminkan setiap konteks baru; sesi, DST, monthly bar, replay, data sparse, dan candle forming butuh sumber konteks nyata. Uji atau tolak secara jujur; jangan menyimpulkan sekadar dari tes satu menit.
- `study_from_output` menyimpan snapshot lama ketika recalculate gagal dan menulis error ke slot bersama. Audit apakah host bisa membaca error **sebelum** menganggap kandidat terpasang; jangan menganggap snapshot/error log membuktikan publish atomik. Uji juga keterasingan state clone/history.
- Saat brief dibuat, `EngineBar` mempunyai field `forming` dan `history`, sedangkan fixture di `opinerust/tests/wasm_execute.rs` tampak belum mengisinya. Jalankan tes WASM setelah memeriksa source dan perbaiki kegagalan jika benar; hasil WASM lama sebelum perubahan terakhir bukan bukti revision ini.
- Hasil sebelumnya yang dilaporkan dalam percakapan: satu suite demo pernah menunjukkan 97 lulus, 1 ignored; ini **historis**, bukan pengganti hasil setelah perubahan sendiri. Catat waktu, perintah, exit code dan target sebenarnya. Tes `wasm-bindgen-test` di Node adalah eksekusi WASM nyata, **bukan** tes browser UI.

Setelah implementasi, perbarui dokumen status yang relevan agar klaim sesuai kode dan bukti baru; jangan hapus catatan keterbatasan atau menulis klaim pemasaran.

## 3. Fase A — baseline upstream dan rebranding aman di runtime

### A0. Baseline sebelum perubahan

1. Verifikasi `origin`, `upstream`, HEAD dan bahwa riwayat upstream masih ada. Periksa workspace, `Cargo.lock`, toolchain, target terinstal, `.github/workflows/`, daftar fitur dan README. Catat versi Rust/target/Node/browser/wasm test runner yang benar-benar tersedia.
2. Jalankan formatter **check**, lint dan tes yang relevan pada upstream yang belum diubah. Periksa perintah CI runtime dan perintah yang diperlukan per crate; suite workspace penuh boleh memakan waktu, laporkan hasil/failure dan lanjut secara terukur. Jangan mengasumsikan seluruh crate dapat berjalan pada WASM: `pine-lang`/`pine-data` mungkin menarik dependensi jaringan native; cari dan uji core path yang benar-benar digunakan bridge.
3. Jalankan minimal tes eksekusi **runtime aktual** di native dan `wasm32-unknown-unknown` dengan dua plot v5, subset v6, `timenow`/`barstate`, sema/parse error dan angka/gap diketahui. Jika runtime crate sendiri butuh host clock agar bisa dieksekusi di WASM, uji seam yang nyata dan dokumentasikan; `cargo check` saja tidak cukup. Catat apakah runner memakai Node atau browser.
4. Catat baseline graph dependensi dan asal crate sebelum diubah (`cargo tree`, lockfile, `cargo metadata`). Simpan hasil dan keterbatasan dalam laporan, bukan menganggap baseline selalu hijau.

### A1. Identitas dan dokumentasi

- Rebranding produk yang kita miliki menjadi **OpineRust** / **OpineRust Runtime**. Jelaskan runtime dan bridge/editor adalah komponen berbeda. Hindari janji v6 penuh atau afiliasi resmi TradingView.
- Pertahankan `LICENSE` MPL-2.0, copyright header dan attribution upstream; catat source, commit awal, repo asal, file yang dimodifikasi serta cara mendapatkan source dari versi yang didistribusikan. Audit lisensi dependensi, assets, tooling dan bundling WASM; catat kewajiban yang relevan dengan akurat, termasuk akses source bagi penerima bila distribusi dilakukan. Ini bukan pendapat hukum final. Repo private tidak otomatis memenuhi kewajiban distribusi source.
- Audit README, workspace package metadata (`repository`/`homepage`), installer, release workflow, CLI, extension, links dan contoh. Link installer upstream tidak boleh disajikan seolah menginstal produk kita. **Jangan** mengaktifkan release/publishing/installer otomatis atau mengganti binary/crate/extension ID secara membabi buta. Jika belum disiapkan di fork, nonaktifkan/labeli instruksi tersebut dengan jujur dan dokumentasikan migrasi.
- Pertahankan nama package/crate `pine-*`, API, dan versi dulu untuk mengurangi pecah integrasi; nama tampilan produk boleh berubah. Jika perlu rename teknis, buat rencana kompatibilitas terpisah dan minta persetujuan, bukan mengganti semuanya sebagai bagian rebranding.
- Jangan salin atau transliterasi kode AGPL dari [Vela-pinets](https://github.com/LuxAlgo/Vela-pinets). Referensi Vela hanya perilaku/layout. Hindari mengganti atau menghapus kredit Pinecone dalam dokumentasi teknis/lisensi.

**Gerbang A:** build/tests tidak memburuk tanpa penjelasan; nama produk jelas; distribusi/licensing dapat diaudit; API crate lama tetap kompatibel; belum ada klaim seluruh v6.

## 4. Fase B — bridge benar sebelum migrasi dependensi atau ekspansi fitur

Baca ulang spesifikasi P0–P5 di brief editor, lalu reproduksi terhadap source aktual. Buat regresi yang gagal sebelum fix bila memungkinkan; jelaskan jika tidak bisa. Utamakan correctness daripada mengubah UI untuk menutupi masalah.

### B1. Eksekusi, data dan warna

- Hilangkan seluruh jalur clock OS yang panic pada WASM, termasuk saat sema, builtins dan error. Kontrak `timenow` berasal dari host, konsisten antara preflight/recalculate/replay; jangan mengganti waktu host dengan timestamp candle seolah identik.
- Definisikan input data: bar tertua→terbaru, UNIX ms, urutan, whitespace/non-finite, jumlah dan timestamp. Pilih gap atau error untuk data invalid; jangan menggeser indeks. Uji numeric fixture append, update candle terakhir berulang, koreksi bar lama, prepend, symbol/timeframe switch, replay maju/mundur, dan input berubah.
- Nilai, warna, metadata plot, script/data/context revision harus berasal dari run yang sama. Uji plot nama kosong/duplikat, semua `na`, gap awal/tengah/akhir, perubahan jumlah/urutan serta opacity. Bila identitas call site tidak bisa dibuktikan, tolak pola ambigu secara jelas.
- Jangan mengklaim `barstate` benar berdasarkan tebakan interval median. Rancang konteks eksplisit menggunakan clock/timeframe/session dan replay/live yang benar-benar disediakan host; kebijakan data belum tersedia harus jelas. Uji DST, jeda sesi, sparse data, bar bulanan dan forming. `varip`/rollback intrabar belum otomatis benar hanya karena ada enum di AST.
- Full replay sinkron adalah prototype: ukur durasi/biaya pada dataset representatif dan sertakan batas waktu/memori/cancellation atau laporkan blocker. Jika async/worker digunakan, owned `Send` data dan generation guard untuk script, chart, symbol/timeframe dan data mencegah job lama menimpa baru. Jangan masukkan `Rc` interpreter ke `Indicator: Send` atau gunakan `unsafe impl Send`.
- Error recalculate harus menjadi status stale/error yang tampak bahkan jika panel ditutup; jangan tampilkan snapshot lama seolah up-to-date atau hilangkan error karena slot hanya dibaca panel. Jelaskan kebijakan nilai lama, panjang/index alignment, recovery, dan sinkronisasi warna/history.

### B2. Transaksi Run/replace dan history

- Audit alur host asli (`prepare` → chart install/calculation → confirmation), `try_add_indicator`, batas study, `ConfiguredStudy`, pane/settings/visibility, `SeriesId`, dan undo/redo. Interface `CustomIndicator` memakai closure yang mengembalikan `Vec`, bukan `Result`: preflight sukses tidak otomatis berarti perhitungan saat `add_indicator` atau replay history sukses. Buat seam fallible/publish yang benar atau laporkan blocker dan **jangan klaim atomic**.
- Jangan bergantung pada sementara menaikkan limit untuk membuktikan transaksi aman: verifikasi batas, penolakan dan pemulihan saat semua jalur error, termasuk ID study lama sudah dihapus/di-undo, limit di bawah jumlah current, perubahan chart antara preflight dan install, pane overlay↔pane, kegagalan recalc saat install atau undo/redo.
- Satu Run sukses A → Run sukses B mengganti study yang sama tanpa duplikasi; Run gagal C mempertahankan study aktif, pane/settings/identity sesuai kontrak, tampilan dan history. Undo/redo replace tepat satu user action. Tentukan apa yang tidak dapat dipertahankan dan tunjukkan user; jangan mengandalkan tes yang hanya mensimulasikan handler.
- Status `Applied` hanya setelah publish berhasil. Runtuhnya update berikutnya tidak boleh mempertahankan indikator sukses palsu. Draft tetap ada; Run bukan Save; Open/Save tanpa store tetap disembunyikan.

### B3. UI dan browser

Tetap ikuti spesifikasi editor kanan P4/P5: akses rail desktop dan compact, panel dapat di-resize dan ditutup tanpa kehilangan draft, Run, Logs, navigasi diagnostik byte UTF-8/CRLF dan keyboard. Uji interaksi aktual, bukan sekadar nama tes yang mengatakan resize atau close. Komponen `StudyEditor` generik juga dipakai terminal; jangan memaksakan nama/status khusus OpineRust kepada host lain.

**Gerbang B:** native dan WASM yang dipakai bridge diuji, error/publish dan undo benar-benar terbukti, data/nilai/warna mengikuti chart; kekurangan tetap diklasifikasikan sebagai blocker, bukan milestone selesai.

## 5. Fase C — gunakan runtime repo sendiri secara reproducible

1. Pilih satu **commit yang benar-benar tersedia di origin** setelah perubahan runtime disetujui/published user, atau gunakan pin upstream HEAD yang sudah ada untuk integrasi awal bila cocok. Jangan menunjuk `main` yang bergerak. Bedakan perubahan lokal runtime yang belum dikomit dengan commit remote: pin git tidak akan otomatis memuat perubahan lokal yang belum dipublikasi. Jangan commit/push sendiri untuk memaksa integrasi; bila perlu commit persetujuan user, laporkan dan berhenti pada gerbang yang tepat.
2. Hubungkan seluruh crate runtime yang dipakai bridge ke **satu revisi** yang sama melalui manifest/lockfile yang konsisten. Evaluasi `[patch.crates-io]`, `git` dependencies atau strategi workspace yang sesuai; verifikasi juga dependensi transitif workspace (`pine-ast`, macro, sema, builtins, interpreter, core) tidak tercampur dengan registry version lain. Jangan memakai path absolut `/root/OpineRust` dalam manifest yang dibagikan. Jika path sementara dipakai eksperimen, jangan tinggalkan dalam hasil final.
3. Jalankan `cargo metadata`, `cargo tree -d` dan inspeksi `Cargo.lock` pada bridge dan demo: tunjukkan URL source + revisi bagi tiap `pine-*`, identitas tipe tidak terduplikasi karena dua sumber, dan dependency graf WASM tidak menarik `pine-data` → native network/ring tanpa dukungan. Uji clean checkout terautentikasi tanpa membocorkan kredensial; cek juga perilaku saat kredensial tidak tersedia.
4. Bandingkan fixture numerik native/WASM terhadap baseline registry dan runtime fork, serta uji demo yang benar-benar memakai dependency hasil pin. Ulangi setelah perubahan terakhir; catat SHA hasil kerja dan asal paket tepatnya.
5. Jika belum mungkin mem-pin perubahan lokal yang belum dipush, atau private dependency tidak tersedia di CI/build user, pertahankan status **integrasi belum selesai**, jelaskan langkah yang memerlukan user. Jangan diam-diam mengganti requirement menjadi local path atau menduplikasi versi.

**Gerbang C:** lockfile dan output build membuktikan crate yang berjalan benar-benar dari revisi fork yang sama; native/WASM dan CI/reproducibility divalidasi atau blocker didokumentasikan.

## 6. Fase D — kompatibilitas Pine Script® v6 bertahap

### D0. Audit dan oracle

- `//@version=6` yang dikenali oleh Pinecone/OpineRust **bukan bukti** kesetaraan semantik TradingView. Audit lexer, parser, AST, semantic analyzer, interpreter, builtins dan host adapter pada revisi yang digunakan; catat mana yang version-gated, sama antarversi, atau belum diketahui. Jangan ulangi klaim terdahulu bahwa semua v5/v6 identik, `switch` tertentu mustahil, atau `varip` benar/salah tanpa fixture yang membuktikan.
- Gunakan dokumentasi referensi resmi Pine dan fixture yang dapat diverifikasi untuk perilaku v5/v6, termasuk contoh perbedaan yang benar. Catat sumber, versi, input bar, expected output, alasan, target dan hasil. Bila membandingkan dengan referensi eksternal, jangan menganggap Vela-pinets oracle mutlak atau menyalin kode AGPL. Jika oracle tidak tersedia, tandai **unverified**, bukan supported.
- Tentukan kebijakan script tanpa anotasi secara eksplisit; bridge saat brief dibuat default ke v5. Jangan mengubah default secara sepihak jika menimbulkan break compatibility; catat usulan dan minta keputusan produk bila diperlukan. Jangan menurunkan script v6 diam-diam ke v5.
- Tempatkan feature capability di registry/analisis syntax atau AST yang memahami scope, member access, alias, string, komentar, multiline dan call site. Daftar token identik saja bisa memblokir properti atau variabel yang tidak berkaitan; jangan membuka fitur hanya karena nama di allowlist. Negative tests wajib: fitur unsupported tidak mengembalikan `Applied`/output sukses yang salah.

### D1. Input dan metadata chart

- Audit `input.int/float/bool/string/source/color` yang upstream **tampak** menyediakan, tetapi jangan menganggap semantics, metadata, duplicates atau overrides benar tanpa tes. Bridge saat ini menolak `input`; hapus penolakan hanya bersama implementasi lengkap yang terbukti, bukan sekadar melonggarkan filter.
- Pemetaan ke `StudyInput`: stabil per deklarasi/call site, default, tipe, nilai, range, step, pilihan, label/group, validasi, reset dan round-trip override. Uji judul kosong/duplikat; jika runtime override by title ambigu, buat solusi yang benar atau tolak dengan diagnostik. Bila satu jenis belum dapat dipetakan, tolak jenis itu dengan jelas. Input settings harus sederhana, sesuai indikator dan tidak menambah symbol/interval generik yang tidak relevan.
- Mapping `syminfo` dan `timeframe` ke data aktual chart/host, termasuk exchange, ticker/tickerid, mintick dan interval. Tidak semua metadata tersedia: jangan mengisi nilai palsu; tandai unavailable atau tolak field yang tidak dapat dihitung benar. Pastikan sema **dan** runtime menerima konteks yang konsisten serta berubah saat symbol/timeframe/replay berganti. Jangan menganggap dukungan `request.security` sudah ada.
- Uji nilai output/gap/warna setelah perubahan input, symbol/timeframe dan reset settings; v5 dan v6 native + WASM + host chart.

### D2. Perbedaan semantik dan fitur berikutnya

Prioritaskan berdasarkan fixture nyata dan dampak skrip pengguna: tipe/konversi boolean dan `na`, evaluasi logis/short-circuit, series/history, dynamic loops dan bounds, UDT/method/enum/switch, intrabar/rollback/`varip`, rendering style/fill/drawings, lalu data lintas simbol/interval. Daftar ini adalah **sasaran audit**, bukan klaim bahwa setiap fitur didukung atau urutan kompatibilitas resmi.

Untuk setiap fitur: spesifikasi perilaku v5/v6 → tes positif/negatif dengan expected numerik atau diagnostik → implementasi sesuai batas runtime → regresi native/WASM dan, bila menyentuh UI/data, demo nyata → perbarui matriks. State v5 tidak boleh diam-diam berubah oleh perbaikan v6. `strategy`, broker, alerts, `request.*`, library imports, tables/drawings/fills dan style lanjutan tetap unsupported sampai host dan renderer lengkap serta diuji. Jangan menyebut Pine v6 penuh hanya karena beberapa fixture lulus.

## 7. Pengujian, build, dan bukti penerimaan

Jalankan dan laporkan perintah **tepat** beserta lokasi kerja, exit code, jumlah tes lulus/gagal/ignored, target Node/browser, versi tool, revision commit dan blocker. Saat memakai pipeline shell, aktifkan `set -o pipefail` atau periksa exit code tiap proses; jangan menyimpulkan sukses dari `tail`, `rg`, HTTP 200, `cargo check`, test hasil lama, atau screenshot kosong.

Minimal verifikasi:

- Runtime: `cargo fmt --check`, `cargo clippy` untuk cakupan yang relevan, tes unit/integrasi workspace atau subset yang dijelaskan, dan eksekusi WASM **runtime aktual**.
- Bridge: native dan `wasm-bindgen-test` aktual terhadap `execute` dan `build_study`, v5 dua plot, v6 subset, input/context yang telah didukung, error sema/runtime/unsupported, host clock, nilai/warna/gap. Jalankan Node WASM dan, terpisah, browser/WASM bila tersedia; catat target runner.
- Chart/host: tes kalkulasi + invalidasi numerik, study limit dan install failure, run A→B→gagal C, undo/redo, pane/overlay, ID/status/settings, chart berubah selama run, closed panel/error, editor UTF-8/keyboard/draft dan layout compact.
- Demo: build **setelah perubahan terakhir**, periksa log sukses dan SHA-256 bundle lokal vs yang server benar-benar layani. Jangan menimpa/hentikan server user; gunakan port tersedia dan sebutkan. Buka demo di browser sebenarnya: rail → Run v5 dua plot → Run ulang → error → subset v6 → unsupported; cek chart overlay/pane, warna/gap, Logs, console, live candle, symbol/timeframe/replay dan settings input sesuai fitur. Review light/dark dan viewport sempit; tampilkan screenshot bila benar-benar bisa dibaca. Jika capture WebGL gagal, laporkan visual/browser sebagai blocker, jangan tebak dari canvas kosong.
- Lisensi: audit file MPL yang diubah dan source distribution untuk runtime serta bundle WASM; cek tidak ada rahasia, dependency campuran, installer upstream menyesatkan, atau kode AGPL yang disalin.

### Definition of done

Pekerjaan boleh disebut selesai **hanya jika**:

1. Runtime OpineRust teridentifikasi jelas, riwayat dan atribusi Pinecone dipertahankan, lisensi dan distribusi dapat diaudit, tanpa publishing tidak sah.
2. Baseline dan regresi native/WASM runtime dan bridge lulus pada revisi yang dipin; dependency graph membuktikan runtime fork dipakai konsisten dan dapat direproduksi sesuai akses private repo.
3. Recalculation/data/warna/error dan Run/replace/undo aman dan teruji pada host chart yang sebenarnya, tidak hanya test simulasi; tidak ada status sukses saat output salah/stale.
4. Editor kanan sesuai brief, termasuk keyboard dan browser visual/data nyata.
5. Setiap klaim v6/v5 disertai fixture dan oracle yang layak; unsupported/unknown ditolak atau dinyatakan jelas, tidak ada klaim kompatibilitas penuh tanpa bukti.
6. Hasil formatter/lint/suite relevan serta keterbatasan dicatat, tidak ada perubahan lokal user tertimpa, dan tidak ada commit/push/release tanpa permintaan eksplisit.

Jika salah satu gerbang tidak bisa dipenuhi karena bug upstream, autentikasi private, toolchain, browser, atau keterbatasan arsitektur, **jangan** menyatakan selesai. Laporkan apa yang terbukti, reproduksi, dampak ke user, tindakan yang masih dibutuhkan dan keputusan yang memerlukan user.

## 8. Format laporan akhir untuk user

Laporkan dengan singkat tetapi spesifik:

1. Apa yang berubah di masing-masing repo dan SHA/source dependency yang benar-benar dipakai.
2. Fitur v5/v6 yang lulus dan yang ditolak/belum terverifikasi, masing-masing dengan fixture dan target uji; jangan membuat klaim global.
3. Perintah dan hasil aktual: fmt, lint, native, WASM Node, browser, demo build/served bundle, suite chart/editor; cantumkan failure/ignored/blocker.
4. Status transaksi, data live/replay, UI visual dan akses private CI.
5. Lisensi/atribusi dan apakah ada tindakan yang harus dilakukan user. Konfirmasikan **tidak ada commit/push** kecuali user memang memintanya.
