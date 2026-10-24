//! `ui-core`: Foundational math, layout, docking, anchor/offset system, state, and task primitives.
//! Pure Rust, zero platform dependencies, strictly non-blocking.

pub mod anchor;
pub mod dock_tree;
pub mod docking;
pub mod id;
pub mod layout;
pub mod math;
pub mod modifier;
pub mod redux;
pub mod state;
pub mod task;
pub mod text_edit;

pub use anchor::{Anchors, Offsets};
pub use dock_tree::{
    ComputedLeafLayout, ComputedSplitterLayout, DockNode, DockTabPanel, DockTree, SplitAxis,
    WorkspaceLayout,
};
pub use docking::{DockPanel, DockSlot, DockWorkspace, ResolvedDockLayout};
pub use id::ElementId;
pub use layout::{Alignment, Breakpoint, Direction, LayoutItem, SizeConstraint};
pub use math::{Color, CornerRadius, Margin, Padding, Rect, Stroke, Vec2};
pub use modifier::Modifier;
pub use redux::{Reducer, Store, UndoManager};
pub use state::{ActionBus, Signal};
pub use task::{TaskController, TaskId, TaskReceipt, TaskReporter, TaskStatus};
pub use text_edit::TextEditBuffer;


