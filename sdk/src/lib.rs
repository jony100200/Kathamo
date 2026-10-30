//! # Kathamo
//!
//! Universal Native UI/UX Framework SDK for Rust.
//!
//! Kathamo is a standalone, reusable, high-performance UI framework for building
//! professional creative applications—including 2D painting workspaces, 3D DCC viewports,
//! procedural nodal graphs, video sequencers, and general desktop engineering applications.
//!
//! ## Quick Start
//!
//! ```rust
//! use kathamo::prelude::*;
//!
//! struct MyApplication {
//!     workspace: WorkspaceHost,
//! }
//!
//! impl KathamoApp for MyApplication {
//!     fn title(&self) -> &str {
//!         "My Creative Studio"
//!     }
//!
//!     fn update(&mut self, ctx: &mut AppContext) {
//!         self.workspace.render(
//!             &mut ctx.cmd_buf,
//!             ctx.viewport,
//!             &ctx.pointer,
//!             &ctx.theme,
//!         );
//!     }
//! }
//! ```

pub mod app;
pub mod components;
pub mod designer;
pub mod editors;
pub mod graph_document;
pub mod inspect;
pub mod layout;
pub mod native;
pub mod prelude;
pub mod runtime;
pub mod snapshot;
pub mod workspace;

// Re-export core crates for low-level access when needed
pub use ui_core as core;
pub use ui_platform as platform;
pub use ui_render as render;
pub use ui_testkit as testkit;
pub use ui_text as text;
pub use ui_themes as themes;
pub use ui_widgets as widgets;

// Top-level re-exports from prelude
pub use prelude::*;
