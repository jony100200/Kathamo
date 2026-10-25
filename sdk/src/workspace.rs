//! Universal Workspace System and Multi-Layout WorkspaceHost for Kathamo SDK.

use std::collections::HashMap;

use ui_core::*;
use ui_platform::{PointerButton, PointerState};
use ui_render::CommandBuffer;
use ui_themes::ThemePalette;
use ui_widgets::DockTreeView;

use crate::editors::{Editor, EditorRegistry};

/// Standard workspace presets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspacePreset {
    Creative2D,
    Spatial3D,
    ProceduralGraph,
    VideoEditing,
    GeneralDesktop,
    QuadSplit,
}

/// A named workspace configuration holding an active dock tree.
pub struct Workspace {
    pub id: String,
    pub name: String,
    pub dock_tree: DockTree,
    pub editors: HashMap<String, Box<dyn Editor>>,
}

impl Workspace {
    pub fn new(id: impl Into<String>, name: impl Into<String>, dock_tree: DockTree) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            dock_tree,
            editors: HashMap::new(),
        }
    }
}

fn collect_node_panels(node: &DockNode, out: &mut Vec<String>) {
    match node {
        DockNode::Leaf { tabs, .. } => {
            for tab in tabs {
                out.push(tab.id.clone());
            }
        }
        DockNode::Split { first, second, .. } => {
            collect_node_panels(first, out);
            collect_node_panels(second, out);
        }
    }
}

/// The central workspace host orchestrating multi-workspace layouts,
/// dock tree splitting, tab docking, and live editor dispatch.
pub struct WorkspaceHost {
    pub registry: EditorRegistry,
    pub workspaces: Vec<Workspace>,
    pub active_workspace_idx: usize,
    pub dock_view: DockTreeView,
}

impl Default for WorkspaceHost {
    fn default() -> Self {
        Self::new()
    }
}

impl WorkspaceHost {
    pub fn new() -> Self {
        Self::with_registry(EditorRegistry::new())
    }

    pub fn with_registry(registry: EditorRegistry) -> Self {
        let mut host = Self {
            registry,
            workspaces: Vec::new(),
            active_workspace_idx: 0,
            dock_view: DockTreeView::new(),
        };

        // Initialize standard preset workspaces
        host.add_preset_workspace("ws_2d", "2D Canvas", WorkspacePreset::Creative2D);
        host.add_preset_workspace("ws_3d", "3D Viewport", WorkspacePreset::Spatial3D);
        host.add_preset_workspace("ws_graph", "Procedural Graph", WorkspacePreset::ProceduralGraph);
        host.add_preset_workspace("ws_video", "Video Editor", WorkspacePreset::VideoEditing);
        host.add_preset_workspace("ws_desktop", "Application", WorkspacePreset::GeneralDesktop);

        host
    }

    /// Add a preset workspace layout.
    pub fn add_preset_workspace(&mut self, id: &str, name: &str, preset: WorkspacePreset) {
        let dock_tree = match preset {
            WorkspacePreset::Creative2D => Self::create_2d_preset_tree(),
            WorkspacePreset::Spatial3D => Self::create_3d_preset_tree(),
            WorkspacePreset::ProceduralGraph => Self::create_graph_preset_tree(),
            WorkspacePreset::VideoEditing => Self::create_video_preset_tree(),
            WorkspacePreset::GeneralDesktop => Self::create_desktop_preset_tree(),
            WorkspacePreset::QuadSplit => Self::create_quad_preset_tree(),
        };

        let mut ws = Workspace::new(id, name, dock_tree);
        // Pre-instantiate editors for the leaf panels in this tree
        self.populate_workspace_editors(&mut ws);
        self.workspaces.push(ws);
    }

    /// Switch active workspace by index.
    pub fn set_active_workspace(&mut self, idx: usize) {
        if idx < self.workspaces.len() {
            self.active_workspace_idx = idx;
        }
    }

    /// Alias for set_active_workspace.
    pub fn switch_workspace(&mut self, idx: usize) {
        self.set_active_workspace(idx);
    }

    /// Get current active workspace.
    pub fn active_workspace(&self) -> &Workspace {
        &self.workspaces[self.active_workspace_idx]
    }

    /// Get current active workspace mutably.
    pub fn active_workspace_mut(&mut self) -> &mut Workspace {
        &mut self.workspaces[self.active_workspace_idx]
    }

    /// Pre-populate editors for leaf panels in a workspace dock tree.
    fn populate_workspace_editors(&self, ws: &mut Workspace) {
        let mut panel_ids = Vec::new();
        collect_node_panels(&ws.dock_tree.root, &mut panel_ids);
        for id in panel_ids {
            if let std::collections::hash_map::Entry::Vacant(e) = ws.editors.entry(id.clone()) {
                if let Some(ed) = self.registry.create(&id) {
                    e.insert(ed);
                }
            }
        }
    }

    /// Ensure all active leaf tabs have an instantiated editor.
    pub fn ensure_editors_instantiated(&mut self) {
        let ws = &mut self.workspaces[self.active_workspace_idx];
        let mut panel_ids = Vec::new();
        collect_node_panels(&ws.dock_tree.root, &mut panel_ids);
        for id in panel_ids {
            if let std::collections::hash_map::Entry::Vacant(e) = ws.editors.entry(id.clone()) {
                if let Some(ed) = self.registry.create(&id) {
                    e.insert(ed);
                }
            }
        }
    }

    // -------------------------------------------------------------------------
    // INPUT DELEGATION
    // -------------------------------------------------------------------------

    pub fn handle_mouse_down(
        &mut self,
        pos: Vec2,
        button: PointerButton,
        bounds: Rect,
        ptr: &PointerState,
    ) -> bool {
        self.ensure_editors_instantiated();
        let ws = &mut self.workspaces[self.active_workspace_idx];
        let layout = ws.dock_tree.compute_layout(bounds);

        // 1. Check if hit dock splitter or tab header
        if self.dock_view.handle_mouse_down(pos, button, &layout, &mut ws.dock_tree) {
            return true;
        }

        // 2. Delegate to active editor in containing leaf
        for leaf in &layout.leaves {
            if leaf.content_rect.contains(pos) {
                if let Some(panel) = leaf.tabs.get(leaf.active_tab) {
                    if let Some(editor) = ws.editors.get_mut(&panel.id) {
                        return editor.handle_mouse_down(pos, button, leaf.content_rect, ptr);
                    }
                }
            }
        }

        false
    }

    pub fn handle_mouse_move(
        &mut self,
        pos: Vec2,
        delta: Vec2,
        bounds: Rect,
        ptr: &PointerState,
    ) -> bool {
        let ws = &mut self.workspaces[self.active_workspace_idx];

        // 1. If actively dragging a dock splitter, delegate to dock view
        if self.dock_view.active_splitter_drag.is_some() || self.dock_view.dragging_tab.is_some() {
            self.dock_view.handle_mouse_move(pos, delta, bounds, &mut ws.dock_tree, ptr);
            return true;
        }

        // 2. Otherwise delegate to editor under pointer
        let layout = ws.dock_tree.compute_layout(bounds);
        for leaf in &layout.leaves {
            if leaf.content_rect.contains(pos) || ptr.has_drag_capture() {
                if let Some(panel) = leaf.tabs.get(leaf.active_tab) {
                    if let Some(editor) = ws.editors.get_mut(&panel.id) {
                        return editor.handle_mouse_move(pos, delta, leaf.content_rect, ptr);
                    }
                }
            }
        }

        false
    }

    pub fn handle_mouse_up(&mut self, pos: Vec2, button: PointerButton, bounds: Rect) -> bool {
        let ws = &mut self.workspaces[self.active_workspace_idx];
        let layout = ws.dock_tree.compute_layout(bounds);

        if self.dock_view.handle_mouse_up(pos, button, &layout, &mut ws.dock_tree) {
            return true;
        }

        // Delegate mouse up to all active editors
        for leaf in &layout.leaves {
            if let Some(panel) = leaf.tabs.get(leaf.active_tab) {
                if let Some(editor) = ws.editors.get_mut(&panel.id) {
                    editor.handle_mouse_up(pos, button, leaf.content_rect);
                }
            }
        }

        false
    }

    pub fn handle_scroll(&mut self, delta: Vec2, pos: Vec2, bounds: Rect) -> bool {
        let ws = &mut self.workspaces[self.active_workspace_idx];
        let layout = ws.dock_tree.compute_layout(bounds);

        for leaf in &layout.leaves {
            if leaf.content_rect.contains(pos) {
                if let Some(panel) = leaf.tabs.get(leaf.active_tab) {
                    if let Some(editor) = ws.editors.get_mut(&panel.id) {
                        return editor.handle_scroll(delta, pos, leaf.content_rect);
                    }
                }
            }
        }

        false
    }

    // -------------------------------------------------------------------------
    // RENDERING
    // -------------------------------------------------------------------------

    pub fn render(
        &mut self,
        cmd_buf: &mut CommandBuffer,
        bounds: Rect,
        ptr: &PointerState,
        palette: &ThemePalette,
    ) {
        self.ensure_editors_instantiated();
        let ws = &mut self.workspaces[self.active_workspace_idx];
        let layout = ws.dock_tree.compute_layout(bounds);

        // 1. Render all leaf editors into their allocated rectangles
        for leaf in &layout.leaves {
            if let Some(panel) = leaf.tabs.get(leaf.active_tab) {
                if let Some(editor) = ws.editors.get_mut(&panel.id) {
                    editor.render(cmd_buf, leaf.content_rect, ptr, palette);
                }
            }
        }

        // 2. Render docking chrome, tab bars, maximize buttons, and splitters on top
        self.dock_view.render(cmd_buf, &layout, &ws.dock_tree, palette, ptr);
    }

    // -------------------------------------------------------------------------
    // PRESET DOCK TREES
    // -------------------------------------------------------------------------

    fn create_2d_preset_tree() -> DockTree {
        DockTree::with_root(DockNode::Split {
            id: "root_split".into(),
            axis: SplitAxis::Horizontal,
            ratio: 0.20,
            first: Box::new(DockNode::Leaf {
                id: "left_outliner".into(),
                tabs: vec![DockTabPanel::new("hierarchy_tree", "Layers / Outliner")],
                active_tab: 0,
            }),
            second: Box::new(DockNode::Split {
                id: "center_right_split".into(),
                axis: SplitAxis::Horizontal,
                ratio: 0.75,
                first: Box::new(DockNode::Leaf {
                    id: "main_canvas".into(),
                    tabs: vec![DockTabPanel::new("canvas_2d", "2D Canvas")],
                    active_tab: 0,
                }),
                second: Box::new(DockNode::Leaf {
                    id: "right_inspector".into(),
                    tabs: vec![DockTabPanel::new("property_inspector", "Properties")],
                    active_tab: 0,
                }),
            }),
        })
    }

    fn create_3d_preset_tree() -> DockTree {
        DockTree::with_root(DockNode::Split {
            id: "root_split".into(),
            axis: SplitAxis::Horizontal,
            ratio: 0.78,
            first: Box::new(DockNode::Leaf {
                id: "main_viewport".into(),
                tabs: vec![DockTabPanel::new("viewport_3d", "3D Viewport")],
                active_tab: 0,
            }),
            second: Box::new(DockNode::Split {
                id: "right_split".into(),
                axis: SplitAxis::Vertical,
                ratio: 0.50,
                first: Box::new(DockNode::Leaf {
                    id: "outliner_leaf".into(),
                    tabs: vec![DockTabPanel::new("hierarchy_tree", "Scene Hierarchy")],
                    active_tab: 0,
                }),
                second: Box::new(DockNode::Leaf {
                    id: "inspector_leaf".into(),
                    tabs: vec![DockTabPanel::new("property_inspector", "Material & Transform")],
                    active_tab: 0,
                }),
            }),
        })
    }

    fn create_graph_preset_tree() -> DockTree {
        DockTree::with_root(DockNode::Split {
            id: "root_split".into(),
            axis: SplitAxis::Horizontal,
            ratio: 0.72,
            first: Box::new(DockNode::Leaf {
                id: "graph_leaf".into(),
                tabs: vec![DockTabPanel::new("node_graph", "Procedural Node Graph")],
                active_tab: 0,
            }),
            second: Box::new(DockNode::Leaf {
                id: "properties_leaf".into(),
                tabs: vec![DockTabPanel::new("property_inspector", "Node Parameters")],
                active_tab: 0,
            }),
        })
    }

    fn create_video_preset_tree() -> DockTree {
        DockTree::with_root(DockNode::Split {
            id: "root_split".into(),
            axis: SplitAxis::Vertical,
            ratio: 0.60,
            first: Box::new(DockNode::Split {
                id: "top_split".into(),
                axis: SplitAxis::Horizontal,
                ratio: 0.65,
                first: Box::new(DockNode::Leaf {
                    id: "video_preview".into(),
                    tabs: vec![DockTabPanel::new("viewport_3d", "Video Preview")],
                    active_tab: 0,
                }),
                second: Box::new(DockNode::Leaf {
                    id: "properties".into(),
                    tabs: vec![DockTabPanel::new("property_inspector", "Clip Properties")],
                    active_tab: 0,
                }),
            }),
            second: Box::new(DockNode::Leaf {
                id: "timeline_leaf".into(),
                tabs: vec![DockTabPanel::new("timeline", "Sequencer Timeline")],
                active_tab: 0,
            }),
        })
    }

    fn create_desktop_preset_tree() -> DockTree {
        DockTree::with_root(DockNode::Split {
            id: "root_split".into(),
            axis: SplitAxis::Horizontal,
            ratio: 0.22,
            first: Box::new(DockNode::Leaf {
                id: "nav_leaf".into(),
                tabs: vec![DockTabPanel::new("hierarchy_tree", "Navigation")],
                active_tab: 0,
            }),
            second: Box::new(DockNode::Split {
                id: "center_split".into(),
                axis: SplitAxis::Vertical,
                ratio: 0.65,
                first: Box::new(DockNode::Leaf {
                    id: "assets_leaf".into(),
                    tabs: vec![DockTabPanel::new("asset_browser", "Asset Collection")],
                    active_tab: 0,
                }),
                second: Box::new(DockNode::Leaf {
                    id: "console_leaf".into(),
                    tabs: vec![DockTabPanel::new("console_log", "Activity & Logs")],
                    active_tab: 0,
                }),
            }),
        })
    }

    fn create_quad_preset_tree() -> DockTree {
        DockTree::with_root(DockNode::Split {
            id: "root_h".into(),
            axis: SplitAxis::Horizontal,
            ratio: 0.5,
            first: Box::new(DockNode::Split {
                id: "left_v".into(),
                axis: SplitAxis::Vertical,
                ratio: 0.5,
                first: Box::new(DockNode::Leaf {
                    id: "top_left".into(),
                    tabs: vec![DockTabPanel::new("canvas_2d", "Top View")],
                    active_tab: 0,
                }),
                second: Box::new(DockNode::Leaf {
                    id: "bot_left".into(),
                    tabs: vec![DockTabPanel::new("viewport_3d", "Front View")],
                    active_tab: 0,
                }),
            }),
            second: Box::new(DockNode::Split {
                id: "right_v".into(),
                axis: SplitAxis::Vertical,
                ratio: 0.5,
                first: Box::new(DockNode::Leaf {
                    id: "top_right".into(),
                    tabs: vec![DockTabPanel::new("node_graph", "Logic View")],
                    active_tab: 0,
                }),
                second: Box::new(DockNode::Leaf {
                    id: "bot_right".into(),
                    tabs: vec![DockTabPanel::new("property_inspector", "Inspector")],
                    active_tab: 0,
                }),
            }),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workspace_host_switching_and_rendering() {
        let mut host = WorkspaceHost::new();
        assert_eq!(host.workspaces.len(), 5);
        assert_eq!(host.active_workspace().name, "2D Canvas");

        // Switch to 3D Viewport workspace
        host.set_active_workspace(1);
        assert_eq!(host.active_workspace().name, "3D Viewport");

        let bounds = Rect::from_min_size(Vec2::ZERO, Vec2::new(1280.0, 720.0));
        let ptr = PointerState::default();
        let mut cmd_buf = CommandBuffer::new();
        let palette = ThemePalette::dark();

        host.render(&mut cmd_buf, bounds, &ptr, &palette);
        assert!(!cmd_buf.commands().is_empty());
    }

    #[test]
    fn test_workspace_mouse_delegation() {
        let mut host = WorkspaceHost::new();
        let bounds = Rect::from_min_size(Vec2::ZERO, Vec2::new(1280.0, 720.0));

        // Click in main canvas area (center of 1280x720)
        let ptr = PointerState::default();
        let handled = host.handle_mouse_down(Vec2::new(600.0, 360.0), PointerButton::Primary, bounds, &ptr);
        assert!(handled);
    }
}
