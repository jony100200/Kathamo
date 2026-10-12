//! Studio property inspector for fine-tuning object parameters.

use crate::controls::NumericScrubber;
use crate::theme::ThemePalette;
use ui_core::{CornerRadius, Rect, Stroke, Vec2};
use ui_render::{CommandBuffer, DrawCommand};

#[derive(Debug, Clone, PartialEq)]
pub struct PropertyInspector {
    pub object_name: String,
    pub pos_x: NumericScrubber,
    pub pos_y: NumericScrubber,
    pub scale: NumericScrubber,
    pub opacity: NumericScrubber,
}

impl Default for PropertyInspector {
    fn default() -> Self {
        Self {
            object_name: "Hero_Sprite_01".into(),
            pos_x: NumericScrubber::new("Pos X", 120.0, -1000.0, 1000.0, 1.0),
            pos_y: NumericScrubber::new("Pos Y", 85.0, -1000.0, 1000.0, 1.0),
            scale: NumericScrubber::new("Scale", 1.0, 0.1, 10.0, 0.05),
            opacity: NumericScrubber::new("Opacity", 1.0, 0.0, 1.0, 0.02),
        }
    }
}

impl PropertyInspector {
    pub fn render(&self, buf: &mut CommandBuffer, viewport: Rect, palette: &ThemePalette) {
        buf.push(DrawCommand::ClipPush { rect: viewport });

        // Background
        buf.push(DrawCommand::Rect {
            rect: viewport,
            fill: palette.surface,
            radius: CornerRadius::ZERO,
            stroke: Stroke::new(1.0, palette.border),
        });

        // Header
        let header_rect = Rect::from_min_size(viewport.min, Vec2::new(viewport.width(), 36.0));
        buf.push(DrawCommand::Rect {
            rect: header_rect,
            fill: palette.elevated,
            radius: CornerRadius::ZERO,
            stroke: Stroke::new(1.0, palette.border),
        });
        buf.push(DrawCommand::Text {
            text: format!("INSPECTOR: {}", self.object_name),
            pos: Vec2::new(header_rect.min.x + 12.0, header_rect.min.y + 11.0),
            size: 12.0,
            color: palette.text,
            strong: true,
        });

        // Properties section
        let mut y = viewport.min.y + 48.0;
        let scrubbers = [&self.pos_x, &self.pos_y, &self.scale, &self.opacity];

        for scrubber in scrubbers {
            let row_rect = Rect::from_min_size(
                Vec2::new(viewport.min.x + 12.0, y),
                Vec2::new(viewport.width() - 24.0, 28.0),
            );
            scrubber.render(buf, row_rect, false, palette);
            y += 36.0;
        }

        buf.push(DrawCommand::ClipPop);
    }
}
