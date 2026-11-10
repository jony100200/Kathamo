//! A small role-based type scale for app chrome: each role pairs a size with a typeface,
//! so screens ask for "a title" or "a value" instead of picking pixel sizes by hand.
//!
//! Sizes sit on whole pixels and step by a steady ratio, so text lines up on the 4 px
//! spacing grid. Numbers use the monospace face, which has tabular (equal-width) digits,
//! so values do not jitter while they change.

use ui_core::{Color, Vec2};
use ui_render::{measure_face, CommandBuffer, DrawCommand, Face};

/// What a piece of text is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TypeRole {
    /// Brand marks and hero titles (Space Grotesk Bold).
    Display,
    /// Screen and card titles (Inter Bold).
    Title,
    /// Panel and section headings (Inter Bold).
    Section,
    /// Running text, list rows and descriptions (Inter Regular).
    Body,
    /// Control labels: buttons, tabs, fields (Inter Medium).
    Label,
    /// Secondary notes, hints and metadata (Inter Regular).
    Caption,
    /// Values and readouts with tabular digits (JetBrains Mono).
    Numeric,
}

/// A size, face and letter spacing for one role.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextStyle {
    pub size: f32,
    pub face: Face,
    /// Extra logical pixels after each glyph.
    pub tracking: f32,
}

impl TypeRole {
    /// Every role, largest first.
    pub const ALL: [TypeRole; 7] = [
        TypeRole::Display,
        TypeRole::Title,
        TypeRole::Section,
        TypeRole::Body,
        TypeRole::Label,
        TypeRole::Caption,
        TypeRole::Numeric,
    ];

    /// The default style for this role.
    pub const fn style(self) -> TextStyle {
        let (size, face) = match self {
            TypeRole::Display => (28.0, Face::Display),
            TypeRole::Title => (18.0, Face::Bold),
            TypeRole::Section => (14.0, Face::Bold),
            TypeRole::Body => (13.0, Face::Regular),
            TypeRole::Label => (13.0, Face::Medium),
            TypeRole::Caption => (11.0, Face::Regular),
            TypeRole::Numeric => (12.0, Face::Mono),
        };
        TextStyle {
            size,
            face,
            tracking: 0.0,
        }
    }
}

impl TextStyle {
    /// The same style at another size.
    pub const fn at(self, size: f32) -> Self {
        Self {
            size,
            face: self.face,
            tracking: self.tracking,
        }
    }

    /// Width of `text` in this style.
    pub fn measure(&self, text: &str) -> f32 {
        let glyphs = text.chars().count().saturating_sub(1) as f32;
        measure_face(text, self.size, self.face) + self.tracking * glyphs
    }

    /// Top offset that centres one line of this style in a box of `height`.
    pub fn centre_offset(&self, height: f32) -> f32 {
        ((height - self.size * 1.28) * 0.5).round()
    }

    /// Pushes `text` at `pos` (top-left of the line box).
    pub fn draw(&self, buf: &mut CommandBuffer, text: &str, pos: Vec2, color: Color) {
        buf.push(DrawCommand::StyledText {
            text: text.to_owned(),
            pos,
            size: self.size,
            color,
            face: self.face,
            tracking: self.tracking,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_scale_steps_down_and_uses_whole_pixels() {
        let sizes: Vec<f32> = TypeRole::ALL[..6].iter().map(|r| r.style().size).collect();
        for pair in sizes.windows(2) {
            assert!(pair[0] >= pair[1], "{sizes:?}");
        }
        for role in TypeRole::ALL {
            let s = role.style().size;
            assert_eq!(s, s.round(), "{role:?}");
            assert!(s >= 11.0, "{role:?} is too small to read");
        }
        assert_eq!(TypeRole::Numeric.style().face, Face::Mono);
    }

    #[test]
    fn draw_pushes_styled_text_in_the_role_face() {
        let mut buf = CommandBuffer::new();
        TypeRole::Title
            .style()
            .draw(&mut buf, "Hello", Vec2::ZERO, Color::WHITE);
        assert!(matches!(
            &buf.commands()[0],
            DrawCommand::StyledText { face: Face::Bold, size, .. } if *size == 18.0
        ));
        assert!(TypeRole::Title.style().measure("Hello") > 0.0);
        assert_eq!(TypeRole::Body.style().at(15.0).size, 15.0);
    }
}
