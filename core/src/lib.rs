//! `ui-core`: Foundational math, layout, docking, anchor/offset system, state, and task primitives.
//! Pure Rust, zero platform dependencies, strictly non-blocking.

pub mod anchor;
pub mod docking;
pub mod layout;
pub mod math;
pub mod modifier;
pub mod state;
pub mod task;

pub use anchor::{Anchors, Offsets};
pub use docking::{DockPanel, DockSlot, DockWorkspace, ResolvedDockLayout};
pub use layout::{Alignment, Breakpoint, Direction, LayoutItem, SizeConstraint};
pub use math::{Color, CornerRadius, Margin, Padding, Rect, Stroke, Vec2};
pub use modifier::Modifier;
pub use state::{ActionBus, Signal};
pub use task::{TaskController, TaskId, TaskReceipt, TaskReporter, TaskStatus};
