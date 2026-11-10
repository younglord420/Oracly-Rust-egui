//! OpineRust: run Pine Script studies on Oracly charts.
//!
//! The user-facing engine is named OpineRust. Execution is powered by the
//! [Pinecone](https://github.com/ferranbt/pinecone) Rust runtime (`pine-core`,
//! `pine-lexer`, `pine-parser`, `pine-interpreter`, `pine-builtins`), which is
//! MPL-2.0. That attribution must stay in distributions and notices.
//!
//! The bridge deliberately depends on the core crates directly instead of the
//! `pine-lang` umbrella: the umbrella pulls `pine-data`, which depends on
//! `ureq`/`ring`, and those cannot compile for `wasm32-unknown-unknown`.
//! Everything here builds for both native and the browser.
//!
//! `pine_builtins::per_bar_variables` also calls `SystemTime::now()` for
//! `timenow`, which panics on `wasm32-unknown-unknown`. [`per_bar`] rebuilds the
//! same per-bar values with a host-supplied clock instead.
//!
//! Only indicator plots are supported today. Declarations, plots, gaps (`na`)
//! and per-bar colours are mapped; drawings, tables, alerts, `request.*` and
//! strategies are reported as unsupported rather than silently dropped.

mod context;
mod engine;
mod settings;
mod study;

pub use context::RunContext;
pub use engine::{
    execute, execute_with_context, execute_with_inputs, Bar as EngineBar, Diagnostic,
    DiagnosticSeverity, EngineError, EngineOutput, InputDefault, PlotSeries, ScriptInfo,
    ScriptInput,
};
pub use settings::{apply_override, map_inputs, MappedInputs};
pub use study::{
    build_study, build_study_in_context, build_study_with_clock, OpineRustStudy, StudyBuildError,
};
