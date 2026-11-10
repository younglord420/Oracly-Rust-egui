//! OpineRust Studio: the editor dock and the language reference.
//!
//! `StudyEditor` is the code canvas (syntax highlighting, line numbers, hover
//! documentation, inline colour pickers, context menu and formatting) and
//! `OpineRustPanel` is the complete dock that wraps it with a Run header, a
//! quick-action shelf and a pinned Logs drawer. Both are host-agnostic: the host
//! owns compilation and installation, and the panel only reports what it needs.

pub mod panel;
pub mod pine_reference;
pub mod study_editor;

pub use panel::{InstallStudy, OpineRustPanel};
pub use study_editor::{
    Diagnostic, Severity, StudyEditor, StudyEditorAction,
};
