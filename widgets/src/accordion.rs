//! High-density Accordion component with exclusive sibling collapsing.
//!
//! Supports both multi-expandable and exclusive accordion modes, interactive
//! chevron toggle headers, and declarative content layout.

use ui_core::{CornerRadius, Rect, Stroke, Vec2};
use ui_render::{CommandBuffer, DrawCommand};
use ui_themes::{FluentTypographyScale, ThemePalette};

use crate::icon_cache::{IconAtlas, StudioIcon};

/// State of an individual accordion section.
#[derive(Debug, Clone, PartialEq)]
pub struct AccordionItemState {
    pub id: &'static str,
    pub title: &'static str,
    pub icon: Option<StudioIcon>,
    pub is_open: bool,
    pub content_height: f32,
}

impl AccordionItemState {
    pub const fn new(id: &'static str, title: &'static str, is_open: bool) -> Self {
        Self {
            id,
            title,
            icon: None,
            is_open,
            content_height: 120.0,
        }
    }

    pub const fn with_icon(mut self, icon: StudioIcon) -> Self {
        self.icon = Some(icon);
        self
    }

    pub const fn with_height(mut self, height: f32) -> Self {
        self.content_height = height;
        self
    }
}

/// Accordion container managing exclusive or independent section folding.
#[derive(Debug, Clone)]
pub struct Accordion {
    pub is_exclusive: bool,
    pub items: Vec<AccordionItemState>,
    pub header_height: f32,
}

impl Default for Accordion {
    fn default() -> Self {
        Self {
            is_exclusive: true,
            items: Vec::new(),
            header_height: 28.0,
        }
    }
}

impl Accordion {
    /// Creates a new Accordion. When `exclusive` is true, opening one section closes others.
    pub fn new(exclusive: bool) -> Self {
        Self {
            is_exclusive: exclusive,
            items: Vec::new(),
            header_height: 28.0,
        }
    }

    pub fn add_item(mut self, item: AccordionItemState) -> Self {
        self.items.push(item);
        self
    }

    /// Toggles the item with the given ID, honoring exclusive mode.
    pub fn toggle(&mut self, id: &str) {
        let mut target_new_state = false;
        if let Some(target) = self.items.iter_mut().find(|i| i.id == id) {
            target.is_open = !target.is_open;
            target_new_state = target.is_open;
        }

        // If exclusive mode and target just opened, close all other items
        if self.is_exclusive && target_new_state {
            for item in &mut self.items {
                if item.id != id {
                    item.is_open = false;
                }
            }
        }
    }

    /// Calculates total required layout height based on currently open items.
    pub fn total_height(&self) -> f32 {
        let mut total = 0.0;
        for item in &self.items {
            total += self.header_height + 4.0;
            if item.is_open {
                total += item.content_height + 4.0;
            }
        }
        total
    }

    /// Renders the accordion headers and returns whether any header was toggled.
    pub fn render_headers(
        &mut self,
        cmd_buf: &mut CommandBuffer,
        icon_atlas: &mut IconAtlas,
        bounds: Rect,
        pointer_pos: Vec2,
        is_clicked: bool,
        palette: &ThemePalette,
    ) -> Option<&'static str> {
        let mut toggled_id = None;
        let mut cur_y = bounds.min.y;

        for item in &mut self.items {
            let header_rect = Rect::from_min_size(
                Vec2::new(bounds.min.x, cur_y),
                Vec2::new(bounds.width(), self.header_height),
            );
            let hovered = header_rect.contains(pointer_pos);
            let hover = ui_core::motion::animate_bool(
                ui_core::motion::rect_key("accordion", header_rect),
                hovered,
                ui_core::motion::HOVER,
            );
            let bg_fill = palette.surface.lerp(palette.elevated, hover);

            // Header background
            cmd_buf.push(DrawCommand::Rect {
                rect: header_rect,
                fill: bg_fill,
                radius: CornerRadius::same(4.0),
                stroke: Stroke::new(1.0, palette.border),
            });

            // Chevron indicator: real vector icons instead of "v" / ">" letters.
            let chevron = if item.is_open {
                StudioIcon::ChevronDown
            } else {
                StudioIcon::ChevronRight
            };
            let chevron_y = header_rect.min.y + (header_rect.height() - 14.0) * 0.5;
            cmd_buf.push(DrawCommand::Icon {
                id: chevron.name(),
                svg: chevron.svg_str(),
                rect: Rect::from_min_size(
                    Vec2::new(header_rect.min.x + 6.0, chevron_y),
                    Vec2::new(14.0, 14.0),
                ),
                color: palette.text_muted.lerp(palette.accent, hover),
            });

            let mut text_x = header_rect.min.x + 22.0;

            // Optional icon
            if let Some(icon) = item.icon {
                let icon_rect = Rect::from_min_size(
                    Vec2::new(text_x, header_rect.min.y + 6.0),
                    Vec2::new(16.0, 16.0),
                );
                icon_atlas.render(cmd_buf, icon, icon_rect, palette.text);
                text_x += 20.0;
            }

            // Title
            let font_size = FluentTypographyScale::Body.pixels();
            cmd_buf.push(DrawCommand::Text {
                pos: Vec2::new(text_x, header_rect.min.y + 6.0),
                text: item.title.to_string(),
                color: palette.text,
                size: font_size - 1.0,
                strong: true,
            });

            if hovered && is_clicked {
                toggled_id = Some(item.id);
            }

            cur_y += self.header_height + 4.0;
            if item.is_open {
                cur_y += item.content_height + 4.0;
            }
        }

        if let Some(id) = toggled_id {
            self.toggle(id);
        }

        toggled_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exclusive_accordion_closes_siblings_on_open() {
        let mut accordion = Accordion::new(true)
            .add_item(AccordionItemState::new("sec1", "Transforms", true))
            .add_item(AccordionItemState::new("sec2", "Materials", false))
            .add_item(AccordionItemState::new("sec3", "Lighting", false));

        assert!(accordion.items[0].is_open);
        assert!(!accordion.items[1].is_open);

        // Opening sec2 must close sec1 in exclusive mode
        accordion.toggle("sec2");
        assert!(!accordion.items[0].is_open);
        assert!(accordion.items[1].is_open);
        assert!(!accordion.items[2].is_open);
    }

    #[test]
    fn test_non_exclusive_accordion_allows_multiple_open() {
        let mut accordion = Accordion::new(false)
            .add_item(AccordionItemState::new("sec1", "Transforms", true))
            .add_item(AccordionItemState::new("sec2", "Materials", false));

        accordion.toggle("sec2");
        assert!(accordion.items[0].is_open);
        assert!(accordion.items[1].is_open);
    }
}
