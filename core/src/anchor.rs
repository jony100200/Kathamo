//! Proportional Anchor and Offset layout system.
//! Provides instant, zero-allocation 4-float positioning for UI elements.

use crate::math::{Rect, Vec2};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Anchors {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

impl Default for Anchors {
    fn default() -> Self {
        Self::TOP_LEFT
    }
}

impl Anchors {
    pub const TOP_LEFT: Self = Self {
        left: 0.0,
        top: 0.0,
        right: 0.0,
        bottom: 0.0,
    };
    pub const TOP_RIGHT: Self = Self {
        left: 1.0,
        top: 0.0,
        right: 1.0,
        bottom: 0.0,
    };
    pub const BOTTOM_LEFT: Self = Self {
        left: 0.0,
        top: 1.0,
        right: 0.0,
        bottom: 1.0,
    };
    pub const BOTTOM_RIGHT: Self = Self {
        left: 1.0,
        top: 1.0,
        right: 1.0,
        bottom: 1.0,
    };
    pub const CENTER: Self = Self {
        left: 0.5,
        top: 0.5,
        right: 0.5,
        bottom: 0.5,
    };

    pub const FULL_RECT: Self = Self {
        left: 0.0,
        top: 0.0,
        right: 1.0,
        bottom: 1.0,
    };
    pub const TOP_WIDE: Self = Self {
        left: 0.0,
        top: 0.0,
        right: 1.0,
        bottom: 0.0,
    };
    pub const BOTTOM_WIDE: Self = Self {
        left: 0.0,
        top: 1.0,
        right: 1.0,
        bottom: 1.0,
    };
    pub const LEFT_WIDE: Self = Self {
        left: 0.0,
        top: 0.0,
        right: 0.0,
        bottom: 1.0,
    };
    pub const RIGHT_WIDE: Self = Self {
        left: 1.0,
        top: 0.0,
        right: 1.0,
        bottom: 1.0,
    };

    pub const fn new(left: f32, top: f32, right: f32, bottom: f32) -> Self {
        Self {
            left,
            top,
            right,
            bottom,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Offsets {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

impl Offsets {
    pub const ZERO: Self = Self {
        left: 0.0,
        top: 0.0,
        right: 0.0,
        bottom: 0.0,
    };

    pub const fn new(left: f32, top: f32, right: f32, bottom: f32) -> Self {
        Self {
            left,
            top,
            right,
            bottom,
        }
    }

    pub fn from_size(size: Vec2) -> Self {
        Self {
            left: 0.0,
            top: 0.0,
            right: size.x.max(0.0),
            bottom: size.y.max(0.0),
        }
    }

    pub fn centered(size: Vec2) -> Self {
        let half_w = size.x * 0.5;
        let half_h = size.y * 0.5;
        Self {
            left: -half_w,
            top: -half_h,
            right: half_w,
            bottom: half_h,
        }
    }
}

impl Anchors {
    /// Resolves child rectangle inside a parent bounding box in 4 float operations.
    /// Infallible: never returns inverted min/max, NaN, or negative size.
    #[inline]
    pub fn resolve(&self, offsets: Offsets, parent: Rect) -> Rect {
        let p_w = parent.width();
        let p_h = parent.height();

        let min_x = parent.min.x + p_w * self.left + offsets.left;
        let min_y = parent.min.y + p_h * self.top + offsets.top;
        let max_x = parent.min.x + p_w * self.right + offsets.right;
        let max_y = parent.min.y + p_h * self.bottom + offsets.bottom;

        Rect::from_min_max(Vec2::new(min_x, min_y), Vec2::new(max_x, max_y))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_rect_anchor_matches_parent_with_zero_offsets() {
        let parent = Rect::from_min_size(Vec2::new(100.0, 50.0), Vec2::new(800.0, 600.0));
        let child = Anchors::FULL_RECT.resolve(Offsets::ZERO, parent);
        assert_eq!(child.min, parent.min);
        assert_eq!(child.max, parent.max);
    }

    #[test]
    fn centered_anchor_positions_element_dead_center() {
        let parent = Rect::from_min_size(Vec2::ZERO, Vec2::new(1000.0, 1000.0));
        let offsets = Offsets::centered(Vec2::new(200.0, 100.0));
        let child = Anchors::CENTER.resolve(offsets, parent);

        assert_eq!(child.center(), parent.center());
        assert_eq!(child.width(), 200.0);
        assert_eq!(child.height(), 100.0);
    }

    #[test]
    fn bottom_wide_dock_creates_pinned_toolbar() {
        let parent = Rect::from_min_size(Vec2::ZERO, Vec2::new(1280.0, 720.0));
        let offsets = Offsets::new(0.0, -48.0, 0.0, 0.0); // 48px height docked to bottom
        let toolbar = Anchors::BOTTOM_WIDE.resolve(offsets, parent);

        assert_eq!(toolbar.min.y, 720.0 - 48.0);
        assert_eq!(toolbar.max.y, 720.0);
        assert_eq!(toolbar.width(), 1280.0);
    }
}
