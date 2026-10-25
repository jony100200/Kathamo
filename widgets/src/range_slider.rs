use ui_core::{Color, CornerRadius, Rect, Stroke, Vec2};
use ui_render::DrawCommand;

use crate::ui::{KolponaUi, Response};

/// Dual-thumb range slider for selecting min and max values within a bounded domain.
#[derive(Debug, Clone)]
pub struct RangeSlider<'a> {
    pub label: Option<&'a str>,
    pub min_value: f32,
    pub max_value: f32,
    pub range_min: f32,
    pub range_max: f32,
    pub step: Option<f32>,
    pub show_values: bool,
}

impl<'a> RangeSlider<'a> {
    pub fn new(min_value: f32, max_value: f32, range_min: f32, range_max: f32) -> Self {
        Self {
            label: None,
            min_value: min_value.clamp(range_min, range_max),
            max_value: max_value.clamp(range_min, range_max),
            range_min,
            range_max,
            step: None,
            show_values: true,
        }
    }

    pub fn label(mut self, label: &'a str) -> Self {
        self.label = Some(label);
        self
    }

    pub fn step(mut self, step: f32) -> Self {
        if step > 0.0 {
            self.step = Some(step);
        }
        self
    }

    pub fn show_values(mut self, show: bool) -> Self {
        self.show_values = show;
        self
    }

    /// Renders the range slider onto the given UI builder.
    pub fn show(
        self,
        ui: &mut KolponaUi,
        current_min: &mut f32,
        current_max: &mut f32,
    ) -> Response {
        let palette = ui.palette;
        let height = 28.0;
        let width = ui.bounds.width() - 16.0;
        let rect = Rect::from_min_size(ui.cursor, Vec2::new(width, height));
        let hovered = rect.contains(ui.pointer.position);

        let font_size = 12.0;
        let track_height = 6.0;
        let thumb_radius = 8.0;

        let label_width = if self.label.is_some() { 70.0 } else { 0.0 };
        let value_width = if self.show_values { 90.0 } else { 0.0 };

        let track_x = rect.min.x + label_width + thumb_radius;
        let track_width =
            (rect.width() - label_width - value_width - (thumb_radius * 2.0)).max(20.0);
        let track_y = rect.min.y + (rect.height() - track_height) * 0.5;
        let track_rect = Rect::from_min_size(
            Vec2::new(track_x, track_y),
            Vec2::new(track_width, track_height),
        );

        // Render Label
        if let Some(lbl) = self.label {
            ui.cmd_buf.push(DrawCommand::Text {
                text: lbl.to_string(),
                pos: Vec2::new(rect.min.x, rect.min.y + (rect.height() - font_size) * 0.5),
                size: font_size,
                color: palette.text_muted,
                strong: false,
            });
        }

        let range_span = (self.range_max - self.range_min).max(0.0001);
        let norm_min = ((*current_min - self.range_min) / range_span).clamp(0.0, 1.0);
        let norm_max = ((*current_max - self.range_min) / range_span).clamp(0.0, 1.0);

        let min_thumb_center_x = track_x + norm_min * track_width;
        let max_thumb_center_x = track_x + norm_max * track_width;
        let thumb_center_y = track_y + track_height * 0.5;

        let min_thumb_rect = Rect::from_min_size(
            Vec2::new(
                min_thumb_center_x - thumb_radius,
                thumb_center_y - thumb_radius,
            ),
            Vec2::new(thumb_radius * 2.0, thumb_radius * 2.0),
        );
        let max_thumb_rect = Rect::from_min_size(
            Vec2::new(
                max_thumb_center_x - thumb_radius,
                thumb_center_y - thumb_radius,
            ),
            Vec2::new(thumb_radius * 2.0, thumb_radius * 2.0),
        );

        let mut changed = false;
        let mut dragging = false;

        // Interaction logic
        if ui.pointer.primary_down && hovered {
            let mouse_x = ui.pointer.position.x;
            let target_norm = ((mouse_x - track_x) / track_width).clamp(0.0, 1.0);
            let mut new_val = self.range_min + target_norm * range_span;

            if let Some(st) = self.step {
                new_val = (new_val / st).round() * st;
                new_val = new_val.clamp(self.range_min, self.range_max);
            }

            let dist_to_min = (mouse_x - min_thumb_center_x).abs();
            let dist_to_max = (mouse_x - max_thumb_center_x).abs();

            if dist_to_min < dist_to_max {
                let clamped = new_val.min(*current_max);
                if (*current_min - clamped).abs() > 0.0001 {
                    *current_min = clamped;
                    changed = true;
                }
            } else {
                let clamped = new_val.max(*current_min);
                if (*current_max - clamped).abs() > 0.0001 {
                    *current_max = clamped;
                    changed = true;
                }
            }
            dragging = true;
        }

        // Draw track background
        ui.cmd_buf.push(DrawCommand::Rect {
            rect: track_rect,
            fill: palette.elevated,
            radius: CornerRadius::same(track_height * 0.5),
            stroke: Stroke::NONE,
        });

        // Draw active fill between thumbs
        let fill_x = track_x + norm_min * track_width;
        let fill_width = ((norm_max - norm_min) * track_width).max(0.0);
        let fill_rect = Rect::from_min_size(
            Vec2::new(fill_x, track_y),
            Vec2::new(fill_width, track_height),
        );
        ui.cmd_buf.push(DrawCommand::Rect {
            rect: fill_rect,
            fill: palette.accent,
            radius: CornerRadius::same(track_height * 0.5),
            stroke: Stroke::NONE,
        });

        // Draw Min Thumb
        let min_thumb_hovered = min_thumb_rect.contains(ui.pointer.position);
        let min_thumb_color = if min_thumb_hovered || dragging {
            Color::WHITE
        } else {
            palette.text
        };
        ui.cmd_buf.push(DrawCommand::Rect {
            rect: min_thumb_rect,
            fill: min_thumb_color,
            radius: CornerRadius::same(thumb_radius),
            stroke: Stroke::new(1.5, palette.border),
        });

        // Draw Max Thumb
        let max_thumb_hovered = max_thumb_rect.contains(ui.pointer.position);
        let max_thumb_color = if max_thumb_hovered || dragging {
            Color::WHITE
        } else {
            palette.text
        };
        ui.cmd_buf.push(DrawCommand::Rect {
            rect: max_thumb_rect,
            fill: max_thumb_color,
            radius: CornerRadius::same(thumb_radius),
            stroke: Stroke::new(1.5, palette.border),
        });

        // Value display text
        if self.show_values {
            let val_str = format!("{:.1} - {:.1}", *current_min, *current_max);
            let val_x = rect.min.x + rect.width() - value_width;
            ui.cmd_buf.push(DrawCommand::Text {
                text: val_str,
                pos: Vec2::new(val_x, rect.min.y + (rect.height() - font_size) * 0.5),
                size: font_size,
                color: palette.text_muted,
                strong: false,
            });
        }

        ui.advance(width, height);

        let clicked = ui.is_clicked_in_rect(rect);
        Response {
            clicked,
            hovered,
            changed,
            dragging,
            activated: clicked,
        }
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
    fn test_range_slider_clamping_and_render() {
        let mut cmd_buf = CommandBuffer::new();
        let mut icon_atlas = IconAtlas::new();
        let palette = ThemePalette::dark();
        let pointer = PointerState::default();
        let bounds = Rect::from_min_size(Vec2::ZERO, Vec2::new(400.0, 300.0));

        let mut ui = KolponaUi::new(&mut cmd_buf, &mut icon_atlas, &palette, bounds, pointer);

        let mut min_val = 10.0;
        let mut max_val = 80.0;

        let resp = RangeSlider::new(min_val, max_val, 0.0, 100.0)
            .label("Domain")
            .show_values(true)
            .show(&mut ui, &mut min_val, &mut max_val);

        assert_eq!(resp.changed, false);
        assert!(min_val < max_val);
        assert!(!cmd_buf.is_empty());
    }
}
