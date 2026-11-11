//! OpineRust: run Pine Script studies on Oracly charts.
//!
//! The user-facing engine is named OpineRust. Execution is powered by the
//! [Pinecone](https://github.com/ferranbt/pinecone) Rust runtime (`pine-core`,
//! `pine-lexer`, `pine-parser`, `pine-interpreter`, `pine-builtins`, which is
//! MPL-2.0). That attribution must stay in distributions and notices.
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
//! # What a script can reach
//!
//! - **Plots** (`plot`, `plotshape`, `plotchar`, …) with per-bar colours.
//! - **Drawings** (`box`, `line`, `label`, `table`, `linefill`, `polyline`),
//!   captured by [`output::DrawingOutputSink`] and mapped to
//!   [`EngineOutput`]'s `boxes`/`lines`/`labels`/`tables`.
//! - **Strategies** (`strategy.entry`, …) executed by the runtime's own broker;
//!   the fills come back as [`TradeMarker`]s.
//! - **`request.*`** and **`import`**, answered from the host through
//!   [`HostEnvironment`]. A host that supplies nothing gets `na` and a named
//!   import failure instead of a blanket "unsupported" compile error.
//!
//! Nothing here fetches market data or reads the wall clock: both are host
//! decisions, so the same code path runs in a browser tab and on a server.

mod context;
mod engine;
mod host;
mod output;
mod settings;
mod study;

#[cfg(feature = "studio")]
pub mod ui;

pub use context::RunContext;
pub use engine::{
    execute, execute_with_context, execute_with_environment, execute_with_inputs, Bar as EngineBar,
    Diagnostic, DiagnosticSeverity, EngineError, EngineOutput, InputDefault, PlotSeries, ScriptBox,
    ScriptCell, ScriptInfo, ScriptInput, ScriptLabel, ScriptLine, ScriptMark, ScriptTable,
    TradeMarker,
};
pub use host::{
    ChartOnlyProvider, HostEnvironment, LibrarySource, MemoryLibraries, SecurityProvider,
};
pub use settings::{apply_override, map_inputs, MappedInputs};
pub use study::{
    build_study, build_study_in_context, build_study_with_clock, build_study_with_environment,
    OpineRustStudy, StudyBuildError,
};

#[cfg(feature = "studio")]
pub use ui::{InstallStudy, OpineRustPanel, StudyEditor, StudyEditorAction};
