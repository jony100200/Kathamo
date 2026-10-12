//! Jetpack Compose inspired Modifier pattern for fluent UI construction.

use crate::anchor::{Anchors, Offsets};
use crate::layout::SizeConstraint;
use crate::math::{Color, Margin, Padding};

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Modifier {
    pub width: SizeConstraint,
    pub height: SizeConstraint,
    pub padding: Padding,
    pub margin: Margin,
    pub anchors: Option<(Anchors, Offsets)>,
    pub background: Option<Color>,
}

impl Modifier {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn width(mut self, constraint: SizeConstraint) -> Self {
        self.width = constraint;
        self
    }

    pub fn height(mut self, constraint: SizeConstraint) -> Self {
        self.height = constraint;
        self
    }

    pub fn fixed_size(mut self, width: f32, height: f32) -> Self {
        self.width = SizeConstraint::Fixed(width);
        self.height = SizeConstraint::Fixed(height);
        self
    }

    pub fn fill_width(mut self) -> Self {
        self.width = SizeConstraint::Flex(1.0);
        self
    }

    pub fn fill_height(mut self) -> Self {
        self.height = SizeConstraint::Flex(1.0);
        self
    }

    pub fn fill_max_size(mut self) -> Self {
        self.width = SizeConstraint::Flex(1.0);
        self.height = SizeConstraint::Flex(1.0);
        self
    }

    pub fn padding(mut self, padding: Padding) -> Self {
        self.padding = padding;
        self
    }

    pub fn margin(mut self, margin: Margin) -> Self {
        self.margin = margin;
        self
    }

    pub fn background(mut self, color: Color) -> Self {
        self.background = Some(color);
        self
    }

    pub fn anchor(mut self, anchors: Anchors, offsets: Offsets) -> Self {
        self.anchors = Some((anchors, offsets));
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modifier_chains_compose_correctly() {
        let m = Modifier::empty()
            .fill_width()
            .height(SizeConstraint::Fixed(48.0))
            .padding(Padding::symmetric(16.0, 8.0))
            .background(Color::BLACK);

        assert_eq!(m.width, SizeConstraint::Flex(1.0));
        assert_eq!(m.height, SizeConstraint::Fixed(48.0));
        assert_eq!(m.padding, Padding::symmetric(16.0, 8.0));
        assert_eq!(m.background, Some(Color::BLACK));
    }
}
