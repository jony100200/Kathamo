//! Studio StyleBox and State Layer rendering primitive.

use ui_core::{Color, CornerRadius, Rect, Stroke, Vec2};
use ui_render::{CommandBuffer, DrawCommand};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InteractionState {
    #[default]
    Default,
    Hovered,
    Pressed,
    Focused,
    Disabled,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Shadow {
    pub offset: Vec2,
    pub blur: f32,
    pub color: Color,
}

impl Default for Shadow {
    fn default() -> Self {
        Self {
            offset: Vec2::new(0.0, 4.0),
            blur: 12.0,
            color: Color::BLACK.with_alpha(70),
        }
    }
}

/// A comprehensive visual box styling model.
#[derive(Debug, Clone, PartialEq)]
pub struct StyleBox {
    pub bg_color: Color,
    pub gradient_end: Option<Color>,
    pub rim_strength: f32,
    pub border: Stroke,
    pub corner_radius: CornerRadius,
    pub shadow: Option<Shadow>,
}

impl Default for StyleBox {
    fn default() -> Self {
        Self {
            bg_color: Color::rgb(25, 27, 28),
            gradient_end: None,
            rim_strength: 0.0,
            border: Stroke::new(1.0, Color::rgb(65, 69, 70)),
            corner_radius: CornerRadius::same(6.0),
            shadow: None,
        }
    }
}

impl StyleBox {
    pub fn new(bg: Color) -> Self {
        Self {
            bg_color: bg,
            gradient_end: None,
            rim_strength: 0.0,
            border: Stroke::NONE,
            corner_radius: CornerRadius::ZERO,
            shadow: None,
        }
    }

    pub fn with_border(mut self, width: f32, color: Color) -> Self {
        self.border = Stroke::new(width, color);
        self
    }

    pub fn with_radius(mut self, radius: f32) -> Self {
        self.corner_radius = CornerRadius::same(radius);
        self
    }

    /// Opt in to an analytical vertical gradient without creating an image texture.
    pub fn with_gradient(mut self, bottom: Color) -> Self {
        self.gradient_end = Some(bottom);
        self
    }

    pub fn with_rim(mut self, strength: f32) -> Self {
        self.rim_strength = strength.clamp(0.0, 1.0);
        self
    }

    pub fn with_shadow(mut self, shadow: Shadow) -> Self {
        self.shadow = Some(shadow);
        self
    }

    /// Renders the StyleBox with Android Material 3 State Layer tints applied mathematically.
    pub fn render(
        &self,
        buf: &mut CommandBuffer,
        rect: Rect,
        state: InteractionState,
        accent_color: Color,
    ) {
        // A feathered SDF shadow; no separately blurred texture.
        if let Some(shadow) = &self.shadow {
            buf.push(DrawCommand::SoftShadow {
                rect,
                radius: self.corner_radius,
                color: shadow.color,
                offset: shadow.offset,
                blur: shadow.blur,
                spread: 0.0,
            });
        }

        // 2. Compute fill color with Material 3 state layer alpha tinting
        let fill = match state {
            InteractionState::Default => self.bg_color,
            InteractionState::Hovered => {
                // +8% white overlay tint
                Color::rgba(
                    self.bg_color.r.saturating_add(20),
                    self.bg_color.g.saturating_add(20),
                    self.bg_color.b.saturating_add(25),
                    self.bg_color.a,
                )
            }
            InteractionState::Pressed => {
                // +14% white overlay tint
                Color::rgba(
                    self.bg_color.r.saturating_add(35),
                    self.bg_color.g.saturating_add(35),
                    self.bg_color.b.saturating_add(45),
                    self.bg_color.a,
                )
            }
            InteractionState::Focused => self.bg_color,
            InteractionState::Disabled => {
                // 38% opacity disabled state
                self.bg_color
                    .with_alpha(((self.bg_color.a as f32) * 0.38) as u8)
            }
        };

        // 3. Compute stroke (2px accent outline on focus)
        let stroke = match state {
            InteractionState::Focused => Stroke::new(2.0, accent_color),
            InteractionState::Disabled => {
                Stroke::new(self.border.width, self.border.color.with_alpha(60))
            }
            _ => self.border,
        };

        if self.gradient_end.is_some() || self.rim_strength > 0.0 {
            let bottom = self.gradient_end.unwrap_or(fill);
            buf.push(DrawCommand::Surface {
                rect,
                fill_top: fill,
                fill_bottom: bottom,
                radius: self.corner_radius,
                stroke,
                rim_strength: self.rim_strength,
            });
        } else {
            buf.push(DrawCommand::Rect {
                rect,
                fill,
                radius: self.corner_radius,
                stroke,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn styled_surface_emits_shadow_then_one_sdf_surface() {
        let box_style = StyleBox::new(Color::rgb(46, 45, 43))
            .with_radius(12.0)
            .with_gradient(Color::rgb(26, 26, 25))
            .with_rim(0.6)
            .with_shadow(Shadow::default());
        let mut buf = CommandBuffer::new();
        box_style.render(
            &mut buf,
            Rect::from_min_size(Vec2::ZERO, Vec2::new(160.0, 90.0)),
            InteractionState::Default,
            Color::WHITE,
        );
        assert_eq!(buf.len(), 2);
        assert!(matches!(buf.commands()[0], DrawCommand::SoftShadow { .. }));
        assert!(matches!(buf.commands()[1], DrawCommand::Surface { .. }));
    }

    #[test]
    fn stylebox_renders_state_layers_accurately() {
        let sb = StyleBox::new(Color::rgb(20, 20, 20))
            .with_border(1.0, Color::rgb(50, 50, 50))
            .with_radius(8.0);

        let mut buf = CommandBuffer::new();
        let rect = Rect::from_min_size(Vec2::ZERO, Vec2::new(100.0, 40.0));
        sb.render(&mut buf, rect, InteractionState::Hovered, Color::WHITE);

        assert_eq!(buf.len(), 1);
        if let DrawCommand::Rect { fill, .. } = &buf.commands()[0] {
            assert!(fill.r > 20, "Hover state must increase luminance");
        } else {
            panic!("Expected DrawCommand::Rect");
        }
    }
}
