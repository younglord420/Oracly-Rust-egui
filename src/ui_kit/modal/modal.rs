//! Modal Widget
//!
//! A modal dialog in the dialog chrome, centred on the screen above a dimmed backdrop.

use crate::localization as l10n;
use egui::{Color32, Context, Id, Ui};

use crate::theme::ThemeExt;
use crate::tokens::DESIGN_TOKENS;
use crate::ui_kit::dialog::{dialog_frame, title_row};

/// A modal dialog widget
pub struct Modal {
    id: Id,
    title: String,
    width: Option<f32>,
    height: Option<f32>,
    min_height: Option<f32>,
    closable: bool,
    dim_background: bool,
    rounding: Option<f32>,
    fill: Option<Color32>,
    backdrop: Option<Color32>,
    top: Option<f32>,
    drag_offset: Option<Id>,
}

/// Minimum window width that keeps a draggable modal comfortably on screen.
const DRAG_MIN_SCREEN_WIDTH: f32 = 640.0;

impl Modal {
    /// Create a new modal with the given title. An empty title leaves the header to the content.
    pub fn new(id: impl std::hash::Hash + std::fmt::Debug, title: impl Into<String>) -> Self {
        Self {
            id: Id::unique(id),
            title: title.into(),
            width: None,
            height: None,
            min_height: None,
            closable: true,
            dim_background: true,
            rounding: None,
            fill: None,
            backdrop: None,
            top: None,
            drag_offset: None,
        }
    }

    /// Set the outer width of the modal, including its 16 px padding
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    /// Set the outer height of the modal, including its 16 px padding, for content that fills
    /// the modal, such as a list in a scroll area. The modal keeps 24 px free above and below it
    /// in the window and follows the window when it shrinks. Overrides [`Self::min_height`].
    pub fn height(mut self, height: f32) -> Self {
        self.height = Some(height);
        self
    }

    /// Set the minimum outer height of the modal, including its 16 px padding
    pub fn min_height(mut self, height: f32) -> Self {
        self.min_height = Some(height);
        self
    }

    /// Set whether the modal can be closed with the close button, Escape or a backdrop click
    pub fn closable(mut self, closable: bool) -> Self {
        self.closable = closable;
        self
    }

    /// Override the surface fill, leaving the theme's panel fill as the default.
    pub fn fill(mut self, fill: Color32) -> Self {
        self.fill = Some(fill);
        self
    }

    /// Override the backdrop colour, leaving the theme's modal overlay as the default.
    pub fn backdrop(mut self, color: Color32) -> Self {
        self.backdrop = Some(color);
        self
    }

    /// Override the corner radius, leaving the theme's radius as the default.
    pub fn rounding(mut self, rounding: f32) -> Self {
        self.rounding = Some(rounding);
        self
    }

    /// Keep a fixed top edge while content height changes; clamp it inside the window.
    pub fn top(mut self, top: f32) -> Self {
        self.top = Some(top);
        self
    }

    /// Set whether to dim the background
    pub fn dim_background(mut self, dim: bool) -> Self {
        self.dim_background = dim;
        self
    }

    /// Let the desktop header drag the modal, storing the offset under `offset`.
    ///
    /// The offset is a temporary value the caller owns: insert zero before showing on a fresh
    /// open, and the modal keeps the user's position for as long as that value lives. Dragging
    /// is disabled on narrow windows, where the modal stays in its normal place.
    pub fn draggable_by_header(mut self, offset: Id) -> Self {
        self.drag_offset = Some(offset);
        self
    }

    /// Show the modal
    ///
    /// Returns `Some(R)` if the modal was shown, where R is the result of the content function.
    /// The `open` parameter controls whether the modal is visible and can be set to `false`
    /// to close the modal.
    pub fn show<R>(
        self,
        ctx: &Context,
        open: &mut bool,
        add_contents: impl FnOnce(&mut Ui) -> R,
    ) -> Option<ModalResponse<R>> {
        if !*open {
            return None;
        }

        let mut frame = dialog_frame(ctx);
        if let Some(fill) = self.fill {
            frame = frame.fill(fill);
        }
        if let Some(rounding) = self.rounding {
            frame = frame.corner_radius(rounding);
        }
        let screen_height =
            (ctx.content_rect().height() - 2.0 * ctx.active_theme().spacing.xxxl).max(0.0);
        let height = self.height.map(|height| height.min(screen_height));
        let size = egui::vec2(
            self.width
                .unwrap_or(DESIGN_TOKENS.sizing.dialog.default_width)
                .min((ctx.content_rect().width() - 48.0).max(0.0)),
            height
                .or(self.min_height)
                .unwrap_or(DESIGN_TOKENS.sizing.dialog.default_min_height)
                .min(screen_height),
        ) - frame.total_margin().sum();
        let size = size.max(egui::Vec2::ZERO);
        let max_height = (height.unwrap_or(screen_height) - frame.total_margin().sum().y).max(0.0);
        let backdrop = if self.dim_background {
            self.backdrop.unwrap_or(ctx.active_theme().ui.modal_overlay)
        } else {
            Color32::TRANSPARENT
        };

        let mut close_clicked = false;
        let mut modal = egui::Modal::new(self.id);
        let screen = ctx.content_rect();
        // Dragging needs a modal that follows a stored offset instead of a fixed position.
        // Narrow windows keep the usual fixed placement.
        let drag_offset = self
            .drag_offset
            .filter(|_| screen.width() >= DRAG_MIN_SCREEN_WIDTH);
        let mut anchor_y = 120.0;
        if let Some(offset_id) = drag_offset {
            let offset = ctx
                .data(|data| data.get_temp::<egui::Vec2>(offset_id))
                .unwrap_or(egui::Vec2::ZERO);
            let top = self.top.unwrap_or(120.0).clamp(
                screen.top() + 24.0,
                (screen.bottom() - height.unwrap_or(0.0) - 24.0).max(screen.top() + 24.0),
            );
            anchor_y = top;
            // The area owns the position, so the shared title row can move it through the
            // offset it stores; re-applying the stored offset every frame keeps it clamped.
            let area = egui::Area::new(self.id)
                .order(egui::Order::Foreground)
                .pivot(egui::Align2::CENTER_TOP)
                .current_pos(egui::pos2(screen.center().x, top) + offset)
                // The modal clamps its own bounds after showing; egui's constraint would
                // otherwise pin a dragged modal to the edge and fight that clamp.
                .constrain(false)
                .movable(false);
            modal = modal.area(area);
        } else if let Some(top) = self.top {
            let top = top.clamp(
                screen.top() + 24.0,
                (screen.bottom() - height.unwrap_or(0.0) - 24.0).max(screen.top() + 24.0),
            );
            modal = modal.area(
                egui::Area::new(self.id)
                    .order(egui::Order::Foreground)
                    .pivot(egui::Align2::CENTER_TOP)
                    .fixed_pos(egui::pos2(screen.center().x, top)),
            );
        }
        let mut drag_state = None;
        let response = modal.frame(frame).backdrop_color(backdrop).show(ctx, |ui| {
            ui.set_width(size.x);
            ui.set_max_width(size.x);
            ui.set_min_height(size.y);
            // Start from the window cap each frame, so scrolling content can grow and shrink.
            ui.set_max_height(max_height);
            if let Some(offset_id) = drag_offset {
                // The shared title row stores the header drag under this key.
                ui.data_mut(|data| {
                    data.insert_temp(ui.layer_id().id.with("dialog_offset_id"), offset_id)
                });
                drag_state = Some((offset_id, anchor_y));
            }
            if !self.title.is_empty() {
                close_clicked = title_row(ui, &self.title, self.closable);
            }
            add_contents(ui)
        });

        if let Some((offset_id, _anchor_y)) = drag_state {
            // Keep the whole modal inside the window: a header drag or content growth must not
            // push the title, footer or close button out of reach.
            let bounds = ctx.content_rect().shrink(8.0);
            let rect = response.response.rect;
            let mut shift = egui::Vec2::ZERO;
            if rect.width() <= bounds.width() {
                if rect.left() < bounds.left() {
                    shift.x = bounds.left() - rect.left();
                } else if rect.right() > bounds.right() {
                    shift.x = bounds.right() - rect.right();
                }
            }
            if rect.height() <= bounds.height() {
                if rect.top() < bounds.top() {
                    shift.y = bounds.top() - rect.top();
                } else if rect.bottom() > bounds.bottom() {
                    shift.y = bounds.bottom() - rect.bottom();
                }
            }
            let current = ctx
                .data(|data| data.get_temp::<egui::Vec2>(offset_id))
                .unwrap_or(egui::Vec2::ZERO);
            ctx.data_mut(|data| data.insert_temp(offset_id, current + shift));
        }

        if !self.title.is_empty() {
            let translated_label = l10n::text(ctx, &self.title);
            response.response.widget_info(|| {
                egui::WidgetInfo::labeled(egui::Role::Dialog, true, &translated_label)
            });
            ctx.accesskit_node_builder(response.response.id, |node| node.set_modal());
        }

        // `should_close` covers Escape and a click on the backdrop.
        let close_requested = self.closable && (close_clicked || response.should_close());
        if close_requested {
            *open = false;
        }

        Some(ModalResponse {
            inner: response.inner,
            close_requested,
        })
    }
}

/// Response from showing a Modal
pub struct ModalResponse<R> {
    /// The result of the content function
    pub inner: R,
    /// Whether the modal was requested to close
    pub close_requested: bool,
}

/// A simple modal that manages its own open state
pub struct SimpleModal {
    modal: Modal,
    open: bool,
}

impl SimpleModal {
    /// Create a new simple modal
    pub fn new(id: impl std::hash::Hash + std::fmt::Debug, title: impl Into<String>) -> Self {
        Self {
            modal: Modal::new(id, title),
            open: false,
        }
    }

    /// Set the width
    pub fn width(mut self, width: f32) -> Self {
        self.modal = self.modal.width(width);
        self
    }

    /// Set closable
    pub fn closable(mut self, closable: bool) -> Self {
        self.modal = self.modal.closable(closable);
        self
    }

    /// Open the modal
    pub fn open(&mut self) {
        self.open = true;
    }

    /// Close the modal
    pub fn close(&mut self) {
        self.open = false;
    }

    /// Check if the modal is open
    pub fn is_open(&self) -> bool {
        self.open
    }

    /// Show the modal
    pub fn show<R>(
        &mut self,
        ctx: &Context,
        add_contents: impl FnOnce(&mut Ui) -> R,
    ) -> Option<ModalResponse<R>> {
        // Clone modal config to avoid borrowing issues
        let modal = Modal::new(self.modal.id, &self.modal.title)
            .width(
                self.modal
                    .width
                    .unwrap_or(DESIGN_TOKENS.sizing.dialog.default_width),
            )
            .closable(self.modal.closable)
            .dim_background(self.modal.dim_background);

        modal.show(ctx, &mut self.open, add_contents)
    }
}

/// Helper function to show a centered modal with standard styling
pub fn show_modal<R>(
    ctx: &Context,
    id: impl std::hash::Hash + std::fmt::Debug,
    title: impl Into<String>,
    open: &mut bool,
    add_contents: impl FnOnce(&mut Ui) -> R,
) -> Option<ModalResponse<R>> {
    Modal::new(id, title).show(ctx, open, add_contents)
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{Pos2, RawInput, Rect, ScrollArea, Vec2};

    #[test]
    fn scrolling_modals_follow_the_window_and_shrinking_content() {
        for fixed in [false, true] {
            let ctx = Context::default();
            let mut open = true;
            let frame = |screen_height: f32, rows: usize, open: &mut bool| {
                let mut area = Id::NULL;
                let mut title_top = 0.0;
                let screen = Rect::from_min_size(Pos2::ZERO, Vec2::new(1000.0, screen_height));
                for _ in 0..3 {
                    ctx.run_ui(
                        RawInput {
                            screen_rect: Some(screen),
                            ..Default::default()
                        },
                        |ui| {
                            let modal = Modal::new("scroll", "Title").width(500.0);
                            let modal = if fixed { modal.height(800.0) } else { modal };
                            modal.show(ui.ctx(), open, |ui| {
                                area = ui.layer_id().id;
                                title_top = ui.min_rect().top();
                                ScrollArea::vertical()
                                    .max_height(ui.available_height())
                                    .show(ui, |ui| {
                                        for i in 0..rows {
                                            ui.label(format!("Row {i}"));
                                        }
                                    });
                            });
                        },
                    )
                    .drop_without_applying_deltas();
                }
                let rect = ctx.memory(|m| m.area_rect(area)).unwrap();
                assert!(
                    rect.top() >= 23.0 && rect.bottom() <= screen_height - 23.0,
                    "fixed {fixed}: {rect:?}"
                );
                assert!(title_top >= 24.0, "title above the window");
                rect
            };
            let tall = frame(900.0, 100, &mut open);
            assert!(tall.height() >= 790.0, "fixed {fixed}: {tall:?}");
            let short = frame(450.0, 100, &mut open);
            assert!(short.height() <= 403.0);
            let small = frame(900.0, 3, &mut open);
            if !fixed {
                assert!(small.height() < 300.0, "{small:?}");
            }
        }
    }

    /// A header drag moves the whole modal, a body drag does not, and the modal stays on screen.
    #[test]
    fn header_drag_moves_the_modal_and_stays_on_screen() {
        use egui::{Event, Key, Modifiers, PointerButton, Pos2, RawInput, Rect, Vec2};
        let ctx = Context::default();
        let screen = Rect::from_min_size(Pos2::ZERO, Vec2::new(1000.0, 800.0));
        let offset_id = Id::unique("drag_test_offset");
        let mut open = true;
        let mut header = Rect::NOTHING;
        let mut body = Rect::NOTHING;
        let mut run = |events: Vec<Event>, header: &mut Rect, body: &mut Rect| {
            for _ in 0..2 {
                ctx.run_ui(
                    RawInput {
                        screen_rect: Some(screen),
                        events: events.clone(),
                        ..Default::default()
                    },
                    |ui| {
                        Modal::new("drag_test", "Title")
                            .width(360.0)
                            .top(120.0)
                            .draggable_by_header(offset_id)
                            .show(ui.ctx(), &mut open, |ui| {
                                let title = ui.min_rect();
                                *header =
                                    Rect::from_min_size(title.min, Vec2::new(title.width(), 28.0));
                                ui.add_space(40.0);
                                let start = ui.cursor().min;
                                ui.label("Body");
                                *body = Rect::from_min_size(start, Vec2::new(80.0, 24.0));
                            });
                    },
                )
                .drop_without_applying_deltas();
            }
        };
        run(Vec::new(), &mut header, &mut body);
        let first = ctx
            .memory(|memory| memory.area_rect(Id::unique("drag_test")))
            .unwrap();
        let press = |pos: Pos2| {
            vec![
                Event::PointerMoved(pos),
                Event::PointerButton {
                    pos,
                    button: PointerButton::Primary,
                    pressed: true,
                    modifiers: Modifiers::NONE,
                },
            ]
        };
        let release = |pos: Pos2| {
            vec![
                Event::PointerMoved(pos),
                Event::PointerButton {
                    pos,
                    button: PointerButton::Primary,
                    pressed: false,
                    modifiers: Modifiers::NONE,
                },
            ]
        };

        let start = header.min + Vec2::new(40.0, 14.0);
        run(press(start), &mut header, &mut body);
        run(
            vec![Event::PointerMoved(start + Vec2::new(60.0, 30.0))],
            &mut header,
            &mut body,
        );
        run(
            vec![Event::PointerMoved(start + Vec2::new(120.0, 60.0))],
            &mut header,
            &mut body,
        );
        run(
            release(start + Vec2::new(120.0, 60.0)),
            &mut header,
            &mut body,
        );
        let moved = ctx
            .memory(|memory| memory.area_rect(Id::unique("drag_test")))
            .unwrap();
        assert!(
            (moved.left() - first.left() - 120.0).abs() <= 2.0
                && (moved.top() - first.top() - 60.0).abs() <= 2.0,
            "header drag moved {first:?} to {moved:?}"
        );

        let start = body.center();
        run(press(start), &mut header, &mut body);
        run(
            vec![Event::PointerMoved(start + Vec2::new(100.0, 50.0))],
            &mut header,
            &mut body,
        );
        run(
            release(start + Vec2::new(100.0, 50.0)),
            &mut header,
            &mut body,
        );
        let after_body_drag = ctx
            .memory(|memory| memory.area_rect(Id::unique("drag_test")))
            .unwrap();
        assert!(
            (after_body_drag.left() - moved.left()).abs() <= 1.0
                && (after_body_drag.top() - moved.top()).abs() <= 1.0,
            "body drag must not move the modal: {moved:?} then {after_body_drag:?}"
        );

        let start = header.min + Vec2::new(40.0, 14.0);
        run(press(start), &mut header, &mut body);
        run(
            vec![Event::PointerMoved(start + Vec2::new(-900.0, 900.0))],
            &mut header,
            &mut body,
        );
        run(
            release(start + Vec2::new(-900.0, 900.0)),
            &mut header,
            &mut body,
        );
        let clamped = ctx
            .memory(|memory| memory.area_rect(Id::unique("drag_test")))
            .unwrap();
        let bounds = screen.shrink(8.0);
        assert!(
            bounds.contains_rect(clamped),
            "dragged modal {clamped:?} escaped {bounds:?}"
        );
        let _ = Key::Escape;
    }
}
