//! Universal Editor Trait and Dynamic Editor Registry for Kathamo SDK.

use std::collections::HashMap;
use std::sync::Arc;

use ui_core::*;
use ui_platform::{ImeEvent, Key, Modifiers, PointerButton, PointerState};
use ui_render::CommandBuffer;
use ui_themes::ThemePalette;
use ui_widgets::{
    CanvasHost, HierarchyTree, MetadataPropertyInspector, NodeEditor, Timeline,
};

use crate::components::{AssetBrowser, ConsoleView};

/// Pluggable editor contract. Any application tool, 2D canvas, 3D viewport,
/// nodal compositor, or media player implements this trait to dock cleanly.
pub trait Editor: Send + Sync {
    /// Unique identifier for this editor type (e.g. "canvas_2d", "viewport_3d", "node_graph").
    fn id(&self) -> &str;

    /// User-facing display title for tabs and headers.
    fn title(&self) -> &str;

    /// Category for organizing editors in menus (e.g. "Viewports", "Animation", "Data").
    fn category(&self) -> &str {
        "General"
    }

    /// Optional icon identifier.
    fn icon(&self) -> Option<&str> {
        None
    }

    /// Primary render dispatch.
    fn render(
        &mut self,
        cmd_buf: &mut CommandBuffer,
        viewport: Rect,
        ptr: &PointerState,
        palette: &ThemePalette,
    );

    /// Handle mouse down interaction within this editor's allocated region.
    fn handle_mouse_down(
        &mut self,
        _pos: Vec2,
        _button: PointerButton,
        _viewport: Rect,
        _ptr: &PointerState,
    ) -> bool {
        false
    }

    /// Handle continuous mouse dragging and movement.
    fn handle_mouse_move(
        &mut self,
        _pos: Vec2,
        _delta: Vec2,
        _viewport: Rect,
        _ptr: &PointerState,
    ) -> bool {
        false
    }

    /// Handle mouse release.
    fn handle_mouse_up(&mut self, _pos: Vec2, _button: PointerButton, _viewport: Rect) -> bool {
        false
    }

    /// Handle mouse wheel or trackpad scroll gestures.
    fn handle_scroll(&mut self, _delta: Vec2, _pos: Vec2, _viewport: Rect) -> bool {
        false
    }

    /// Keyboard and text events for the focused editor instance.
    fn handle_key_down(&mut self, _key: Key, _modifiers: Modifiers) -> bool { false }
    fn handle_text_input(&mut self, _text: &str) -> bool { false }
    fn handle_ime(&mut self, _event: ImeEvent) -> bool { false }

    /// Optional serialization of view state (e.g. pan/zoom, active tab, camera position).
    fn save_state(&self) -> Option<String> {
        None
    }

    /// Optional restoration of view state.
    fn restore_state(&mut self, _state: &str) {}
}

pub type EditorFactory = Arc<dyn Fn() -> Box<dyn Editor> + Send + Sync>;

/// Metadata describing a registered editor in the registry.
#[derive(Clone)]
pub struct EditorDescriptor {
    pub id: String,
    pub title: String,
    pub category: String,
    pub factory: EditorFactory,
}

/// Dynamic registry allowing consuming applications and plugins to register
/// custom editors without modifying Kathamo internals.
#[derive(Default, Clone)]
pub struct EditorRegistry {
    descriptors: HashMap<String, EditorDescriptor>,
}

impl EditorRegistry {
    pub fn new() -> Self {
        let mut reg = Self {
            descriptors: HashMap::new(),
        };
        reg.register_builtins();
        reg
    }

    /// Register a custom editor factory.
    pub fn register_factory<F>(
        &mut self,
        id: impl Into<String>,
        title: impl Into<String>,
        category: impl Into<String>,
        factory: F,
    ) where
        F: Fn() -> Box<dyn Editor> + Send + Sync + 'static,
    {
        let id_str = id.into();
        self.descriptors.insert(
            id_str.clone(),
            EditorDescriptor {
                id: id_str,
                title: title.into(),
                category: category.into(),
                factory: Arc::new(factory),
            },
        );
    }

    /// Register an editor implementing Default.
    pub fn register<E: Editor + Default + 'static>(
        &mut self,
        id: impl Into<String>,
        title: impl Into<String>,
        category: impl Into<String>,
    ) {
        self.register_factory(id, title, category, || Box::new(E::default()));
    }

    /// Create an instance of an editor by registered ID.
    pub fn create(&self, id: &str) -> Option<Box<dyn Editor>> {
        self.descriptors.get(id).map(|d| (d.factory)())
    }

    /// Get descriptor metadata for an editor.
    pub fn get_descriptor(&self, id: &str) -> Option<&EditorDescriptor> {
        self.descriptors.get(id)
    }

    /// List all registered editors: `(id, title, category)`.
    pub fn list_registered(&self) -> Vec<(&str, &str, &str)> {
        let mut list: Vec<_> = self
            .descriptors
            .values()
            .map(|d| (d.id.as_str(), d.title.as_str(), d.category.as_str()))
            .collect();
        list.sort_by(|a, b| a.0.cmp(b.0));
        list
    }

    /// Register Kathamo's standard built-in creative editors.
    fn register_builtins(&mut self) {
        self.register_factory("canvas_2d", "2D Canvas", "Viewports", || {
            Box::new(CanvasEditor::new_2d())
        });
        self.register_factory("viewport_3d", "3D Viewport", "Viewports", || {
            Box::new(CanvasEditor::new_3d_mock())
        });
        self.register_factory("hierarchy_tree", "Outliner / Layers", "Structure", || {
            Box::new(HierarchyEditor::default())
        });
        self.register_factory("property_inspector", "Properties", "Inspectors", || {
            Box::new(InspectorEditor::default())
        });
        self.register_factory("node_graph", "Procedural Graph", "Node Logic", || {
            Box::new(NodeGraphEditor::default())
        });
        self.register_factory("timeline", "Timeline / Tracks", "Animation", || {
            Box::new(TimelineEditor::default())
        });
        self.register_factory("asset_browser", "Asset Browser", "Data", || {
            Box::new(AssetBrowserEditor::default())
        });
        self.register_factory("console_log", "Log Console", "Diagnostics", || {
            Box::new(ConsoleEditor::default())
        });
    }
}

// -----------------------------------------------------------------------------
// STANDARD BUILT-IN EDITORS WRAPPING KATHAMO WORKSPACE COMPONENTS
// -----------------------------------------------------------------------------

/// Built-in Canvas Editor (2D brush surface or 3D GPU texture blit host).
pub struct CanvasEditor {
    pub host: CanvasHost,
    pub title: String,
}

impl CanvasEditor {
    pub fn new_2d() -> Self {
        Self {
            host: CanvasHost::new_2d(),
            title: "2D Canvas".into(),
        }
    }

    pub fn new_3d_mock() -> Self {
        Self {
            host: CanvasHost::new_texture_host(2001, (1280, 720)),
            title: "3D Viewport".into(),
        }
    }
}

impl Editor for CanvasEditor {
    fn id(&self) -> &str {
        "canvas"
    }

    fn title(&self) -> &str {
        &self.title
    }

    fn category(&self) -> &str {
        "Viewports"
    }

    fn render(
        &mut self,
        cmd_buf: &mut CommandBuffer,
        viewport: Rect,
        _ptr: &PointerState,
        palette: &ThemePalette,
    ) {
        self.host.render(cmd_buf, viewport, palette);
    }

    fn handle_mouse_down(&mut self, pos: Vec2, button: PointerButton, viewport: Rect, _ptr: &PointerState) -> bool {
        self.host.handle_mouse_down(pos, button, viewport);
        true
    }

    fn handle_mouse_move(
        &mut self,
        pos: Vec2,
        delta: Vec2,
        viewport: Rect,
        ptr: &PointerState,
    ) -> bool {
        self.host.handle_mouse_move(pos, delta, viewport, ptr);
        true
    }

    fn handle_mouse_up(&mut self, pos: Vec2, button: PointerButton, _viewport: Rect) -> bool {
        self.host.handle_mouse_up(pos, button);
        true
    }

    fn handle_scroll(&mut self, delta: Vec2, pos: Vec2, viewport: Rect) -> bool {
        self.host.handle_scroll(delta, pos, viewport);
        true
    }
}

/// Built-in Hierarchy / Outliner Editor.
#[derive(Default)]
pub struct HierarchyEditor {
    pub tree: HierarchyTree,
}

impl Editor for HierarchyEditor {
    fn id(&self) -> &str {
        "hierarchy_tree"
    }

    fn title(&self) -> &str {
        "Outliner"
    }

    fn category(&self) -> &str {
        "Structure"
    }

    fn render(
        &mut self,
        cmd_buf: &mut CommandBuffer,
        viewport: Rect,
        _ptr: &PointerState,
        palette: &ThemePalette,
    ) {
        self.tree.render(cmd_buf, viewport, palette);
    }

    fn handle_mouse_down(&mut self, pos: Vec2, button: PointerButton, viewport: Rect, ptr: &PointerState) -> bool {
        self.tree.handle_mouse_down(pos, button, viewport, ptr).is_some()
    }
}

/// Built-in Property Inspector Editor.
pub struct InspectorEditor {
    pub inspector: MetadataPropertyInspector,
}

impl Default for InspectorEditor {
    fn default() -> Self {
        Self {
            inspector: MetadataPropertyInspector::new("Object Properties", Vec::new()),
        }
    }
}

impl Editor for InspectorEditor {
    fn id(&self) -> &str {
        "property_inspector"
    }

    fn title(&self) -> &str {
        "Properties"
    }

    fn category(&self) -> &str {
        "Inspectors"
    }

    fn render(
        &mut self,
        cmd_buf: &mut CommandBuffer,
        viewport: Rect,
        _ptr: &PointerState,
        palette: &ThemePalette,
    ) {
        self.inspector.render(cmd_buf, viewport, palette);
    }

    fn handle_mouse_down(&mut self, pos: Vec2, button: PointerButton, viewport: Rect, _ptr: &PointerState) -> bool {
        self.inspector.handle_mouse_down(pos, button, viewport).is_some()
    }

    fn handle_mouse_move(
        &mut self,
        pos: Vec2,
        delta: Vec2,
        _viewport: Rect,
        ptr: &PointerState,
    ) -> bool {
        self.inspector.handle_mouse_move(pos, delta, ptr).is_some()
    }

    fn handle_mouse_up(&mut self, pos: Vec2, button: PointerButton, _viewport: Rect) -> bool {
        self.inspector.handle_mouse_up(pos, button);
        true
    }
}

/// Built-in Procedural Nodal Graph Editor.
#[derive(Default)]
pub struct NodeGraphEditor {
    pub editor: NodeEditor,
}

impl Editor for NodeGraphEditor {
    fn id(&self) -> &str {
        "node_graph"
    }

    fn title(&self) -> &str {
        "Node Graph"
    }

    fn category(&self) -> &str {
        "Node Logic"
    }

    fn render(
        &mut self,
        cmd_buf: &mut CommandBuffer,
        viewport: Rect,
        _ptr: &PointerState,
        palette: &ThemePalette,
    ) {
        self.editor.render(cmd_buf, viewport, palette);
    }

    fn handle_mouse_down(&mut self, pos: Vec2, button: PointerButton, viewport: Rect, _ptr: &PointerState) -> bool {
        self.editor.handle_mouse_down(pos, button, viewport);
        true
    }

    fn handle_mouse_move(
        &mut self,
        pos: Vec2,
        delta: Vec2,
        viewport: Rect,
        _ptr: &PointerState,
    ) -> bool {
        self.editor.handle_mouse_move(pos, delta, viewport);
        true
    }

    fn handle_mouse_up(&mut self, pos: Vec2, button: PointerButton, viewport: Rect) -> bool {
        self.editor.handle_mouse_up(pos, button, viewport);
        true
    }

    fn handle_scroll(&mut self, delta: Vec2, pos: Vec2, viewport: Rect) -> bool {
        self.editor.handle_scroll(delta, pos, viewport);
        true
    }

    fn handle_key_down(&mut self, key: Key, _modifiers: Modifiers) -> bool {
        self.editor.handle_key_down(key)
    }
}

/// Built-in Timeline Editor.
#[derive(Default)]
pub struct TimelineEditor {
    pub timeline: Timeline,
}

impl Editor for TimelineEditor {
    fn id(&self) -> &str {
        "timeline"
    }

    fn title(&self) -> &str {
        "Timeline"
    }

    fn category(&self) -> &str {
        "Animation"
    }

    fn render(
        &mut self,
        cmd_buf: &mut CommandBuffer,
        viewport: Rect,
        _ptr: &PointerState,
        palette: &ThemePalette,
    ) {
        self.timeline.render(cmd_buf, viewport, palette);
    }
}

/// Built-in Asset Browser Editor.
#[derive(Default)]
pub struct AssetBrowserEditor {
    pub browser: AssetBrowser,
}

impl Editor for AssetBrowserEditor {
    fn id(&self) -> &str {
        "asset_browser"
    }

    fn title(&self) -> &str {
        "Asset Browser"
    }

    fn category(&self) -> &str {
        "Data"
    }

    fn render(
        &mut self,
        cmd_buf: &mut CommandBuffer,
        viewport: Rect,
        ptr: &PointerState,
        palette: &ThemePalette,
    ) {
        self.browser.render(cmd_buf, viewport, ptr, palette);
    }

    fn handle_mouse_down(&mut self, pos: Vec2, button: PointerButton, viewport: Rect, _ptr: &PointerState) -> bool {
        self.browser.handle_mouse_down(pos, button, viewport).is_some()
    }
}

/// Built-in Console Log Editor.
#[derive(Default)]
pub struct ConsoleEditor {
    pub console: ConsoleView,
}

impl Editor for ConsoleEditor {
    fn id(&self) -> &str {
        "console_log"
    }

    fn title(&self) -> &str {
        "Log Console"
    }

    fn category(&self) -> &str {
        "Diagnostics"
    }

    fn render(
        &mut self,
        cmd_buf: &mut CommandBuffer,
        viewport: Rect,
        _ptr: &PointerState,
        palette: &ThemePalette,
    ) {
        self.console.render(cmd_buf, viewport, palette);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_editor_registry_builtins() {
        let registry = EditorRegistry::new();
        assert!(registry.create("canvas_2d").is_some());
        assert!(registry.create("viewport_3d").is_some());
        assert!(registry.create("hierarchy_tree").is_some());
        assert!(registry.create("property_inspector").is_some());
        assert!(registry.create("node_graph").is_some());
        assert!(registry.create("timeline").is_some());
        assert!(registry.create("asset_browser").is_some());
        assert!(registry.create("console_log").is_some());

        let list = registry.list_registered();
        assert_eq!(list.len(), 8);
    }

    #[test]
    fn test_custom_editor_registration() {
        struct CustomEditor;
        impl Editor for CustomEditor {
            fn id(&self) -> &str {
                "custom_view"
            }
            fn title(&self) -> &str {
                "Custom View"
            }
            fn render(
                &mut self,
                _cmd: &mut CommandBuffer,
                _vp: Rect,
                _ptr: &PointerState,
                _pal: &ThemePalette,
            ) {
            }
        }

        let mut registry = EditorRegistry::new();
        registry.register_factory("custom_view", "Custom View", "MyCategory", || {
            Box::new(CustomEditor)
        });

        let editor = registry.create("custom_view");
        assert!(editor.is_some());
        assert_eq!(editor.unwrap().title(), "Custom View");
    }
}
