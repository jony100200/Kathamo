//! Shared geometric helpers so the GPU shader and the CPU rasterizer produce the
//! same shapes (same SDF formulas, same bezier flattening, same text placement).

use ui_core::{CornerRadius, Vec2};

/// Signed distance from `p` (relative to the box centre) to a rounded box with
/// per-corner radii. Y points down. Negative = inside.
#[inline]
pub fn sd_round_box(p: Vec2, half: Vec2, radius: CornerRadius) -> f32 {
    let r = if p.x > 0.0 {
        if p.y > 0.0 {
            radius.bottom_right
        } else {
            radius.top_right
        }
    } else if p.y > 0.0 {
        radius.bottom_left
    } else {
        radius.top_left
    };
    let r = r.clamp(0.0, half.x.min(half.y));
    let qx = p.x.abs() - half.x + r;
    let qy = p.y.abs() - half.y + r;
    let outside = Vec2::new(qx.max(0.0), qy.max(0.0)).length();
    qx.max(qy).min(0.0) + outside - r
}

/// Distance from `p` to the segment `a`-`b`.
#[inline]
pub fn segment_distance(p: Vec2, a: Vec2, b: Vec2) -> f32 {
    let pa = p - a;
    let ba = b - a;
    let len_sq = ba.length_sq();
    let h = if len_sq > 0.0 {
        ((pa.x * ba.x + pa.y * ba.y) / len_sq).clamp(0.0, 1.0)
    } else {
        0.0
    };
    (pa - ba * h).length()
}

/// Flattens a cubic bezier into line segments with a length-adaptive segment count.
pub fn flatten_cubic(p0: Vec2, p1: Vec2, p2: Vec2, p3: Vec2) -> Vec<Vec2> {
    let approx_len = (p1 - p0).length() + (p2 - p1).length() + (p3 - p2).length();
    let segments = ((approx_len / 6.0).ceil() as usize).clamp(4, 96);
    (0..=segments)
        .map(|i| {
            let t = i as f32 / segments as f32;
            let mt = 1.0 - t;
            p0 * (mt * mt * mt)
                + p1 * (3.0 * mt * mt * t)
                + p2 * (3.0 * mt * t * t)
                + p3 * (t * t * t)
        })
        .collect()
}

#[inline]
pub fn scale_radius(r: CornerRadius, s: f32) -> CornerRadius {
    CornerRadius {
        top_left: r.top_left * s,
        top_right: r.top_right * s,
        bottom_right: r.bottom_right * s,
        bottom_left: r.bottom_left * s,
    }
}

#[inline]
pub fn max_radius(r: CornerRadius) -> f32 {
    r.top_left
        .max(r.top_right)
        .max(r.bottom_right)
        .max(r.bottom_left)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rounded_box_distance_is_signed() {
        let half = Vec2::new(10.0, 5.0);
        let r = CornerRadius::same(2.0);
        assert!(sd_round_box(Vec2::ZERO, half, r) < 0.0);
        assert!((sd_round_box(Vec2::new(10.0, 0.0), half, r)).abs() < 1e-4);
        assert!(sd_round_box(Vec2::new(12.0, 0.0), half, r) > 1.9);
        // The corner is rounded: the exact corner point lies outside.
        assert!(sd_round_box(Vec2::new(10.0, 5.0), half, r) > 0.5);
    }

    #[test]
    fn bezier_flattening_hits_endpoints() {
        let pts = flatten_cubic(
            Vec2::ZERO,
            Vec2::new(50.0, 0.0),
            Vec2::new(50.0, 100.0),
            Vec2::new(100.0, 100.0),
        );
        assert_eq!(pts[0], Vec2::ZERO);
        assert!((pts.last().unwrap().x - 100.0).abs() < 1e-3);
        assert!(pts.len() > 8);
    }
}
