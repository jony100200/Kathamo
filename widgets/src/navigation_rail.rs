//! High-density workstation NavigationRail shell primitive.
//!
//! Provides a docked vertical navigation shell with leading, main, and trailing
//! item stacks, label display modes, badge overlays, and active pill indicators.

use ui_core::{Color, CornerRadius, Rect, Stroke, Vec2};
use ui_render::{CommandBuffer, DrawCommand};
use ui_themes::{FluentTypographyScale, ThemePalette};

use crate::icon_cache::{IconAtlas, StudioIcon};

/// Anchor position of the rail within the workstation window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RailAnchor {
    #[default]
    Left,
    Right,
}

/// Label display policy for rail items.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RailLabelMode {
    None,
    #[default]
    IconOnly,
    SelectedOnly,
    AlwaysShow,
}

/// Vertical alignment of the main item group.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RailAlignment {
    #[default]
    Top,
    Center,
    Bottom,
}

/// Single item in the navigation rail.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NavigationRailItem {
    pub id: &'static str,
    pub icon: StudioIcon,
    pub label: &'static str,
    pub badge_count: Option<usize>,
}

impl NavigationRailItem {
    pub const fn new(id: &'static str, icon: StudioIcon, label: &'static str) -> Self {
        Self {
            id,
            icon,
            label,
            badge_count: None,
        }
    }

    pub const fn with_badge(mut self, count: usize) -> Self {
        self.badge_count = Some(count);
        self
    }
}

/// Configuration and state for the NavigationRail component.
#[derive(Debug, Clone)]
pub struct NavigationRail {
    pub anchor: RailAnchor,
    pub label_mode: RailLabelMode,
    pub alignment: RailAlignment,
    pub leading_items: Vec<NavigationRailItem>,
    pub main_items: Vec<NavigationRailItem>,
    pub trailing_items: Vec<NavigationRailItem>,
    pub selected_id: Option<&'static str>,
}

impl Default for NavigationRail {
    fn default() -> Self {
        Self {
            anchor: RailAnchor::Left,
            label_mode: RailLabelMode::IconOnly,
            alignment: RailAlignment::Top,
            leading_items: Vec::new(),
            main_items: Vec::new(),
            trailing_items: Vec::new(),
            selected_id: None,
        }
    }
}

impl NavigationRail {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_selected(mut self, id: &'static str) -> Self {
        self.selected_id = Some(id);
        self
    }

    pub fn add_main(mut self, item: NavigationRailItem) -> Self {
        self.main_items.push(item);
        self
    }

    pub fn add_trailing(mut self, item: NavigationRailItem) -> Self {
        self.trailing_items.push(item);
        self
    }

    /// Computes the fixed layout width for the rail based on label mode.
    pub fn width(&self) -> f32 {
        match self.label_mode {
            RailLabelMode::None | RailLabelMode::IconOnly => 48.0,
            RailLabelMode::SelectedOnly => 56.0,
            RailLabelMode::AlwaysShow => 64.0,
        }
    }

    /// Renders the navigation rail and returns the item ID if an item was clicked.
    pub fn render(
        &self,
        cmd_buf: &mut CommandBuffer,
        icon_atlas: &mut IconAtlas,
        bounds: Rect,
        pointer_pos: Vec2,
        is_clicked: bool,
        palette: &ThemePalette,
    ) -> Option<&'static str> {
        let mut clicked_id = None;

        // Rail background body
        cmd_buf.push(DrawCommand::Rect {
            rect: bounds,
            fill: palette.surface,
            radius: CornerRadius::ZERO,
            stroke: Stroke::NONE,
        });

        // 1px border on the inner workstation edge
        let border_x = match self.anchor {
            RailAnchor::Left => bounds.max.x - 1.0,
            RailAnchor::Right => bounds.min.x,
        };
        cmd_buf.push(DrawCommand::Line {
            start: Vec2::new(border_x, bounds.min.y),
            end: Vec2::new(border_x, bounds.max.y),
            stroke: Stroke::new(1.0, palette.border),
        });

        let item_height = match self.label_mode {
            RailLabelMode::AlwaysShow => 52.0,
            _ => 40.0,
        };
        let item_width = bounds.width() - 8.0;

        // Render leading items at top
        let mut top_y = bounds.min.y + 6.0;
        for item in &self.leading_items {
            let item_rect = Rect::from_min_size(
                Vec2::new(bounds.min.x + 4.0, top_y),
                Vec2::new(item_width, item_height),
            );
            if self.render_item(
                cmd_buf,
                icon_atlas,
                item,
                item_rect,
                pointer_pos,
                is_clicked,
                palette,
            ) {
                clicked_id = Some(item.id);
            }
            top_y += item_height + 4.0;
        }

        // Render trailing items pinned to bottom
        let mut bottom_y = bounds.max.y - 6.0;
        for item in self.trailing_items.iter().rev() {
            bottom_y -= item_height;
            let item_rect = Rect::from_min_size(
                Vec2::new(bounds.min.x + 4.0, bottom_y),
                Vec2::new(item_width, item_height),
            );
            if self.render_item(
                cmd_buf,
                icon_atlas,
                item,
                item_rect,
                pointer_pos,
                is_clicked,
                palette,
            ) {
                clicked_id = Some(item.id);
            }
            bottom_y -= 4.0;
        }

        // Render main items in middle area
        let main_available_height = (bottom_y - top_y).max(0.0);
        let total_main_height = self.main_items.len() as f32 * (item_height + 4.0);
        let mut main_y = match self.alignment {
            RailAlignment::Top => top_y + 4.0,
            RailAlignment::Center => {
                top_y + (main_available_height - total_main_height).max(0.0) * 0.5
            }
            RailAlignment::Bottom => bottom_y - total_main_height,
        };

        for item in &self.main_items {
            let item_rect = Rect::from_min_size(
                Vec2::new(bounds.min.x + 4.0, main_y),
                Vec2::new(item_width, item_height),
            );
            if self.render_item(
                cmd_buf,
                icon_atlas,
                item,
                item_rect,
                pointer_pos,
                is_clicked,
                palette,
            ) {
                clicked_id = Some(item.id);
            }
            main_y += item_height + 4.0;
        }

        clicked_id
    }

    fn render_item(
        &self,
        cmd_buf: &mut CommandBuffer,
        icon_atlas: &mut IconAtlas,
        item: &NavigationRailItem,
        rect: Rect,
        pointer_pos: Vec2,
        is_clicked: bool,
        palette: &ThemePalette,
    ) -> bool {
        let is_selected = self.selected_id == Some(item.id);
        let hovered = rect.contains(pointer_pos);

        // Background pill
        if is_selected {
            // Selected active pill with accent fill
            cmd_buf.push(DrawCommand::Rect {
                rect,
                fill: palette.accent,
                radius: CornerRadius::same(6.0),
                stroke: Stroke::NONE,
            });
        } else if hovered {
            // Hover pill
            cmd_buf.push(DrawCommand::Rect {
                rect,
                fill: palette.elevated,
                radius: CornerRadius::same(6.0),
                stroke: Stroke::NONE,
            });
        }

        // Center icon
        let icon_color = if is_selected {
            Color::WHITE
        } else if hovered {
            palette.text
        } else {
            palette.text_muted
        };

        let icon_size = 20.0;
        let icon_y = match self.label_mode {
            RailLabelMode::AlwaysShow => rect.min.y + 6.0,
            _ => rect.min.y + (rect.height() - icon_size) * 0.5,
        };
        let icon_x = rect.min.x + (rect.width() - icon_size) * 0.5;
        let icon_rect =
            Rect::from_min_size(Vec2::new(icon_x, icon_y), Vec2::new(icon_size, icon_size));
        icon_atlas.render(cmd_buf, item.icon, icon_rect, icon_color);

        // Label if enabled
        let show_label = match self.label_mode {
            RailLabelMode::AlwaysShow => true,
            RailLabelMode::SelectedOnly => is_selected,
            _ => false,
        };

        if show_label {
            let font_size = FluentTypographyScale::Caption.pixels();
            let label_len = item.label.len() as f32 * 6.0;
            let text_x = rect.min.x + (rect.width() - label_len).max(0.0) * 0.5;
            let text_y = icon_rect.max.y + 2.0;
            cmd_buf.push(DrawCommand::Text {
                pos: Vec2::new(text_x, text_y),
                text: item.label.to_string(),
                color: icon_color,
                size: font_size - 1.0,
                strong: is_selected,
            });
        }

        // Badge indicator if present
        if let Some(count) = item.badge_count {
            if count > 0 {
                let badge_radius = 7.0;
                let badge_center = Vec2::new(icon_rect.max.x + 2.0, icon_rect.min.y - 2.0);
                let badge_rect = Rect::from_min_size(
                    Vec2::new(badge_center.x - badge_radius, badge_center.y - badge_radius),
                    Vec2::new(badge_radius * 2.0, badge_radius * 2.0),
                );
                cmd_buf.push(DrawCommand::Rect {
                    rect: badge_rect,
                    fill: palette.accent,
                    radius: CornerRadius::same(badge_radius),
                    stroke: Stroke::new(1.5, palette.surface),
                });
                let badge_text = if count > 99 {
                    "99+".to_string()
                } else {
                    count.to_string()
                };
                cmd_buf.push(DrawCommand::Text {
                    pos: Vec2::new(badge_center.x - 3.5, badge_center.y - 4.5),
                    text: badge_text,
                    color: Color::WHITE,
                    size: 9.0,
                    strong: true,
                });
            }
        }

        hovered && is_clicked
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_navigation_rail_geometry_and_click() {
        let mut cmd_buf = CommandBuffer::new();
        let mut icon_atlas = IconAtlas::new();
        let palette = ThemePalette::dark();

        let rail = NavigationRail::new()
            .with_selected("canvas")
            .add_main(NavigationRailItem::new(
                "canvas",
                StudioIcon::Sparkles,
                "Canvas",
            ))
            .add_main(NavigationRailItem::new("nodes", StudioIcon::Sparkles, "Nodes").with_badge(3))
            .add_trailing(NavigationRailItem::new(
                "settings",
                StudioIcon::Sparkles,
                "Settings",
            ));

        let bounds = Rect::from_min_size(Vec2::ZERO, Vec2::new(48.0, 600.0));
        let clicked = rail.render(
            &mut cmd_buf,
            &mut icon_atlas,
            bounds,
            Vec2::new(24.0, 20.0),
            true,
            &palette,
        );

        assert_eq!(clicked, Some("canvas"));
        assert!(!cmd_buf.is_empty());
    }
}
