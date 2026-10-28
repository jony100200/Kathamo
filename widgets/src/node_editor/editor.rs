use super::geometry::{
    align_nodes, input_pin_world_pos, output_pin_world_pos, screen_to_world, toolbar_rects,
    world_to_screen,
};
use super::minimap::render_minimap;
use super::presets::create_default_graph;
use super::types::PresetNode;
use super::render::{
    render_connections, render_grid, render_hud, render_marquee, render_nodes,
    render_pending_wire, render_search_overlay, render_toolbar,
};
use super::search_menu::SearchMenuState;
use super::types::{
    DraggingNode, GraphNode, NodeAlignment, NodeConnection, NodeGroup, NodePin, PendingWire,
    ToolbarAction,
};
use std::collections::{HashMap, HashSet, VecDeque};
use ui_core::{CornerRadius, Rect, Stroke, Vec2};
use ui_render::DrawCommand;
use ui_platform::{Key, PointerButton};
use ui_render::CommandBuffer;
use ui_themes::ThemePalette;

#[derive(Clone, Debug, PartialEq)]
pub struct NodeEditor {
    pub nodes: Vec<GraphNode>,
    pub connections: Vec<NodeConnection>,
    pub groups: Vec<NodeGroup>,
    pub pan: Vec2,
    pub zoom: f32,
    pub selected_node: Option<String>,
    pub selected_nodes: Vec<String>,
    pub dragging_node: Option<DraggingNode>,
    pub pending_wire: Option<PendingWire>,
    pub marquee_start: Option<Vec2>,
    pub marquee_current: Option<Vec2>,
    pub search_state: SearchMenuState,
    pub show_minimap: bool,
}

impl Default for NodeEditor {
    fn default() -> Self {
        let (nodes, connections, groups) = create_default_graph();
        Self {
            nodes,
            connections,
            groups,
            pan: Vec2::new(40.0, 40.0),
            zoom: 1.0,
            selected_node: None,
            selected_nodes: vec![],
            dragging_node: None,
            pending_wire: None,
            marquee_start: None,
            marquee_current: None,
            search_state: SearchMenuState::default(),
            show_minimap: true,
        }
    }
}

impl NodeEditor {
    pub fn is_node_selected(&self, id: &str) -> bool {
        self.selected_nodes.iter().any(|s| s == id)
    }

    pub fn select_node(&mut self, id: &str, multi: bool) {
        if multi {
            if let Some(pos) = self.selected_nodes.iter().position(|s| s == id) {
                self.selected_nodes.remove(pos);
            } else {
                self.selected_nodes.push(id.to_string());
            }
            self.selected_node = self.selected_nodes.last().cloned();
        } else {
            self.selected_nodes = vec![id.to_string()];
            self.selected_node = Some(id.to_string());
        }
    }

    pub fn clear_selection(&mut self) {
        self.selected_nodes.clear();
        self.selected_node = None;
    }

    pub fn align_selected(&mut self, alignment: NodeAlignment) {
        align_nodes(&mut self.nodes, &self.selected_nodes, alignment);
    }

    pub fn open_search_menu(&mut self, world_pos: Vec2) {
        self.search_state.open(world_pos);
    }

    pub fn close_search_menu(&mut self) {
        self.search_state.close();
    }

    pub fn filter_presets(&self) -> Vec<PresetNode> {
        self.search_state.filter_presets()
    }

    pub fn world_to_screen(&self, world: Vec2, viewport_origin: Vec2) -> Vec2 {
        world_to_screen(world, self.pan, self.zoom, viewport_origin)
    }

    pub fn screen_to_world(&self, screen: Vec2, viewport_origin: Vec2) -> Vec2 {
        screen_to_world(screen, self.pan, self.zoom, viewport_origin)
    }

    pub fn zoom_at(&mut self, factor: f32, anchor_screen: Vec2, viewport_origin: Vec2) {
        let old_zoom = self.zoom;
        let new_zoom = (old_zoom * factor).clamp(0.2, 3.0);
        let world_anchor = (anchor_screen - viewport_origin) * (1.0 / old_zoom) - self.pan;
        self.pan = (anchor_screen - viewport_origin) * (1.0 / new_zoom) - world_anchor;
        self.zoom = new_zoom;
    }

    pub fn pan_by(&mut self, delta_screen: Vec2) {
        self.pan = self.pan + delta_screen * (1.0 / self.zoom);
    }

    pub fn input_pin_world_pos(&self, node: &GraphNode, pin_idx: usize) -> Vec2 {
        input_pin_world_pos(node, pin_idx)
    }

    pub fn output_pin_world_pos(&self, node: &GraphNode, pin_idx: usize) -> Vec2 {
        output_pin_world_pos(node, pin_idx)
    }

    pub fn toolbar_rects(viewport: Rect) -> Vec<(ToolbarAction, Rect, &'static str)> {
        toolbar_rects(viewport)
    }

    pub fn hit_test_toolbar(&self, screen_pos: Vec2, viewport: Rect) -> Option<ToolbarAction> {
        let rects: Vec<(ToolbarAction, Rect, &'static str)> = Self::toolbar_rects(viewport);
        for (action, rect, _) in rects {
            if rect.contains(screen_pos) {
                return Some(action);
            }
        }
        None
    }

    pub fn spawn_node(
        &mut self,
        title: &str,
        category: &str,
        world_pos: Vec2,
        inputs: Vec<(&str, &str)>,
        outputs: Vec<(&str, &str)>,
    ) -> String {
        let id = format!("node_{}", self.nodes.len() + 1);
        let in_pins = inputs
            .into_iter()
            .map(|(name, ptype)| NodePin {
                id: format!("{}_{}", id, name.to_lowercase().replace(' ', "_")),
                name: name.into(),
                pin_type: ptype.into(),
                is_output: false,
            })
            .collect();
        let out_pins = outputs
            .into_iter()
            .map(|(name, ptype)| NodePin {
                id: format!("{}_{}", id, name.to_lowercase().replace(' ', "_")),
                name: name.into(),
                pin_type: ptype.into(),
                is_output: true,
            })
            .collect();

        self.nodes.push(GraphNode {
            id: id.clone(),
            title: title.into(),
            pos: world_pos,
            size: Vec2::new(210.0, 140.0),
            inputs: in_pins,
            outputs: out_pins,
            category: category.into(),
            is_collapsed: false,
            preview_color: None,
        });
        id
    }

    pub fn spawn_preset(&mut self, preset: PresetNode, world_pos: Vec2) -> String {
        let id = format!("node_{}", self.nodes.len() + 1);
        let node = preset.create_node(id.clone(), world_pos);
        self.nodes.push(node);
        id
    }

    pub fn execution_order(&self) -> Vec<String> {
        let mut in_degrees: HashMap<String, usize> = HashMap::new();
        let mut adj: HashMap<String, Vec<String>> = HashMap::new();

        for node in &self.nodes {
            in_degrees.insert(node.id.clone(), 0);
            adj.insert(node.id.clone(), Vec::new());
        }

        for conn in &self.connections {
            if let Some(deg) = in_degrees.get_mut(&conn.to_node) {
                *deg += 1;
            }
            if let Some(list) = adj.get_mut(&conn.from_node) {
                list.push(conn.to_node.clone());
            }
        }

        let mut queue: VecDeque<String> = VecDeque::new();
        for (id, &deg) in &in_degrees {
            if deg == 0 {
                queue.push_back(id.clone());
            }
        }

        let mut order = Vec::new();
        while let Some(curr) = queue.pop_front() {
            order.push(curr.clone());
            if let Some(neighbors) = adj.get(&curr) {
                for nxt in neighbors {
                    if let Some(deg) = in_degrees.get_mut(nxt) {
                        *deg -= 1;
                        if *deg == 0 {
                            queue.push_back(nxt.clone());
                        }
                    }
                }
            }
        }

        order
    }

    pub fn export_pipeline_manifest(&self) -> String {
        let order = self.execution_order();
        let mut lines = Vec::new();
        lines.push("# Rupayan Flow / Sutro Visual Pipeline Manifest".to_string());
        lines.push(format!("Total Nodes: {}", self.nodes.len()));
        lines.push(format!("Total Connections: {}", self.connections.len()));
        lines.push("Execution Schedule:".to_string());

        for (idx, node_id) in order.iter().enumerate() {
            if let Some(node) = self.nodes.iter().find(|n| n.id == *node_id) {
                lines.push(format!("  [{}] Node '{}' ({})", idx + 1, node.title, node.category));
            }
        }

        lines.join("\n")
    }

    pub fn hit_test_pin(&self, world_pos: Vec2) -> Option<(String, String, bool)> {
        let pin_hit_radius = 12.0;
        for node in &self.nodes {
            for (idx, pin) in node.inputs.iter().enumerate() {
                let pos = self.input_pin_world_pos(node, idx);
                if (pos - world_pos).length() <= pin_hit_radius {
                    return Some((node.id.clone(), pin.id.clone(), false));
                }
            }
            for (idx, pin) in node.outputs.iter().enumerate() {
                let pos = self.output_pin_world_pos(node, idx);
                if (pos - world_pos).length() <= pin_hit_radius {
                    return Some((node.id.clone(), pin.id.clone(), true));
                }
            }
        }
        None
    }

    pub fn hit_test_node(&self, world_pos: Vec2) -> Option<String> {
        for node in self.nodes.iter().rev() {
            let rect = Rect::from_min_size(node.pos, node.size);
            if rect.contains(world_pos) {
                return Some(node.id.clone());
            }
        }
        None
    }

    pub fn can_reach(&self, start: &str, target: &str) -> bool {
        if start == target {
            return true;
        }
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        queue.push_back(start.to_string());
        visited.insert(start.to_string());

        while let Some(curr) = queue.pop_front() {
            for conn in &self.connections {
                if conn.from_node == curr && visited.insert(conn.to_node.clone()) {
                    if conn.to_node == target {
                        return true;
                    }
                    queue.push_back(conn.to_node.clone());
                }
            }
        }
        false
    }

    pub fn delete_selected(&mut self) {
        let to_remove = self.selected_nodes.clone();
        if to_remove.is_empty() {
            return;
        }
        self.nodes.retain(|n| !to_remove.contains(&n.id));
        self.connections.retain(|c| {
            !to_remove.contains(&c.from_node) && !to_remove.contains(&c.to_node)
        });
        self.selected_nodes.clear();
        self.selected_node = None;
    }

    pub fn handle_mouse_down(
        &mut self,
        screen_pos: Vec2,
        button: PointerButton,
        viewport: Rect,
    ) {
        self.handle_mouse_down_with_modifiers(screen_pos, button, viewport, false);
    }

    pub fn handle_mouse_down_with_modifiers(
        &mut self,
        screen_pos: Vec2,
        button: PointerButton,
        viewport: Rect,
        shift_held: bool,
    ) {
        if self.search_state.is_open {
            self.search_state.close();
            return;
        }

        if button == PointerButton::Primary {
            if let Some(action) = self.hit_test_toolbar(screen_pos, viewport) {
                match action {
                    ToolbarAction::SearchAdd => {
                        let center_world = self.screen_to_world(viewport.center(), viewport.min);
                        self.open_search_menu(center_world);
                    }
                    ToolbarAction::AddTexture => {
                        let pos = self.screen_to_world(viewport.center(), viewport.min);
                        self.spawn_preset(PresetNode::TextureSource, pos);
                    }
                    ToolbarAction::AddStylize | ToolbarAction::AddFilter => {
                        let pos = self.screen_to_world(viewport.center(), viewport.min);
                        self.spawn_preset(PresetNode::StylizeColor, pos);
                    }
                    ToolbarAction::AddSeamless => {
                        let pos = self.screen_to_world(viewport.center(), viewport.min);
                        self.spawn_preset(PresetNode::MakeSeamless, pos);
                    }
                    ToolbarAction::AddPbr => {
                        let pos = self.screen_to_world(viewport.center(), viewport.min);
                        self.spawn_preset(PresetNode::PbrGenerator, pos);
                    }
                    ToolbarAction::AddHeightToNormal => {
                        let pos = self.screen_to_world(viewport.center(), viewport.min);
                        self.spawn_preset(PresetNode::HeightToNormal, pos);
                    }
                    ToolbarAction::AddVideo | ToolbarAction::AddExport => {
                        let pos = self.screen_to_world(viewport.center(), viewport.min);
                        self.spawn_preset(PresetNode::PresentationVideo, pos);
                    }
                    ToolbarAction::AddReroute => {
                        let pos = self.screen_to_world(viewport.center(), viewport.min);
                        self.spawn_preset(PresetNode::Reroute, pos);
                    }
                    ToolbarAction::DeleteSelected => self.delete_selected(),
                    ToolbarAction::AlignLeft => self.align_selected(NodeAlignment::Left),
                    ToolbarAction::AlignTop => self.align_selected(NodeAlignment::Top),
                    ToolbarAction::ResetView => {
                        self.pan = Vec2::new(40.0, 40.0);
                        self.zoom = 1.0;
                    }
                    ToolbarAction::ToggleMinimap => self.show_minimap = !self.show_minimap,
                }
                return;
            }

            let world_pos = self.screen_to_world(screen_pos, viewport.min);

            if let Some((node_id, pin_id, is_output)) = self.hit_test_pin(world_pos) {
                self.pending_wire = Some(PendingWire {
                    from_node: node_id,
                    from_pin: pin_id,
                    is_output,
                    current_world_pos: world_pos,
                });
                return;
            }

            if let Some(node_id) = self.hit_test_node(world_pos) {
                if !shift_held && !self.is_node_selected(&node_id) {
                    self.clear_selection();
                }
                self.select_node(&node_id, shift_held);
                if let Some(node) = self.nodes.iter().find(|n| n.id == node_id) {
                    self.dragging_node = Some(DraggingNode {
                        node_id: node.id.clone(),
                        offset: world_pos - node.pos,
                    });
                }
                return;
            }

            if !shift_held {
                self.clear_selection();
            }
            self.marquee_start = Some(world_pos);
            self.marquee_current = Some(world_pos);
        }
    }

    pub fn handle_mouse_move(
        &mut self,
        screen_pos: Vec2,
        delta_screen: Vec2,
        viewport: Rect,
    ) {
        self.handle_mouse_move_with_buttons(screen_pos, delta_screen, viewport, false);
    }

    pub fn handle_mouse_move_with_buttons(
        &mut self,
        screen_pos: Vec2,
        delta_screen: Vec2,
        viewport: Rect,
        middle_down: bool,
    ) {
        if middle_down {
            self.pan_by(delta_screen);
            return;
        }

        let world_pos = self.screen_to_world(screen_pos, viewport.min);

        if let Some(wire) = &mut self.pending_wire {
            wire.current_world_pos = world_pos;
            return;
        }

        if let Some(drag) = &self.dragging_node {
            if let Some(node) = self.nodes.iter_mut().find(|n| n.id == drag.node_id) {
                let target_pos = world_pos - drag.offset;
                let delta = target_pos - node.pos;
                node.pos = target_pos;

                for sel_id in &self.selected_nodes {
                    if *sel_id != drag.node_id {
                        if let Some(other) = self.nodes.iter_mut().find(|n| n.id == *sel_id) {
                            other.pos = other.pos + delta;
                        }
                    }
                }
            }
            return;
        }

        if self.marquee_start.is_some() {
            self.marquee_current = Some(world_pos);
        }
    }

    pub fn handle_mouse_up(&mut self, screen_pos: Vec2, button: PointerButton, viewport: Rect) {
        if button == PointerButton::Primary {
            let world_pos = self.screen_to_world(screen_pos, viewport.min);

            if let (Some(start), Some(curr)) = (self.marquee_start.take(), self.marquee_current.take()) {
                let diff: Vec2 = start - curr;
                if diff.length() > 3.0 {
                    let box_rect = Rect::from_min_max(
                        Vec2::new(start.x.min(curr.x), start.y.min(curr.y)),
                        Vec2::new(start.x.max(curr.x), start.y.max(curr.y)),
                    );
                    for node in &self.nodes {
                        let node_rect = Rect::from_min_size(node.pos, node.size);
                        if box_rect.intersects(node_rect) && !self.is_node_selected(&node.id) {
                            self.selected_nodes.push(node.id.clone());
                        }
                    }
                    self.selected_node = self.selected_nodes.last().cloned();
                }
            }

            self.dragging_node = None;

            if let Some(wire) = self.pending_wire.take() {
                if let Some((target_node_id, target_pin_id, target_is_output)) = self.hit_test_pin(world_pos) {
                    if wire.from_node != target_node_id && wire.is_output != target_is_output {
                        let (from_n, from_p, to_n, to_p) = if wire.is_output {
                            (wire.from_node, wire.from_pin, target_node_id, target_pin_id)
                        } else {
                            (target_node_id, target_pin_id, wire.from_node, wire.from_pin)
                        };

                        let from_type = self.nodes.iter().find(|n| n.id == from_n)
                            .and_then(|n| n.outputs.iter().find(|p| p.id == from_p))
                            .map(|p| p.pin_type.clone()).unwrap_or_default();
                        let to_type = self.nodes.iter().find(|n| n.id == to_n)
                            .and_then(|n| n.inputs.iter().find(|p| p.id == to_p))
                            .map(|p| p.pin_type.clone()).unwrap_or_default();

                        let types_match = from_type == to_type || from_type == "Any" || to_type == "Any";
                        let creates_cycle = self.can_reach(&to_n, &from_n);

                        if types_match && !creates_cycle {
                            self.connections.retain(|c| !(c.to_node == to_n && c.to_pin == to_p));
                            self.connections.push(NodeConnection {
                                from_node: from_n,
                                from_pin: from_p,
                                to_node: to_n,
                                to_pin: to_p,
                            });
                        }
                    }
                }
            }
        }
    }

    pub fn handle_key_down(&mut self, key: Key) -> bool {
        if self.search_state.is_open {
            if let Some(preset) = self.search_state.handle_key_down(key) {
                let pos = self.search_state.world_pos;
                let id = self.spawn_preset(preset, pos);
                self.clear_selection();
                self.select_node(&id, false);
            }
            return true;
        }

        match key {
            Key::Delete | Key::Backspace => {
                if !self.selected_nodes.is_empty() {
                    self.delete_selected();
                    true
                } else {
                    false
                }
            }
            Key::Tab => {
                let pos = self.nodes.last().map(|n| n.pos + Vec2::new(n.size.x + 40.0, 0.0)).unwrap_or(Vec2::new(100.0, 100.0));
                self.open_search_menu(pos);
                true
            }
            Key::Escape => {
                self.clear_selection();
                true
            }
            _ => false,
        }
    }

    pub fn handle_scroll(&mut self, delta: Vec2, screen_pos: Vec2, viewport: Rect) {
        let zoom_factor = if delta.y > 0.0 { 1.1 } else { 0.9 };
        self.zoom_at(zoom_factor, screen_pos, viewport.min);
    }

    pub fn render(&self, buf: &mut CommandBuffer, viewport: Rect, palette: &ThemePalette) {
        buf.push(DrawCommand::ClipPush { rect: viewport });

        // Canvas backdrop
        buf.push(DrawCommand::Rect {
            rect: viewport,
            fill: palette.canvas,
            radius: CornerRadius::ZERO,
            stroke: Stroke::NONE,
        });

        // 1. Grid
        render_grid(buf, viewport, self.pan, self.zoom, palette);

        // 2. Connections
        render_connections(buf, &self.nodes, &self.connections, self.pan, self.zoom, viewport.min, palette);

        // 3. Pending wire
        if let Some(wire) = &self.pending_wire {
            render_pending_wire(buf, &self.nodes, wire, self.pan, self.zoom, viewport.min, palette);
        }

        // 3.5. Marquee box
        if let (Some(start), Some(curr)) = (self.marquee_start, self.marquee_current) {
            render_marquee(buf, start, curr, self.pan, self.zoom, viewport.min, palette);
        }

        // 4. Nodes
        render_nodes(buf, &self.nodes, &self.selected_nodes, self.pan, self.zoom, viewport.min, palette);

        // 5. Toolbar
        render_toolbar(buf, viewport, palette);

        // 6. HUD
        render_hud(buf, viewport, self.nodes.len(), self.connections.len(), self.zoom, palette);

        // 7. Search overlay
        render_search_overlay(buf, &self.search_state, self.pan, self.zoom, viewport.min, palette);

        // 8. Minimap
        if self.show_minimap {
            render_minimap(buf, &self.nodes, self.pan, self.zoom, viewport, palette);
        }

        buf.push(DrawCommand::ClipPop);
    }
}
