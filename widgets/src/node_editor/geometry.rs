use super::types::{GraphNode, NodeAlignment, ToolbarAction};
use ui_core::{Rect, Vec2};

pub fn world_to_screen(world: Vec2, pan: Vec2, zoom: f32, viewport_origin: Vec2) -> Vec2 {
    viewport_origin + (world + pan) * zoom
}

pub fn screen_to_world(screen: Vec2, pan: Vec2, zoom: f32, viewport_origin: Vec2) -> Vec2 {
    (screen - viewport_origin) * (1.0 / zoom) - pan
}

pub fn input_pin_world_pos(node: &GraphNode, pin_idx: usize) -> Vec2 {
    if node.is_reroute() {
        return node.pos + Vec2::new(0.0, node.size.y * 0.5);
    }
    let header_height = 36.0;
    let pin_spacing = 24.0;
    let y = node.pos.y + header_height + 16.0 + (pin_idx as f32 * pin_spacing);
    Vec2::new(node.pos.x, y)
}

pub fn output_pin_world_pos(node: &GraphNode, pin_idx: usize) -> Vec2 {
    if node.is_reroute() {
        return node.pos + Vec2::new(node.size.x, node.size.y * 0.5);
    }
    let header_height = 36.0;
    let pin_spacing = 24.0;
    let y = node.pos.y + header_height + 16.0 + (pin_idx as f32 * pin_spacing);
    Vec2::new(node.pos.x + node.size.x, y)
}

pub fn toolbar_rects(viewport: Rect) -> Vec<(ToolbarAction, Rect, &'static str)> {
    let mut actions = Vec::new();
    let btn_width = 86.0;
    let btn_height = 26.0;
    let spacing = 6.0;
    let start_x = viewport.min.x + 16.0;
    let y = viewport.min.y + 12.0;

    let items = [
        (ToolbarAction::SearchAdd, "+ Add Node..."),
        (ToolbarAction::AddTexture, "+ Texture"),
        (ToolbarAction::AddStylize, "+ Stylize"),
        (ToolbarAction::AddSeamless, "+ Seamless"),
        (ToolbarAction::AddPbr, "+ PBR"),
        (ToolbarAction::AddHeightToNormal, "+ Normal"),
        (ToolbarAction::AddVideo, "+ Present"),
        (ToolbarAction::AddReroute, "+ Reroute"),
        (ToolbarAction::AlignLeft, "Align L"),
        (ToolbarAction::AlignTop, "Align T"),
        (ToolbarAction::ToggleMinimap, "Minimap"),
        (ToolbarAction::ResetView, "Reset"),
        (ToolbarAction::DeleteSelected, "Delete"),
    ];

    for (i, (act, label)) in items.into_iter().enumerate() {
        let x = start_x + (i as f32 * (btn_width + spacing));
        let rect = Rect::from_min_size(Vec2::new(x, y), Vec2::new(btn_width, btn_height));
        actions.push((act, rect, label));
    }

    actions
}

pub fn align_nodes(nodes: &mut [GraphNode], selected_ids: &[String], alignment: NodeAlignment) {
    if selected_ids.len() < 2 {
        return;
    }

    let min_x = nodes
        .iter()
        .filter(|n| selected_ids.contains(&n.id))
        .map(|n| n.pos.x)
        .fold(f32::INFINITY, f32::min);

    let max_x = nodes
        .iter()
        .filter(|n| selected_ids.contains(&n.id))
        .map(|n| n.pos.x + n.size.x)
        .fold(f32::NEG_INFINITY, f32::max);

    let min_y = nodes
        .iter()
        .filter(|n| selected_ids.contains(&n.id))
        .map(|n| n.pos.y)
        .fold(f32::INFINITY, f32::min);

    let max_y = nodes
        .iter()
        .filter(|n| selected_ids.contains(&n.id))
        .map(|n| n.pos.y + n.size.y)
        .fold(f32::NEG_INFINITY, f32::max);

    for node in nodes.iter_mut() {
        if selected_ids.contains(&node.id) {
            match alignment {
                NodeAlignment::Left => node.pos.x = min_x,
                NodeAlignment::Right => node.pos.x = max_x - node.size.x,
                NodeAlignment::Top => node.pos.y = min_y,
                NodeAlignment::Bottom => node.pos.y = max_y - node.size.y,
            }
        }
    }
}
