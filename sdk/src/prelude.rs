//! Convenient Master Prelude for Kathamo SDK.
//!
//! ```rust
//! use kathamo::prelude::*;
//! ```

// 1. Math & Primitives
pub use ui_core::{
    resolve_grid_layout, resolve_stack_layout, resolve_wrap_layout, Breakpoint, Color,
    CornerRadius, Rect, SizeConstraint, Stroke, Vec2,
};

// 2. State & Data Binding
pub use ui_core::{ActionBus, Signal, Store, UndoManager};

// 3. Platform & Input
pub use ui_platform::{
    DpiScale, FocusManager, ImeEvent, Key, Modifiers, PointerButton, PointerState, Shortcut,
    TextInputEvent,
};

// 4. Render & Commands
pub use ui_render::{
    BackendKind, CommandBuffer, CpuBackend, DrawCommand, GpuBackend, RenderBackend, RenderStats,
};

// 5. Themes & Design Tokens
pub use ui_themes::{StudioTheme, ThemeMode, ThemePalette};

// 6. Docking & Workspace Geometry
pub use ui_core::{DockDropZone, DockNode, DockTabPanel, DockTree, SplitAxis};

// 7. Core Workspace & Editors
pub use crate::editors::{
    AssetBrowserEditor, CanvasEditor, ConsoleEditor, Editor, EditorDescriptor, EditorRegistry,
    HierarchyEditor, InspectorEditor, NodeGraphEditor, TimelineEditor,
};
pub use crate::workspace::{DetachedPanel, Workspace, WorkspaceHost, WorkspacePreset};

pub use crate::designer::{
    DesignDocument, DesignElement, DesignKind, DesignPlacement, DesignTemplate,
};
pub use crate::graph_document::GraphDocument;
pub use crate::runtime::{
    ComponentRegistry, DesignRuntime, DesignWidget, ResponsiveRule, UiEvent, UiEventKind, UiValue,
};

// 8. Creative Components
pub use crate::components::{
    AssetBrowser, AssetItem, AssetViewMode, ConsoleLogEntry, ConsoleLogLevel, ConsoleView,
    ContextMenu, DialogAction, DialogHost, Menu, MenuBar, MenuItem, NotificationHost,
    NotificationLevel, NotificationToast,
};

// 9. Declarative Layout Builder
pub use crate::layout::{LayoutBuilder, LayoutDirection};

// 10. Application Lifecycle & Context
pub use crate::app::{AppContext, HeadlessAppRunner, ImageFrame, KathamoApp, WindowRequest};
pub use crate::native::{run_native, NativeAppRunner};

// 11. Agent Inspection & Verification
pub use crate::inspect::InterfaceInspector;
pub use ui_core::{UiInspectorNode, UiInspectorSnapshot};

// 12. Standard Widgets
pub use ui_widgets::{
    CanvasHost, CanvasHostKind, CanvasStroke, CanvasTool, ChatMessage, ChatRole, ChatTranscript,
    GraphNode, HierarchyEvent, HierarchyItem, HierarchyTree, KathamoUi, KolponaUi, MarkdownView,
    MetadataPropertyInspector, NodeConnection, NodeEditor, NodeGroup, NodePin,
    PropertyChangedEvent, PropertyItem, PropertyValue, Response, Timeline, TimelineTrack,
};
