# Matriks cakupan PineTS terhadap OpineRust

**Tanggal**: 2026-11-12
**Acuan**: nama fungsi di `/root/pintes/PineTS/src/namespaces`. Kodenya AGPL-3.0 dan tidak disalin.
**Pembanding**: `#[builtin(name = ...)]` di `/root/OpineRust/crates/pine-builtins`, field yang didaftarkan di `chart`, `math`, `color`, dan `timeframe`, serta `time` / `year` / `month` / … di `src/time/mod.rs`.

File ini antrian kerjanya. Aturan selesai dan urutan paket ada di [`opinerust-pinets-execution-blueprint.md`](opinerust-pinets-execution-blueprint.md). Gerbang SMC tidak menghapus baris yang tersisa di sini.

## Arti status

| Status | Arti | Selesai bila |
| :--- | :--- | :--- |
| **beda** | Namanya ada, perilakunya sudah terbukti tidak sama dengan PineTS. | Tes perilaku untuk catatan di baris itu lulus. Kalau fungsi itu menggambar, hasilnya sampai ke chart. |
| **belum** | PineTS punya fungsi ini. OpineRust tidak mendaftarkannya. | Builtin ada dan satu tes pendek mengunci masukan serta keluaran. |
| **ada** | Namanya terdaftar. Kesamaan hasil belum dikunci tes. | Satu tes parity per fungsi lulus. Namespace tidak ditutup selama masih ada baris **ada** tanpa tes. |

`param`, operator internal `__*`, dan kata kunci yang ikut terpindai tidak masuk daftar. `*.any` adalah bentuk panggilan namespace (`box(na)`, `color(na)`, `line(na)`), bukan method yang ditulis `box.any` di skrip. Kalau barisnya **belum**, cast `na` bertipe itu belum ada.

`footprint` dan volume row tidak masuk antrian. Keduanya tetap study Rust Oracly.

## Jumlah

- Fungsi yang dibandingkan: **486**
- **beda**: 16
- **belum**: 78
- **ada**: 392

Urutan di dalam satu namespace: semua **beda**, lalu semua **belum**, lalu tes parity untuk setiap **ada**. Menutup SMC tidak mencoret sisa baris.

## `chart.point`

beda 1, belum 0, ada 4.

### Perilaku beda

- `chart.point.new` — Argumen `na` tetap `na`. Terkunci oleh `a_drawing_point_follows_its_xloc`. Pin chart ada di `6750407`.

### Ada, tes parity belum menutup

`chart.point.copy`, `chart.point.from_index`, `chart.point.from_time`, `chart.point.now`

## `chart`

beda 0, belum 0, ada 9.

### Ada, tes parity belum menutup

`chart.bg_color`, `chart.fg_color`, `chart.is_heikinashi`, `chart.is_kagi`, `chart.is_linebreak`, `chart.is_pnf`, `chart.is_range`, `chart.is_renko`, `chart.is_standard`

## `box`

beda 2, belum 1, ada 27.

### Perilaku beda

- `box.set_bottom_right_point` — Mengikuti `xloc` kotak. Terkunci oleh `a_drawing_point_follows_its_xloc`. Pin chart ada di `6750407`.
- `box.set_top_left_point` — `bar_time` membaca waktu, `bar_index` membaca indeks. Terkunci oleh tes yang sama. Pin chart ada di `6750407`.

### Belum ada

`box.any`

### Ada, tes parity belum menutup

`box.copy`, `box.delete`, `box.get_bottom`, `box.get_left`, `box.get_right`, `box.get_top`, `box.new`, `box.set_bgcolor`, `box.set_border_color`, `box.set_border_style`, `box.set_border_width`, `box.set_bottom`, `box.set_extend`, `box.set_left`, `box.set_lefttop`, `box.set_right`, `box.set_rightbottom`, `box.set_text`, `box.set_text_color`, `box.set_text_font_family`, `box.set_text_formatting`, `box.set_text_halign`, `box.set_text_size`, `box.set_text_valign`, `box.set_text_wrap`, `box.set_top`, `box.set_xloc`

## `line`

beda 2, belum 1, ada 19.

### Perilaku beda

- `line.set_first_point` — `chart.point.now()` pada garis `bar_time` memakai waktu. Terkunci oleh `a_drawing_point_follows_its_xloc`. Pin chart ada di `6750407`.
- `line.set_second_point` — Sama. Terkunci oleh tes yang sama. Pin chart ada di `6750407`.

### Belum ada

`line.any`

### Ada, tes parity belum menutup

`line.copy`, `line.delete`, `line.get_price`, `line.get_x1`, `line.get_x2`, `line.get_y1`, `line.get_y2`, `line.new`, `line.set_color`, `line.set_extend`, `line.set_style`, `line.set_width`, `line.set_x1`, `line.set_x2`, `line.set_xloc`, `line.set_xy1`, `line.set_xy2`, `line.set_y1`, `line.set_y2`

## `label`

beda 1, belum 1, ada 20.

### Perilaku beda

- `label.set_point` — Mengikuti `xloc` label. Terkunci oleh `a_drawing_point_follows_its_xloc`. Pin chart ada di `6750407`.

### Belum ada

`label.any`

### Ada, tes parity belum menutup

`label.copy`, `label.delete`, `label.get_text`, `label.get_x`, `label.get_y`, `label.new`, `label.set_color`, `label.set_size`, `label.set_style`, `label.set_text`, `label.set_text_font_family`, `label.set_text_formatting`, `label.set_textalign`, `label.set_textcolor`, `label.set_tooltip`, `label.set_x`, `label.set_xloc`, `label.set_xy`, `label.set_y`, `label.set_yloc`

## `polyline`

beda 0, belum 1, ada 2.

### Belum ada

`polyline.any`

### Ada, tes parity belum menutup

`polyline.delete`, `polyline.new`

## `linefill`

beda 0, belum 1, ada 5.

### Belum ada

`linefill.any`

### Ada, tes parity belum menutup

`linefill.delete`, `linefill.get_line1`, `linefill.get_line2`, `linefill.new`, `linefill.set_color`

## `array`

beda 0, belum 7, ada 48.

### Belum ada

`array.new_bool`, `array.new_box`, `array.new_color`, `array.new_label`, `array.new_line`, `array.new_linefill`, `array.new_table`

### Ada, tes parity belum menutup

`array.abs`, `array.avg`, `array.binary_search`, `array.binary_search_leftmost`, `array.binary_search_rightmost`, `array.clear`, `array.concat`, `array.copy`, `array.covariance`, `array.every`, `array.fill`, `array.first`, `array.from`, `array.get`, `array.includes`, `array.indexof`, `array.insert`, `array.join`, `array.last`, `array.lastindexof`, `array.max`, `array.median`, `array.min`, `array.mode`, `array.new`, `array.new_float`, `array.new_int`, `array.new_string`, `array.percentile_linear_interpolation`, `array.percentile_nearest_rank`, `array.percentrank`, `array.pop`, `array.push`, `array.range`, `array.remove`, `array.reverse`, `array.set`, `array.shift`, `array.size`, `array.slice`, `array.some`, `array.sort`, `array.sort_indices`, `array.standardize`, `array.stdev`, `array.sum`, `array.unshift`, `array.variance`

## `matrix`

beda 0, belum 15, ada 34.

### Belum ada

`matrix.avg`, `matrix.is_antidiagonal`, `matrix.is_antisymmetric`, `matrix.is_binary`, `matrix.is_diagonal`, `matrix.is_identity`, `matrix.is_square`, `matrix.is_stochastic`, `matrix.is_symmetric`, `matrix.is_triangular`, `matrix.is_zero`, `matrix.max`, `matrix.median`, `matrix.min`, `matrix.mode`

### Ada, tes parity belum menutup

`matrix.add_col`, `matrix.add_row`, `matrix.col`, `matrix.columns`, `matrix.concat`, `matrix.copy`, `matrix.det`, `matrix.diff`, `matrix.eigenvalues`, `matrix.eigenvectors`, `matrix.elements_count`, `matrix.fill`, `matrix.get`, `matrix.inv`, `matrix.kron`, `matrix.mult`, `matrix.new`, `matrix.pinv`, `matrix.pow`, `matrix.rank`, `matrix.remove_col`, `matrix.remove_row`, `matrix.reshape`, `matrix.reverse`, `matrix.row`, `matrix.rows`, `matrix.set`, `matrix.sort`, `matrix.submatrix`, `matrix.sum`, `matrix.swap_columns`, `matrix.swap_rows`, `matrix.trace`, `matrix.transpose`

## `map`

beda 0, belum 0, ada 11.

### Ada, tes parity belum menutup

`map.clear`, `map.contains`, `map.copy`, `map.get`, `map.keys`, `map.new`, `map.put`, `map.put_all`, `map.remove`, `map.size`, `map.values`

## `ta`

beda 0, belum 8, ada 59.

### Belum ada

`ta.accdist`, `ta.iii`, `ta.nvi`, `ta.obv`, `ta.pvi`, `ta.pvt`, `ta.wad`, `ta.wvad`

### Ada, tes parity belum menutup

`ta.alma`, `ta.atr`, `ta.barssince`, `ta.bb`, `ta.bbw`, `ta.cci`, `ta.change`, `ta.cmo`, `ta.cog`, `ta.correlation`, `ta.cross`, `ta.crossover`, `ta.crossunder`, `ta.cum`, `ta.dev`, `ta.dmi`, `ta.ema`, `ta.falling`, `ta.highest`, `ta.highestbars`, `ta.hma`, `ta.kc`, `ta.kcw`, `ta.linreg`, `ta.lowest`, `ta.lowestbars`, `ta.macd`, `ta.max`, `ta.median`, `ta.mfi`, `ta.min`, `ta.mode`, `ta.mom`, `ta.percentile_linear_interpolation`, `ta.percentile_nearest_rank`, `ta.percentrank`, `ta.pivot_point_levels`, `ta.pivothigh`, `ta.pivotlow`, `ta.range`, `ta.rci`, `ta.rising`, `ta.rma`, `ta.roc`, `ta.rsi`, `ta.sar`, `ta.sma`, `ta.stdev`, `ta.stoch`, `ta.supertrend`, `ta.swma`, `ta.tr`, `ta.tsi`, `ta.valuewhen`, `ta.variance`, `ta.vwap`, `ta.vwma`, `ta.wma`, `ta.wpr`

## `math`

beda 0, belum 1, ada 28.

### Belum ada

`math.ln`

### Ada, tes parity belum menutup

`math.abs`, `math.acos`, `math.asin`, `math.atan`, `math.avg`, `math.ceil`, `math.cos`, `math.e`, `math.exp`, `math.floor`, `math.log`, `math.log10`, `math.max`, `math.min`, `math.phi`, `math.pi`, `math.pow`, `math.random`, `math.round`, `math.round_to_mintick`, `math.rphi`, `math.sign`, `math.sin`, `math.sqrt`, `math.sum`, `math.tan`, `math.todegrees`, `math.toradians`

## `request`

beda 0, belum 0, ada 11.

### Ada, tes parity belum menutup

`request.currency_rate`, `request.dividends`, `request.earnings`, `request.economic`, `request.financial`, `request.footprint`, `request.quandl`, `request.security`, `request.security_lower_tf`, `request.seed`, `request.splits`

## `input`

beda 1, belum 1, ada 12.

### Perilaku beda

- `input.source` — Builtin ada. Override dari dialog studi diabaikan.

### Belum ada

`input.any`

`input.symbol`, `input.timeframe`, dan `input.text_area` terdaftar lewat macro `input_string_like!`, jadi mereka masuk kelompok ada.

### Ada, tes parity belum menutup

`input.bool`, `input.color`, `input.enum`, `input.float`, `input.int`, `input.price`, `input.session`, `input.string`, `input.symbol`, `input.text_area`, `input.time`, `input.timeframe`

## `str`

beda 0, belum 0, ada 17.

### Ada, tes parity belum menutup

`str.contains`, `str.endswith`, `str.format`, `str.format_time`, `str.length`, `str.lower`, `str.pos`, `str.repeat`, `str.replace`, `str.replace_all`, `str.split`, `str.startswith`, `str.substring`, `str.tonumber`, `str.tostring`, `str.trim`, `str.upper`

## `color`

beda 0, belum 1, ada 24.

### Belum ada

`color.any`

### Ada, tes parity belum menutup

`color.aqua`, `color.b`, `color.black`, `color.blue`, `color.from_gradient`, `color.fuchsia`, `color.g`, `color.gray`, `color.green`, `color.lime`, `color.maroon`, `color.navy`, `color.new`, `color.olive`, `color.orange`, `color.purple`, `color.r`, `color.red`, `color.rgb`, `color.silver`, `color.t`, `color.teal`, `color.white`, `color.yellow`

## `time`

beda 0, belum 0, ada 10.

### Ada, tes parity belum menutup

`dayofmonth`, `dayofweek`, `hour`, `minute`, `month`, `second`, `time`, `timestamp`, `weekofyear`, `year`

## `timeframe`

beda 0, belum 0, ada 14.

### Ada, tes parity belum menutup

`timeframe.change`, `timeframe.from_seconds`, `timeframe.in_seconds`, `timeframe.isdaily`, `timeframe.isdwm`, `timeframe.isintraday`, `timeframe.isminutes`, `timeframe.ismonthly`, `timeframe.isseconds`, `timeframe.isticks`, `timeframe.isweekly`, `timeframe.main_period`, `timeframe.multiplier`, `timeframe.period`

## `plot`

beda 1, belum 0, ada 8.

### Perilaku beda

- `plotcandle` — Builtin ada. Sink jembatan membersihkan plotcandle setiap bar, jadi hanya bar terakhir yang selamat.

### Ada, tes parity belum menutup

`barcolor`, `bgcolor`, `fill`, `plot`, `plotarrow`, `plotbar`, `plotchar`, `plotshape`

## `strategy`

beda 0, belum 39, ada 16.

### Belum ada

`strategy.account_currency`, `strategy.any`, `strategy.avg_losing_trade`, `strategy.avg_losing_trade_percent`, `strategy.avg_trade`, `strategy.avg_trade_percent`, `strategy.avg_winning_trade`, `strategy.avg_winning_trade_percent`, `strategy.cash`, `strategy.closedtrades`, `strategy.equity`, `strategy.eventrades`, `strategy.fixed`, `strategy.grossloss`, `strategy.grossloss_percent`, `strategy.grossprofit`, `strategy.grossprofit_percent`, `strategy.initial_capital`, `strategy.long`, `strategy.losstrades`, `strategy.margin_liquidation_price`, `strategy.max_contracts_held_all`, `strategy.max_contracts_held_long`, `strategy.max_contracts_held_short`, `strategy.max_drawdown`, `strategy.max_drawdown_percent`, `strategy.max_runup`, `strategy.max_runup_percent`, `strategy.netprofit`, `strategy.netprofit_percent`, `strategy.openprofit`, `strategy.openprofit_percent`, `strategy.opentrades`, `strategy.percent_of_equity`, `strategy.position_avg_price`, `strategy.position_entry_name`, `strategy.position_size`, `strategy.short`, `strategy.wintrades`

`strategy.risk` bukan satu fungsi. Enam setter-nya sudah terdaftar dan masuk kelompok ada.

### Ada, tes parity belum menutup

`strategy.cancel`, `strategy.cancel_all`, `strategy.close`, `strategy.close_all`, `strategy.convert_to_account`, `strategy.convert_to_symbol`, `strategy.default_entry_qty`, `strategy.entry`, `strategy.exit`, `strategy.order`, `strategy.risk.allow_entry_in`, `strategy.risk.max_cons_loss_days`, `strategy.risk.max_drawdown`, `strategy.risk.max_intraday_filled_orders`, `strategy.risk.max_intraday_loss`, `strategy.risk.max_position_size`

## `table`

beda 8, belum 1, ada 14.

### Perilaku beda

- `table.cell_set_text_font_family` — execute membuang font.
- `table.cell_set_text_formatting` — execute membuang formatting.
- `table.cell_set_tooltip` — execute membuang tooltip.
- `table.merge_cells` — execute mengabaikan rentang sel.
- `table.set_border_color` — execute membuang warna.
- `table.set_border_width` — execute membuang lebar.
- `table.set_frame_color` — execute membuang warna.
- `table.set_frame_width` — execute membuang lebar.

### Belum ada

`table.any`

### Ada, tes parity belum menutup

`table.cell`, `table.cell_set_bgcolor`, `table.cell_set_height`, `table.cell_set_text`, `table.cell_set_text_color`, `table.cell_set_text_halign`, `table.cell_set_text_size`, `table.cell_set_text_valign`, `table.cell_set_width`, `table.clear`, `table.delete`, `table.new`, `table.set_bgcolor`, `table.set_position`

## Di luar antrian

`footprint.*` di PineTS tidak dipindahkan. Order flow, footprint, volume profile dalam, dan heatmap DOM tetap study Rust.

## Cara memperbarui

Ulangi perbandingan nama bila salah satu pohon bertambah fungsi. Status **ada** tidak menjadi selesai tanpa nama tes di perubahan yang sama. Baris tidak dihapus hanya karena skrip yang sedang dibuka tidak memanggilnya.
