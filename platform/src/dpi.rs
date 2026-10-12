//! DPI scaling and unit conversions between logical and physical pixels.

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DpiScale {
    factor: f32,
}

impl Default for DpiScale {
    fn default() -> Self {
        Self { factor: 1.0 }
    }
}

impl DpiScale {
    pub fn new(factor: f32) -> Self {
        let f = if factor.is_finite() && factor > 0.1 {
            factor
        } else {
            1.0
        };
        Self { factor: f }
    }

    pub fn factor(&self) -> f32 {
        self.factor
    }

    #[inline]
    pub fn logical_to_physical(&self, logical: f32) -> f32 {
        logical * self.factor
    }

    #[inline]
    pub fn physical_to_logical(&self, physical: f32) -> f32 {
        physical / self.factor
    }

    #[inline]
    pub fn scale_vec2(&self, v: ui_core::Vec2) -> ui_core::Vec2 {
        ui_core::Vec2::new(v.x * self.factor, v.y * self.factor)
    }

    #[inline]
    pub fn unscale_vec2(&self, v: ui_core::Vec2) -> ui_core::Vec2 {
        ui_core::Vec2::new(v.x / self.factor, v.y / self.factor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dpi_scale_converts_correctly() {
        let scale = DpiScale::new(1.5);
        assert_eq!(scale.logical_to_physical(100.0), 150.0);
        assert_eq!(scale.physical_to_logical(150.0), 100.0);
    }
}
