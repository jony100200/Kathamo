//! `ui-platform`: OS window lifecycle, zero-flicker startup fences, input, and DPI management.

pub mod accessibility;
pub mod dpi;
pub mod input;
pub mod window;

pub use accessibility::{
    AccessibilityAction, AccessibilityNode, AccessibilityRole, AccessibilityTree,
};
pub use dpi::DpiScale;
pub use input::{
    FocusManager, ImeEvent, Key, Modifiers, PointerButton, PointerState, Shortcut, TextInputEvent,
};
pub use window::{LifecycleCoordinator, WindowLifecycle, WindowOptions};
pub use winit;
