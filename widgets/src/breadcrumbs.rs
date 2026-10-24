use ui_core::{CornerRadius, Rect, Stroke, Vec2};
use ui_render::DrawCommand;

use crate::ui::{KolponaUi, Response};

/// Breadcrumb navigation item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BreadcrumbItem<'a> {
    pub label: &'a str,
    pub id: Option<&'a str>,
}

impl<'a> BreadcrumbItem<'a> {
    pub fn new(label: &'a str) -> Self {
        Self { label, id: None }
    }

    pub fn with_id(label: &'a str, id: &'a str) -> Self {
        Self {
            label,
            id: Some(id),
        }
    }
}

/// Hierarchical Breadcrumbs navigation component.
#[derive(Debug, Clone)]
pub struct Breadcrumbs<'a> {
    pub items: &'a [BreadcrumbItem<'a>],
    pub separator: &'a str,
    pub font_size: f32,
}

impl<'a> Breadcrumbs<'a> {
    pub fn new(items: &'a [BreadcrumbItem<'a>]) -> Self {
        Self {
            items,
            separator: "/",
            font_size: 13.0,
        }
    }

    pub fn separator(mut self, sep: &'a str) -> Self {
        self.separator = sep;
        self
    }

    pub fn font_size(mut self, size: f32) -> Self {
        self.font_size = size;
        self
    }

    /// Renders the breadcrumbs and returns the index of any clicked breadcrumb item.
    pub fn show(self, ui: &mut KolponaUi) -> (Response, Option<usize>) {
        let palette = ui.palette;
        let height = 24.0;
        let width = ui.bounds.width() - 16.0;
        let rect = Rect::from_min_size(ui.cursor, Vec2::new(width, height));

        let mut current_x = rect.min.x;
        let char_width = self.font_size * 0.55;
        let sep_width = (self.separator.len() as f32 * char_width) + 16.0;

        let mut clicked_idx = None;
        let mut overall_hovered = false;

        let total_items = self.items.len();

        for (idx, item) in self.items.iter().enumerate() {
            let item_text_width = item.label.len() as f32 * char_width;
            let item_rect = Rect::from_min_size(
                Vec2::new(current_x, rect.min.y),
                Vec2::new(item_text_width + 8.0, height),
            );

            let is_last = idx == total_items - 1;
            let item_hovered = item_rect.contains(ui.pointer.position);
            if item_hovered {
                overall_hovered = true;
            }

            let text_color = if is_last {
                palette.text
            } else if item_hovered {
                palette.accent
            } else {
                palette.text_muted
            };

            // Background highlight on hover if not last
            if item_hovered && !is_last {
                ui.cmd_buf.push(DrawCommand::Rect {
                    rect: item_rect,
                    fill: palette.elevated,
                    radius: CornerRadius::same(4.0),
                    stroke: Stroke::NONE,
                });
            }

            // Draw item label
            ui.cmd_buf.push(DrawCommand::Text {
                text: item.label.to_string(),
                pos: Vec2::new(current_x + 4.0, rect.min.y + (height - self.font_size) * 0.5),
                size: self.font_size,
                color: text_color,
                strong: is_last,
            });

            // Click handling
            if !is_last && ui.is_clicked_in_rect(item_rect) {
                clicked_idx = Some(idx);
            }

            current_x += item_text_width + 12.0;

            // Draw separator if not last
            if !is_last {
                ui.cmd_buf.push(DrawCommand::Text {
                    text: self.separator.to_string(),
                    pos: Vec2::new(current_x, rect.min.y + (height - self.font_size) * 0.5),
                    size: self.font_size,
                    color: palette.border,
                    strong: false,
                });
                current_x += sep_width;
            }
        }

        ui.advance(width, height);

        let resp = Response {
            hovered: overall_hovered,
            clicked: clicked_idx.is_some(),
            changed: clicked_idx.is_some(),
            dragging: false,
            activated: clicked_idx.is_some(),
        };

        (resp, clicked_idx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ui_platform::input::PointerState;
    use ui_render::CommandBuffer;
    use ui_themes::ThemePalette;
    use crate::icon_cache::IconAtlas;

    #[test]
    fn test_breadcrumbs_render() {
        let mut cmd_buf = CommandBuffer::new();
        let mut icon_atlas = IconAtlas::new();
        let palette = ThemePalette::dark();
        let pointer = PointerState::default();
        let bounds = Rect::from_min_size(Vec2::ZERO, Vec2::new(500.0, 200.0));

        let mut ui = KolponaUi::new(
            &mut cmd_buf,
            &mut icon_atlas,
            &palette,
            bounds,
            pointer,
        );

        let items = [
            BreadcrumbItem::new("Workspace"),
            BreadcrumbItem::new("Materials"),
            BreadcrumbItem::new("Concrete_Rough"),
        ];

        let (resp, clicked) = Breadcrumbs::new(&items).separator(">").show(&mut ui);
        assert_eq!(clicked, None);
        assert_eq!(resp.hovered, false);
        assert!(!cmd_buf.is_empty());
    }
}
