//! Segmented tab strip: tabs sized from measured text with a sliding active pill.
//!
//! Layout and hit-testing share [`tab_rects`], so a click always lands on the tab
//! that was drawn. The active pill springs between tabs instead of jumping.

use ui_core::{motion, Color, CornerRadius, Rect, Stroke, Vec2};
use ui_render::{CommandBuffer, DrawCommand};
use ui_themes::ThemePalette;

pub const TAB_TEXT_PX: f32 = 12.0;
const PAD_X: f32 = 14.0;
const INSET: f32 = 2.0;

/// Tab rectangles laid out left to right from `origin`, each as wide as its label.
pub fn tab_rects(labels: &[&str], origin: Vec2, height: f32) -> Vec<Rect> {
    let mut x = origin.x + INSET;
    labels
        .iter()
        .map(|label| {
            let w = ui_render::measure_text(label, TAB_TEXT_PX, true) + PAD_X * 2.0;
            let r = Rect::from_min_size(
                Vec2::new(x, origin.y + INSET),
                Vec2::new(w, height - INSET * 2.0),
            );
            x += w;
            r
        })
        .collect()
}

/// Container rectangle that encloses `rects`.
pub fn strip_bounds(rects: &[Rect], origin: Vec2, height: f32) -> Rect {
    let right = rects.last().map_or(origin.x, |r| r.max.x) + INSET;
    Rect::from_min_size(origin, Vec2::new(right - origin.x, height))
}

/// Index of the tab under `pos`, if any.
pub fn hit(rects: &[Rect], pos: Vec2) -> Option<usize> {
    rects.iter().position(|r| r.contains(pos))
}

/// Draws the strip. `rects` come from [`tab_rects`] or any caller layout.
pub fn render(
    buf: &mut CommandBuffer,
    labels: &[&str],
    rects: &[Rect],
    bounds: Rect,
    active: usize,
    pointer: Vec2,
    palette: &ThemePalette,
) {
    if labels.is_empty() || rects.len() != labels.len() {
        return;
    }
    buf.push(DrawCommand::Rect {
        rect: bounds,
        fill: palette.canvas.with_alpha(200),
        radius: CornerRadius::same(7.0),
        stroke: Stroke::new(1.0, palette.border.with_alpha(160)),
    });

    // Sliding pill: x and width spring toward the active tab.
    let target = rects[active.min(rects.len() - 1)];
    let key = motion::rect_key("tab_strip", bounds);
    let x = motion::animate(key, target.min.x, motion::SLIDE);
    let w = motion::animate(key ^ 1, target.width(), motion::SLIDE);
    let pill = Rect::from_min_size(Vec2::new(x, target.min.y), Vec2::new(w, target.height()));
    buf.push(DrawCommand::SoftShadow {
        rect: pill,
        radius: CornerRadius::same(5.0),
        color: Color::rgba(0, 0, 0, 70),
        offset: Vec2::new(0.0, 1.0),
        blur: 4.0,
        spread: 0.0,
    });
    buf.push(DrawCommand::Surface {
        rect: pill,
        fill_top: palette.elevated_high,
        fill_bottom: palette.elevated,
        radius: CornerRadius::same(5.0),
        stroke: Stroke::new(1.0, palette.border.with_alpha(200)),
        rim_strength: 0.5,
    });

    for (i, (label, r)) in labels.iter().zip(rects).enumerate() {
        let is_active = i == active;
        let hover = motion::animate_bool(
            motion::rect_key("tab_strip_hover", *r),
            r.contains(pointer) && !is_active,
            motion::HOVER,
        );
        let color = if is_active {
            palette.text
        } else {
            palette.text_muted.lerp(palette.text, hover * 0.7)
        };
        let text_w = ui_render::measure_text(label, TAB_TEXT_PX, is_active);
        buf.push(DrawCommand::Text {
            text: (*label).to_string(),
            pos: Vec2::new(
                r.min.x + (r.width() - text_w) * 0.5,
                r.min.y + (r.height() - TAB_TEXT_PX) * 0.5 - 1.0,
            ),
            size: TAB_TEXT_PX,
            color,
            strong: is_active,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tabs_are_measured_contiguous_and_hit_testable() {
        let labels = ["2D Canvas", "3D Spatial", "Logic Graph"];
        let rects = tab_rects(&labels, Vec2::new(10.0, 0.0), 28.0);
        assert_eq!(rects.len(), 3);
        for pair in rects.windows(2) {
            assert!(
                (pair[0].max.x - pair[1].min.x).abs() < 0.01,
                "tabs must not overlap"
            );
        }
        assert!(rects[2].width() > rects[0].width() * 0.8);
        let mid = rects[1].center();
        assert_eq!(hit(&rects, mid), Some(1));
        let bounds = strip_bounds(&rects, Vec2::new(10.0, 0.0), 28.0);
        assert!(bounds.max.x >= rects[2].max.x);
    }
}
