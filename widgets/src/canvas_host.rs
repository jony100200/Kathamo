//! Reusable CanvasHost supporting application-owned GPU textures, 2D/3D viewports,
//! camera pan/zoom transformations, coordinate mapping, and input delegation.

use crate::theme::ThemePalette;
use ui_core::{Color, CornerRadius, Rect, Stroke, Vec2};
use ui_platform::{PointerButton, PointerState};
use ui_render::{CommandBuffer, DrawCommand};

/// Active creative tool within the canvas.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CanvasTool {
    #[default]
    Select,
    Pan,
    Brush,
    Eraser,
    ShapeRect,
    Eyedropper,
}

/// A freehand or vector stroke rendered onto a 2D canvas layer.
#[derive(Debug, Clone, PartialEq)]
pub struct CanvasStroke {
    pub points: Vec<Vec2>, // World coordinates
    pub color: Color,
    pub width: f32,
    pub is_eraser: bool,
}

/// Canvas hosting kind: 2D vector/layer canvas, or external 3D/video texture viewport.
#[derive(Debug, Clone, PartialEq)]
pub enum CanvasHostKind {
    /// 2D multi-layer creative canvas with strokes and shapes.
    Creative2D {
        strokes: Vec<CanvasStroke>,
        active_stroke: Option<CanvasStroke>,
    },
    /// External application-owned GPU framebuffer or video texture surface.
    ExternalTexture {
        texture_id: u64,
        resolution: (u32, u32),
        uv_rect: Rect,
    },
}

impl Default for CanvasHostKind {
    fn default() -> Self {
        Self::Creative2D {
            strokes: Vec::new(),
            active_stroke: None,
        }
    }
}

/// Reusable workstation viewport host.
#[derive(Debug, Clone, PartialEq)]
pub struct CanvasHost {
    pub pan: Vec2,
    pub zoom: f32,
    pub active_tool: CanvasTool,
    pub brush_size: f32,
    pub brush_color: Color,
    pub kind: CanvasHostKind,
    pub show_grid: bool,
    pub show_checkerboard: bool,
    pub is_panning: bool,
}

impl Default for CanvasHost {
    fn default() -> Self {
        Self {
            pan: Vec2::new(120.0, 120.0),
            zoom: 1.0,
            active_tool: CanvasTool::Select,
            brush_size: 14.0,
            brush_color: Color::rgb(70, 140, 245),
            kind: CanvasHostKind::default(),
            show_grid: true,
            show_checkerboard: true,
            is_panning: false,
        }
    }
}

impl CanvasHost {
    pub fn new_2d() -> Self {
        Self::default()
    }

    pub fn new_texture_host(texture_id: u64, resolution: (u32, u32)) -> Self {
        Self {
            kind: CanvasHostKind::ExternalTexture {
                texture_id,
                resolution,
                uv_rect: Rect::from_min_max(Vec2::ZERO, Vec2::new(1.0, 1.0)),
            },
            ..Default::default()
        }
    }

    /// Transforms world coordinates to screen viewport coordinates.
    pub fn world_to_screen(&self, world: Vec2, viewport_origin: Vec2) -> Vec2 {
        Vec2::new(
            viewport_origin.x + (world.x + self.pan.x) * self.zoom,
            viewport_origin.y + (world.y + self.pan.y) * self.zoom,
        )
    }

    /// Transforms screen coordinates to world coordinates.
    pub fn screen_to_world(&self, screen: Vec2, viewport_origin: Vec2) -> Vec2 {
        Vec2::new(
            (screen.x - viewport_origin.x) / self.zoom - self.pan.x,
            (screen.y - viewport_origin.y) / self.zoom - self.pan.y,
        )
    }

    /// Zooms anchored around a specific screen position.
    pub fn zoom_at(&mut self, factor: f32, anchor_screen: Vec2, viewport_origin: Vec2) {
        let before_world = self.screen_to_world(anchor_screen, viewport_origin);
        self.zoom = (self.zoom * factor).clamp(0.05, 30.0);
        let after_world = self.screen_to_world(anchor_screen, viewport_origin);
        self.pan = self.pan + (after_world - before_world);
    }

    /// Pans the canvas by a screen-space delta.
    pub fn pan_by(&mut self, delta_screen: Vec2) {
        self.pan = self.pan + delta_screen * (1.0 / self.zoom);
    }

    /// Resets pan and zoom to default origin.
    pub fn reset_view(&mut self) {
        self.pan = Vec2::new(100.0, 100.0);
        self.zoom = 1.0;
    }

    /// Clears all strokes on a 2D canvas.
    pub fn clear_strokes(&mut self) {
        if let CanvasHostKind::Creative2D { strokes, active_stroke } = &mut self.kind {
            strokes.clear();
            *active_stroke = None;
        }
    }

    /// Handles pointer press events.
    pub fn handle_mouse_down(&mut self, pos: Vec2, button: PointerButton, viewport: Rect) {
        if !viewport.contains(pos) {
            return;
        }

        if button == PointerButton::Middle || (button == PointerButton::Primary && self.active_tool == CanvasTool::Pan) {
            self.is_panning = true;
            return;
        }

        if button == PointerButton::Primary {
            let world_pos = self.screen_to_world(pos, viewport.min);
            match self.active_tool {
                CanvasTool::Brush | CanvasTool::Eraser => {
                    if let CanvasHostKind::Creative2D { active_stroke, .. } = &mut self.kind {
                        *active_stroke = Some(CanvasStroke {
                            points: vec![world_pos],
                            color: if self.active_tool == CanvasTool::Eraser {
                                Color::TRANSPARENT
                            } else {
                                self.brush_color
                            },
                            width: self.brush_size,
                            is_eraser: self.active_tool == CanvasTool::Eraser,
                        });
                    }
                }
                _ => {}
            }
        }
    }

    /// Handles pointer movement and active drawing/panning.
    pub fn handle_mouse_move(&mut self, pos: Vec2, delta: Vec2, viewport: Rect, ptr: &PointerState) {
        if self.is_panning || ptr.middle_down || (ptr.primary_down && self.active_tool == CanvasTool::Pan) {
            self.pan_by(delta);
            return;
        }

        if ptr.primary_down && (self.active_tool == CanvasTool::Brush || self.active_tool == CanvasTool::Eraser) {
            let world_pos = self.screen_to_world(pos, viewport.min);
            if let CanvasHostKind::Creative2D { active_stroke, .. } = &mut self.kind {
                if let Some(stroke) = active_stroke {
                    if stroke.points.last().map_or(true, |&last| (last - world_pos).length() > 2.0) {
                        stroke.points.push(world_pos);
                    }
                }
            }
        }
    }

    /// Handles pointer release events, committing strokes.
    pub fn handle_mouse_up(&mut self, _pos: Vec2, button: PointerButton) {
        if button == PointerButton::Middle || button == PointerButton::Primary {
            self.is_panning = false;
        }

        if button == PointerButton::Primary {
            if let CanvasHostKind::Creative2D { strokes, active_stroke } = &mut self.kind {
                if let Some(stroke) = active_stroke.take() {
                    if stroke.points.len() >= 2 {
                        strokes.push(stroke);
                    }
                }
            }
        }
    }

    /// Handles scroll wheel zooming.
    pub fn handle_scroll(&mut self, delta: Vec2, cursor_pos: Vec2, viewport: Rect) {
        let factor = if delta.y > 0.0 { 1.12 } else { 0.89 };
        self.zoom_at(factor, cursor_pos, viewport.min);
    }

    /// Renders the complete canvas host into the draw command buffer.
    pub fn render(&self, buf: &mut CommandBuffer, viewport: Rect, palette: &ThemePalette) {
        buf.push(DrawCommand::ClipPush { rect: viewport });

        // Base canvas background
        buf.push(DrawCommand::Rect {
            rect: viewport,
            fill: palette.canvas,
            radius: CornerRadius::ZERO,
            stroke: Stroke::NONE,
        });

        // Checkerboard transparency background
        if self.show_checkerboard {
            let check_size = (20.0 * self.zoom).clamp(8.0, 60.0);
            let offset_x = (self.pan.x * self.zoom).rem_euclid(check_size * 2.0);
            let offset_y = (self.pan.y * self.zoom).rem_euclid(check_size * 2.0);
            let check_dark = palette.surface.with_alpha(40);

            let mut y = viewport.min.y + offset_y - check_size * 2.0;
            let mut row_even = false;
            while y < viewport.max.y {
                let mut x = viewport.min.x + offset_x - check_size * 2.0;
                let mut col_even = row_even;
                while x < viewport.max.x {
                    if col_even {
                        let check_rect = Rect::from_min_size(
                            Vec2::new(x, y),
                            Vec2::new(check_size, check_size),
                        );
                        if let Some(clipped) = check_rect.intersect(viewport) {
                            buf.push(DrawCommand::Rect {
                                rect: clipped,
                                fill: check_dark,
                                radius: CornerRadius::ZERO,
                                stroke: Stroke::NONE,
                            });
                        }
                    }
                    col_even = !col_even;
                    x += check_size;
                }
                row_even = !row_even;
                y += check_size;
            }
        }

        // Adaptive spatial grid
        if self.show_grid {
            let grid_size = 40.0 * self.zoom;
            if grid_size > 8.0 {
                let offset_x = (self.pan.x * self.zoom).rem_euclid(grid_size);
                let offset_y = (self.pan.y * self.zoom).rem_euclid(grid_size);
                let grid_stroke = Stroke::new(1.0, palette.border.with_alpha(35));

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
        }

        // Render hosted content
        match &self.kind {
            CanvasHostKind::Creative2D { strokes, active_stroke } => {
                // Committed strokes
                for stroke in strokes {
                    self.render_stroke(buf, stroke, viewport.min);
                }
                // In-progress stroke
                if let Some(stroke) = active_stroke {
                    self.render_stroke(buf, stroke, viewport.min);
                }
            }
            CanvasHostKind::ExternalTexture { texture_id, uv_rect, .. } => {
                let world_rect = Rect::from_min_size(Vec2::ZERO, Vec2::new(512.0, 512.0));
                let screen_min = self.world_to_screen(world_rect.min, viewport.min);
                let screen_max = self.world_to_screen(world_rect.max, viewport.min);
                let quad = Rect::from_min_max(screen_min, screen_max);

                if viewport.intersects(quad) {
                    buf.push(DrawCommand::Image {
                        rect: quad,
                        texture_id: *texture_id,
                        uv_rect: *uv_rect,
                        tint: Color::WHITE,
                    });
                    buf.push(DrawCommand::Rect {
                        rect: quad,
                        fill: Color::TRANSPARENT,
                        radius: CornerRadius::ZERO,
                        stroke: Stroke::new(1.5, palette.accent),
                    });
                }
            }
        }

        // Canvas HUD: zoom level and active tool indicator
        let hud_w = 170.0;
        let hud_rect = Rect::from_min_size(
            Vec2::new(viewport.max.x - hud_w - 12.0, viewport.max.y - 36.0),
            Vec2::new(hud_w, 28.0),
        );
        buf.push(DrawCommand::Rect {
            rect: hud_rect,
            fill: palette.surface.with_alpha(220),
            radius: CornerRadius::same(6.0),
            stroke: Stroke::new(1.0, palette.border),
        });
        buf.push(DrawCommand::Text {
            text: format!("Zoom: {:.0}% | {:?}", self.zoom * 100.0, self.active_tool),
            pos: Vec2::new(hud_rect.min.x + 10.0, hud_rect.min.y + 7.0),
            size: 11.5,
            color: palette.text,
            strong: true,
        });

        buf.push(DrawCommand::ClipPop);
    }

    fn render_stroke(&self, buf: &mut CommandBuffer, stroke: &CanvasStroke, viewport_origin: Vec2) {
        if stroke.points.len() < 2 {
            return;
        }

        let stroke_w = (stroke.width * self.zoom).clamp(1.0, 100.0);
        let color = if stroke.is_eraser {
            Color::BLACK.with_alpha(180)
        } else {
            stroke.color
        };

        for w in stroke.points.windows(2) {
            let p0 = self.world_to_screen(w[0], viewport_origin);
            let p1 = self.world_to_screen(w[1], viewport_origin);
            buf.push(DrawCommand::Line {
                start: p0,
                end: p1,
                stroke: Stroke::new(stroke_w, color),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ui_themes::ThemePalette;

    #[test]
    fn test_canvas_coordinate_transforms() {
        let mut host = CanvasHost::new_2d();
        host.pan = Vec2::new(100.0, 50.0);
        host.zoom = 2.0;

        let origin = Vec2::new(20.0, 30.0);
        let world_pt = Vec2::new(50.0, 40.0);

        let screen_pt = host.world_to_screen(world_pt, origin);
        let recovered_world = host.screen_to_world(screen_pt, origin);

        assert!((world_pt.x - recovered_world.x).abs() < 1e-4);
        assert!((world_pt.y - recovered_world.y).abs() < 1e-4);
    }

    #[test]
    fn test_canvas_pan_and_zoom() {
        let mut host = CanvasHost::new_2d();
        let viewport = Rect::from_min_size(Vec2::ZERO, Vec2::new(800.0, 600.0));

        let ptr = PointerState::default();

        // Pan delta
        host.handle_mouse_move(Vec2::new(100.0, 100.0), Vec2::new(10.0, -5.0), viewport, &ptr);
        // Pan only changes if space is down or middle click is dragging
        host.is_panning = true;
        host.handle_mouse_move(Vec2::new(100.0, 100.0), Vec2::new(10.0, -5.0), viewport, &ptr);
        assert_eq!(host.pan, Vec2::new(130.0, 115.0));

        // Zoom at point
        let prev_zoom = host.zoom;
        host.handle_scroll(Vec2::new(0.0, 1.0), Vec2::new(400.0, 300.0), viewport);
        assert!(host.zoom > prev_zoom, "Scroll up should zoom in");
    }

    #[test]
    fn test_canvas_drawing_records_strokes() {
        let mut host = CanvasHost::new_2d();
        host.active_tool = CanvasTool::Brush;
        let viewport = Rect::from_min_size(Vec2::ZERO, Vec2::new(600.0, 400.0));

        host.handle_mouse_down(Vec2::new(100.0, 100.0), PointerButton::Primary, viewport);
        if let CanvasHostKind::Creative2D { ref active_stroke, .. } = host.kind {
            assert!(active_stroke.is_some());
        }

        let ptr = PointerState {
            primary_down: true,
            ..Default::default()
        };
        host.handle_mouse_move(Vec2::new(110.0, 105.0), Vec2::new(10.0, 5.0), viewport, &ptr);
        if let CanvasHostKind::Creative2D { ref active_stroke, .. } = host.kind {
            assert_eq!(active_stroke.as_ref().unwrap().points.len(), 2);
        }

        host.handle_mouse_up(Vec2::new(110.0, 105.0), PointerButton::Primary);
        if let CanvasHostKind::Creative2D { ref strokes, ref active_stroke } = host.kind {
            assert!(active_stroke.is_none());
            assert_eq!(strokes.len(), 1);
            assert_eq!(strokes[0].points.len(), 2);
        }
    }

    #[test]
    fn test_canvas_render_emits_commands() {
        let host = CanvasHost::new_texture_host(42, (1920, 1080));
        let mut buf = CommandBuffer::new();
        let palette = ThemePalette::dark();
        let viewport = Rect::from_min_size(Vec2::ZERO, Vec2::new(800.0, 600.0));

        host.render(&mut buf, viewport, &palette);
        assert!(!buf.commands().is_empty());

        let has_image = buf.commands().iter().any(|cmd| matches!(cmd, DrawCommand::Image { texture_id: 42, .. }));
        assert!(has_image, "CanvasHost texture kind must emit DrawCommand::Image");
    }
}
