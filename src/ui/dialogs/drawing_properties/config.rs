//! Configuration for the drawing properties dialog.

use crate::tokens::DESIGN_TOKENS;

/// Preferred content height; stacked sections scroll above the pinned footer.
const HEIGHT: f32 = 450.0;

/// Configuration for the drawing properties dialog
#[derive(Clone, Debug)]
pub struct DrawingPropertiesConfig {
    /// Dialog width
    pub width: f32,
    /// Dialog height
    pub height: f32,
    /// Tab height
    pub tab_height: f32,
    /// Content padding
    pub content_padding: f32,
    /// Row spacing
    pub row_spacing: f32,
    /// Label width
    pub label_width: f32,
    /// Control width
    pub control_width: f32,
}

impl Default for DrawingPropertiesConfig {
    fn default() -> Self {
        let settings = &DESIGN_TOKENS.sizing.settings_dialog;
        Self {
            width: 520.0,
            height: HEIGHT,
            tab_height: settings.tab_height,
            content_padding: crate::theme::Theme::default().spacing.lg,
            row_spacing: crate::theme::Theme::default().spacing.md,
            label_width: settings.label_width,
            control_width: settings.dropdown_width,
        }
    }
}
