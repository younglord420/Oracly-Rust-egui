# Blueprint Eksekusi: Samakan OpineRust dan Oracly dengan Perilaku PineTS

**Tanggal**: 2026-11-12  
**File ini adalah antrian kerja.** Dokumen lama [`opinerust-pinets-luxalgo-full-compatibility-plan.md`](opinerust-pinets-luxalgo-full-compatibility-plan.md) tetap berlaku untuk lisensi dan arsitektur. Centang "jalan" di §3 dokumen itu tidak boleh dipakai sebagai alasan berhenti.

**Yang membaca dokumen ini mengerjakan perilaku bahasa, lalu membuktikannya di chart.** Satu error di satu skrip tidak menutup pekerjaan. Satu tes 6 bar yang memanggil setter langsung juga tidak menutup pekerjaan.

---

## 1. Syarat selesai

Pekerjaan satu perilaku selesai hanya jika tiga hal ini ada sekaligus:

1. Tes Rust pendek mengunci aturan itu, tanpa membaca `smc.txt`.
2. Hasil tes sama dengan perilaku PineTS yang dijelaskan di bagian 4. Kode TypeScript-nya tidak disalin.
3. Kalau perilaku itu menghasilkan gambar, gambar itu tampil di chart Oracly pada data yang sama dengan yang diuji.

Gerbang gambar pertama, setelah paket 1–3 di bawah lulus, adalah skrip Smart Money Concepts di `/root/egui-charts/smc.txt` pada BTC 1D:

- Kotak order block internal dan swing tampil sebagai persegi panjang berwarna, merentang dari waktu bar order block sampai bar terakhir, dengan `extend` kanan.
- Garis struktur (solid untuk swing, putus-putus untuk internal) menghubungkan swing ke harga tembus.
- Label **BOS** / **CHoCH** duduk di atas atau di bawah jangkar sesuai `label.style_label_up` / `label.style_label_down`.
- Zoom dan geser tidak menjalankan skrip lagi. Candle yang ada tidak berubah, gambar yang sudah ada hanya berpindah.

Zona premium/discount **default-nya mati** di `smc.txt` baris 131 (`showPremiumDiscountZonesInput = input.bool(false, ...)`). Pita lebar di TradingView hanya wajib setelah input itu dinyalakan. Jangan mengklaim zona itu hilang sebagai bug selama inputnya masih mati. Order block internal default-nya hidup (`showInternalOrderBlocksInput`).

`smc.txt` berlisensi CC BY-NC-SA. Jangan di-commit, jangan di-vendor, jangan dijadikan fixture di dalam crate.

---

## 2. Tiga pohon kode, dan yang tidak boleh dikerjakan

| Pohon | Peran | Boleh diubah |
| :--- | :--- | :--- |
| `/root/pintes/PineTS` | Runtime LuxAlgo (AGPL-3.0). Acuan perilaku. Sudah ada di disk, cabang lokal `oracly/rust-runtime` berhenti di irisan SMA (`docs/oracly/STATUS.md`, 2026-10-29) dan tertulis "bukan chart". | Tidak. Jangan lanjutkan runtime Rust di dalam folder ini. Jangan salin `src/` ke repo lain. |
| `/root/OpineRust` | Runtime Pine kita (MPL-2.0, fork pinecone). | Ya. Perilaku bahasa ditulis di sini. |
| `/root/egui-charts` | Chart egui + crate `opinerust` (jembatan). | Ya. Konversi gambar dan jalur zoom ditulis di sini. |

Demo yang sedang jalan: `trunk serve --release --port 8081 --address 0.0.0.0` di `examples/demo`. Jangan dimatikan. `trunk build --release` hanya menulis `examples/demo/dist` dan selesai. Rebuild demo hanya setelah tes paket yang bersangkutan lulus, dan hanya kalau pengguna meminta rebuild.

Pin crate `pine-*` di jembatan menunjuk revisi git OpineRust. Perubahan runtime lokal tidak masuk demo sampai commit dan pin diperbarui. Jangan meninggalkan path dependency.

WASM tidak boleh memanggil `SystemTime` atau socket. Jembatan tidak boleh menarik crate payung `pine-lang` (ureq/ring tidak membangun `wasm32`).

Order flow, footprint, volume profile dalam, dan heatmap DOM tetap study Rust. Jangan dipindah ke Pine di pekerjaan ini.

---

## 3. Cara memakai PineTS

Baca perilaku, tulis ulang di Rust, kunci dengan tes.

Urutan baca untuk satu perilaku:

1. Namespace di `/root/pintes/PineTS/src/namespaces/<nama>/`.
2. Tes di `/root/pintes/PineTS/tests/` yang memakai perilaku itu, sebagai contoh masukan dan keluaran. Jangan menyalin badan tes ke repo kita. Tulis tes baru dengan angka sendiri.
3. Bandingkan dengan builtin OpineRust di `/root/OpineRust/crates/pine-builtins` dan dengan konverter di `/root/egui-charts/opinerust/src/engine.rs` serta `study.rs`.

Yang dicatat di tes kita adalah aturan bahasa, misalnya: "kotak `xloc.bar_time` yang diisi `set_top_left_point(chart.point.new(waktu, na, harga))` menyimpan `waktu` di `left` dan `harga` di `top`". Bukan kutipan file TypeScript.

Antrian fungsi ada di [`opinerust-pinets-coverage-matrix.md`](opinerust-pinets-coverage-matrix.md): 486 nama, 16 perilakunya beda, 78 belum ada, 392 namanya ada tetapi tes parity belum menutup. `footprint` tidak dipindahkan. Itu tetap study Rust Oracly.

---

## 4. Perbedaan yang sudah terbukti, dan yang harus dibuktikan dulu

### 4.1 Koordinat `chart.point` tidak mengikuti `xloc` kotak

PineTS, `src/namespaces/chart/resolvePoint.ts`: `xloc` gambar yang memilih field. `bar_time` membaca `point.time`. `bar_index` membaca `point.index`. `chart.point.now()` mengisi keduanya, dan pada garis `bar_time` yang dipakai adalah waktu.

OpineRust, `crates/pine-builtins/src/chart/mod.rs` fungsi `point_coords`: kalau `index` berhingga, waktu diabaikan. `xloc` kotak tidak ikut dibaca. `box.set_top_left_point` di `crates/pine-builtins/src/box/mod.rs` memanggil `point_coords` tanpa `xloc`.

SMC mengisi order block begini (`smc.txt` sekitar baris 543–544):

```pine
b_ox.set_top_left_point(chart.point.new(eachOrderBlock.barTime, na, eachOrderBlock.barHigh))
b_ox.set_bottom_right_point(chart.point.new(last_bar_time, na, eachOrderBlock.barLow))
```

`chart.point.new` di OpineRust bertipe `time: f64`, `index: f64`, `price: f64` (`ChartPointNew`). Argumen `na` pada `index` harus tetap `na`, bukan `0` dan bukan error yang membatalkan setter. Ini yang pertama kali diuji, karena kalau `na` menjadi `0`, `point_coords` memakai indeks 0 dan waktu bar order block hilang.

Perbaikan yang ditulis: setter kotak, garis, dan label memilih field dengan aturan `resolvePoint` di atas, memakai `xloc` objek yang sedang diubah. Tesnya empat kasus, masing-masing skrip pendek:

| Kasus | Harapan |
| :--- | :--- |
| `xloc.bar_time`, point `(time=T, index=na, price=P)` | x = T, y = P |
| `xloc.bar_index`, point `(time=na, index=I, price=P)` | x = I, y = P |
| `xloc.bar_time`, point `now` (time dan index sama-sama terisi) | x = time, bukan index |
| `chart.point.new(T, na, P)` tidak melempar dan tidak menyimpan index 0 | index tetap `na` |

### 4.2 Kotak `na` dibuang, garis tanpa `y` juga dibuang

SMC membuat kotak di bar pertama sebagai `box.new(na, na, na, na, xloc = xloc.bar_time, extend = extend.right)` lalu mengisi sudutnya di `drawOrderBlocks`. Objeknya harus tetap hidup selama `na`.

Jembatan sudah memetakan waktu ke indeks bar di `script_box` (`opinerust/src/engine.rs`, `placed_x`). Setelah itu `drawings_from` di `opinerust/src/study.rs` membuang kotak yang `left`, `right`, `top`, atau `bottom`-nya tidak berhingga. `script_line` membuang garis yang `y1` atau `y2`-nya tidak berhingga sebelum sempat dipetakan.

Itu benar untuk objek yang pada bar terakhir memang masih `na`. Itu salah kalau setter sudah menulis koordinat tetapi field yang dibaca keliru (bagian 4.1), atau kalau `barTime` di dalam user-defined type masih `na` karena histori `time` tidak terekam pada bar tempat pivot disimpan.

Tes zona 6 bar `a_zone_box_spans_its_times_and_a_label_sits_below` hanya memanggil setter pada `barstate.islast` dengan `time[4]`. Tes itu boleh tetap. Tes itu tidak menggantikan jalur SMC: array `box`, user-defined type `orderBlock`, `array.get`, `set_top_left_point` pada bar terakhir.

Langkah bukti, sebelum mengubah lagi tempat pembuangan:

1. Jalankan `smc.txt` utuh pada deret OHLCV yang berayun, panjang cukup untuk menghasilkan pivot (ratusan bar, bukan 6). Jangan commit skripnya.
2. Cetak jumlah `output.boxes`, berapa yang keempat sisinya berhingga, dan nilai `left/right/top/bottom` tiga kotak pertama sesudah `drawOrderBlocks`.
3. Kalau sisi masih `na`, lacak `orderBlock.barTime` pada saat `storeOrdeBlock`. `time` harus sudah menjadi series di `opinerust/src/engine.rs` (`series("time", ...)`). Kalau lookback `time[n]` di dalam fungsi yang hanya jalan pada sebagian bar tetap `na`, perbaiki histori series dulu. Jangan mengganti pembuangan kotak dengan menggambar `na` sebagai harga 0.

Warna `na` tetap "tidak digambar", bukan diganti warna default. PineTS mempertahankan `NaN` dan `null` untuk `bgcolor = na` dan `color(na)`. Border order block internal di SMC memang `na`. Isi warnanya yang harus kelihatan.

### 4.3 Zoom menghitung ulang karena chart dan skrip satu thread

Tiga fakta di kode sekarang:

- `src/widget/frame.rs` meminta repaint terus selama `zoom_animation` atau `price_zoom_animation` ada.
- `Indicator::is_incremental` default-nya `false` (`src/studies/indicator_trait.rs`). Study Pine memakai default itu, jadi setiap invalidasi dari bar 0 menjalankan seluruh skrip.
- `run_key` di `opinerust/src/study.rs` memasukkan `now_ms`. Jam berubah, kunci cache tidak cocok, skrip dijalankan dari bar pertama.

PineTS tidak menggambar chart. Kehalusan zoom adalah pekerjaan Oracly:

- Perubahan skala dan geser tidak memanggil `calculate`.
- Kunci cache tidak berisi jam dinding. Isinya panjang bar, waktu dan OHLC bar pertama serta terakhir, ticker, multiplier, dan input.
- Hitung ulang penuh hanya saat bar berubah, input berubah, atau simbol/timeframe berubah.
- Satu hitungan SMC yang panjang tidak boleh menahan frame zoom. Pecah replay supaya satu giliran tetap di bawah satu frame, dan frame gambar memakai hasil terakhir yang sudah jadi.

Jitter scroll tanpa skrip Pine adalah penggambar candle immediate-mode (`src/chart/rendering/candles/candlestick.rs` menggambar setiap candle yang terlihat tiap frame). Itu paket terpisah, setelah zoom berhenti memanggil skrip. Jangan mencampurnya ke perbaikan `box`.

### 4.4 Yang dokumen lama centang sebagai selesai, dan kenyataannya

| Centang lama | Kenyataan 2026-11-12 |
| :--- | :--- |
| Drawing `box`/`line`/`label` jalan | Setter ada. Chart BTC 1D tetap tanpa persegi panjang order block. |
| `xloc.bar_time` diubah ke indeks terdekat | `placed_x` ada. Koordinat sumbernya masih bisa salah karena bagian 4.1. |
| `bar_index[n]` diperbaiki | `time` juga dipublikasikan sebagai series di salinan lokal. Belum dibuktikan lewat `orderBlock.barTime` pada `smc.txt`. |
| `request.security` jalan via host | Demo memakai `HostEnvironment` kosong. Level mingguan/bulanan menjadi `na`. Order block SMC tidak membutuhkan feed kedua. Level HTF membutuhkan. |
| Table jalan | Frame, border, dan `merge_cells` dibuang runtime. |
| Array/matrix inti teruji | Method yang dipakai SMC (`slice`, `indexof`, `shift`, array bertipe `box` dan user-defined type) belum jadi gerbang. |
| `input.*` sebagian | `input.source` masih diabaikan. `input.inline` diterima dan diabaikan. Itu cukup untuk SMC. |

---

## 5. Aturan yang melarang pekerjaan setengah

- Jangan menambah tes yang hanya mengulang setter pada 6 bar lalu menyatakan order block selesai.
- Jangan mencentang satu namespace "jalan" sebelum tes bagian 6 untuk namespace itu lulus.
- Jangan memperbaiki hanya baris yang disebut di chat. Kalau satu skrip gagal, tambahkan perilaku yang hilang ke paket yang sesuai, dengan tes perilaku itu.
- Jangan membuat runtime ketiga. `/root/pintes/PineTS/crates/oracly-runtime` tidak dilanjutkan.
- Jangan memformat ulang `opinerust/src/ui/mod.rs` dan `opinerust/src/ui/study_editor.rs`. `cargo fmt -- <file>` tanpa edition crate merusak `let` chain.
- `cargo test -p opinerust` bukan anggota workspace. Pakai `cargo test --manifest-path /root/egui-charts/opinerust/Cargo.toml`. `cargo test` hanya menerima satu filter nama.
- Satu sesi yang menyentuh runtime wajib menjalankan tes crate yang diubah. Satu sesi yang menyentuh gambar wajib menjalankan `smc.txt` dan melaporkan jumlah kotak berhingga. Klaim "sudah tampil" hanya setelah jumlah itu lebih dari nol dan koordinatnya jatuh di dalam rentang bar.

---

## 6. Paket kerja, berurutan

Setiap paket selesai sebelum paket berikutnya dimulai, kecuali paket 4 (zoom) boleh berjalan setelah paket 1 karena tidak berbagi objek gambar. Paket 5 tidak dimulai sebelum paket 1–3 lulus.

### Paket 1. Titik, kotak, garis, label

**Runtime.** Samakan pemilihan koordinat dengan bagian 4.1 untuk `box.set_top_left_point`, `box.set_bottom_right_point`, `line.set_first_point`, `line.set_second_point`, `label.set_point`, dan konstruktor `new` yang menerima dua `chart.point`. `na` pada argumen `f64` konstruktor point tetap `na`.

Objek yang dibuat `box.new(na,na,na,na)` tetap ada di sink. Setter mengubah objek yang sama, termasuk objek yang diambil dari `array.get`. Salinan yang terlepas dari array adalah bug: SMC mengambil `b_oxes.get(index)` lalu memanggil setter.

`extend.right` tersimpan di objek dan sampai ke `Drawing.extend_right`. `xloc.bar_time` tersimpan sebagai waktu UNIX sampai `placed_x` mengubahnya menjadi indeks bar. Waktu di luar rentang menempel ke bar ujung, seperti `placed_x` sekarang.

Warna `na` dan `color(na)` tidak diganti warna default di objek. Alpha Pine: `color.new(c, t)` dengan `t` 0–100, 0 berarti pekat. `t = 80` tetap kelihatan (alpha sekitar 51).

**Tes runtime**, di `/root/OpineRust`, tanpa `smc.txt`:

- Empat kasus titik di bagian 4.1.
- Kotak lahir `na`, disimpan di `array.new<box>()`, diisi pada bar terakhir lewat `array.get`, keempat sisi berhingga, `extend` kanan, `xloc` waktu.
- Garis `xloc.bar_time` dari `chart.point.now()` memakai waktu, bukan indeks.
- Label `style_label_down` menyimpan style itu.

**Jembatan.** `script_box` / `script_line` / `script_label` memakai koordinat yang sudah berhingga. `placed_x` tetap satu-satunya tempat waktu menjadi indeks. `drawings_from` tetap membuang sisi yang masih `na` pada bar terakhir, dan tidak membuang sisi yang sudah berhingga. Warna isi dan border yang ada tidak ditimpa style plot 0 (`src/chart/indicators/plots.rs`, `mapped_drawings`, sudah punya cabang ini. Jangan dibalik).

**Selesai jika** tes runtime lulus dan tes jembatan `a_zone_box_spans_its_times_and_a_label_sits_below` tetap lulus, ditambah satu tes jembatan baru: array kotak yang diisi pada bar terakhir menghasilkan `DrawingToolType::Rect` dengan indeks bar di dalam rentang deret.

### Paket 2. Series, `var`, dan user-defined type

SMC menyimpan `barTime = time` di dalam type `orderBlock` pada bar pivot, lalu membacanya jauh di belakang. Aturan yang dikunci:

- `time` dan `bar_index` adalah series pada setiap bar, termasuk bar yang tidak menjalankan subscript.
- Subscript `time[n]` di dalam `if` yang baru jalan pada bar ke-k tetap mengembalikan waktu bar `k-n`.
- Field user-defined type menyimpan angka itu apa adanya, bukan `na`, setelah assignment `barTime := time`.
- `var` array dan `var` kotak bertahan antar bar. `barstate.isfirst` hanya membuat kotak sekali.

**Tes:** skrip pendek dengan type `T` berisi `float t`, assignment `t := time` di dalam `if bar_index == 2`, pembacaan `obj.t` pada bar terakhir sama dengan waktu bar 2. Tes kedua: `var array<box>` diisi sekali, setter pada bar terakhir mengubah elemen 0, `array.size` tetap 1.

**Selesai jika** kedua tes lulus di runtime dan, pada jejak bagian 4.2, `orderBlock.barTime` berhingga untuk order block yang `drawOrderBlocks` gambar.

### Paket 3. Array yang dipakai skrip struktur

Method yang harus lulus tes sebelum SMC dijalankan sebagai gerbang, masing-masing satu assert:

- `array.new<box>()`, `push`, `get`, `set`, `size`, `shift`, `pop`, `slice`, `indexof`, `clear`.
- `array.new<line>()` dan `array.new<label>()` dengan method yang sama.
- Array user-defined type: `push` objek, `get` mengembalikan field yang di-assign, `slice(0, n)` memotong dari depan.

`indexof` untuk objek gambar membandingkan identitas handle, bukan angka yang kebetulan sama.

**Selesai jika** tes-tes itu lulus dan jejak bagian 4.2 menunjukkan minimal satu kotak SMC dengan empat sisi berhingga setelah `drawOrderBlocks`.

### Paket 4. Zoom dan geser tidak menjalankan skrip

File: `opinerust/src/study.rs` (`run_key`, penutup `calculate`), `src/widget/studies.rs` (`refresh_values`), `src/widget/frame.rs` (repaint animasi zoom).

- Hapus `now_ms` dari `run_key`.
- `calculate` pada study Pine, ketika kunci bar dan input sama dengan hitungan terakhir, mengembalikan baris yang sudah disimpan dan tidak memanggil interpreter.
- `refresh_studies` tidak menandai `pending_from = Some(0)` hanya karena `BarClock` direkonstruksi dengan isi yang sama. `set_bar_clock` sudah membandingkan `PartialEq`. Jangan mengganti jam tiap frame menjadi alasan replay.
- Selama animasi zoom, painter memakai drawing dan nilai study yang sudah ada.

**Tes:** dua kali `calculate` pada bar yang sama dan input yang sama hanya mengeksekusi skrip sekali. Tes chart: mengubah bar spacing tidak menambah eksekusi study. Tes yang sudah ada `a_failed_run_keeps_the_previous_drawings` tetap lulus. `calculate(&[])` tetap error `NoBars`. Chart tidak memanggil `update(&[], 0)`.

**Selesai jika** tes itu lulus. Rebuild demo tidak termasuk syarat paket ini sampai pengguna meminta. Jitter candle tanpa Pine tidak termasuk paket ini.

### Paket 5. Gerbang SMC

Jalankan `/root/egui-charts/smc.txt` pada deret harian sintetis yang meniru struktur BTC: ada higher high, lower low, dan tembusan close, panjang minimal 300 bar. Jangan commit skrip atau deret itu.

Lulus hanya jika sekaligus:

- Proses tidak mengembalikan error runtime.
- Ada minimal satu `Rect` order block dengan isi warna, `left < right` dalam rentang indeks bar, `top` dan `bottom` di sekitar harga bar sumber.
- Ada garis struktur berhingga.
- Ada label yang teksnya `BOS` atau `CHoCH`.
- Hitungan kedua pada deret yang sama tidak mengeksekusi interpreter lagi (paket 4).

Kalau kotak masih nol, kembali ke jejak bagian 4.2. Jangan menulis cabang khusus string "order block" atau "SMC" di runtime.

Setelah tes itu lulus dan pengguna meminta, `trunk build --release` di `examples/demo` tanpa mematikan server. Pengguna yang menilai chart BTC 1D. Agen tidak menyatakan "sudah sama dengan TradingView" dari tes unit saja.

Input yang default-nya mati (premium/discount, sebagian swing) tidak diharapkan tampil sampai inputnya dihidupkan di tes terpisah. Tes terpisah itu menyalakan `showPremiumDiscountZonesInput` lewat override input dan mengharapkan dua kotak zona. Kalau override input bool belum sampai ke runtime, itu perbaikan `input.bool`, bukan cabang SMC.

### Paket 6. Seluruh matriks, tidak berhenti di SMC

Sumbernya [`opinerust-pinets-coverage-matrix.md`](opinerust-pinets-coverage-matrix.md). Paket 1–5 mengerjakan baris **beda** pada `chart.point`, `box`, `line`, `label`, plus array dan series yang gerbang SMC butuhkan, plus zoom. Setelah kotak SMC tampil, sisa matriks tetap dikerjakan. SMC yang sudah benar tidak mencoret baris lain.

Urutan sisa baris, namespace demi namespace, sesuai urutan di matriks:

1. Setiap baris **beda** yang belum lulus di paket 1–4. Termasuk `input.source`, setter tabel yang membuang frame, border, merge, font, dan tooltip, serta `plotcandle` yang hanya menyimpan bar terakhir.
2. Setiap baris **belum**. Yang sekarang belum ada antara lain `array.new_box`, `array.new_line`, `array.new_label`, delapan `ta.*` volume (`ta.obv`, `ta.accdist`, `ta.nvi`, `ta.pvi`, `ta.pvt`, `ta.wad`, `ta.wvad`, `ta.iii`), lima belas `matrix.*` (`avg`, `min`, `max`, `median`, `mode`, dan `is_*`), `math.ln`, cast `box(na)` / `line(na)` / `label(na)` / `color(na)`, dan 39 fungsi `strategy.*` statistik serta posisi.
3. Setiap baris **ada**: satu tes parity per fungsi, pada deret tetap, sebelum namespace itu ditutup. `ta.sma` yang namanya sudah terdaftar tetap hutang tes ini. Tidak ada namespace yang dicentang hanya karena SMC tidak memanggilnya.

`request.security` namanya ada. Yang kurang di demo adalah feed kedua. Tanpa host, hasil tetap `na` dan skrip tidak crash.

Skrip LuxAlgo yang gagal dicari namanya di matriks. Perbaikan menulis tes untuk baris itu. Kalau namanya tidak ada di matriks, baris baru ditambahkan dulu, lalu dikerjakan. Tambalan untuk satu token hanya masuk kalau tes perilaku mencakup token itu pada skrip pendek.

Mutasi gambar per bar tetap keadaan akhir objek. Riwayat koordinat tengah bar hanya dikerjakan kalau satu baris matriks atau satu tes parity memang membedakan keadaan tengah dan keadaan akhir.

### Paket 7. Gambar candle saat zoom, setelah paket 4

Candle digambar ulang tiap frame di `render_candles`. Setelah skrip tidak lagi ikut zoom, ukur sisa biaya itu. Perbaikan yang sah: mesh candle dan garis study yang skala-nya saja berubah dipakai kembali, bukan disusun ulang dari primitif egui. Jangan mulai paket ini sebelum paket 4 terbukti. Kalau tidak, biaya skrip dan biaya gambar tetap tercampur.

---

## 7. Perintah verifikasi

Runtime:

```bash
cd /root/OpineRust && cargo test --workspace
```

Jembatan:

```bash
cd /root/egui-charts/opinerust && cargo test
```

Chart, hanya kalau menyentuh `src/chart` atau `src/widget`:

```bash
cd /root/egui-charts && cargo test --lib --features ui
```

WASM jembatan, hanya kalau menyentuh eksekusi yang ikut ter-bundle:

```bash
cd /root/egui-charts/opinerust && CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner cargo test --target wasm32-unknown-unknown --test wasm_execute
```

Jejak SMC (paket 5) adalah tes Rust yang membaca `smc.txt` dari path absolut di mesin ini dan di-`ignore` atau di-cfg di luar CI kalau file itu tidak ada. Tes default crate tidak boleh gagal hanya karena file CC itu tidak ada di checkout orang lain. Di mesin ini file itu ada, dan paket 5 tidak lulus tanpa menjalankannya.

---

## 8. Di luar pekerjaan ini

- Menyalin atau menerjemahkan sumber PineTS / Vela ke Rust.
- Melanjutkan `/root/pintes/PineTS/crates/oracly-runtime`.
- Memindahkan footprint, DOM, dan volume profile dalam ke Pine.
- Mengklaim seluruh Pine TradingView. Yang dikerjakan adalah permukaan yang PineTS sudah jalankan, lalu gerbang gambar SMC.
- Commit `smc.txt`.
- Mematikan `trunk serve` di port 8081.
- Mengubah git config, atau mengganti remote `Oracly-Rust-Ugui` menjadi `Oracly-Rust-egui`, kecuali pengguna meminta.
