//! Infinite 2D pan/zoom canvas viewport with coordinate transformation and spatial grid.

use crate::theme::ThemePalette;
use ui_core::{Color, CornerRadius, Rect, Stroke, Vec2};
use ui_render::{CommandBuffer, DrawCommand};

#[derive(Debug, Clone, PartialEq)]
pub struct CanvasShape {
    pub id: String,
    pub rect: Rect, // World coordinates
    pub fill: Color,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InfiniteCanvas {
    pub pan: Vec2,
    pub zoom: f32,
    pub shapes: Vec<CanvasShape>,
    pub selected_id: Option<String>,
}

impl Default for InfiniteCanvas {
    fn default() -> Self {
        Self {
            pan: Vec2::new(100.0, 100.0),
            zoom: 1.0,
            shapes: vec![
                CanvasShape {
                    id: "shape_hero".into(),
                    rect: Rect::from_min_size(Vec2::new(50.0, 50.0), Vec2::new(120.0, 80.0)),
                    fill: Color::rgb(70, 110, 240),
                    label: "Player Sprite".into(),
                },
                CanvasShape {
                    id: "shape_bg".into(),
                    rect: Rect::from_min_size(Vec2::new(220.0, 70.0), Vec2::new(160.0, 100.0)),
                    fill: Color::rgb(45, 160, 110),
                    label: "Tileset Atlas".into(),
                },
            ],
            selected_id: None,
        }
    }
}

impl InfiniteCanvas {
    pub fn world_to_screen(&self, world: Vec2, viewport_origin: Vec2) -> Vec2 {
        Vec2::new(
            viewport_origin.x + (world.x + self.pan.x) * self.zoom,
            viewport_origin.y + (world.y + self.pan.y) * self.zoom,
        )
    }

    pub fn screen_to_world(&self, screen: Vec2, viewport_origin: Vec2) -> Vec2 {
        Vec2::new(
            (screen.x - viewport_origin.x) / self.zoom - self.pan.x,
            (screen.y - viewport_origin.y) / self.zoom - self.pan.y,
        )
    }

    pub fn zoom_at(&mut self, factor: f32, anchor_screen: Vec2, viewport_origin: Vec2) {
        let before_world = self.screen_to_world(anchor_screen, viewport_origin);
        self.zoom = (self.zoom * factor).clamp(0.1, 10.0);
        let after_world = self.screen_to_world(anchor_screen, viewport_origin);
        self.pan = self.pan + (after_world - before_world);
    }

    pub fn pan_by(&mut self, delta_screen: Vec2) {
        self.pan = self.pan + delta_screen * (1.0 / self.zoom);
    }

    pub fn render(&self, buf: &mut CommandBuffer, viewport: Rect, palette: &ThemePalette) {
        buf.push(DrawCommand::ClipPush { rect: viewport });

        // Background canvas fill
        buf.push(DrawCommand::Rect {
            rect: viewport,
            fill: palette.canvas,
            radius: CornerRadius::ZERO,
            stroke: Stroke::NONE,
        });

        // Grid lines
        let grid_size = 40.0 * self.zoom;
        if grid_size > 8.0 {
            let offset_x = (self.pan.x * self.zoom).rem_euclid(grid_size);
            let offset_y = (self.pan.y * self.zoom).rem_euclid(grid_size);
            let grid_stroke = Stroke::new(1.0, palette.border.with_alpha(40));

            let mut x = viewport.min.x + offset_x;
            while x < viewport.max.x {
                buf.push(DrawCommand::Line {
                    start: Vec2::new(x, viewport.min.y),
                    end: Vec2::new(x, viewport.max.y),
                    stroke: grid_stroke,
                });
                x += grid_size;
            }

            let mut y = viewport.min.y + offset_y;
            while y < viewport.max.y {
                buf.push(DrawCommand::Line {
                    start: Vec2::new(viewport.min.x, y),
                    end: Vec2::new(viewport.max.x, y),
                    stroke: grid_stroke,
                });
                y += grid_size;
            }
        }

        // Draw shapes in world space projected to screen
        for shape in &self.shapes {
            let screen_min = self.world_to_screen(shape.rect.min, viewport.min);
            let screen_max = self.world_to_screen(shape.rect.max, viewport.min);
            let screen_rect = Rect::from_min_max(screen_min, screen_max);

            if viewport.intersects(screen_rect) {
                let is_sel = self.selected_id.as_deref() == Some(&shape.id);
                buf.push(DrawCommand::Rect {
                    rect: screen_rect,
                    fill: shape.fill,
                    radius: CornerRadius::same(6.0 * self.zoom),
                    stroke: if is_sel {
                        Stroke::new(2.0, palette.accent)
                    } else {
                        Stroke::new(1.0, palette.border)
                    },
                });

                buf.push(DrawCommand::Text {
                    text: shape.label.clone(),
                    pos: Vec2::new(screen_rect.min.x + 8.0, screen_rect.min.y + 8.0),
                    size: (12.0 * self.zoom).clamp(8.0, 24.0),
                    color: Color::WHITE,
                    strong: true,
                });
            }
        }

        // HUD overlay with zoom percentage
        let hud_rect = Rect::from_min_size(
            Vec2::new(viewport.max.x - 110.0, viewport.max.y - 35.0),
            Vec2::new(100.0, 25.0),
        );
        buf.push(DrawCommand::Rect {
            rect: hud_rect,
            fill: palette.surface.with_alpha(200),
            radius: CornerRadius::same(4.0),
            stroke: Stroke::new(1.0, palette.border),
        });
        buf.push(DrawCommand::Text {
            text: format!("Zoom: {:.0}%", self.zoom * 100.0),
            pos: Vec2::new(hud_rect.min.x + 8.0, hud_rect.min.y + 5.0),
            size: 11.0,
            color: palette.text,
            strong: true,
        });

        buf.push(DrawCommand::ClipPop);
    }
}
