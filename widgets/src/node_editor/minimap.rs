use super::types::{category_color, GraphNode};
use crate::theme::ThemePalette;
use ui_core::{CornerRadius, Rect, Stroke, Vec2};
use ui_render::{CommandBuffer, DrawCommand};

pub fn minimap_rect(viewport: Rect) -> Rect {
    let map_w = 160.0;
    let map_h = 100.0;
    Rect::from_min_size(
        Vec2::new(viewport.min.x + 16.0, viewport.max.y - map_h - 16.0),
        Vec2::new(map_w, map_h),
    )
}

pub fn render_minimap(
    buf: &mut CommandBuffer,
    nodes: &[GraphNode],
    _pan: Vec2,
    _zoom: f32,
    viewport: Rect,
    palette: &ThemePalette,
) {
    if nodes.is_empty() {
        return;
    }

    let map_rect = minimap_rect(viewport);

    // Frame backdrop
    buf.push(DrawCommand::Rect {
        rect: map_rect,
        fill: palette.surface.with_alpha(220),
        radius: CornerRadius::same(6.0),
        stroke: Stroke::new(1.0, palette.border),
    });

    let mut min_bound = nodes[0].pos;
    let mut max_bound = nodes[0].pos + nodes[0].size;
    for n in nodes {
        min_bound.x = min_bound.x.min(n.pos.x);
        min_bound.y = min_bound.y.min(n.pos.y);
        max_bound.x = max_bound.x.max(n.pos.x + n.size.x);
        max_bound.y = max_bound.y.max(n.pos.y + n.size.y);
    }
    min_bound = min_bound - Vec2::splat(100.0);
    max_bound = max_bound + Vec2::splat(100.0);
    let span = max_bound - min_bound;
    if span.x > 0.0 && span.y > 0.0 {
        let scale = (map_rect.size().x / span.x).min(map_rect.size().y / span.y);
        for n in nodes {
            let norm_pos = (n.pos - min_bound) * scale;
            let norm_size = Vec2::new((n.size.x * scale).max(3.0), (n.size.y * scale).max(3.0));
            let dot_rect = Rect::from_min_size(
                Vec2::new(map_rect.min.x + norm_pos.x, map_rect.min.y + norm_pos.y),
                norm_size,
            );
            let col = category_color(&n.category, palette);
            buf.push(DrawCommand::Rect {
                rect: dot_rect,
                fill: col,
                radius: CornerRadius::same(1.0),
                stroke: Stroke::NONE,
            });
        }
    }
}
