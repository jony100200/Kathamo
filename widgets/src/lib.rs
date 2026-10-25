//! `ui-widgets`: Commercial-grade creative controls and studio workspaces.

pub mod accordion;
pub mod adapters;
pub mod breadcrumbs;
pub mod canvas;
pub mod canvas_host;
pub mod color_picker;
pub mod command_palette;
pub mod controls;
pub mod dock_view;
pub mod hierarchy_tree;
pub mod icon_cache;
pub mod input;
pub mod inspector;
pub mod navigation_rail;
pub mod node_editor;
pub mod property_inspector;
pub mod range_slider;
pub mod rich_text;
pub mod stylebox;
pub mod timeline;
pub mod tooltip;
pub mod ui;
pub mod virtual_list;
pub mod workspace_host;

pub use accordion::{Accordion, AccordionItemState};
pub use breadcrumbs::{BreadcrumbItem, Breadcrumbs};
pub use color_picker::{ColorWheel, HsvColor};
pub use icon_cache::{CachedVectorIcon, IconAtlas, IntoStudioIcon, StudioIcon};
pub use input::{
    render_checkbox, render_dropdown, render_model_card, render_progress_bar,
    render_segmented_tabs, render_stat_card, render_text_input, render_toggle, CheckboxState,
    DropdownState, ProgressState,
};
pub use navigation_rail::{
    NavigationRail, NavigationRailItem, RailAlignment, RailAnchor, RailLabelMode,
};
pub use range_slider::RangeSlider;
pub use rich_text::{MarkdownView, ChatMessage, ChatRole, ChatTranscript};
pub use tooltip::TooltipOverlay;
pub use ui::{KathamoUi, KolponaUi, Response};

pub use ui_text as text;
pub use ui_text::{FontAssetMeta, FontDefaults, FontManifest, FontRegistry};
pub use ui_themes as theme;
pub use ui_themes::{
    metrics, typography, StudioFontBinaries, StudioTheme, ThemeMode, ThemePalette, TypographyScale,
};

pub use adapters::{
    ModelSelectorAdapter, ServiceStatusAdapter, TaskProgressAdapter, ToolConfigAdapter,
};
pub use canvas::{CanvasShape, InfiniteCanvas};
pub use canvas_host::{CanvasHost, CanvasHostKind, CanvasStroke, CanvasTool};
pub use command_palette::{CommandPalette, PaletteCommand};
pub use controls::{
    render_badge, render_button, render_glass_card, render_status_pill, ButtonStyle,
    NumericScrubber,
};
pub use dock_view::{DockTreeView, DraggingTabState};
pub use hierarchy_tree::{HierarchyEvent, HierarchyItem, HierarchyTree};
pub use inspector::PropertyInspector;
pub use node_editor::{GraphNode, NodeConnection, NodeEditor, NodeGroup, NodePin};
pub use property_inspector::{
    MetadataPropertyInspector, PropertyChangedEvent, PropertyItem, PropertyValue,
};
pub use stylebox::{InteractionState, Shadow, StyleBox};
pub use timeline::{Timeline, TimelineTrack};
pub use virtual_list::VirtualListState;
pub use workspace_host::{CreativeDocument, PresentationMode, WorkspaceHost};
