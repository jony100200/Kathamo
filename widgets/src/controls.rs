//! Interactive studio controls: buttons, badges, and numeric drag-scrubbers.

use crate::theme::ThemePalette;
use ui_core::{CornerRadius, Rect, Stroke, Vec2};
use ui_render::{CommandBuffer, DrawCommand};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonStyle {
    Primary,
    Secondary,
    Ghost,
    Danger,
}

pub fn render_button(
    buf: &mut CommandBuffer,
    rect: Rect,
    label: &str,
    style: ButtonStyle,
    is_hovered: bool,
    palette: &ThemePalette,
) {
    let (fill, stroke, text_color) = match style {
        ButtonStyle::Primary => (
            if is_hovered {
                palette.accent_hover
            } else {
                palette.accent
            },
            Stroke::NONE,
            palette.canvas,
        ),
        ButtonStyle::Secondary => (
            if is_hovered {
                palette.elevated
            } else {
                palette.surface
            },
            Stroke::new(1.0, palette.border),
            palette.text,
        ),
        ButtonStyle::Ghost => (
            if is_hovered {
                palette.surface
            } else {
                ui_core::Color::TRANSPARENT
            },
            Stroke::NONE,
            palette.text_muted,
        ),
        ButtonStyle::Danger => (palette.danger, Stroke::NONE, palette.text),
    };

    buf.push(DrawCommand::Rect {
        rect,
        fill,
        radius: CornerRadius::same(6.0),
        stroke,
    });

    buf.push(DrawCommand::Text {
        text: label.to_string(),
        pos: Vec2::new(rect.min.x + 12.0, rect.min.y + 8.0),
        size: 13.0,
        color: text_color,
        strong: style == ButtonStyle::Primary,
    });
}

pub fn render_badge(
    buf: &mut CommandBuffer,
    pos: Vec2,
    text: &str,
    is_success: bool,
    palette: &ThemePalette,
) {
    let color = if is_success {
        palette.success
    } else {
        palette.warning
    };
    let width = (text.len() as f32 * 7.5 + 16.0).max(60.0);
    let rect = Rect::from_min_size(pos, Vec2::new(width, 22.0));

    buf.push(DrawCommand::Rect {
        rect,
        fill: color.with_alpha(35),
        radius: CornerRadius::same(4.0),
        stroke: Stroke::new(1.0, color),
    });

    buf.push(DrawCommand::Text {
        text: text.to_string(),
        pos: Vec2::new(rect.min.x + 6.0, rect.min.y + 4.0),
        size: 11.0,
        color,
        strong: true,
    });
}

/// Draggable numeric scrubber (scrubs value continuously when dragged).
#[derive(Debug, Clone, PartialEq)]
pub struct NumericScrubber {
    pub label: String,
    pub value: f32,
    pub min: f32,
    pub max: f32,
    pub step: f32,
}

impl NumericScrubber {
    pub fn new(label: impl Into<String>, value: f32, min: f32, max: f32, step: f32) -> Self {
        Self {
            label: label.into(),
            value: value.clamp(min, max),
            min,
            max,
            step,
        }
    }

    pub fn drag_by(&mut self, delta_x: f32) {
        self.value = (self.value + delta_x * self.step).clamp(self.min, self.max);
    }

    pub fn render(
        &self,
        buf: &mut CommandBuffer,
        rect: Rect,
        is_hovered: bool,
        palette: &ThemePalette,
    ) {
        let fill = if is_hovered {
            palette.elevated
        } else {
            palette.surface
        };

        buf.push(DrawCommand::Rect {
            rect,
            fill,
            radius: CornerRadius::same(4.0),
            stroke: Stroke::new(1.0, palette.border),
        });

        // Value proportion bar
        let range = (self.max - self.min).max(0.001);
        let ratio = ((self.value - self.min) / range).clamp(0.0, 1.0);
        let bar_rect =
            Rect::from_min_size(rect.min, Vec2::new(rect.width() * ratio, rect.height()));
        buf.push(DrawCommand::Rect {
            rect: bar_rect,
            fill: palette.accent.with_alpha(45),
            radius: CornerRadius::same(4.0),
            stroke: Stroke::NONE,
        });

        buf.push(DrawCommand::Text {
            text: format!("{}: {:.2}", self.label, self.value),
            pos: Vec2::new(rect.min.x + 8.0, rect.min.y + 6.0),
            size: 12.0,
            color: palette.text,
            strong: false,
        });
    }
}

/// Renders a modern glassmorphic card container with dual-stroke lighting elevation.
/// Inspired by Linear & Raycast UI:
/// - 1px hairline specular top highlight (simulating subtle studio downlight)
/// - Translucent backdrop tint
/// - Ambient soft drop shadow
pub fn render_glass_card(
    buf: &mut CommandBuffer,
    rect: Rect,
    palette: &ThemePalette,
    radius: f32,
) {
    // 1. Ambient drop shadow
    let shadow_rect = Rect::from_min_max(
        rect.min + Vec2::new(0.0, 4.0),
        rect.max + Vec2::new(0.0, 8.0),
    );
    buf.push(DrawCommand::Rect {
        rect: shadow_rect,
        fill: ui_core::Color::rgba(0, 0, 0, 45),
        radius: CornerRadius::same(radius + 2.0),
        stroke: Stroke::NONE,
    });

    // 2. Glassmorphic translucent body
    buf.push(DrawCommand::Rect {
        rect,
        fill: palette.surface.with_alpha(215),
        radius: CornerRadius::same(radius),
        stroke: Stroke::new(1.0, palette.border.with_alpha(140)),
    });

    // 3. Dual-stroke specular highlight: 1px hairline top rim
    if rect.width() > 16.0 {
        let highlight_rect = Rect::from_min_size(
            Vec2::new(rect.min.x + radius.min(8.0), rect.min.y),
            Vec2::new((rect.width() - (radius.min(8.0) * 2.0)).max(1.0), 1.0),
        );
        buf.push(DrawCommand::Rect {
            rect: highlight_rect,
            fill: ui_core::Color::rgba(255, 255, 255, 30),
            radius: CornerRadius::ZERO,
            stroke: Stroke::NONE,
        });
    }
}

/// Renders a status pill badge with a live glowing/solid indicator dot.
pub fn render_status_pill(
    buf: &mut CommandBuffer,
    pos: Vec2,
    text: &str,
    dot_color: ui_core::Color,
    palette: &ThemePalette,
) {
    let width = (text.len() as f32 * 7.0 + 28.0).max(54.0);
    let rect = Rect::from_min_size(pos, Vec2::new(width, 22.0));

    // Pill container
    buf.push(DrawCommand::Rect {
        rect,
        fill: palette.surface.with_alpha(180),
        radius: CornerRadius::same(11.0),
        stroke: Stroke::new(1.0, palette.border.with_alpha(160)),
    });

    // Glowing/solid indicator dot (6px x 6px)
    let dot_center = Vec2::new(rect.min.x + 10.0, rect.min.y + 11.0);
    let dot_rect = Rect::from_min_size(dot_center - Vec2::splat(3.0), Vec2::splat(6.0));
    buf.push(DrawCommand::Rect {
        rect: dot_rect,
        fill: dot_color,
        radius: CornerRadius::same(3.0),
        stroke: Stroke::NONE,
    });

    // Label
    buf.push(DrawCommand::Text {
        text: text.to_string(),
        pos: Vec2::new(rect.min.x + 18.0, rect.min.y + 4.5),
        size: 11.0,
        color: palette.text,
        strong: false,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glass_card_emits_shadow_body_and_specular_highlight() {
        let mut buf = CommandBuffer::new();
        let palette = ThemePalette::dark();
        let rect = Rect::from_min_size(Vec2::ZERO, Vec2::new(200.0, 100.0));
        render_glass_card(&mut buf, rect, &palette, 8.0);
        assert_eq!(buf.len(), 3);
    }

    #[test]
    fn status_pill_emits_container_dot_and_text() {
        let mut buf = CommandBuffer::new();
        let palette = ThemePalette::dark();
        render_status_pill(&mut buf, Vec2::ZERO, "Ready", palette.success, &palette);
        assert_eq!(buf.len(), 3);
    }
}

