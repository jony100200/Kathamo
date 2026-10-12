//! Geometric and color primitives for ui-core.
//! Strictly pure data: infallible, NaN-safe, zero allocations.

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub const ZERO: Self = Self { x: 0.0, y: 0.0 };

    #[inline]
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    #[inline]
    pub const fn splat(v: f32) -> Self {
        Self { x: v, y: v }
    }

    #[inline]
    pub fn length_sq(self) -> f32 {
        self.x * self.x + self.y * self.y
    }

    #[inline]
    pub fn length(self) -> f32 {
        self.length_sq().sqrt()
    }

    #[inline]
    pub fn safe(self) -> Self {
        Self {
            x: if self.x.is_finite() { self.x } else { 0.0 },
            y: if self.y.is_finite() { self.y } else { 0.0 },
        }
    }
}

impl std::ops::Add for Vec2 {
    type Output = Self;
    #[inline]
    fn add(self, rhs: Self) -> Self {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

impl std::ops::Sub for Vec2 {
    type Output = Self;
    #[inline]
    fn sub(self, rhs: Self) -> Self {
        Self {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

impl std::ops::Mul<f32> for Vec2 {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: f32) -> Self {
        Self {
            x: self.x * rhs,
            y: self.y * rhs,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Rect {
    pub min: Vec2,
    pub max: Vec2,
}

impl Rect {
    pub const ZERO: Self = Self {
        min: Vec2::ZERO,
        max: Vec2::ZERO,
    };

    #[inline]
    pub fn from_min_max(min: Vec2, max: Vec2) -> Self {
        Self {
            min: Vec2::new(min.x.min(max.x), min.y.min(max.y)).safe(),
            max: Vec2::new(min.x.max(max.x), min.y.max(max.y)).safe(),
        }
    }

    #[inline]
    pub fn from_min_size(min: Vec2, size: Vec2) -> Self {
        let size = size.safe();
        Self::from_min_max(min, min + Vec2::new(size.x.max(0.0), size.y.max(0.0)))
    }

    #[inline]
    pub fn width(self) -> f32 {
        (self.max.x - self.min.x).max(0.0)
    }

    #[inline]
    pub fn height(self) -> f32 {
        (self.max.y - self.min.y).max(0.0)
    }

    #[inline]
    pub fn size(self) -> Vec2 {
        Vec2::new(self.width(), self.height())
    }

    #[inline]
    pub fn center(self) -> Vec2 {
        Vec2::new(
            (self.min.x + self.max.x) * 0.5,
            (self.min.y + self.max.y) * 0.5,
        )
    }

    #[inline]
    pub fn contains(self, pt: Vec2) -> bool {
        pt.x >= self.min.x && pt.x <= self.max.x && pt.y >= self.min.y && pt.y <= self.max.y
    }

    #[inline]
    pub fn intersects(self, other: Self) -> bool {
        self.min.x <= other.max.x
            && self.max.x >= other.min.x
            && self.min.y <= other.max.y
            && self.max.y >= other.min.y
    }

    #[inline]
    pub fn shrink(self, margin: Margin) -> Self {
        Self::from_min_max(
            Vec2::new(self.min.x + margin.left, self.min.y + margin.top),
            Vec2::new(self.max.x - margin.right, self.max.y - margin.bottom),
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Margin {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

impl Margin {
    pub const ZERO: Self = Self {
        top: 0.0,
        right: 0.0,
        bottom: 0.0,
        left: 0.0,
    };

    #[inline]
    pub const fn same(val: f32) -> Self {
        Self {
            top: val,
            right: val,
            bottom: val,
            left: val,
        }
    }

    #[inline]
    pub const fn symmetric(horizontal: f32, vertical: f32) -> Self {
        Self {
            top: vertical,
            right: horizontal,
            bottom: vertical,
            left: horizontal,
        }
    }
}

pub type Padding = Margin;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct CornerRadius {
    pub top_left: f32,
    pub top_right: f32,
    pub bottom_right: f32,
    pub bottom_left: f32,
}

impl CornerRadius {
    pub const ZERO: Self = Self {
        top_left: 0.0,
        top_right: 0.0,
        bottom_right: 0.0,
        bottom_left: 0.0,
    };

    #[inline]
    pub const fn same(r: f32) -> Self {
        Self {
            top_left: r,
            top_right: r,
            bottom_right: r,
            bottom_left: r,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const TRANSPARENT: Self = Self {
        r: 0,
        g: 0,
        b: 0,
        a: 0,
    };
    pub const BLACK: Self = Self {
        r: 0,
        g: 0,
        b: 0,
        a: 255,
    };
    pub const WHITE: Self = Self {
        r: 255,
        g: 255,
        b: 255,
        a: 255,
    };

    #[inline]
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    #[inline]
    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    #[inline]
    pub fn with_alpha(self, a: u8) -> Self {
        Self {
            r: self.r,
            g: self.g,
            b: self.b,
            a,
        }
    }

    #[inline]
    pub fn to_f32_rgba(self) -> [f32; 4] {
        [
            self.r as f32 / 255.0,
            self.g as f32 / 255.0,
            self.b as f32 / 255.0,
            self.a as f32 / 255.0,
        ]
    }

    #[inline]
    pub fn to_u32(self) -> u32 {
        ((self.a as u32) << 24) | ((self.r as u32) << 16) | ((self.g as u32) << 8) | (self.b as u32)
    }

    /// Calculate relative luminance per WCAG 2.1 specs.
    pub fn luminance(self) -> f32 {
        fn channel(v: u8) -> f32 {
            let c = v as f32 / 255.0;
            if c <= 0.04045 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        }
        0.2126 * channel(self.r) + 0.7152 * channel(self.g) + 0.0722 * channel(self.b)
    }

    /// Contrast ratio against another color (WCAG 2.1 formula).
    pub fn contrast_ratio(self, other: Self) -> f32 {
        let l1 = self.luminance();
        let l2 = other.luminance();
        (l1.max(l2) + 0.05) / (l1.min(l2) + 0.05)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Stroke {
    pub width: f32,
    pub color: Color,
}

impl Stroke {
    pub const NONE: Self = Self {
        width: 0.0,
        color: Color::TRANSPARENT,
    };

    #[inline]
    pub const fn new(width: f32, color: Color) -> Self {
        Self { width, color }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rect_operations_are_safe_and_sane() {
        let r = Rect::from_min_size(Vec2::new(10.0, 20.0), Vec2::new(100.0, 50.0));
        assert_eq!(r.width(), 100.0);
        assert_eq!(r.height(), 50.0);
        assert!(r.contains(Vec2::new(50.0, 45.0)));
        assert!(!r.contains(Vec2::new(5.0, 45.0)));

        let bad = Rect::from_min_size(Vec2::new(f32::NAN, f32::INFINITY), Vec2::new(-50.0, -10.0));
        assert_eq!(bad.width(), 0.0);
        assert_eq!(bad.height(), 0.0);
    }

    #[test]
    fn contrast_ratio_complies_with_wcag() {
        let white = Color::WHITE;
        let black = Color::BLACK;
        assert!(white.contrast_ratio(black) > 20.0);
    }
}
