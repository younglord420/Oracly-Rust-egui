//! The dark, light and high-contrast themes.

use egui::Color32;

use super::{ChartColors, Radii, Spacing, TextSizes, Theme, UiColors};

const fn rgb(r: u8, g: u8, b: u8) -> Color32 {
    Color32::from_rgb(r, g, b)
}

/// Every preset with its name: "dark", "light" and "high-contrast".
pub fn presets() -> [(&'static str, Theme); 3] {
    [
        ("dark", Theme::dark()),
        ("light", Theme::light()),
        ("high-contrast", Theme::high_contrast()),
    ]
}

impl Theme {
    /// Dark UI and chart ("black velvet with violet neon"). This is the default theme.
    pub fn dark() -> Self {
        // Refero Resend palette:
        // Void Black (#000000), Surface Lift (#0b0e14), Graphite Hairline (#292d30),
        // Charcoal (#464a4d), Iron (#6e727a), Ash Gray (#a1a4a5), Smoke Gray (#abafb4),
        // Bone White (#f0f0f0), White (#ffffff), Iris Violet (#9281f7), Iris Violet Glow (#baa7ff),
        // Signal Blue (#3b9eff), Sky Blue (#70b8ff), Pulse Green (#3ad389),
        // Alarm Red (#ff9592), Crimson (#ff6465), Amber (#ffca16).
        let accent = rgb(146, 129, 247);
        Self {
            dark: true,
            ui: UiColors {
                panel_bg: Color32::BLACK,
                panel_bg_secondary: rgb(11, 14, 20),
                panel_bg_floating: rgb(11, 14, 20),
                modal_overlay: Color32::from_black_alpha(180),
                text: rgb(240, 240, 240),
                text_secondary: rgb(171, 175, 180),
                text_muted: rgb(161, 164, 165),
                text_disabled: rgb(70, 74, 77),
                border: rgb(41, 45, 48),
                border_control: rgb(110, 114, 122),
                btn_bg_hover: rgb(27, 29, 32),
                btn_bg_active: rgb(41, 45, 48),
                selection_bg: rgb(72, 58, 160),
                selection_text: Color32::WHITE,
                focus: Color32::WHITE,
                accent,
                accent_hover: rgb(166, 150, 251),
                accent_active: rgb(186, 167, 255),
                accent_text: Color32::BLACK,
                danger_bg: rgb(204, 48, 50),
                danger_bg_hover: rgb(186, 40, 42),
                danger_bg_active: rgb(166, 32, 34),
                danger_text: Color32::WHITE,
                success: rgb(58, 211, 137),
                take_profit: rgb(58, 211, 137),
                stop_loss: rgb(255, 202, 22),
                warning: rgb(255, 202, 22),
                error: rgb(255, 149, 146),
                info: rgb(112, 184, 255),
            },
            chart: ChartColors {
                bg: Color32::BLACK,
                bg_axis: Color32::BLACK,
                bg_tooltip: rgb(11, 14, 20),
                grid_line: rgb(24, 27, 30),
                grid_line_major: Color32::from_rgba_unmultiplied(240, 240, 240, 18),
                axis_text: rgb(240, 240, 240),
                axis_text_secondary: rgb(161, 164, 165),
                crosshair_line: Color32::from_rgba_unmultiplied(161, 164, 165, 130),
                crosshair_label_bg: rgb(41, 45, 48),
                crosshair_label_text: Color32::WHITE,
                candle_up: rgb(58, 211, 137),
                candle_down: rgb(255, 100, 101),
                volume_up_color: rgb(58, 211, 137),
                volume_down_color: rgb(255, 100, 101),
                volume_alpha: 112,
                session_quote_tag: rgb(255, 202, 22),
                volume_tag: rgb(58, 211, 137),
                marker_outline: rgb(240, 240, 240),
            },
            text: TextSizes::default(),
            spacing: Spacing::default(),
            radius: Radii::default(),
        }
    }

    /// Light UI and chart.
    pub fn light() -> Self {
        let dark = Self::dark();
        let accent = rgb(109, 86, 224);
        let accent_hover = rgb(93, 70, 207);
        let accent_active = rgb(76, 55, 186);
        Self {
            dark: false,
            ui: UiColors {
                panel_bg: Color32::WHITE,
                panel_bg_secondary: rgb(235, 235, 235),
                panel_bg_floating: Color32::WHITE,
                modal_overlay: Color32::from_black_alpha(96),
                text: rgb(18, 20, 22),
                text_secondary: rgb(70, 74, 77),
                text_muted: rgb(100, 102, 106),
                text_disabled: rgb(153, 153, 153),
                border: rgb(235, 235, 235),
                border_control: rgb(125, 130, 138),
                btn_bg_hover: rgb(234, 234, 234),
                btn_bg_active: rgb(214, 214, 214),
                accent,
                accent_hover,
                accent_active,
                accent_text: Color32::WHITE,
                selection_bg: rgb(229, 224, 255),
                selection_text: accent_active,
                focus: accent_active,
                success: rgb(20, 118, 66),
                take_profit: rgb(20, 118, 66),
                stop_loss: rgb(180, 112, 0),
                warning: rgb(160, 75, 8),
                error: rgb(198, 40, 40),
                info: rgb(2, 119, 189),
                ..dark.ui
            },
            chart: ChartColors {
                bg: Color32::WHITE,
                bg_axis: Color32::WHITE,
                bg_tooltip: Color32::WHITE,
                grid_line: rgb(228, 230, 233),
                grid_line_major: rgb(214, 217, 220),
                axis_text: rgb(18, 20, 22),
                axis_text_secondary: rgb(70, 74, 77),
                crosshair_line: Color32::from_rgba_unmultiplied(110, 114, 122, 210),
                crosshair_label_bg: rgb(18, 20, 22),
                crosshair_label_text: Color32::WHITE,
                candle_up: rgb(16, 163, 104),
                candle_down: rgb(229, 62, 62),
                volume_up_color: rgb(16, 163, 104),
                volume_down_color: rgb(229, 62, 62),
                volume_alpha: 112,
                session_quote_tag: rgb(217, 130, 0),
                volume_tag: rgb(16, 163, 104),
                marker_outline: Color32::WHITE,
            },
            ..dark
        }
    }

    /// Black surfaces with white text, for low vision and bright surroundings.
    ///
    /// Every text colour except disabled text reads at 7:1 or better on panels and
    /// windows, and outlines and the focus ring at 4.5:1 or better.
    pub fn high_contrast() -> Self {
        let dark = Self::dark();
        Self {
            ui: UiColors {
                panel_bg: Color32::BLACK,
                panel_bg_secondary: rgb(18, 18, 18),
                panel_bg_floating: rgb(10, 10, 10),
                modal_overlay: Color32::TRANSPARENT,
                text: Color32::WHITE,
                text_secondary: rgb(235, 235, 235),
                text_muted: rgb(200, 200, 200),
                text_disabled: rgb(140, 140, 140),
                border: rgb(150, 150, 150),
                border_control: rgb(200, 200, 200),
                btn_bg_hover: rgb(45, 45, 45),
                btn_bg_active: rgb(70, 70, 70),
                // A dark selection keeps the selection text, which egui also uses to outline a
                // focused text input, light enough to read on the black panels.
                selection_bg: rgb(41, 71, 102),
                selection_text: Color32::WHITE,
                focus: rgb(255, 214, 0),
                // The shades get lighter, away from the black text on them.
                accent: rgb(102, 178, 255),
                accent_hover: rgb(140, 198, 255),
                accent_active: rgb(179, 218, 255),
                accent_text: Color32::BLACK,
                danger_bg: rgb(255, 120, 120),
                danger_bg_hover: rgb(255, 154, 154),
                danger_bg_active: rgb(255, 188, 188),
                danger_text: Color32::BLACK,
                success: rgb(90, 220, 120),
                take_profit: rgb(0, 166, 147),
                stop_loss: rgb(255, 166, 0),
                warning: rgb(255, 200, 60),
                error: rgb(255, 170, 170),
                info: rgb(120, 200, 255),
            },
            chart: ChartColors {
                bg: Color32::BLACK,
                bg_axis: Color32::BLACK,
                bg_tooltip: rgb(10, 10, 10),
                grid_line: Color32::from_white_alpha(40),
                grid_line_major: Color32::from_white_alpha(64),
                axis_text: Color32::WHITE,
                axis_text_secondary: rgb(200, 200, 200),
                crosshair_line: Color32::WHITE,
                crosshair_label_bg: rgb(45, 45, 45),
                crosshair_label_text: Color32::WHITE,
                candle_up: rgb(0, 220, 130),
                candle_down: rgb(255, 95, 95),
                volume_up_color: rgb(0, 220, 130),
                volume_down_color: rgb(255, 95, 95),
                volume_alpha: 128,
                session_quote_tag: rgb(255, 200, 60),
                volume_tag: rgb(0, 220, 130),
                marker_outline: Color32::WHITE,
            },
            ..dark
        }
    }
}
