# Inline text editing for every drawing that carries text

Status: implemented. The sections below are the behaviour the code follows.

## Goal

Any drawing whose text is typed by the user is edited **directly on the chart canvas**, in a
small "Enter Text" box placed where that text is painted. The drawing properties dialog is no
longer part of the typing flow; it opens only from the gear (Settings) button or the context menu.

Today the floating selection toolbar's **T** ("Label") button opens the full properties dialog on
its Label tab, and `begin_edit` refuses most text tools. Both must change.

## Current behaviour (what is wrong)

1. `src/widget/tool_interaction/text.rs`, `begin_edit` (lines 9–44) only accepts `TextLabel`,
   `AnchoredText`, `Callout`, `Table`, `Image`, `Tweet`, `Idea`. Every other tool returns early, with
   the comment "Note, pin, comment, signpost, and flag text is edited in the properties dialog."
   Comment, Note, Pin, Signpost, Flag Mark and every line/shape label therefore cannot be typed on
   the canvas.
2. `inline()` (lines 47–52) and `show_editor` (lines 54–245) only know two in-canvas layouts: the
   plain Text box and the Callout bubble. Everything else falls back to a generic popup frame with
   a "Ctrl/Cmd+Enter to save, Esc to cancel" hint.
3. `examples/demo/controller.rs` lines 1155–1157 and `terminal/src/drawings.rs` lines 177–181 map
   `FloatingToolbarAction::OpenText` to the properties dialog (`PropertiesTab::Text`).
4. The demo decides whether to show **T** with `d.tool_type.supports_text_style()`
   (`examples/demo/controller.rs` ~line 1107). That is true for tools whose text is computed
   (Fibonacci, patterns, measures, Price Label…), so **T** appears on tools that have nothing to
   type. The terminal app never calls `set_text_available`, so **T** never shows there.
5. Renderers other than Text/Callout ignore `Drawing::text_editing`, so the painted text would be
   drawn twice under an inline editor.

## Scope: which tools

Use `DrawingToolType::supports_text()` (`src/config/drawing_tool/behavior.rs` line 127) as the
single source of truth. It is `is_text_annotation() || supports_line_label()`.

| Group | Tools | Editor placement |
|---|---|---|
| Plain text | `TextLabel`, `AnchoredText` | unchanged: top-left on `points[0]` |
| Bubble | `Callout`, `Comment` | centred on `box_anchor()`, same frame as Callout today |
| Plate on a pole | `Signpost` | centred on `box_anchor()`, plate frame (pad 9×6, radius 5) |
| Note | `Note` | top-left on `points[0]`, plate frame (pad 8×5, radius 5) |
| Pin | `AnchoredNote` | where the pin's text card is painted (right of the head, see `render_anchored_note`) |
| Flag | `FlagNote` | centred horizontally under the glyph: y = `points[0].y + font_size * 0.5 + spacing.xs` |
| Line / shape / channel labels | every `supports_line_label()` tool (Trend Line, Ray, Extended Line, Horizontal Line/Ray, Vertical Line, Cross Line, Info Line, Arrow, Arrow Marks, Path, Polyline, Rect, Rotated Rect, Circle, Ellipse, Triangle, Arc, Curve, Double Curve, Sector, Brush, Parallel Channel, Flat Top/Bottom, Disjoint Channel) | the label position computed by `place_line_label_shape` (see below) |
| Unchanged special editors | `Table`, `Image`, `Tweet`, `Idea` | keep today's popup editors |

Out of scope (no **T** button, no editor): tools whose text is generated — `PriceLabel`,
`PriceNote`, Fibonacci tools, patterns, measures, positions. Their text colour and size stay on
the toolbar / dialog as today.

## Required behaviour

### Opening the editor

The inline editor opens:

1. **T button** on the floating selection toolbar, for any selected drawing with
   `supports_text()` (plus `Table`/`Image`/`Tweet`/`Idea`, which open their own editors).
2. **Double-click** on a drawing (already wired in `src/widget/tool_interaction/mod.rs`
   lines 288–303 through `hit_test`), now for every `supports_text()` tool.
3. **Placement**: right after a new drawing is created, but **only** for
   `is_text_annotation()` tools (and `Image`). Line/shape tools must **not** open an editor after
   placement — e.g. finishing a Path with a double-click must not pop an editor. Change the
   `drawings.len() > count_before` branch accordingly.

Locked or hidden drawings never open an editor (existing check in `begin_edit`).

### While editing

- The editor is a borderless `TextEdit::multiline` inside a thin frame, with hint text
  "Enter Text" (localized through `l10n::text`), focused immediately.
- Font: the drawing's own text font (`annotation_font()` for labels/notes; `font_size` for
  Text). Colour: the drawing's text colour (`text_color32()` / the ink used by its renderer).
  Bold/italic follow the drawing's settings if feasible; otherwise plain is acceptable.
- Every frame the draft is written to the drawing so the shape follows the typed text
  (bubble/plate grows, line label moves). Use `manager.drawings[i].text = Some(draft.clone())`
  without recording history per keystroke; record one history entry on commit (see below).
  If live write-through is too invasive, at minimum keep the current "commit on finish" model —
  but the editor must sit exactly where the text is painted.
- `Drawing::text_editing = true` for the whole session, and **every renderer that paints typed
  text must skip that text while it is set** (the drawing's body — line, bubble outline, pole,
  pin, flag glyph — still paints):
  - `paint_optional_line_label` in `src/drawings/rendering/style.rs` (line ~74): return early
    when `self.text_editing`.
  - `render_note`, `render_anchored_note`, `render_flag_note` in
    `src/drawings/rendering/annotations/notes.rs`.
  - `render_signpost` in `annotations/signpost.rs`.
  - `paint_bubble` in `annotations/callout.rs` already handles it for Callout and Comment.
  - `render_text_label` in `annotations/text.rs` already handles it.
- Rotated line labels: the editor is axis-aligned (never rotated). Place its top-left at the
  rotated label's anchor/top-left so it sits on the label.
- The editor stays inside the pane (existing clamping + `ScrollArea` max height).

### Keys and finishing

Same as Text today:

- `Enter` inserts a newline.
- `Ctrl+Enter` / `Cmd+Enter` commits.
- A click outside the editor commits (existing `clicked_elsewhere` logic, guarded by
  `opened_frame`).
- `Escape` cancels and restores the previous text.
- Empty result:
  - `TextLabel` / `AnchoredText`: delete the drawing (existing behaviour).
  - Every other tool: set `text = None` (the drawing stays; Note/Comment/Signpost then show their
    placeholder, line labels disappear).
- Commit goes through `manager.edit_drawing(id, …)` so it is one undoable step.
- Remove the "Ctrl/Cmd+Enter to save, Esc to cancel" hint for all in-canvas modes (keep it only
  for Table/Image/Tweet/Idea).

## Implementation plan

### 1. Public entry point in the library

`begin_edit` is `pub(super)` inside `src/widget/tool_interaction`, so hosts cannot call it.

- Move the body of `begin_edit` to a public method on `DrawingManager`
  (`src/drawings/manager/`), e.g.

  ```rust
  /// Opens the on-canvas text editor for a drawing whose text is typed. Returns whether it opened.
  pub fn edit_text(&mut self, id: usize) -> bool
  ```

  `TextEditState` is `pub(crate)` in `src/drawings/manager/mod.rs` line 141; it can stay crate
  private. Keep `text::begin_edit` as a thin wrapper (or call `edit_text` directly from
  `tool_interaction/mod.rs`).
- Accept: `tool.supports_text() || matches!(tool, Image | Table | Tweet | Idea)`.
- Set `text_editing = true` for all in-canvas tools (everything except Table/Image/Tweet/Idea).
- Optionally also expose `Chart::edit_drawing_text(id)` forwarding to
  `self.drawings_mut().edit_text(id)` if that matches other `Chart` conveniences.

### 2. Editor geometry per tool

Replace the `plain` / `bubble` booleans in `show_editor` with an enum, e.g.

```rust
enum Layout {
    Plain,                // TextLabel, AnchoredText: top-left at points[0] + (2-5, 2-3)
    Centered { pad, radius }, // Callout, Comment (bubble), Signpost (plate): centred on box_anchor()
    Plate { pad, radius },    // Note: top-left at points[0]
    Pin,                      // AnchoredNote: right of the pin head
    Below,                    // FlagNote: centred under the glyph
    LineLabel,                // supports_line_label(): at the painted label
    Popup,                    // Table, Image, Tweet, Idea: current popup
}
```

Frame styling:

- `Plain` and `LineLabel`: transparent fill, 1 px stroke of `ink.gamma_multiply(0.3)`, radius 4,
  margin 5×3 (same as Text today).
- `Centered`, `Plate`, `Pin`: the same fill/stroke/radius/padding the renderer uses for that
  plate, so the editor looks like the drawing itself.
- `Below`: like `Plain`.

Size: keep today's estimate (`cols * font * 0.62 + 24` by `lines * font * 1.4 + 14`) using the
draft or the hint as sample, clamped to the pane.

### 3. Line-label position helper

`place_line_label_shape` in `src/drawings/rendering/style.rs` (line 123) is private and needs a
galley. Add a crate-visible helper next to it, e.g.

```rust
/// Screen rect of the user label for `text`, as `paint_optional_line_label` would place it
/// (axis-aligned box around the rotated text's start). `None` when the tool has no line label.
pub(crate) fn line_label_rect(&self, ctx: &egui::Context, text: &str, pane: Rect) -> Option<Rect>
```

- Lay out `text` (or "Enter Text" when empty) with `annotation_font()` via `ctx.fonts_mut(...)`
  (or a `Painter` built from the ui) and call `place_line_label_shape(galley, color, pane)`.
- Use the returned `TextShape.pos` as the editor's top-left (minus the frame margin) and the
  galley size as the minimum editor size.
- `pane` must be the same rect the renderer receives (the price pane rect passed to
  `show_editor`).
- Horizontal/vertical/cross lines, rectangles and channels are already handled inside
  `place_line_label_shape`; nothing tool-specific should be re-implemented.

### 4. Floating toolbar and hosts

- `examples/demo/controller.rs`:
  - ~line 1107: compute **T** availability with `supports_text()` (plus Table/Image/Tweet/Idea),
    not `supports_text_style()`.
  - lines 1155–1157: `FloatingToolbarAction::OpenText => { self.chart.drawings_mut().edit_text(id); }`
    (wrap in `user_action` only if other toolbar actions do so for history).
- `terminal/src/drawings.rs`:
  - call `toolbar.set_text_available(...)` with the same rule;
  - lines 177–181: same `edit_text(id)` call instead of opening the properties dialog.
- The properties dialog keeps its Label tab for people who open Settings on purpose; do not
  remove it.

### 5. Tests to add / update

Replace `dialog_edited_annotations_open_no_editor_and_empty_text_is_dropped` in
`src/widget/tool_interaction/text.rs` with tests that cover:

1. For **every** `DrawingToolType` with `supports_text()`: after `edit_text(id)`, a frame shows a
   focused `TextEdit`; typing "Hello" and pressing Ctrl+Enter sets `drawing.text == Some("Hello")`;
   `text_editing` is false afterwards.
2. Escape restores the previous text.
3. Emptying the text deletes `TextLabel`/`AnchoredText`, and sets `text = None` for every other
   tool (the drawing remains).
4. The editor rect intersects the rect where the text is painted:
   - line labels: compare with `line_label_rect`;
   - Comment/Signpost: contains `box_anchor()`;
   - Note: its min is near `points[0]`;
   - Flag: below `points[0]`.
5. While `text_editing` is set, the renderer paints no text shape containing the drawing's
   text for each tool (reuse the shape-collecting helpers in the renderer tests).
6. Double-click on a Trend Line with no active tool opens its editor; finishing a Path with a
   double-click does **not** open one; placing a Comment/Note/Signpost/Flag/Pin opens one.
7. `PriceLabel`, `PriceNote` and a Fibonacci tool: `edit_text` returns false and no editor shows.
8. Demo test (`examples/demo/controller.rs` tests): selecting a Trend Line and triggering
   `OpenText` leaves `drawing_props.is_open == false` and `chart.drawings().text_editor` set
   (add a `pub fn is_editing_text(&self) -> bool` on `DrawingManager` if a public probe is
   needed).

Update any test that asserted the old list in `begin_edit`
(`placement_and_double_click_edit_annotation_text`) and the accessibility tests if the toolbar
labels change.

## Constraints

- Follow the existing code style: comment density, naming, `l10n::text` for user-visible strings.
- Comments state constraints only; no "changed because" notes.
- Do not mention third-party product names in code, comments, tests, docs or commit messages.
  Before committing run:

  ```bash
  git diff | grep -iE "^\+.*(trading""view|v""ela)" | grep -v WaveLabel
  ```

  It must print nothing.
- Run `cargo test` (library) and the demo tests; all must pass. Run `cargo clippy` with the
  project's settings.
- Rebuild the demo in release: `cd examples/demo && trunk serve --release` and check by hand:
  1. Select a Trend Line, click **T** → an "Enter Text" box appears on the line, no dialog.
  2. Type, click elsewhere → label painted at the same spot.
  3. Place a Comment → box opens inside the bubble immediately.
  4. Double-click a Note, Signpost, Flag Mark, Pin, Rectangle, Parallel Channel → editor on the
     text position for each.
  5. Price Label / Fibonacci selected → no **T** button.
