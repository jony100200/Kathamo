//! Reusable WorkspaceHost coordinating Create / Split / Graph presentation modes
//! on a shared creative document without state resets or data duplication.

use crate::canvas_host::CanvasHost;
use crate::dock_view::DockTreeView;
use crate::hierarchy_tree::{HierarchyEvent, HierarchyItem, HierarchyTree};
use crate::node_editor::NodeEditor;
use crate::property_inspector::{MetadataPropertyInspector, PropertyItem, PropertyValue};
use crate::theme::ThemePalette;
use ui_core::{Color, DockTabPanel, DockTree, Rect, SplitAxis, Vec2};
use ui_platform::{PointerButton, PointerState};
use ui_render::CommandBuffer;

/// Presentation modes of the workstation workspace.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PresentationMode {
    #[default]
    Create, // Canvas viewport with layers and inspector
    Split,  // Canvas and Node graph side-by-side
    Graph,  // Full procedural node graph
}

/// A shared, unified creative document data model.
#[derive(Debug, Clone, PartialEq)]
pub struct CreativeDocument {
    pub title: String,
    pub canvas: CanvasHost,
    pub hierarchy: HierarchyTree,
    pub inspector: MetadataPropertyInspector,
    pub graph: NodeEditor,
    pub mode: PresentationMode,
    pub dock_tree: DockTree,
    pub dock_view: DockTreeView,
}

impl Default for CreativeDocument {
    fn default() -> Self {
        // Initial layers
        let items = vec![
            HierarchyItem::new("layer_fg", "Hero Character").with_depth(0),
            HierarchyItem::new("layer_fx", "Glow & Particles").with_depth(0),
            HierarchyItem::new("layer_bg", "Cyber City Skyline").with_depth(0),
        ];
        let hierarchy = HierarchyTree::new(items);

        // Initial inspector properties for the active layer
        let props = vec![
            PropertyItem::float("opacity", "Opacity", 1.0, 0.0, 1.0, 0.02).with_category("Appearance"),
            PropertyItem::enum_choice("blend", "Blend Mode", 0, vec!["Normal".into(), "Multiply".into(), "Screen".into(), "Overlay".into()]).with_category("Appearance"),
            PropertyItem::vec2("offset", "Offset", 0.0, 0.0, -1000.0, 1000.0).with_category("Transform"),
            PropertyItem::color("tint", "Color Tint", Color::rgb(255, 255, 255)).with_category("Appearance"),
            PropertyItem::boolean("visible", "Visible", true).with_category("General"),
        ];
        let inspector = MetadataPropertyInspector::new("Hero Character Properties", props);

        // Initialize docking hierarchy
        let mut dock = DockTree::new("main_view", DockTabPanel::new("canvas", "Viewport Canvas"));
        dock.split_leaf(
            "main_view",
            "split_left",
            "left_panel",
            SplitAxis::Horizontal,
            0.22,
            DockTabPanel::new("hierarchy", "Layers"),
        );
        dock.split_leaf(
            "main_view",
            "split_right",
            "right_panel",
            SplitAxis::Horizontal,
            0.78,
            DockTabPanel::new("inspector", "Inspector"),
        );

        Self {
            title: "Untitled Project".into(),
            canvas: CanvasHost::new_2d(),
            hierarchy,
            inspector,
            graph: NodeEditor::default(),
            mode: PresentationMode::Create,
            dock_tree: dock,
            dock_view: DockTreeView::new(),
        }
    }
}

/// Workspace host coordinator.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct WorkspaceHost {
    pub document: CreativeDocument,
}

impl WorkspaceHost {
    pub fn new() -> Self {
        Self::default()
    }

    /// Switches the presentation mode smoothly without resetting state.
    pub fn set_mode(&mut self, mode: PresentationMode) {
        self.document.mode = mode;
    }

    /// Handles pointer down across the active presentation layout.
    pub fn handle_mouse_down(&mut self, pos: Vec2, button: PointerButton, viewport: Rect, ptr: &PointerState) {
        match self.document.mode {
            PresentationMode::Create => {
                let layout = self.document.dock_tree.compute_layout(viewport);
                if self.document.dock_view.handle_mouse_down(pos, button, &layout, &mut self.document.dock_tree) {
                    return;
                }

                for leaf in &layout.leaves {
                    if leaf.content_rect.contains(pos) {
                        let active_tab_id = leaf.tabs.get(leaf.active_tab).map_or("", |t| t.id.as_str());
                        match active_tab_id {
                            "canvas" => self.document.canvas.handle_mouse_down(pos, button, leaf.content_rect),
                            "hierarchy" => {
                                if let Some(event) = self.document.hierarchy.handle_mouse_down(pos, button, leaf.content_rect, ptr) {
                                    self.sync_hierarchy_event(event);
                                }
                            }
                            "inspector" => {
                                if let Some(change) = self.document.inspector.handle_mouse_down(pos, button, leaf.content_rect) {
                                    self.sync_property_change(change);
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
            PresentationMode::Split => {
                let half_w = viewport.width() * 0.5;
                let canvas_rect = Rect::from_min_size(viewport.min, Vec2::new(half_w, viewport.height()));
                let graph_rect = Rect::from_min_size(Vec2::new(viewport.min.x + half_w, viewport.min.y), Vec2::new(half_w, viewport.height()));

                if canvas_rect.contains(pos) {
                    self.document.canvas.handle_mouse_down(pos, button, canvas_rect);
                } else if graph_rect.contains(pos) {
                    self.document.graph.handle_mouse_down(pos, button, graph_rect);
                }
            }
            PresentationMode::Graph => {
                self.document.graph.handle_mouse_down(pos, button, viewport);
            }
        }
    }

    /// Handles pointer movement across the active presentation layout.
    pub fn handle_mouse_move(&mut self, pos: Vec2, delta: Vec2, viewport: Rect, ptr: &PointerState) {
        match self.document.mode {
            PresentationMode::Create => {
                let layout = self.document.dock_tree.compute_layout(viewport);
                self.document.dock_view.handle_mouse_move(pos, delta, viewport, &mut self.document.dock_tree, ptr);

                for leaf in &layout.leaves {
                    let active_tab_id = leaf.tabs.get(leaf.active_tab).map_or("", |t| t.id.as_str());
                    match active_tab_id {
                        "canvas" => self.document.canvas.handle_mouse_move(pos, delta, leaf.content_rect, ptr),
                        "inspector" => {
                            if let Some(change) = self.document.inspector.handle_mouse_move(pos, delta, ptr) {
                                self.sync_property_change(change);
                            }
                        }
                        _ => {}
                    }
                }
            }
            PresentationMode::Split => {
                let half_w = viewport.width() * 0.5;
                let canvas_rect = Rect::from_min_size(viewport.min, Vec2::new(half_w, viewport.height()));
                let graph_rect = Rect::from_min_size(Vec2::new(viewport.min.x + half_w, viewport.min.y), Vec2::new(half_w, viewport.height()));

                if canvas_rect.contains(pos) || ptr.primary_down {
                    self.document.canvas.handle_mouse_move(pos, delta, canvas_rect, ptr);
                }
                if graph_rect.contains(pos) || ptr.primary_down {
                    self.document.graph.handle_mouse_move(pos, delta, graph_rect);
                }
            }
            PresentationMode::Graph => {
                self.document.graph.handle_mouse_move(pos, delta, viewport);
            }
        }
    }

    /// Handles pointer release.
    pub fn handle_mouse_up(&mut self, pos: Vec2, button: PointerButton, viewport: Rect) {
        match self.document.mode {
            PresentationMode::Create => {
                let layout = self.document.dock_tree.compute_layout(viewport);
                self.document.dock_view.handle_mouse_up(pos, button, &layout, &mut self.document.dock_tree);

                for leaf in &layout.leaves {
                    let active_tab_id = leaf.tabs.get(leaf.active_tab).map_or("", |t| t.id.as_str());
                    match active_tab_id {
                        "canvas" => self.document.canvas.handle_mouse_up(pos, button),
                        "inspector" => self.document.inspector.handle_mouse_up(pos, button),
                        _ => {}
                    }
                }
            }
            PresentationMode::Split => {
                let half_w = viewport.width() * 0.5;
                let graph_rect = Rect::from_min_size(Vec2::new(viewport.min.x + half_w, viewport.min.y), Vec2::new(half_w, viewport.height()));
                self.document.canvas.handle_mouse_up(pos, button);
                self.document.graph.handle_mouse_up(pos, button, graph_rect);
            }
            PresentationMode::Graph => {
                self.document.graph.handle_mouse_up(pos, button, viewport);
            }
        }
    }

    /// Handles scroll wheel zooming.
    pub fn handle_scroll(&mut self, delta: Vec2, pos: Vec2, viewport: Rect) {
        match self.document.mode {
            PresentationMode::Create => {
                let layout = self.document.dock_tree.compute_layout(viewport);
                for leaf in &layout.leaves {
                    if leaf.content_rect.contains(pos) {
                        let active_tab_id = leaf.tabs.get(leaf.active_tab).map_or("", |t| t.id.as_str());
                        if active_tab_id == "canvas" {
                            self.document.canvas.handle_scroll(delta, pos, leaf.content_rect);
                        }
                    }
                }
            }
            PresentationMode::Split => {
                let half_w = viewport.width() * 0.5;
                let canvas_rect = Rect::from_min_size(viewport.min, Vec2::new(half_w, viewport.height()));
                let graph_rect = Rect::from_min_size(Vec2::new(viewport.min.x + half_w, viewport.min.y), Vec2::new(half_w, viewport.height()));

                if canvas_rect.contains(pos) {
                    self.document.canvas.handle_scroll(delta, pos, canvas_rect);
                } else if graph_rect.contains(pos) {
                    self.document.graph.handle_scroll(delta, pos, graph_rect);
                }
            }
            PresentationMode::Graph => {
                self.document.graph.handle_scroll(delta, pos, viewport);
            }
        }
    }

    /// Synchronizes hierarchy tree actions to inspector and canvas.
    fn sync_hierarchy_event(&mut self, event: HierarchyEvent) {
        match event {
            HierarchyEvent::Selected(_id) => {
                if let Some(item) = self.document.hierarchy.selected_item() {
                    let props = vec![
                        PropertyItem::float("opacity", "Opacity", item.opacity, 0.0, 1.0, 0.02).with_category("Appearance"),
                        PropertyItem::boolean("visible", "Visible", item.visible).with_category("General"),
                        PropertyItem::boolean("locked", "Locked", item.locked).with_category("General"),
                    ];
                    self.document.inspector.set_properties(format!("{} Properties", item.name), props);
                }
            }
            _ => {}
        }
    }

    /// Synchronizes property changes back to active hierarchy items or tools.
    fn sync_property_change(&mut self, change: crate::property_inspector::PropertyChangedEvent) {
        if let Some(item) = self.document.hierarchy.selected_item_mut() {
            match change.id.as_str() {
                "opacity" => {
                    if let PropertyValue::Float { value, .. } = change.new_value {
                        item.opacity = value;
                    }
                }
                "visible" => {
                    if let PropertyValue::Bool(b) = change.new_value {
                        item.visible = b;
                    }
                }
                "locked" => {
                    if let PropertyValue::Bool(b) = change.new_value {
                        item.locked = b;
                    }
                }
                _ => {}
            }
        }
    }

    /// Renders the current presentation mode into the command buffer.
    pub fn render(&self, buf: &mut CommandBuffer, viewport: Rect, ptr: &PointerState, palette: &ThemePalette) {
        match self.document.mode {
            PresentationMode::Create => {
                let layout = self.document.dock_tree.compute_layout(viewport);

                // Render leaf contents
                for leaf in &layout.leaves {
                    let active_tab_id = leaf.tabs.get(leaf.active_tab).map_or("", |t| t.id.as_str());
                    match active_tab_id {
                        "canvas" => self.document.canvas.render(buf, leaf.content_rect, palette),
                        "hierarchy" => self.document.hierarchy.render(buf, leaf.content_rect, palette),
                        "inspector" => self.document.inspector.render(buf, leaf.content_rect, palette),
                        _ => {}
                    }
                }

                // Render dock chrome (headers, tabs, splitters, drop guides)
                self.document.dock_view.render_chrome(buf, &layout, &self.document.dock_tree, ptr, palette);
            }
            PresentationMode::Split => {
                let half_w = viewport.width() * 0.5;
                let canvas_rect = Rect::from_min_size(viewport.min, Vec2::new(half_w, viewport.height()));
                let graph_rect = Rect::from_min_size(Vec2::new(viewport.min.x + half_w, viewport.min.y), Vec2::new(half_w, viewport.height()));

                self.document.canvas.render(buf, canvas_rect, palette);
                self.document.graph.render(buf, graph_rect, palette);

                // Splitter line
                buf.push(ui_render::DrawCommand::Line {
                    start: Vec2::new(viewport.min.x + half_w, viewport.min.y),
                    end: Vec2::new(viewport.min.x + half_w, viewport.max.y),
                    stroke: ui_core::Stroke::new(2.0, palette.accent),
                });
            }
            PresentationMode::Graph => {
                self.document.graph.render(buf, viewport, palette);
            }
        }
    }
}
