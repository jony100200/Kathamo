//! `ui-widgets`: Commercial-grade creative controls and studio workspaces.

pub mod canvas;
pub mod command_palette;
pub mod controls;
pub mod icon_cache;
pub mod input;
pub mod inspector;
pub mod node_editor;
pub mod stylebox;
pub mod timeline;
pub mod ui;
pub mod virtual_list;

pub use icon_cache::{CachedVectorIcon, IconAtlas, IntoStudioIcon, StudioIcon};
pub use input::{
    render_checkbox, render_dropdown, render_progress_bar, CheckboxState, DropdownState,
    ProgressState,
};
pub use ui::{KathamoUi, KolponaUi, Response};

pub use ui_text as text;
pub use ui_text::{FontAssetMeta, FontDefaults, FontManifest, FontRegistry};
pub use ui_themes as theme;
pub use ui_themes::{
    metrics, typography, StudioFontBinaries, StudioTheme, ThemeMode, ThemePalette, TypographyScale,
};

pub use canvas::{CanvasShape, InfiniteCanvas};
pub use command_palette::{CommandPalette, PaletteCommand};
pub use controls::{
    render_badge, render_button, render_glass_card, render_status_pill, ButtonStyle,
    NumericScrubber,
};
pub use inspector::PropertyInspector;
pub use node_editor::{GraphNode, NodeConnection, NodeEditor, NodePin};
pub use stylebox::{InteractionState, Shadow, StyleBox};
pub use timeline::{Timeline, TimelineTrack};
pub use virtual_list::VirtualListState;
