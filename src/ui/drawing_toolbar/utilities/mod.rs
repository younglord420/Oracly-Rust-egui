//! Bottom toolbar utility btns.
//!
//! These are the action btns at the bottom of the left toolbar:
//! - Measure tool
//! - Magnet mode with dropdown
//! - Keep drawing toggle
//! - Lock drawings toggle
//! - Drawing colour swatch
//! - Hide/show objects dropdown with drawing templates
//! - Favourite tools strip toggle
//! - Remove objects dropdown

mod color;
mod favorites;
mod hide_menu;
mod lock;
mod magnet;
mod measure;
mod remove_menu;
mod stay_in_drawing;
mod templates;

pub use color::ColorButton;
pub use favorites::FavoritesButton;
pub use hide_menu::HideMenu;
pub use lock::LockButton;
pub use magnet::MagnetButton;
pub use measure::MeasureButton;
pub use remove_menu::RemoveMenu;
pub use stay_in_drawing::StayInDrawingButton;
pub use templates::TemplatesButton;
