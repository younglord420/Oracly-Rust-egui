//! Cursors category for left toolbar.
//!
//! Contains cursor/pointer tools:
//! - Cross (crosshair cursor)
//! - Dot (simple dot cursor)
//! - Arrow (pointer arrow)
//! - Eraser

use crate::drawings::DrawingToolType;
use crate::icons::{Icon, icons};
use crate::tokens::DESIGN_TOKENS;
use crate::ui::drawing_toolbar::submenu_builder::{
    FlyoutRow, flyout_row, row_width, section_divider,
};
use crate::ui::drawing_toolbar::toolbar::CursorStateData;
use egui::Ui;

use super::{DrawingToolbarAction, ToolCategory};

/// Cursor tool types (not drawing tools, but cursor modes)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CursorType {
    /// Crosshair cursor.
    Cross,
    /// Dot cursor.
    Dot,
    /// Arrow cursor.
    Arrow,
    /// Pointer with a presentation halo.
    Demonstration,
    /// Trail that follows the pointer while the button is held, then fades.
    Magic,
    /// Click a drawing to delete it.
    Eraser,
}

impl CursorType {
    /// Values in display order.
    pub fn all() -> &'static [CursorType] {
        &[
            CursorType::Cross,
            CursorType::Dot,
            CursorType::Arrow,
            CursorType::Demonstration,
            CursorType::Magic,
            CursorType::Eraser,
        ]
    }

    /// Display name for this value.
    pub fn name(&self) -> &'static str {
        match self {
            CursorType::Cross => "Cross",
            CursorType::Dot => "Dot",
            CursorType::Arrow => "Arrow",
            CursorType::Demonstration => "Demonstration",
            CursorType::Magic => "Magic",
            CursorType::Eraser => "Eraser",
        }
    }

    /// Icon representing this value.
    pub fn icon(&self) -> &'static Icon {
        match self {
            CursorType::Cross => &icons::CROSSHAIR,
            CursorType::Dot => &icons::DOT,
            CursorType::Arrow => &icons::ARROW,
            CursorType::Demonstration => &icons::DEMONSTRATION,
            CursorType::Magic => &icons::MAGIC_CURSOR,
            CursorType::Eraser => &icons::ERASER,
        }
    }

    /// Keyboard shortcut label, when one is assigned.
    pub fn shortcut(&self) -> Option<&'static str> {
        match self {
            CursorType::Cross => None,
            CursorType::Dot => None,
            CursorType::Arrow => None,
            CursorType::Demonstration => None,
            CursorType::Magic => None,
            CursorType::Eraser => None,
        }
    }
}

/// Cursors category implementation
pub struct CursorsCategory;

impl ToolCategory for CursorsCategory {
    fn name(&self) -> &'static str {
        "Cursors"
    }

    fn tooltip(&self) -> &'static str {
        "Cursors"
    }

    fn icon(&self) -> &'static Icon {
        &icons::CROSSHAIR
    }

    fn curr_tool_icon(&self, _sel: Option<DrawingToolType>) -> &'static Icon {
        // Cursors don't have drawing tools, always show Cross
        &icons::CROSSHAIR
    }

    fn all_tools(&self) -> Vec<DrawingToolType> {
        // Cursors category has no drawing tools
        vec![]
    }

    fn sections(&self) -> Vec<(&'static str, Vec<DrawingToolType>)> {
        // Cursors don't use drawing tools
        vec![]
    }

    fn render_submenu(
        &self,
        ui: &mut Ui,
        _sel_tool: Option<DrawingToolType>,
        _favorites: &mut Vec<DrawingToolType>,
    ) -> DrawingToolbarAction {
        // The toolbar stores its cursor state for the frame before it shows this menu.
        let Some(state) = ui
            .ctx()
            .data(|d| d.get_temp::<CursorStateData>(CursorStateData::id()))
        else {
            return DrawingToolbarAction::None;
        };
        // Match tool flyouts: 18px icons, body type, shared row width math.
        let width = CursorType::all()
            .iter()
            .map(|cursor| {
                row_width(
                    ui,
                    &FlyoutRow {
                        icon: Some(cursor.icon()),
                        label: cursor.name(),
                        shortcut: cursor.shortcut(),
                        selected: state.is_active(*cursor),
                        ..Default::default()
                    },
                )
            })
            .fold(
                DESIGN_TOKENS.sizing.drawing_toolbar_ext.submenu_width_xl,
                f32::max,
            );
        ui.set_min_width(width);

        let mut action = DrawingToolbarAction::None;
        for cursor in CursorType::all() {
            if *cursor == CursorType::Eraser {
                section_divider(ui, width);
            }
            let response = flyout_row(
                ui,
                width,
                &FlyoutRow {
                    icon: Some(cursor.icon()),
                    label: cursor.name(),
                    shortcut: cursor.shortcut(),
                    selected: state.is_active(*cursor),
                    check: state.is_active(*cursor),
                    ..Default::default()
                },
            )
            .row;
            if response.has_focus() {
                response.scroll_to_me(None);
            }
            if response.clicked() {
                action = match cursor {
                    CursorType::Eraser => DrawingToolbarAction::ToggleEraserMode,
                    _ => DrawingToolbarAction::SetCursorType(*cursor),
                };
            }
        }
        action
    }
}
