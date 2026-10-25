//! Universal Workspace System and Multi-Layout WorkspaceHost for Kathamo SDK.

use std::collections::HashMap;

use ui_core::*;
use ui_platform::{ImeEvent, Key, Modifiers, PointerButton, PointerState};
use ui_render::{CommandBuffer, DrawCommand};
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

fn collect_node_panels(node: &DockNode, out: &mut Vec<DockTabPanel>) {
    match node {
        DockNode::Leaf { tabs, .. } => {
            out.extend(tabs.iter().cloned());
        }
        DockNode::Split { first, second, .. } => {
            collect_node_panels(first, out);
            collect_node_panels(second, out);
        }
    }
}


fn encode_field(value: &str) -> String {
    value.as_bytes().iter().map(|b| format!("{b:02x}")).collect()
}

fn decode_field(value: &str) -> Result<String, String> {
    if value.len() % 2 != 0 { return Err("invalid encoded workspace field".into()); }
    let mut bytes = Vec::with_capacity(value.len() / 2);
    for chunk in value.as_bytes().chunks(2) {
        let hex = std::str::from_utf8(chunk).map_err(|e| e.to_string())?;
        bytes.push(u8::from_str_radix(hex, 16).map_err(|e| e.to_string())?);
    }
    String::from_utf8(bytes).map_err(|e| e.to_string())
}

fn encode_node(node: &DockNode, out: &mut String) {
    match node {
        DockNode::Leaf { id, tabs, active_tab } => {
            out.push_str(&format!("L\t{}\t{}\t{}\n", encode_field(id), active_tab, tabs.len()));
            for tab in tabs {
                out.push_str(&format!("T\t{}\t{}\t{}\t{}\n",
                    encode_field(&tab.id), encode_field(&tab.title),
                    if tab.is_closable { 1 } else { 0 },
                    tab.factory_id.as_ref().map_or("-".to_string(), |v| encode_field(v))));
            }
        }
        DockNode::Split { id, axis, ratio, first, second } => {
            out.push_str(&format!("S\t{}\t{}\t{}\n", encode_field(id),
                if *axis == SplitAxis::Horizontal { "H" } else { "V" }, ratio));
            encode_node(first, out);
            encode_node(second, out);
        }
    }
}

fn decode_node<'a>(lines: &mut impl Iterator<Item = &'a str>, depth: usize) -> Result<DockNode, String> {
    if depth > 64 { return Err("workspace tree exceeds maximum depth".into()); }
    let line = lines.next().ok_or("incomplete workspace tree")?;
    let parts: Vec<&str> = line.split('\t').collect();
    match parts.first().copied() {
        Some("L") if parts.len() == 4 => {
            let id = decode_field(parts[1])?;
            let active_tab = parts[2].parse::<usize>().map_err(|e| e.to_string())?;
            let count = parts[3].parse::<usize>().map_err(|e| e.to_string())?;
            if count > 10000 { return Err("workspace has too many tabs".into()); }
            let mut tabs = Vec::with_capacity(count);
            for _ in 0..count {
                let tab = lines.next().ok_or("missing dock tab")?;
                let fields: Vec<&str> = tab.split('\t').collect();
                if fields.len() != 5 || fields[0] != "T" { return Err("invalid dock tab".into()); }
                tabs.push(DockTabPanel {
                    id: decode_field(fields[1])?,
                    title: decode_field(fields[2])?,
                    is_closable: fields[3] == "1",
                    factory_id: if fields[4] == "-" { None } else { Some(decode_field(fields[4])?) },
                });
            }
            if active_tab >= tabs.len() && !tabs.is_empty() {
                return Err("workspace active tab out of range".into());
            }
            Ok(DockNode::Leaf { id, tabs, active_tab })
        }
        Some("S") if parts.len() == 4 => {
            let id = decode_field(parts[1])?;
            let axis = match parts[2] {
                "H" => SplitAxis::Horizontal,
                "V" => SplitAxis::Vertical,
                _ => return Err("invalid split axis".into()),
            };
            let ratio = parts[3].parse::<f32>().map_err(|e| e.to_string())?;
            if !ratio.is_finite() || !(0.05..=0.95).contains(&ratio) {
                return Err("invalid workspace split ratio".into());
            }
            let first = Box::new(decode_node(lines, depth + 1)?);
            let second = Box::new(decode_node(lines, depth + 1)?);
            Ok(DockNode::Split { id, axis, ratio, first, second })
        }
        _ => Err("invalid workspace node".into()),
    }
}

/// The central workspace host orchestrating multi-workspace layouts,
/// dock tree splitting, tab docking, and live editor dispatch.
pub struct WorkspaceHost {
    pub registry: EditorRegistry,
    pub workspaces: Vec<Workspace>,
    pub active_workspace_idx: usize,
    pub dock_view: DockTreeView,
    pub focused_panel_id: Option<String>,
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
            focused_panel_id: None,
        };

        // Initialize standard preset workspaces
        host.add_preset_workspace("ws_2d", "2D Canvas", WorkspacePreset::Creative2D);
        host.add_preset_workspace("ws_3d", "3D Viewport", WorkspacePreset::Spatial3D);
        host.add_preset_workspace("ws_graph", "Procedural Graph", WorkspacePreset::ProceduralGraph);
        host.add_preset_workspace("ws_video", "Video Editor", WorkspacePreset::VideoEditing);
        host.add_preset_workspace("ws_desktop", "Application", WorkspacePreset::GeneralDesktop);

        host
    }

    /// Create a single workspace for a consuming application.
    pub fn with_preset(
        registry: EditorRegistry,
        id: &str,
        name: &str,
        preset: WorkspacePreset,
    ) -> Self {
        let mut host = Self {
            registry,
            workspaces: Vec::new(),
            active_workspace_idx: 0,
            dock_view: DockTreeView::new(),
            focused_panel_id: None,
        };
        host.add_preset_workspace(id, name, preset);
        host
    }

    /// Host an application's custom dock tree with its own registered editors.
    pub fn from_workspace(registry: EditorRegistry, mut workspace: Workspace) -> Self {
        let mut host = Self {
            registry,
            workspaces: Vec::new(),
            active_workspace_idx: 0,
            dock_view: DockTreeView::new(),
            focused_panel_id: None,
        };
        host.populate_workspace_editors(&mut workspace);
        host.workspaces.push(workspace);
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
        for panel in panel_ids {
            let factory = panel.factory_id.as_deref().unwrap_or(&panel.id);
            if let std::collections::hash_map::Entry::Vacant(e) = ws.editors.entry(panel.id.clone()) {
                if let Some(ed) = self.registry.create(factory) {
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
        for panel in panel_ids {
            let factory = panel.factory_id.as_deref().unwrap_or(&panel.id);
            if let std::collections::hash_map::Entry::Vacant(e) = ws.editors.entry(panel.id.clone()) {
                if let Some(ed) = self.registry.create(factory) {
                    e.insert(ed);
                }
            }
        }
    }

    /// Export all workspace layouts and optional editor state in a versioned, UTF-8-safe format.
    /// Save this string in the consuming application's own settings location.
    pub fn save_layouts(&self) -> String {
        let mut out = String::from("KATHAMO_UI_1\n");
        let active = self.workspaces.get(self.active_workspace_idx)
            .map_or("", |ws| ws.id.as_str());
        out.push_str(&format!("ACTIVE\t{}\n", encode_field(active)));
        for ws in &self.workspaces {
            out.push_str(&format!("WORKSPACE\t{}\t{}\t{}\t{}\t{}\n",
                encode_field(&ws.id), encode_field(&ws.name),
                ws.dock_tree.splitter_thickness, ws.dock_tree.tab_header_height,
                ws.dock_tree.maximized_leaf.as_deref().map_or("-".to_string(), encode_field)));
            encode_node(&ws.dock_tree.root, &mut out);
            let mut states: Vec<_> = ws.editors.iter().filter_map(|(id, editor)|
                editor.save_state().map(|state| (id.as_str(), state))
            ).collect();
            states.sort_by_key(|(id, _)| *id);
            for (id, state) in states {
                out.push_str(&format!("EDITOR\t{}\t{}\n", encode_field(id), encode_field(&state)));
            }
            out.push_str("END\n");
        }
        out
    }

    /// Restore docking and editor state using the host's registered editor factories.
    /// The current UI remains unchanged if parsing fails.
    pub fn restore_layouts(&mut self, text: &str) -> Result<(), String> {
        let mut lines = text.lines();
        if lines.next() != Some("KATHAMO_UI_1") { return Err("unsupported Kathamo layout version".into()); }
        let active_line = lines.next().ok_or("missing active workspace")?;
        let active = active_line.strip_prefix("ACTIVE\t").ok_or("missing active ID")?;
        let active_id = decode_field(active)?;
        let mut workspaces = Vec::new();
        while let Some(line) = lines.next() {
            let fields: Vec<&str> = line.split('\t').collect();
            if fields.len() != 6 || fields[0] != "WORKSPACE" {
                return Err("invalid workspace header".into());
            }
            if workspaces.len() >= 128 { return Err("too many saved workspaces".into()); }
            let thickness = fields[3].parse::<f32>().map_err(|e| e.to_string())?;
            let header_height = fields[4].parse::<f32>().map_err(|e| e.to_string())?;
            if !thickness.is_finite() || thickness <= 0.0 ||
               !header_height.is_finite() || header_height <= 0.0 {
                return Err("invalid dock metrics".into());
            }
            let mut tree = DockTree::with_root(decode_node(&mut lines, 0)?);
            tree.splitter_thickness = thickness;
            tree.tab_header_height = header_height;
            tree.maximized_leaf = if fields[5] == "-" { None } else { Some(decode_field(fields[5])?) };
            let mut ws = Workspace::new(decode_field(fields[1])?, decode_field(fields[2])?, tree);
            self.populate_workspace_editors(&mut ws);
            loop {
                let state_line = lines.next().ok_or("missing workspace end marker")?;
                if state_line == "END" { break; }
                let parts: Vec<&str> = state_line.split('\t').collect();
                if parts.len() != 3 || parts[0] != "EDITOR" { return Err("invalid editor state".into()); }
                if let Some(editor) = ws.editors.get_mut(&decode_field(parts[1])?) {
                    editor.restore_state(&decode_field(parts[2])?);
                }
            }
            workspaces.push(ws);
        }
        if workspaces.is_empty() { return Err("workspace file is empty".into()); }
        let active_index = workspaces.iter().position(|ws| ws.id == active_id).unwrap_or(0);
        self.workspaces = workspaces;
        self.active_workspace_idx = active_index;
        self.dock_view = DockTreeView::new();
        self.focused_panel_id = None;
        Ok(())
    }

    pub fn save_layouts_to(&self, path: impl AsRef<std::path::Path>) -> std::io::Result<()> {
        std::fs::write(path, self.save_layouts())
    }

    pub fn load_layouts_from(&mut self, path: impl AsRef<std::path::Path>) -> Result<(), String> {
        let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        self.restore_layouts(&text)
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

        // Focus the active pane or the clicked tab before delegating input.
        if button == PointerButton::Primary {
            for leaf in &layout.leaves {
                if leaf.bounds.contains(pos) {
                    if leaf.header_rect.contains(pos) {
                        let tab_index = ((pos.x - leaf.header_rect.min.x - 8.0) / 114.0).floor() as isize;
                        if tab_index >= 0 {
                            if let Some(tab) = leaf.tabs.get(tab_index as usize) {
                                self.focused_panel_id = Some(tab.id.clone());
                            }
                        }
                    } else if let Some(panel) = leaf.tabs.get(leaf.active_tab) {
                        self.focused_panel_id = Some(panel.id.clone());
                    }
                    break;
                }
            }
        }

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

    /// Traverse pane focus in a stable order; Tab/Shift+Tab switches the active pane.
    pub fn handle_key_down(&mut self, key: Key, modifiers: Modifiers) -> bool {
        if key == Key::Escape && (self.dock_view.dragging_tab.is_some() || self.dock_view.active_splitter_drag.is_some()) {
            self.dock_view.dragging_tab = None;
            self.dock_view.active_splitter_drag = None;
            return true;
        }
        let ws = &mut self.workspaces[self.active_workspace_idx];
        if key == Key::Tab && !modifiers.alt && !modifiers.command {
            let mut ids = Vec::new();
            collect_node_panels(&ws.dock_tree.root, &mut ids);
            if ids.is_empty() { return false; }
            let index = self.focused_panel_id.as_ref()
                .and_then(|focus| ids.iter().position(|p| &p.id == focus));
            let next = match index {
                Some(i) if modifiers.shift => (i + ids.len() - 1) % ids.len(),
                Some(i) => (i + 1) % ids.len(),
                None if modifiers.shift => ids.len() - 1,
                None => 0,
            };
            let id = ids[next].id.clone();
            Self::activate_tab_for_panel(&mut ws.dock_tree.root, &id);
            self.focused_panel_id = Some(id);
            return true;
        }
        self.focused_panel_id.as_ref()
            .and_then(|id| ws.editors.get_mut(id))
            .is_some_and(|editor| editor.handle_key_down(key, modifiers))
    }

    fn activate_tab_for_panel(node: &mut DockNode, panel_id: &str) -> bool {
        match node {
            DockNode::Leaf { tabs, active_tab, .. } => {
                if let Some(index) = tabs.iter().position(|p| p.id == panel_id) {
                    *active_tab = index;
                    true
                } else { false }
            }
            DockNode::Split { first, second, .. } =>
                Self::activate_tab_for_panel(first, panel_id) ||
                Self::activate_tab_for_panel(second, panel_id),
        }
    }

    pub fn handle_text_input(&mut self, text: &str) -> bool {
        let ws = &mut self.workspaces[self.active_workspace_idx];
        self.focused_panel_id.as_ref().and_then(|id| ws.editors.get_mut(id))
            .is_some_and(|editor| editor.handle_text_input(text))
    }

    pub fn handle_ime(&mut self, event: ImeEvent) -> bool {
        let ws = &mut self.workspaces[self.active_workspace_idx];
        self.focused_panel_id.as_ref().and_then(|id| ws.editors.get_mut(id))
            .is_some_and(|editor| editor.handle_ime(event))
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
                    cmd_buf.push(DrawCommand::ClipPush {
                        rect: leaf.content_rect,
                    });
                    editor.render(cmd_buf, leaf.content_rect, ptr, palette);
                    cmd_buf.push(DrawCommand::ClipPop);
                }
            }
        }

        // The focused editor has a visible focus ring.
        if let Some(focused) = &self.focused_panel_id {
            if let Some(leaf) = layout.leaves.iter().find(|l|
                l.tabs.get(l.active_tab).is_some_and(|tab| &tab.id == focused)) {
                cmd_buf.push(DrawCommand::Rect {
                    rect: leaf.content_rect,
                    fill: Color::TRANSPARENT,
                    radius: CornerRadius::ZERO,
                    stroke: Stroke::new(1.0, palette.accent),
                });
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
    fn workspace_layout_state_roundtrips() {
        let mut host = WorkspaceHost::new();
        host.set_active_workspace(3);
        let saved = host.save_layouts();
        let mut restored = WorkspaceHost::new();
        restored.restore_layouts(&saved).unwrap();
        assert_eq!(restored.active_workspace_idx, 3);
        assert_eq!(restored.workspaces.len(), host.workspaces.len());
        assert_eq!(restored.active_workspace().dock_tree, host.active_workspace().dock_tree);
        assert!(restored.restore_layouts("invalid").is_err());
        assert_eq!(restored.active_workspace_idx, 3);
    }

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
        let clip_pushes = cmd_buf.commands().iter().filter(|cmd| matches!(cmd, DrawCommand::ClipPush { .. })).count();
        let clip_pops = cmd_buf.commands().iter().filter(|cmd| matches!(cmd, DrawCommand::ClipPop)).count();
        assert!(clip_pushes > 0, "editors must be clipped to their dock content bounds");
        assert_eq!(clip_pushes, clip_pops, "editor clip regions must be balanced");
    }

    #[test]
    fn multiple_independent_panes_can_share_one_editor_factory() {
        let mut tree = DockTree::new(
            "left",
            DockTabPanel::new("canvas_a", "Canvas A").with_factory("canvas_2d"),
        );
        assert!(tree.split_leaf(
            "left",
            "canvas_split",
            "right",
            SplitAxis::Horizontal,
            0.5,
            DockTabPanel::new("canvas_b", "Canvas B").with_factory("canvas_2d"),
        ));

        let host = WorkspaceHost::from_workspace(
            EditorRegistry::new(),
            Workspace::new("my_ui", "Two Viewports", tree),
        );
        assert_eq!(host.workspaces.len(), 1);
        assert_eq!(host.active_workspace().editors.len(), 2);
        assert!(host.active_workspace().editors.contains_key("canvas_a"));
        assert!(host.active_workspace().editors.contains_key("canvas_b"));
    }

    #[test]
    fn single_preset_does_not_initialize_unrelated_workspaces() {
        let host = WorkspaceHost::with_preset(
            EditorRegistry::new(), "game_editor", "Game Editor", WorkspacePreset::Spatial3D,
        );
        assert_eq!(host.workspaces.len(), 1);
        assert_eq!(host.active_workspace().name, "Game Editor");
        assert!(host.active_workspace().editors.contains_key("viewport_3d"));
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
