use super::geometry::{input_pin_world_pos, output_pin_world_pos, toolbar_rects, world_to_screen};
use super::search_menu::SearchMenuState;
use super::types::{
    category_color, pin_type_color, GraphNode, NodeConnection, PendingWire, ToolbarAction,
};
use crate::theme::ThemePalette;
use ui_core::{CornerRadius, Rect, Stroke, Vec2};
use ui_render::{CommandBuffer, DrawCommand};

pub fn render_grid(
    buf: &mut CommandBuffer,
    viewport: Rect,
    pan: Vec2,
    zoom: f32,
    palette: &ThemePalette,
) {
    let grid_size = 32.0 * zoom;
    if grid_size <= 8.0 {
        return;
    }
    let offset_x = (pan.x * zoom).rem_euclid(grid_size);
    let offset_y = (pan.y * zoom).rem_euclid(grid_size);
    let grid_stroke = Stroke::new(1.0, palette.border.with_alpha(35));

    let mut x = viewport.min.x + offset_x;
    while x < viewport.max.x {
        buf.push(DrawCommand::Line {
            start: Vec2::new(x, viewport.min.y),
            end: Vec2::new(x, viewport.max.y),
            stroke: grid_stroke,
        });
        x += grid_size;
    }

    let mut y = viewport.min.y + offset_y;
    while y < viewport.max.y {
        buf.push(DrawCommand::Line {
            start: Vec2::new(viewport.min.x, y),
            end: Vec2::new(viewport.max.x, y),
            stroke: grid_stroke,
        });
        y += grid_size;
    }
}

pub fn render_connections(
    buf: &mut CommandBuffer,
    nodes: &[GraphNode],
    connections: &[NodeConnection],
    pan: Vec2,
    zoom: f32,
    viewport_origin: Vec2,
    palette: &ThemePalette,
) {
    for conn in connections {
        let from_n = nodes.iter().find(|n| n.id == conn.from_node);
        let to_n = nodes.iter().find(|n| n.id == conn.to_node);
        if let (Some(from_n), Some(to_n)) = (from_n, to_n) {
            let out_idx = from_n.outputs.iter().position(|p| p.id == conn.from_pin);
            let in_idx = to_n.inputs.iter().position(|p| p.id == conn.to_pin);
            if let (Some(out_idx), Some(in_idx)) = (out_idx, in_idx) {
                let start_world = output_pin_world_pos(from_n, out_idx);
                let end_world = input_pin_world_pos(to_n, in_idx);
                let start = world_to_screen(start_world, pan, zoom, viewport_origin);
                let end = world_to_screen(end_world, pan, zoom, viewport_origin);

                let dist = (end.x - start.x).abs().max(40.0 * zoom) * 0.5;
                let ctrl1 = Vec2::new(start.x + dist, start.y);
                let ctrl2 = Vec2::new(end.x - dist, end.y);
                let wire_color = pin_type_color(&from_n.outputs[out_idx].pin_type, palette);

                buf.push(DrawCommand::CubicBezier {
                    start,
                    ctrl1,
                    ctrl2,
                    end,
                    stroke: Stroke::new(2.5 * zoom, wire_color),
                });
            }
        }
    }
}

pub fn render_pending_wire(
    buf: &mut CommandBuffer,
    nodes: &[GraphNode],
    pending_wire: &PendingWire,
    pan: Vec2,
    zoom: f32,
    viewport_origin: Vec2,
    palette: &ThemePalette,
) {
    if let Some(from_n) = nodes.iter().find(|n| n.id == pending_wire.from_node) {
        let (start_world, pin_type) = if pending_wire.is_output {
            let idx = from_n
                .outputs
                .iter()
                .position(|p| p.id == pending_wire.from_pin)
                .unwrap_or(0);
            (
                output_pin_world_pos(from_n, idx),
                from_n
                    .outputs
                    .get(idx)
                    .map(|p| p.pin_type.as_str())
                    .unwrap_or("Any"),
            )
        } else {
            let idx = from_n
                .inputs
                .iter()
                .position(|p| p.id == pending_wire.from_pin)
                .unwrap_or(0);
            (
                input_pin_world_pos(from_n, idx),
                from_n
                    .inputs
                    .get(idx)
                    .map(|p| p.pin_type.as_str())
                    .unwrap_or("Any"),
            )
        };

        let start = world_to_screen(start_world, pan, zoom, viewport_origin);
        let end = world_to_screen(pending_wire.current_world_pos, pan, zoom, viewport_origin);
        let dist = (end.x - start.x).abs().max(40.0 * zoom) * 0.5;
        let (ctrl1, ctrl2) = if pending_wire.is_output {
            (Vec2::new(start.x + dist, start.y), Vec2::new(end.x - dist, end.y))
        } else {
            (Vec2::new(start.x - dist, start.y), Vec2::new(end.x + dist, end.y))
        };

        buf.push(DrawCommand::CubicBezier {
            start,
            ctrl1,
            ctrl2,
            end,
            stroke: Stroke::new(2.0 * zoom, pin_type_color(pin_type, palette)),
        });
    }
}

pub fn render_marquee(
    buf: &mut CommandBuffer,
    start: Vec2,
    current: Vec2,
    pan: Vec2,
    zoom: f32,
    viewport_origin: Vec2,
    palette: &ThemePalette,
) {
    let s_screen = world_to_screen(start, pan, zoom, viewport_origin);
    let c_screen = world_to_screen(current, pan, zoom, viewport_origin);
    let rect = Rect::from_min_max(
        Vec2::new(s_screen.x.min(c_screen.x), s_screen.y.min(c_screen.y)),
        Vec2::new(s_screen.x.max(c_screen.x), s_screen.y.max(c_screen.y)),
    );
    buf.push(DrawCommand::Rect {
        rect,
        fill: palette.accent.with_alpha(25),
        radius: CornerRadius::ZERO,
        stroke: Stroke::new(1.0, palette.accent.with_alpha(150)),
    });
}

pub fn render_nodes(
    buf: &mut CommandBuffer,
    nodes: &[GraphNode],
    selected_nodes: &[String],
    pan: Vec2,
    zoom: f32,
    viewport_origin: Vec2,
    palette: &ThemePalette,
) {
    for node in nodes {
        let is_selected = selected_nodes.contains(&node.id);
        let screen_pos = world_to_screen(node.pos, pan, zoom, viewport_origin);
        let screen_size = node.size * zoom;
        let node_rect = Rect::from_min_size(screen_pos, screen_size);

        if node.is_reroute() {
            let radius = 7.0 * zoom;
            buf.push(DrawCommand::Rect {
                rect: Rect::from_min_size(screen_pos, screen_size),
                fill: if is_selected { palette.accent } else { palette.surface },
                radius: CornerRadius::same(radius),
                stroke: Stroke::new(1.5 * zoom, if is_selected { palette.accent } else { palette.border }),
            });
            continue;
        }

        // Card body
        buf.push(DrawCommand::Rect {
            rect: node_rect,
            fill: palette.surface,
            radius: CornerRadius::same(6.0 * zoom),
            stroke: Stroke::new(
                if is_selected { 2.0 * zoom } else { 1.0 },
                if is_selected { palette.accent } else { palette.border },
            ),
        });

        // Header bar
        let header_height = 36.0 * zoom;
        let header_rect = Rect::from_min_size(screen_pos, Vec2::new(screen_size.x, header_height));
        let cat_color = category_color(&node.category, palette);

        buf.push(DrawCommand::Rect {
            rect: header_rect,
            fill: cat_color.with_alpha(20),
            radius: CornerRadius {
                top_left: 6.0 * zoom,
                top_right: 6.0 * zoom,
                bottom_left: 0.0,
                bottom_right: 0.0,
            },
            stroke: Stroke::NONE,
        });

        // Accent bar
        let accent_height = 4.0 * zoom;
        buf.push(DrawCommand::Rect {
            rect: Rect::from_min_size(screen_pos, Vec2::new(screen_size.x, accent_height)),
            fill: cat_color,
            radius: CornerRadius {
                top_left: 6.0 * zoom,
                top_right: 6.0 * zoom,
                bottom_left: 0.0,
                bottom_right: 0.0,
            },
            stroke: Stroke::NONE,
        });

        // Header title
        buf.push(DrawCommand::Text {
            text: node.title.clone(),
            pos: screen_pos + Vec2::new(12.0 * zoom, 22.0 * zoom),
            size: 13.0 * zoom,
            color: palette.text,
            strong: true,
        });

        // Input pins
        for (idx, pin) in node.inputs.iter().enumerate() {
            let p_world = input_pin_world_pos(node, idx);
            let p_screen = world_to_screen(p_world, pan, zoom, viewport_origin);
            let p_color = pin_type_color(&pin.pin_type, palette);
            let pin_radius = 4.5 * zoom;
            buf.push(DrawCommand::Rect {
                rect: Rect::from_min_size(
                    Vec2::new(p_screen.x - pin_radius, p_screen.y - pin_radius),
                    Vec2::new(pin_radius * 2.0, pin_radius * 2.0),
                ),
                fill: p_color,
                radius: CornerRadius::same(pin_radius),
                stroke: Stroke::new(1.0, palette.canvas),
            });
            buf.push(DrawCommand::Text {
                text: pin.name.clone(),
                pos: p_screen + Vec2::new(10.0 * zoom, 4.0 * zoom),
                size: 11.0 * zoom,
                color: palette.text_muted,
                strong: false,
            });
        }

        // Output pins
        for (idx, pin) in node.outputs.iter().enumerate() {
            let p_world = output_pin_world_pos(node, idx);
            let p_screen = world_to_screen(p_world, pan, zoom, viewport_origin);
            let p_color = pin_type_color(&pin.pin_type, palette);
            let pin_radius = 4.5 * zoom;
            buf.push(DrawCommand::Rect {
                rect: Rect::from_min_size(
                    Vec2::new(p_screen.x - pin_radius, p_screen.y - pin_radius),
                    Vec2::new(pin_radius * 2.0, pin_radius * 2.0),
                ),
                fill: p_color,
                radius: CornerRadius::same(pin_radius),
                stroke: Stroke::new(1.0, palette.canvas),
            });
            let text_offset_x = (pin.name.len() as f32 * 6.5 + 10.0) * zoom;
            buf.push(DrawCommand::Text {
                text: pin.name.clone(),
                pos: p_screen - Vec2::new(text_offset_x, -4.0 * zoom),
                size: 11.0 * zoom,
                color: palette.text_muted,
                strong: false,
            });
        }
    }
}

pub fn render_toolbar(buf: &mut CommandBuffer, viewport: Rect, palette: &ThemePalette) {
    let t_rects: Vec<(ToolbarAction, Rect, &'static str)> = toolbar_rects(viewport);
    for (action, rect, label) in t_rects {
        let is_primary = matches!(action, ToolbarAction::SearchAdd);
        buf.push(DrawCommand::Rect {
            rect,
            fill: if is_primary { palette.accent.with_alpha(40) } else { palette.surface.with_alpha(200) },
            radius: CornerRadius::same(4.0),
            stroke: Stroke::new(1.0, if is_primary { palette.accent } else { palette.border }),
        });
        buf.push(DrawCommand::Text {
            text: label.to_string(),
            pos: rect.min + Vec2::new(8.0, 7.0),
            size: 11.0,
            color: if is_primary { palette.accent } else { palette.text },
            strong: is_primary,
        });
    }
}

pub fn render_hud(
    buf: &mut CommandBuffer,
    viewport: Rect,
    node_count: usize,
    conn_count: usize,
    zoom: f32,
    palette: &ThemePalette,
) {
    let hud_pos = Vec2::new(viewport.min.x + 16.0, viewport.max.y - 32.0);
    let hud_text = format!(
        "Nodes: {} | Cables: {} | Zoom: {:.0}% | Drag to wire | Box select: LMB drag | Shift: Multi-select | Tab: Add",
        node_count,
        conn_count,
        zoom * 100.0
    );
    buf.push(DrawCommand::Text {
        text: hud_text,
        pos: hud_pos,
        size: 11.0,
        color: palette.text_muted,
        strong: false,
    });
}

pub fn render_search_overlay(
    buf: &mut CommandBuffer,
    search: &SearchMenuState,
    pan: Vec2,
    zoom: f32,
    viewport_origin: Vec2,
    palette: &ThemePalette,
) {
    if !search.is_open {
        return;
    }

    let menu_screen = world_to_screen(search.world_pos, pan, zoom, viewport_origin);
    let menu_size = Vec2::new(260.0, 240.0);
    let menu_rect = Rect::from_min_size(menu_screen, menu_size);

    buf.push(DrawCommand::Rect {
        rect: menu_rect,
        fill: palette.surface,
        radius: CornerRadius::same(8.0),
        stroke: Stroke::new(1.5, palette.accent),
    });

    let header_rect = Rect::from_min_size(menu_screen, Vec2::new(menu_size.x, 34.0));
    buf.push(DrawCommand::Rect {
        rect: header_rect,
        fill: palette.elevated,
        radius: CornerRadius {
            top_left: 8.0,
            top_right: 8.0,
            bottom_left: 0.0,
            bottom_right: 0.0,
        },
        stroke: Stroke::NONE,
    });

    let search_label = if search.query.is_empty() {
        "Type to filter presets...".to_string()
    } else {
        format!("Search: {}_", search.query)
    };

    buf.push(DrawCommand::Text {
        text: search_label,
        pos: menu_screen + Vec2::new(12.0, 10.0),
        size: 12.0,
        color: if search.query.is_empty() { palette.text_muted } else { palette.accent },
        strong: !search.query.is_empty(),
    });

    let filtered = search.filter_presets();
    let row_height = 28.0;
    let list_start_y = menu_screen.y + 40.0;

    for (idx, preset) in filtered.iter().take(6).enumerate() {
        let y = list_start_y + (idx as f32 * row_height);
        let row_rect = Rect::from_min_size(Vec2::new(menu_screen.x + 4.0, y), Vec2::new(menu_size.x - 8.0, row_height));
        let is_selected = idx == search.selected_index;

        if is_selected {
            buf.push(DrawCommand::Rect {
                rect: row_rect,
                fill: palette.accent.with_alpha(40),
                radius: CornerRadius::same(4.0),
                stroke: Stroke::NONE,
            });
        }

        let cat_color = category_color(preset.category(), palette);
        buf.push(DrawCommand::Rect {
            rect: Rect::from_min_size(Vec2::new(menu_screen.x + 8.0, y + 6.0), Vec2::new(3.0, 16.0)),
            fill: cat_color,
            radius: CornerRadius::same(1.0),
            stroke: Stroke::NONE,
        });

        buf.push(DrawCommand::Text {
            text: preset.title().into(),
            pos: Vec2::new(menu_screen.x + 18.0, y + 7.0),
            size: 11.5,
            color: if is_selected { palette.accent } else { palette.text },
            strong: is_selected,
        });
    }
}
