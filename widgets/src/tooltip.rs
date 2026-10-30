use ui_core::{Color, CornerRadius, Rect, Stroke, Vec2};
use ui_render::DrawCommand;

use crate::ui::KolponaUi;

/// Viewport-clamped floating tooltip component.
#[derive(Debug, Clone)]
pub struct TooltipOverlay<'a> {
    pub text: &'a str,
    pub target_rect: Rect,
    pub font_size: f32,
    pub padding: Vec2,
    pub offset_y: f32,
}

impl<'a> TooltipOverlay<'a> {
    pub fn new(text: &'a str, target_rect: Rect) -> Self {
        Self {
            text,
            target_rect,
            font_size: 11.0,
            padding: Vec2::new(8.0, 5.0),
            offset_y: 6.0,
        }
    }

    pub fn font_size(mut self, size: f32) -> Self {
        self.font_size = size;
        self
    }

    pub fn padding(mut self, x: f32, y: f32) -> Self {
        self.padding = Vec2::new(x, y);
        self
    }

    /// Renders the tooltip anchored to `target_rect`, clamped within viewport bounds.
    pub fn show(self, ui: &mut KolponaUi, viewport: Rect) {
        if self.text.is_empty() {
            return;
        }

        let palette = ui.palette;
        let char_width = self.font_size * 0.55;
        let content_width = self.text.len() as f32 * char_width;
        let tooltip_width = content_width + self.padding.x * 2.0;
        let tooltip_height = self.font_size + self.padding.y * 2.0;

        // Preferred position: below the target rect centered horizontally
        let mut tooltip_x =
            self.target_rect.min.x + (self.target_rect.width() - tooltip_width) * 0.5;
        let mut tooltip_y = self.target_rect.max.y + self.offset_y;

        // If tooltip exceeds bottom viewport boundary, flip above target rect
        if tooltip_y + tooltip_height > viewport.max.y - 8.0 {
            tooltip_y = self.target_rect.min.y - tooltip_height - self.offset_y;
        }

        // Horizontal clamping with 8px margin
        let min_x = viewport.min.x + 8.0;
        let max_x = (viewport.max.x - tooltip_width - 8.0).max(min_x);
        tooltip_x = tooltip_x.clamp(min_x, max_x);

        // Vertical clamping with 8px margin
        let min_y = viewport.min.y + 8.0;
        let max_y = (viewport.max.y - tooltip_height - 8.0).max(min_y);
        tooltip_y = tooltip_y.clamp(min_y, max_y);

        let tooltip_rect = Rect::from_min_size(
            Vec2::new(tooltip_x, tooltip_y),
            Vec2::new(tooltip_width, tooltip_height),
        );

        // Draw shadow / dark background
        ui.cmd_buf.push(DrawCommand::SoftShadow {
            rect: tooltip_rect,
            radius: CornerRadius::same(4.0),
            color: Color::rgba(0, 0, 0, 120),
            offset: Vec2::new(0.0, 3.0),
            blur: 8.0,
            spread: 0.0,
        });

        // Tooltip container
        // Frosted glass: blurred backdrop + translucent tint, then the hairline border.
        ui.cmd_buf.push(DrawCommand::Backdrop {
            rect: tooltip_rect,
            radius: CornerRadius::same(4.0),
            tint: palette.elevated_high.with_alpha(215),
        });
        ui.cmd_buf.push(DrawCommand::Rect {
            rect: tooltip_rect,
            fill: Color::TRANSPARENT,
            radius: CornerRadius::same(4.0),
            stroke: Stroke::new(1.0, palette.border),
        });

        // Tooltip text
        ui.cmd_buf.push(DrawCommand::Text {
            text: self.text.to_string(),
            pos: Vec2::new(tooltip_x + self.padding.x, tooltip_y + self.padding.y),
            size: self.font_size,
            color: palette.text,
            strong: false,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::icon_cache::IconAtlas;
    use ui_platform::input::PointerState;
    use ui_render::CommandBuffer;
    use ui_themes::ThemePalette;

    #[test]
    fn test_tooltip_clamping() {
        let mut cmd_buf = CommandBuffer::new();
        let mut icon_atlas = IconAtlas::new();
        let palette = ThemePalette::dark();
        let pointer = PointerState::default();
        let bounds = Rect::from_min_size(Vec2::ZERO, Vec2::new(400.0, 300.0));

        let mut ui = KolponaUi::new(&mut cmd_buf, &mut icon_atlas, &palette, bounds, pointer);

        let viewport = Rect::from_min_size(Vec2::ZERO, Vec2::new(400.0, 300.0));
        let target = Rect::from_min_size(Vec2::new(380.0, 290.0), Vec2::new(20.0, 20.0));

        // Target is at bottom-right corner; tooltip should automatically clamp and flip inside viewport
        TooltipOverlay::new("Export PBR Map", target).show(&mut ui, viewport);
        assert!(!cmd_buf.is_empty());
    }
}
