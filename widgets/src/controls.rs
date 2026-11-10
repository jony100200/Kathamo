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
    render_button_pressed(buf, rect, label, style, is_hovered, false, palette);
}

/// [`render_button`] with a pressed state: while `is_pressed` the fill eases toward the
/// palette's pressed colour with the `motion::PRESS` spring.
pub fn render_button_pressed(
    buf: &mut CommandBuffer,
    rect: Rect,
    label: &str,
    style: ButtonStyle,
    is_hovered: bool,
    is_pressed: bool,
    palette: &ThemePalette,
) {
    let press = ui_core::motion::animate_bool(
        ui_core::motion::rect_key("button_press", rect),
        is_pressed,
        ui_core::motion::PRESS,
    );
    // Hover is a short spring between the rest and hover fills, not a binary flip.
    let t = ui_core::motion::animate_bool(
        ui_core::motion::rect_key("button", rect),
        is_hovered,
        ui_core::motion::HOVER,
    );
    // Flat, calm controls: a raised fill with a hairline border, the accent only for the main
    // action. No gloss: depth comes from the panel around them.
    let (rest, hover, stroke, text_color) = match style {
        ButtonStyle::Primary => (
            palette.accent,
            palette.accent_hover,
            Stroke::NONE,
            palette.canvas,
        ),
        ButtonStyle::Secondary => (
            palette.elevated,
            palette.elevated_high,
            Stroke::new(1.0, palette.border.lerp(palette.text_muted, t * 0.25)),
            palette.text,
        ),
        ButtonStyle::Ghost => (
            palette.surface.with_alpha(0),
            palette.surface,
            Stroke::NONE,
            palette.text_muted.lerp(palette.text, t),
        ),
        ButtonStyle::Danger => (
            palette.danger,
            palette.danger.lerp(ui_core::Color::WHITE, 0.12),
            Stroke::NONE,
            palette.text,
        ),
    };
    let states = palette.states();
    let pressed = match style {
        ButtonStyle::Primary => states.accent_pressed,
        ButtonStyle::Danger => palette.danger.lerp(palette.canvas, 0.18),
        ButtonStyle::Secondary | ButtonStyle::Ghost => states.pressed,
    };
    let fill = rest.lerp(hover, t).lerp(pressed, press);

    buf.push(DrawCommand::Rect {
        rect,
        fill,
        radius: CornerRadius::same(BUTTON_RADIUS),
        stroke,
    });

    // The label is centred; one that does not fit starts at the left padding and is cut
    // with an ellipsis.
    let strong = style == ButtonStyle::Primary;
    // Half a pixel of slack: width = text + padding, and float rounding must not cut it.
    let room = (rect.width() - 2.0 * BUTTON_PAD + 0.5).max(0.0);
    let shown = ui_render::fit_text(label, BUTTON_TEXT, strong, room);
    let w = ui_render::measure_text(&shown, BUTTON_TEXT, strong);
    let x = if w <= room {
        rect.center().x - w * 0.5
    } else {
        rect.min.x + BUTTON_PAD
    };
    buf.push(DrawCommand::Text {
        text: shown,
        pos: Vec2::new(x, rect.center().y - 8.0),
        size: BUTTON_TEXT,
        color: text_color,
        strong,
    });
}

/// Button label size, side padding and corner radius.
pub const BUTTON_TEXT: f32 = 12.5;
pub const BUTTON_PAD: f32 = 14.0;
pub const BUTTON_RADIUS: f32 = 6.0;

/// Width of a button whose label is `text` (plus `icon` extra pixels for a leading icon).
pub fn button_width(text: &str, strong: bool, icon: f32) -> f32 {
    (ui_render::measure_text(text, BUTTON_TEXT, strong) + 2.0 * BUTTON_PAD + icon).max(64.0)
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
    let width = (ui_render::measure_text(text, 11.5, false) + 16.0).max(48.0);
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
pub fn render_glass_card(buf: &mut CommandBuffer, rect: Rect, palette: &ThemePalette, radius: f32) {
    // The soft shadow and rim are analytical SDF passes, not opaque rectangles.
    buf.push(DrawCommand::SoftShadow {
        rect,
        radius: CornerRadius::same(radius),
        color: ui_core::Color::rgba(0, 0, 0, 85),
        offset: Vec2::new(0.0, 5.0),
        blur: 12.0,
        spread: 0.0,
    });
    buf.push(DrawCommand::Surface {
        rect,
        fill_top: palette.elevated.with_alpha(240),
        fill_bottom: palette.surface.with_alpha(225),
        radius: CornerRadius::same(radius),
        stroke: Stroke::new(1.0, palette.border.with_alpha(180)),
        rim_strength: 0.75,
    });
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
    fn glass_card_emits_shadow_then_lit_surface() {
        let mut buf = CommandBuffer::new();
        let palette = ThemePalette::dark();
        let rect = Rect::from_min_size(Vec2::ZERO, Vec2::new(200.0, 100.0));
        render_glass_card(&mut buf, rect, &palette, 8.0);
        assert_eq!(buf.len(), 2);
        assert!(matches!(buf.commands()[0], DrawCommand::SoftShadow { .. }));
        assert!(matches!(buf.commands()[1], DrawCommand::Surface { .. }));
    }

    #[test]
    fn status_pill_emits_container_dot_and_text() {
        let mut buf = CommandBuffer::new();
        let palette = ThemePalette::dark();
        render_status_pill(&mut buf, Vec2::ZERO, "Ready", palette.success, &palette);
        assert_eq!(buf.len(), 3);
    }
}
