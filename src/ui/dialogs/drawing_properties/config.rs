//! Configuration for the drawing properties dialog.

use crate::tokens::DESIGN_TOKENS;

/// Configuration for the drawing properties dialog
#[derive(Clone, Debug)]
pub struct DrawingPropertiesConfig {
    /// Dialog width
    pub width: f32,
    /// Fixed dialog height, or `0.0` to fit the visible sections.
    pub height: f32,
    /// Tab height (unused; retained for host compatibility)
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
        let theme = crate::theme::Theme::default();
        Self {
            width: 332.0,
            height: 0.0,
            tab_height: settings.tab_height,
            content_padding: theme.spacing.sm,
            row_spacing: theme.spacing.xs,
            label_width: settings.label_width,
            control_width: settings.dropdown_width,
        }
    }
}
