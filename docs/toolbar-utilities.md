# Drawing toolbar utilities: Zoom, Measure, Icons, Magnet, Keep drawing, Lock all, Arrows

Status: implemented. The sections below are the behaviour the code follows.

Reference behaviour comes from the reference repo checked out at `/tmp/refrepo`:
`src/renderers/native/drawings/{DrawingToolbar,MeasureOverlay,UserDrawingController,DrawingPainter}.ts`,
`src/renderers/native/core/InputController.ts`, `src/core/drawings/{DrawingController,geometry}.ts`,
`src/core/drawings/types/GlyphStamp.ts`. Where the reference has no equivalent, this document says
so and keeps our current feature instead of inventing one.

Line numbers are from commit `f1d6401`; search by the quoted code if they drift.

---

## 1. Remove Zoom In / Zoom Out from the drawing toolbar

Zoom Out only appears after a Zoom In box zoom, so both buttons go. The chart API stays:
`Chart::set_zoom_mode`, `Chart::execute_box_zoom`, `Chart::zoom_back`,
`ChartState::has_zoom_history` and the box-zoom code in `src/widget/pan_zoom.rs` with its tests.

Delete:

- `src/ui/drawing_toolbar/utilities/zoom.rs`, its `pub use zoom::{ZoomInButton, ZoomOutButton}` in
  `utilities/mod.rs` (line 34), and the import in `toolbar/mod.rs` (line 79).
- `toolbar/mod.rs` `render_bottom_tools`: the "Zoom In" and "Zoom Out (only if history)" blocks
  (lines 568–579).
- `toolbar/mod.rs` line 404: the cursor highlight becomes `self.state.sel_tool.is_none()`.
  Line 130 (`self.state.zoom_mode_active = false;`) goes too.
- `ToolbarState::{zoom_mode_active, has_zoom_history}` (`state.rs` lines 74–77, 127–128, 148, 172).
- `DrawingToolbarAction::{ZoomIn, ZoomOut}` (`actions.rs` lines 61–64).
- Demo `examples/demo/controller.rs`: the sync lines ~2001–2003 (`zoom_mode_active`,
  `has_zoom_history`), the `ZoomIn`/`ZoomOut` arms in `handle_drawing_toolbar` (~2057–2065) and in
  the fall-through match (~2196–2197), and the tests that call them (~5311–5330, ~6520–6545).
- Terminal `terminal/src/drawings.rs`: ~347–349, ~405–411 and the test around ~842–858.
- Toolbar tests that set or assert `zoom_mode_active` (`toolbar/mod.rs` ~1118–1140).

Acceptance: no Zoom In/Out button in the rail; `cargo test` in the library, demo and terminal passes;
`rg "ZoomInButton|zoom_mode_active|has_zoom_history" src/ui examples terminal` finds nothing
(the library `Chart` field `zoom_mode_active` in `src/widget` stays).

---

## 2. Measure: a transient ruler, not a drawing

### Current (wrong)

- `MeasureButton` (`utilities/measure.rs`) returns `SelectTool(DrawingToolType::Measure)`. The ruler
  is then placed as an ordinary drawing: pushed to `DrawingManager::drawings`, recorded in history,
  queued for sync/storage, selectable, listed in the object tree, still there after pan/zoom.
- Painting (`src/drawings/rendering/measurements.rs` `render_measure`, lines 17–161):
  - solid hairline cross through the centre, a right-pointing arrow at the right edge, a `+` at the
    centre, and a vertical arrow;
  - label box at the top-left above the rect, filled with the direction colour;
  - label text from `measurement_label` (line 473): change + percent, plus ticks/pips, bars +
    duration, plus a `Vol …` line;
  - colour from `Drawing::measure_color` (`drawing.rs` ~931): blue up / red down.
- There is no Shift+press shortcut and nothing clears the ruler.

### Reference behaviour (`MeasureOverlay.ts`, `UserDrawingController.ts`)

- The ruler is **renderer-local state**, never a drawing: not stored, not undoable, not selectable,
  not in the object tree, not synced.
- **Arm**: the toolbar Measure button toggles measure mode (button highlighted while armed).
  Picking a drawing tool or the eraser disarms it; arming it clears the active tool and selection.
- **Shift + press on the empty plot** arms measure mode and starts the ruler at that point in one
  gesture (a Shift press on a drawing keeps its add-to-selection meaning).
- **Gestures**: click, move, click; or press, drag, release. A release more than 3 px from the press
  finishes it (`DRAG_SLOP = 3`); otherwise the second click finishes it. Moving with no button
  down sizes the ruler while measuring.
- After finishing, measure mode **disarms** but the finished ruler **stays visible** until the next
  press on the chart, a pan, or a wheel zoom, which clears it. Escape cancels an armed/in-progress
  ruler.
- Endpoints use the effective magnet (snapped when the magnet is on, see section 4).
- **Painting**, in screen pixels:
  - direction colour: fixed bullish when `end.price >= start.price`, else bearish (not the theme's
    candle colours); use `DESIGN_TOKENS.semantic` bullish/bearish tokens;
  - box fill: direction colour at alpha 0.12;
  - two **dashed** (4 on, 4 off) 1 px lines in the direction colour: vertical through `midX` from
    `y1` to `y2`, horizontal through `midY` from `x1` to `x2`;
  - one filled arrow at the **destination edge** on `midX`: at the top when up, the bottom when down;
    tip on the edge, base 7 px inside, half-width 5 px. No right arrow, no `+`;
  - label: font 12 px, two lines
    1. `{+/-change} ({+/-percent}%)` with 2 decimals (`signed(n)` = `+12.34` / `-5.00`),
    2. `{N} bar(s), {duration}` (just the duration when bars are unknown);
  - label box: width = widest line + 16, line height 16, height = lines × 16 + 8; centred on
    `midX`, top at `bottom + 8`, flipped to `top − height − 8` when it would overflow the pane
    bottom; fill = theme background, 1 px border in the direction colour, radius 4; text in the
    theme text colour, centred.
  - The ruler paints above every drawing.

### Implementation

1. **State** in `DrawingManager` (crate-private):
   ```rust
   pub(crate) struct Ruler { start: Pos2, end: Pos2, press: Pos2, finished: bool }
   pub(crate) measure_mode: bool,
   pub(crate) ruler: Option<Ruler>,
   ```
   plus `pub fn set_measure_mode(&mut self, on: bool)` and `pub fn is_measuring(&self) -> bool`.
   Arming clears `active_tool`, `curr_drawing`, eraser mode and selection. `set_active_tool(Some(_))`
   and `set_eraser_mode(true)` disarm it.
2. **Input** in `src/widget/tool_interaction/mod.rs` before the active-tool branch:
   - any primary press while `ruler.finished` clears it (then the press continues normally);
   - Shift + primary press on the empty plot with no active tool, not over a drawing
     (`hit_test` is `None`): arm and start;
   - while `measure_mode`: press starts or finishes, move updates `end`, release past 3 px finishes;
     finishing sets `measure_mode = false` and keeps `ruler`;
   - Escape while armed or measuring: disarm and clear;
   - apply `snap_point` to `start`/`end` when the magnet is effective.
3. **Clear on pan/zoom**: clear a finished ruler when the viewport changes (bar spacing, start index or
   price range differs from the frame it was drawn in) or on scroll/zoom input in `pan_zoom.rs`.
4. **Paint** after `render_all` in `tool_interaction`, on the pane painter, per the reference spec
   above. Price/time come from `y_to_price` / `x_to_bar` at paint time; bars between = rounded bar
   index difference; duration via the existing `format_duration`/`elapsed_ms` helpers in
   `measurements.rs`.
5. **Toolbar**: `MeasureButton` returns a new `DrawingToolbarAction::ToggleMeasure`; the button is
   active while `is_measuring()`. Demo and terminal map it to `set_measure_mode(!is_measuring())`
   and sync the highlight from the manager each frame.
6. **Legacy**: keep `DrawingToolType::Measure` so saved charts still load and render; the toolbar no
   longer creates it. Do not change `DateRange`, `PriceRange`, `DateAndPriceRange` or `InfoLine`
   labels (they keep ticks/pips/volume).

### Tests

- Toggle on, click A, move, click B: no drawing added, `can_undo()` false, nothing queued in
  `pending_sync`, `measure_mode` false, ruler still painted.
- Press-drag-release finishes in one gesture; a release within 3 px does not.
- Next press clears the finished ruler; a wheel zoom or pan clears it; Escape cancels.
- Shift+press on empty plot starts it; Shift+press on a drawing does not.
- Painted shapes: dashed segments, one arrow at the destination edge, label lines exactly
  `"+10.00 (+5.00%)"` and `"3 bars, 3h"` for a known case; label below the box, above when near the
  bottom.

---

## 3. Icons, emoji and stickers

The reference has only a small icon stamp: 12 glyphs (`★ ☆ ● ◆ ▲ ▼ ➤ ⚑ ✚ ✕ ❤ ♦`), named sizes
small 16 / normal 22 / large 32 / huge 44 px, fixed on screen, tinted with the line colour, one
anchor at the centre. Our picker (icons, emoji, stickers, search, recents) is a superset; keep it and
fix the defects below.

### Bugs

1. **Rail click does nothing.** `quick_pick_category` returns `None` for `"Icons/Emojis"`
   (`toolbar/mod.rs` line 452), so only hover opens the picker. Fix: a click arms the most recent glyph
   (`glyph_picker.recent[0]`, else `icon:star`) at the picker size, returning `InsertGlyph`; the flyout
   still opens on hover. Show that glyph on the rail button instead of the generic `EMOJI_ICON` once
   one has been used.
2. **Font-size presets shrink glyphs.** The properties dialog Text tab offers 10/12/16/24
   (`tabs/text.rs` `NAMED_FONT_SIZES`), and the floating toolbar is driven by
   `supports_text_style()` which is true for `FontIcon`/`Sticker`. A 32 px glyph becomes 12 px when
   "Normal" is picked. Fix: for `FontIcon` and `Sticker`, use the stamp sizes 16/22/32/44
   (Small/Normal/Large/Huge) in the dialog, keep any custom picker size in the list, and do not offer
   bold/italic/text colour for them (`supports_text_style` returns false; their colour is the line colour).
3. **Hidden background circle.** `render_font_icon` (`rendering/media.rs` lines 419–436) paints a
   filled circle with a border when `stroke_width > 2.0` for font glyphs. The width chip on the floating
   toolbar changes `stroke_width`, so emoji suddenly gain a disc. Remove that branch and hide the width
   and line-style chips for `FontIcon`/`Sticker` in the demo/terminal selection toolbar.
4. **Sticker outline invisible on dark theme.** `glyphs::paint` forces sticker strokes to
   `rgb(35, 39, 47)` (`glyphs.rs` lines 37–41). On a dark chart the outline and eyes disappear. Use the
   theme text colour for sticker outlines and features (`painter.ctx().active_theme().ui.text`), keep
   the fills.
5. **Default text `"*"`.** `render_font_icon` falls back to `"*"` when `text` is `None`. A `FontIcon`
   without a glyph should paint `icon:star` (the reference default `★`).

### Keep

- Picker groups, search, recents (max 12), drag value 8–128 px for the placement size.
- Corner handles resize the glyph (`services/handle.rs` lines 102–110, 278–286).
- Keep-drawing re-arms the same glyph and size after each placement
  (`manager/interaction.rs` lines 683–704).

### Tests

- Clicking the rail button with an empty recents list arms `icon:star` at 32 px; with recents, the
  first recent glyph.
- Dialog Text tab for a `FontIcon` lists 16/22/32/44 and no bold/italic.
- `stroke_width = 4` on an emoji paints no circle shape.
- A sticker painted with the dark theme uses a stroke colour different from the dark background.

---

## 4. Magnet

### Bugs

1. **Clicking the magnet button does nothing.** `MagnetButton::show` is only passed to
   `menu_button`, and `quick_pick_category` returns `None` for `"Magnet"` (line 452). Reference: the
   icon click toggles the **last-used strength** on/off (default strength Weak); the menu opens from
   hover/the chevron.
   Fix: on click return `ToggleMagnet`; in `apply_pick` (lines ~800–803) a toggle while off turns it on
   with `state.magnet_type`, while on turns it off. Remember the last non-off type. Remove the
   early `return None` for "off while off" only for the menu's "Off" row (keep that row idempotent).
2. **OHLC duplicates Strong.** `MagnetType::OHLC` behaves exactly like Strong (see the comment in the
   demo sync, `controller.rs` ~2011). Reference modes are Off / Weak / Strong. Remove the OHLC row from
   `MagnetButton::show_submenu`; keep the enum variant deserialisable but map it to Strong.
3. **Ctrl/Cmd inverts instead of forcing Strong.** `DrawingManager::magnet_enabled`
   (`manager/mod.rs` lines 517–520) is `magnet_mode ^ ctrl`, so holding Ctrl with the magnet on turns
   snapping **off**, and with it off uses whatever `strong_magnet` holds. Reference
   (`geometry.ts` lines 36–41): holding Ctrl/Cmd is a momentary **Strong** override regardless of the
   sticky mode. Fix: `effective = if ctrl { Strong } else if magnet_mode { sticky type } else { Off }`,
   and use it in `snap_point` and `tool_interaction/mod.rs` (target gathering, lines 120–215).
   Update `modifier_placement_constrains_shapes_and_temporarily_inverts_magnet`.
4. **No snap indicator.** Reference paints a ring on the point the next anchor will snap to while
   placing or dragging with an effective magnet: radius 5.5, stroke 1.5 px, theme text colour,
   alpha 0.9 (`DrawingPainter.paintSnapRing`). Add it in `tool_interaction` after drawings paint, only
   when a snap target was actually used for the hover/drag point.
5. **Button state.** Reference: icon at 50% opacity when off; tooltip `Magnet snap: Off`,
   `Magnet snap: Weak`, `Magnet snap: Strong`. Ours uses a fixed "Magnet mode" tooltip.

### Keep

- "Snap to indicators" row (our addition; the reference has none).

### Tests

- Click toggles off → Weak (default) → off; after choosing Strong in the menu, click toggles
  off → Strong.
- Ctrl held with magnet off snaps strongly; Ctrl held with magnet Weak still snaps (strong).
- Menu shows exactly Off, Weak magnet, Strong magnet, Snap to indicators.
- Snap ring painted at the target while placing a trend line with the magnet on; none with it off.

---

## 5. Keep drawing (stay in drawing mode)

Reference (`DrawingController.ts` lines 679–682): after a placement, **every tool reverts to the
pointer unless stay mode is on, except Brush and Highlighter, which always stay armed**. Tooltip:
"Stay in drawing mode".

### Bugs

1. Brush and Highlighter disarm after every stroke when stay mode is off.
   `finish_curr_drawing` (`manager/interaction.rs` lines 696–698) clears `active_tool` for all tools.
   Fix: skip the reset for `DrawingToolType::Brush | DrawingToolType::Highlighter`.
2. Tooltip says "Keep drawing" (`utilities/stay_in_drawing.rs` line 17). Use "Stay in drawing mode"
   (and add the localisation key).

### Check (no change unless broken)

- Escape disarms the tool even with stay mode on (`selection.rs` Escape test ~585 covers the
  non-stay case; add the stay-mode case).
- The tool flyout stays open after a pick while stay mode is on (`toolbar/mod.rs` lines 755–760) —
  keep.

### Tests

- Stay off: placing a TrendLine disarms; drawing two Brush strokes in a row works without
  re-picking.
- Stay on: two TrendLines in a row without re-picking; Escape disarms.

---

## 6. Lock all drawings

The reference has no lock-all button in the drawing toolbar; it locks/unlocks all members of a group
from the object tree header. Our toolbar button stays; fix its inconsistencies.

### Bugs

1. **Two code paths.** The demo calls `DrawingManager::toggle_lock_all()` (decides from the
   drawings, `controller.rs` ~2174), the terminal uses the toolbar's flipped `state.lock_all` and
   `edit_drawings` (`terminal/src/drawings.rs` ~492–500). Add one library method:
   ```rust
   /// Locks or unlocks every drawing as one undo step; returns whether anything changed.
   pub fn set_lock_all(&mut self, locked: bool) -> bool
   ```
   implement `toggle_lock_all` with it, and have both hosts call
   `set_lock_all(!all_locked)` where `all_locked` is computed from the manager, not the toolbar.
2. **Button with nothing to lock.** With no drawings the button still toggles its local state for a
   frame. Disable it (`add_enabled_ui`) when there are no drawings; tooltip stays
   "Lock all drawings".
3. **Toolbar flips its own state.** `render_bottom_tools` sets `self.state.lock_all = !…` before the
   host acts (line 596). Remove that; the host syncs `lock_all` from the drawings every frame.

### Open question for the user (do not decide silently)

After "Lock all", a newly placed drawing is unlocked, so the button returns to "unlocked". Either keep
this (lock-all is a one-shot action), or make it a mode that also locks new drawings until turned off.
Implement the one-shot behaviour unless the user chooses the mode.

### Tests

- Mixed locked/unlocked → click locks all, one undo restores the mix.
- All locked → click unlocks all.
- No drawings → button disabled, no history entry.
- Demo and terminal produce the same result for the same drawings.

---

## 7. Arrows (Arrow Marker, Arrow, Arrow Mark Up/Down/Left/Right)

Reference: `src/core/drawings/types/{Arrow,ArrowMark}.ts`, `DrawingPainter.ts` (`paintArrowhead`
lines 1781–1795, `paintArrowMark` 1749–1761, `labelLayout` 1921–1927), and the toolbar group
`ARROW_TYPES = ['arrow', 'arrowmarkup', 'arrowmarkdown']` (`core/drawings/toolbar.ts` line 94).

### Bugs

1. **Arrow Marker is a duplicate of Arrow.** `render_arrow_marker` (`rendering/lines.rs`
   lines 472–475) just calls `render_arrow`, so the flyout offers two tools that paint the same
   thing. Arrow Marker is also missing from the line-ends list in the properties dialog
   (`tabs/style.rs` lines 151–157), so its head cannot be changed. The reference has no Arrow
   Marker. Fix: remove `ArrowMarker` from the ARROWS section (`src/drawings/categories.rs`
   ~line 210) and from `src/config/drawing_tool/categories.rs` (~lines 119, 288); keep the enum
   variant so saved drawings still load and render.
2. **Arrow head too small on thin lines.** Head length is `max(4, width × 4)`
   (`rendering/style.rs` line 674 with `ArrowStyle::size_multiplier`, `domain/options.rs` line 70),
   so a 1 px arrow gets a 4 px head. Reference: length `max(9, width × 3.5)`, spread π/7 each side
   of the shaft. Fix `render_marker` for `ArrowStyle::Arrow`/`OpenArrow` to that geometry
   (tip, `tip − len·(cos(a∓π/7), sin(a∓π/7))`).
3. **Blunt tip on thick arrows.** The shaft is stroked all the way to the tip and the head is
   painted on top. Near the tip the head is narrower than the stroke, so the shaft's butt end shows
   beside the point. Fix: for an end with `ArrowStyle::Arrow`, stop the shaft at the head base
   (`tip − dir · len · cos(π/7)`); do it for both ends when the start also has a head. Apply in
   `render_arrow` and the `finish_style` segment rewrite (`style.rs` lines 393–406) so dashed
   styles stop there too.
4. **Arrow Mark size follows the line width.** `arrow_mark_glyph` (`rendering/lines.rs`
   lines 501–504) scales by `stroke_width / 2`. Reference glyph is constant pixels
   (head 11, head width 9, stem width 3.5, length 22; `ArrowMark.ts` lines 8–9) and its settings are
   only colour + text. Fix: remove the scale; hide the width and line-style chips for the four
   Arrow Mark tools in the demo and terminal selection toolbars, and drop their line width/style
   rows from the properties dialog.
5. **Arrow Mark label covers the glyph.** Arrow Marks are `supports_line_label()`, so their label
   goes through `place_line_label_shape` (`rendering/style.rs`): one point, Top alignment, label
   centred above the tip. Arrow Mark Down's body rises 22 px above the tip, so the label overlaps
   it; Left/Right overlap partially. Reference (`labelLayout`): centred on the glyph bounds,
   `top = bounds.y − 4 − label height` for Arrow Mark **Down**, `top = bounds.bottom + 4` for every
   other mark. Add that case at the top of `place_line_label_shape` (using `arrow_mark_glyph`
   bounds); `line_label_rect` and the inline label editor follow automatically.
6. **Arrow Mark Up and Arrow Mark Right paint a broken glyph** (confirmed by the user; Down and
   Left look right). `render_arrow_mark` (`lines.rs` lines 492–496) paints the concave 7-point arrow
   with `Shape::convex_polygon`, tip first. epaint's `fill_closed_path`
   (`epaint-0.37.0/src/tessellator.rs` lines 895–902) **reverses** any path whose
   `cw_signed_area` is negative, then `triangulate_convex_path` (lines 864–872) fans from the new
   first vertex. After the reversal that vertex is a head-base corner instead of the tip, and the
   fan from that corner to the stem's bottom corners fills a wedge outside the arrow beside the
   stem.
   The base (Up) glyph is counter-clockwise in screen space (signed area −352). The per-direction
   transform in `arrow_mark_glyph` (lines 515–520) keeps or flips that:

   | Direction | Transform | Determinant | Winding | Reversed by epaint |
   |---|---|---|---|---|
   | Up | (x, y) | +1 | counter-clockwise | yes → broken |
   | Right | (−y, x) | +1 | counter-clockwise | yes → broken |
   | Down | (x, −y) | −1 | clockwise | no |
   | Left | (y, x) | −1 | clockwise | no |

   Fix (either):
   - **Minimal:** keep the tip at index 0 and make the polygon clockwise: when its signed area is
     negative, reverse `points[1..]` only. epaint then does not reverse it, and the fan from the tip
     is correct because the arrow is star-shaped from the tip.
   - **Robust (preferred):** paint two convex shapes in the same colour, the head triangle
     `[tip, head-left, head-right]` and the stem rectangle, each clockwise. Keep
     `arrow_mark_glyph` (the 7-point outline) for hit testing and bounds.

### Tests

- Arrow at width 1: head length 9 px; at width 4: 14 px.
- Thick arrow: no shaft segment reaches the tip (shaft ends at the head base).
- Arrow Mark glyph identical at `stroke_width` 1 and 6.
- Arrow Mark Down label rect lies above the glyph bounds; Up/Left/Right below; neither intersects the
  glyph.
- For all four Arrow Marks, tessellate the painted shapes and check that a point beside the stem
  below the head (tip + (7, 18) rotated by the direction) is **not** covered, and the stem centre
  is. The existing `arrow_marks_point_away_from_the_anchor_in_the_line_colour` only checks vertex
  positions, which is why this bug passed.
- ARROWS flyout lists Arrow, Arrow Mark Up/Down/Left/Right and no Arrow Marker; a saved
  `ArrowMarker` drawing still loads and paints.

---

## Constraints

- Follow existing style; comments only for constraints the code cannot show.
- No third-party product names in code, comments, tests, docs or commit messages. Before committing:
  ```bash
  git diff | grep -iE "^\+.*(trading""view|v""ela)" | grep -v WaveLabel
  ```
  must print nothing.
- Run `cargo fmt --check`, `cargo clippy --all-features --all-targets -- -D warnings`,
  `cargo clippy --no-default-features --all-targets -- -D warnings`, `cargo test --lib`, the demo
  tests (`cd examples/demo && cargo test`) and `cd terminal && cargo check --tests`.
- Rebuild the demo in release (`cd examples/demo && trunk serve --release --address 127.0.0.1 --port 8081`)
  and check by hand: no Zoom buttons; Measure ruler click-click and Shift+drag, cleared on next click;
  magnet click toggles and Ctrl forces strong with a snap ring; two brush strokes in a row;
  lock-all disabled with no drawings; icon rail click places the last glyph; stickers visible on the
  dark theme; thin and thick arrows have sharp, visible heads; Arrow Mark Up/Down/Left/Right all paint clean
  arrows with no wedge beside the stem; Arrow Mark size ignores width and its
  label sits clear of the glyph; no Arrow Marker in the ARROWS flyout.
