//! Overlays for 2D and 3D views: a corner badge for render status ("64 spp · GPU") and an
//! axis orientation gizmo for orbit cameras.
//!
//! The gizmo maths is pure (no drawing state), so apps can use it for hit-testing or
//! their own drawing too.

use ui_core::{Color, CornerRadius, Rect, Stroke, Vec2};
use ui_render::{CommandBuffer, DrawCommand};
use ui_themes::ThemePalette;

use crate::type_scale::TypeRole;

/// Which corner of a view an overlay sits in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Corner {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

/// Places a box of `size` in `corner` of `view`, `inset` pixels from both edges.
pub fn corner_rect(view: Rect, corner: Corner, size: Vec2, inset: f32) -> Rect {
    let x = match corner {
        Corner::TopLeft | Corner::BottomLeft => view.min.x + inset,
        Corner::TopRight | Corner::BottomRight => view.max.x - inset - size.x,
    };
    let y = match corner {
        Corner::TopLeft | Corner::TopRight => view.min.y + inset,
        Corner::BottomLeft | Corner::BottomRight => view.max.y - inset - size.y,
    };
    Rect::from_min_size(Vec2::new(x, y), size)
}

/// A small rounded, translucent badge with monospace text in a corner of a view.
/// `live` adds an accent dot (for example while a render is still refining).
/// Returns the badge rectangle.
pub fn view_badge(
    buf: &mut CommandBuffer,
    view: Rect,
    corner: Corner,
    text: &str,
    live: bool,
    palette: &ThemePalette,
) -> Rect {
    let style = TypeRole::Numeric.style().at(11.0);
    let dot = if live { 12.0 } else { 0.0 };
    let size = Vec2::new((style.measure(text) + 16.0 + dot).round(), 22.0);
    let rect = corner_rect(view, corner, size, 8.0);
    buf.push(DrawCommand::Rect {
        rect,
        fill: palette.canvas.with_alpha(196),
        radius: CornerRadius::same(11.0),
        stroke: Stroke::new(1.0, palette.border.with_alpha(200)),
    });
    let mut x = rect.min.x + 8.0;
    if live {
        let d = Rect::from_min_size(Vec2::new(x, rect.center().y - 3.0), Vec2::splat(6.0));
        buf.push(DrawCommand::Rect {
            rect: d,
            fill: palette.accent,
            radius: CornerRadius::same(3.0),
            stroke: Stroke::NONE,
        });
        x += dot;
    }
    let y = rect.min.y + style.centre_offset(rect.height());
    style.draw(buf, text, Vec2::new(x, y), palette.text);
    rect
}

/// One axis direction the gizmo can snap a view to.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AxisView {
    pub label: &'static str,
    /// Camera yaw (radians) that looks down this axis.
    pub yaw: f32,
    /// Camera pitch (radians) that looks down this axis.
    pub pitch: f32,
    /// World direction of the axis.
    pub dir: [f32; 3],
}

/// The six axis views, positive axes first in X, Y, Z order.
pub const AXIS_VIEWS: [AxisView; 6] = [
    AxisView {
        label: "X",
        yaw: std::f32::consts::FRAC_PI_2,
        pitch: 0.0,
        dir: [1.0, 0.0, 0.0],
    },
    AxisView {
        label: "-X",
        yaw: -std::f32::consts::FRAC_PI_2,
        pitch: 0.0,
        dir: [-1.0, 0.0, 0.0],
    },
    AxisView {
        label: "Y",
        yaw: 0.0,
        pitch: 1.4,
        dir: [0.0, 1.0, 0.0],
    },
    AxisView {
        label: "-Y",
        yaw: 0.0,
        pitch: -1.4,
        dir: [0.0, -1.0, 0.0],
    },
    AxisView {
        label: "Z",
        yaw: 0.0,
        pitch: 0.0,
        dir: [0.0, 0.0, 1.0],
    },
    AxisView {
        label: "-Z",
        yaw: std::f32::consts::PI,
        pitch: 0.0,
        dir: [0.0, 0.0, -1.0],
    },
];

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// Screen offset (x right, y down, each -1..1) of a world `axis` for a Y-up orbit camera at
/// `yaw`/`pitch`, and the axis depth toward the viewer (positive faces the camera).
pub fn axis_on_screen(yaw: f32, pitch: f32, axis: [f32; 3]) -> (Vec2, f32) {
    let toward = [
        yaw.sin() * pitch.cos(),
        pitch.sin(),
        yaw.cos() * pitch.cos(),
    ];
    let right = cross([0.0, 1.0, 0.0], toward);
    let len = dot(right, right).sqrt();
    let right = if len > 1e-4 {
        [right[0] / len, right[1] / len, right[2] / len]
    } else {
        [1.0, 0.0, 0.0]
    };
    let up = cross(toward, right);
    (
        Vec2::new(dot(right, axis), -dot(up, axis)),
        dot(toward, axis),
    )
}

/// Conventional axis colour: X red, Y green, Z blue.
pub fn axis_colour(label: &str) -> Color {
    match label.trim_start_matches('-') {
        "X" => Color::rgb(239, 83, 80),
        "Y" => Color::rgb(102, 187, 106),
        _ => Color::rgb(66, 135, 245),
    }
}

/// Draws an axis gizmo of `radius` centred at `centre` and returns the index into
/// [`AXIS_VIEWS`] of the bubble under `pointer`, if any (click it to snap the view).
pub fn axis_gizmo(
    buf: &mut CommandBuffer,
    centre: Vec2,
    radius: f32,
    yaw: f32,
    pitch: f32,
    pointer: Vec2,
    palette: &ThemePalette,
) -> Option<usize> {
    let frame = Rect::from_min_size(centre - Vec2::splat(radius), Vec2::splat(radius * 2.0));
    buf.push(DrawCommand::Rect {
        rect: frame,
        fill: palette.canvas.with_alpha(110),
        radius: CornerRadius::same(radius),
        stroke: Stroke::new(1.0, palette.border.with_alpha(140)),
    });
    let reach = radius * 0.7;
    let mut order: Vec<usize> = (0..AXIS_VIEWS.len()).collect();
    let depth = |i: usize| axis_on_screen(yaw, pitch, AXIS_VIEWS[i].dir).1;
    order.sort_by(|a, b| depth(*a).total_cmp(&depth(*b)));
    let mut hit = None;
    let label_style = TypeRole::Numeric.style().at(10.0);
    for i in order {
        let view = AXIS_VIEWS[i];
        let (off, facing) = axis_on_screen(yaw, pitch, view.dir);
        let tip = centre + off * reach;
        let colour = axis_colour(view.label);
        let positive = !view.label.starts_with('-');
        if positive {
            buf.push(DrawCommand::Line {
                start: centre,
                end: tip,
                stroke: Stroke::new(1.5, colour),
            });
        }
        let r = if positive {
            radius * 0.2
        } else {
            radius * 0.13
        };
        let bubble = Rect::from_min_size(tip - Vec2::splat(r), Vec2::splat(r * 2.0));
        let alpha = if facing >= 0.0 { 255 } else { 150 };
        let fill = colour.with_alpha(if positive { alpha } else { 70 });
        buf.push(DrawCommand::Rect {
            rect: bubble,
            fill,
            radius: CornerRadius::same(r),
            stroke: Stroke::new(1.0, colour.with_alpha(alpha)),
        });
        if positive {
            let w = label_style.measure(view.label);
            let pos = Vec2::new(
                tip.x - w * 0.5,
                bubble.min.y + label_style.centre_offset(bubble.height()),
            );
            label_style.draw(buf, view.label, pos, Color::rgb(20, 20, 24));
        }
        if bubble.expand(2.0).contains(pointer) {
            hit = Some(i);
        }
    }
    hit
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn looking_down_z_puts_x_right_and_y_up() {
        let (x, _) = axis_on_screen(0.0, 0.0, [1.0, 0.0, 0.0]);
        let (y, _) = axis_on_screen(0.0, 0.0, [0.0, 1.0, 0.0]);
        let (_, z_depth) = axis_on_screen(0.0, 0.0, [0.0, 0.0, 1.0]);
        assert!((x.x - 1.0).abs() < 1e-4 && x.y.abs() < 1e-4);
        assert!((y.y + 1.0).abs() < 1e-4, "screen y grows downward");
        assert!((z_depth - 1.0).abs() < 1e-4);
    }

    #[test]
    fn every_axis_view_faces_its_own_axis() {
        for view in AXIS_VIEWS {
            let (_, depth) = axis_on_screen(view.yaw, view.pitch, view.dir);
            assert!(depth > 0.9, "{}: {depth}", view.label);
        }
    }

    #[test]
    fn corner_rect_insets_from_each_corner() {
        let view = Rect::from_min_size(Vec2::ZERO, Vec2::new(100.0, 50.0));
        let r = corner_rect(view, Corner::BottomRight, Vec2::new(20.0, 10.0), 4.0);
        assert_eq!(r.max, Vec2::new(96.0, 46.0));
        let r = corner_rect(view, Corner::TopLeft, Vec2::new(20.0, 10.0), 4.0);
        assert_eq!(r.min, Vec2::new(4.0, 4.0));
    }

    #[test]
    fn badge_and_gizmo_draw_and_the_gizmo_reports_hits() {
        let palette = ThemePalette::dark();
        let mut buf = CommandBuffer::new();
        let view = Rect::from_min_size(Vec2::ZERO, Vec2::new(400.0, 300.0));
        let badge = view_badge(
            &mut buf,
            view,
            Corner::TopRight,
            "64 spp · GPU",
            true,
            &palette,
        );
        assert!(badge.max.x <= view.max.x && badge.min.x > 200.0);
        let centre = Vec2::new(100.0, 100.0);
        // Looking down +Z, the +Z bubble sits on the centre.
        let hit = axis_gizmo(&mut buf, centre, 40.0, 0.0, 0.0, centre, &palette);
        assert_eq!(hit.map(|i| AXIS_VIEWS[i].label), Some("Z"));
        let miss = axis_gizmo(&mut buf, centre, 40.0, 0.0, 0.0, Vec2::ZERO, &palette);
        assert_eq!(miss, None);
    }
}
