//! Transient measure ruler. It is armed from the toolbar or Shift+press, and it is not a drawing.

use crate::drawings::DrawingManager;
use crate::drawings::manager::Ruler;
use egui::{Pos2, Rect, Response, Ui};

const DRAG_SLOP: f32 = 3.0;

/// Handles the ruler for this frame. Returns whether the ruler owns pointer input.
pub(super) fn handle(
    ui: &Ui,
    manager: &mut DrawingManager,
    response: &Response,
    price_rect: Rect,
    view: (u32, u64, u64, u64),
) -> bool {
    let scrolled = ui.input(|input| input.smooth_scroll_delta != egui::Vec2::ZERO);
    if manager
        .ruler
        .as_ref()
        .is_some_and(|ruler| ruler.finished && (ruler.view != view || scrolled))
    {
        manager.ruler = None;
    }
    if ui.input(|input| input.key_pressed(egui::Key::Escape))
        && (manager.measure_mode || manager.ruler.is_some())
    {
        manager.set_measure_mode(false);
        return true;
    }
    let press = ui.input(|input| input.pointer.primary_pressed());
    let release = ui.input(|input| input.pointer.primary_released());
    let pos = response
        .interact_pointer_pos()
        .or_else(|| response.hover_pos());
    if press && manager.ruler.as_ref().is_some_and(|ruler| ruler.finished) {
        manager.ruler = None;
    }
    let shift = ui.input(|input| input.modifiers.shift);
    if !manager.measure_mode
        && press
        && shift
        && manager.active_tool.is_none()
        && !manager.eraser_mode
        && let Some(pos) = interaction_press(ui, pos)
        && price_rect.contains(pos)
        && manager.hit_test(pos).is_none()
    {
        manager.set_measure_mode(true);
        let snapped = snapped(manager, pos);
        manager.ruler = Some(Ruler {
            start: snapped,
            end: snapped,
            press: pos,
            finished: false,
            view,
        });
        return true;
    }
    if !manager.measure_mode {
        return false;
    }
    let Some(pos) = pos else {
        return true;
    };
    let graphic = snapped(manager, pos);
    if press {
        match manager.ruler.as_ref().map(|ruler| ruler.finished) {
            Some(false) => {
                if let Some(ruler) = manager.ruler.as_mut() {
                    ruler.end = graphic;
                    ruler.finished = true;
                    ruler.view = view;
                }
                manager.measure_mode = false;
            }
            _ => {
                let raw = interaction_press(ui, Some(pos)).unwrap_or(pos);
                manager.ruler = Some(Ruler {
                    start: graphic,
                    end: graphic,
                    press: raw,
                    finished: false,
                    view,
                });
            }
        }
    } else if let Some(ruler) = manager.ruler.as_mut()
        && !ruler.finished
    {
        ruler.end = graphic;
        ruler.view = view;
    }
    let dragged = release
        && manager
            .ruler
            .as_ref()
            .is_some_and(|ruler| !ruler.finished && ruler.press.distance(pos) > DRAG_SLOP);
    if dragged {
        if let Some(ruler) = manager.ruler.as_mut() {
            ruler.end = graphic;
            ruler.finished = true;
        }
        manager.measure_mode = false;
    }
    true
}

fn interaction_press(ui: &Ui, fallback: Option<Pos2>) -> Option<Pos2> {
    super::interaction::primary_press_pos(ui).or(fallback)
}

fn snapped(manager: &DrawingManager, pos: Pos2) -> Pos2 {
    manager.snap_point(pos, &manager.drawings)
}
