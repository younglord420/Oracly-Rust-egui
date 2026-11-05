//! Text and annotation tool rendering implementations
//!
//! Split into submodules for maintainability:
//! - `text` - Text label and anchored text rendering
//! - `notes` - Note, anchored note, and flag note rendering
//! - `callout` - Callout and comment rendering
//! - `labels` - Price label and price note rendering
//! - `table` - Table rendering
//! - `signpost` - Signpost rendering

mod callout;
mod labels;
mod notes;
mod signpost;
mod table;
mod text;

use crate::drawings::domain::Drawing;
use crate::theme::ThemeExt;
use egui::{Color32, Rect, Stroke, epaint::StrokeKind};

/// Black or white ink that stays readable on `color`.
pub(super) fn contrast_ink(color: Color32) -> Color32 {
    let lum = 0.2126 * f32::from(color.r())
        + 0.7152 * f32::from(color.g())
        + 0.0722 * f32::from(color.b());
    if lum > 165.0 {
        Color32::from_rgb(18, 20, 24)
    } else {
        Color32::WHITE
    }
}

/// Theme axis background used when a text box has no fill of its own.
pub(super) fn axis_background(painter: &egui::Painter, gamma: f32) -> Color32 {
    painter
        .ctx()
        .active_theme()
        .chart
        .bg_axis
        .gamma_multiply(gamma)
}

/// The text body is open when the paint was asked to open it, or the pointer is on `marker`.
pub(super) fn body_open(drawing: &Drawing, painter: &egui::Painter, marker: Rect) -> bool {
    drawing.annotation_open
        || painter.ctx().input(|input| {
            input
                .pointer
                .hover_pos()
                .is_some_and(|pos| marker.contains(pos))
        })
}

pub(super) fn paint_label_box(
    painter: &egui::Painter,
    rect: Rect,
    rounding: f32,
    fill: Option<Color32>,
    border: Option<Stroke>,
) {
    if let Some(fill) = fill {
        painter.rect_filled(rect, rounding, fill);
    }
    if let Some(border) = border {
        painter.rect_stroke(rect, rounding, border, StrokeKind::Inside);
    }
}
