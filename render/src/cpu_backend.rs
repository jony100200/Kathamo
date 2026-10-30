//! Safe-mode software CPU rasterizer.
//! Infallible fallback: runs on any machine with zero GPU driver dependencies.
//! Also powers offline screenshot captures, headless CI tests and the reference
//! image used to verify the GPU output.
//!
//! Feature parity with the GPU path: anti-aliased rounded rectangles with inset
//! borders, thick anti-aliased lines and beziers, clip rectangles, source-over alpha
//! blending, real shaped text (rustybuzz + ab_glyph) and SVG icons (resvg), all at
//! the configured DPI scale factor.

use crate::backend::{BackendKind, RenderBackend, RenderStats};
use crate::command::{CommandBuffer, DrawCommand};
use crate::shapes::{flatten_cubic, max_radius, scale_radius, sd_round_box, segment_distance};
use crate::text::{self, Face, GlyphCache, GlyphKey, Mask};
use crate::vector::{IconKey, IconRasterCache};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use ui_core::{Color, CornerRadius, Rect, Vec2};

struct CpuImage {
    width: u32,
    height: u32,
    rgba: Arc<[u8]>,
}

pub struct CpuBackend {
    width: u32,
    height: u32,
    scale: f32,
    pixels: Vec<u32>, // 0xAARRGGBB, always opaque
    images: HashMap<u64, CpuImage>,
    clip_stack: Vec<[i32; 4]>,
    glyphs: GlyphCache,
    icons: IconRasterCache,
    stats: RenderStats,
}

impl CpuBackend {
    pub fn new() -> Self {
        Self {
            width: 0,
            height: 0,
            scale: 1.0,
            pixels: Vec::new(),
            images: HashMap::new(),
            clip_stack: Vec::new(),
            glyphs: GlyphCache::new(),
            icons: IconRasterCache::new(),
            stats: RenderStats {
                backend: Some(BackendKind::SoftwareCpuSafeMode),
                ..Default::default()
            },
        }
    }

    /// Keep a CPU-readable frame for software rendering and device-loss fallback.
    pub fn upload_rgba8_image(
        &mut self,
        id: u64,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<(), String> {
        if width == 0
            || height == 0
            || (width as usize)
                .checked_mul(height as usize)
                .and_then(|n| n.checked_mul(4))
                != Some(rgba.len())
        {
            return Err("RGBA8 image length must be width * height * 4".into());
        }
        self.images.insert(
            id,
            CpuImage {
                width,
                height,
                rgba: Arc::from(rgba),
            },
        );
        Ok(())
    }

    pub fn remove_image(&mut self, id: u64) {
        self.images.remove(&id);
    }

    fn draw_image(&mut self, id: u64, rect: Rect, uv: Rect, tint: Color) {
        let Some(image) = self.images.get(&id) else {
            let scale = self.scale;
            let clip = self.clip();
            self.fill_px_rect(
                (rect.min.x * scale).round() as i32,
                (rect.min.y * scale).round() as i32,
                (rect.max.x * scale).round() as i32,
                (rect.max.y * scale).round() as i32,
                tint,
                clip,
            );
            return;
        };
        let (image_width, image_height, rgba) = (image.width, image.height, image.rgba.clone());
        let clip = self.clip();
        let scale = self.scale;
        let x0 = (rect.min.x * scale).floor() as i32;
        let y0 = (rect.min.y * scale).floor() as i32;
        let x1 = (rect.max.x * scale).ceil() as i32;
        let y1 = (rect.max.y * scale).ceil() as i32;
        let w = (x1 - x0).max(1) as f32;
        let h = (y1 - y0).max(1) as f32;
        let left = x0.max(clip[0]).max(0);
        let top = y0.max(clip[1]).max(0);
        let right = x1.min(clip[2]).min(self.width as i32);
        let bottom = y1.min(clip[3]).min(self.height as i32);
        for y in top..bottom {
            for x in left..right {
                let u = (uv.min.x + (x as f32 - x0 as f32 + 0.5) / w * uv.width()).clamp(0.0, 1.0);
                let v = (uv.min.y + (y as f32 - y0 as f32 + 0.5) / h * uv.height()).clamp(0.0, 1.0);
                let ix = ((u * image_width as f32) as u32).min(image_width - 1);
                let iy = ((v * image_height as f32) as u32).min(image_height - 1);
                let at = ((iy * image_width + ix) * 4) as usize;
                let (r, g, b, a) = (rgba[at], rgba[at + 1], rgba[at + 2], rgba[at + 3]);
                let color = Color::rgba(
                    (r as u16 * tint.r as u16 / 255) as u8,
                    (g as u16 * tint.g as u16 / 255) as u8,
                    (b as u16 * tint.b as u16 / 255) as u8,
                    (a as u16 * tint.a as u16 / 255) as u8,
                );
                self.blend(x, y, color, 1.0, clip);
            }
        }
    }

    pub fn pixels(&self) -> &[u32] {
        &self.pixels
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn scale_factor(&self) -> f32 {
        self.scale
    }

    #[inline]
    fn clip(&self) -> [i32; 4] {
        self.clip_stack
            .last()
            .copied()
            .unwrap_or([0, 0, self.width as i32, self.height as i32])
    }

    /// Source-over blend of `color` with fractional `coverage` into pixel (x, y).
    #[inline]
    fn blend(&mut self, x: i32, y: i32, color: Color, coverage: f32, clip: [i32; 4]) {
        if x < clip[0] || y < clip[1] || x >= clip[2] || y >= clip[3] {
            return;
        }
        let a = (color.a as f32 / 255.0) * coverage.clamp(0.0, 1.0);
        if a <= 0.002 {
            return;
        }
        let idx = y as usize * self.width as usize + x as usize;
        if a >= 0.998 {
            self.pixels[idx] = 0xFF00_0000 | (color.to_u32() & 0x00FF_FFFF);
            return;
        }
        let dst = self.pixels[idx];
        let mix = |s: u8, d: u32| -> u32 { (s as f32 * a + d as f32 * (1.0 - a) + 0.5) as u32 };
        let r = mix(color.r, (dst >> 16) & 0xFF);
        let g = mix(color.g, (dst >> 8) & 0xFF);
        let b = mix(color.b, dst & 0xFF);
        self.pixels[idx] = 0xFF00_0000 | (r << 16) | (g << 8) | b;
    }

    fn fill_px_rect(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, color: Color, clip: [i32; 4]) {
        let x0 = x0.max(clip[0]);
        let y0 = y0.max(clip[1]);
        let x1 = x1.min(clip[2]);
        let y1 = y1.min(clip[3]);
        if x0 >= x1 || y0 >= y1 || color.a == 0 {
            return;
        }
        if color.a == 255 {
            let px = 0xFF00_0000 | (color.to_u32() & 0x00FF_FFFF);
            for y in y0..y1 {
                let row = y as usize * self.width as usize;
                self.pixels[row + x0 as usize..row + x1 as usize].fill(px);
            }
        } else {
            for y in y0..y1 {
                for x in x0..x1 {
                    self.blend(x, y, color, 1.0, clip);
                }
            }
        }
    }

    fn draw_rect(
        &mut self,
        rect: Rect,
        fill: Color,
        radius: CornerRadius,
        stroke: ui_core::Stroke,
    ) {
        let s = self.scale;
        let clip = self.clip();
        let min = rect.min * s;
        let max = rect.max * s;
        let radius = scale_radius(radius, s);
        let bw = if stroke.color.a > 0 {
            stroke.width * s
        } else {
            0.0
        };

        if max_radius(radius) <= 0.0 {
            // Axis-aligned fast path, snapped to whole pixels.
            let (x0, y0, x1, y1) = (
                min.x.round() as i32,
                min.y.round() as i32,
                max.x.round() as i32,
                max.y.round() as i32,
            );
            if bw > 0.0 {
                let b = bw.round().max(1.0) as i32;
                self.fill_px_rect(x0 + b, y0 + b, x1 - b, y1 - b, fill, clip);
                self.fill_px_rect(x0, y0, x1, y0 + b, stroke.color, clip);
                self.fill_px_rect(x0, y1 - b, x1, y1, stroke.color, clip);
                self.fill_px_rect(x0, y0 + b, x0 + b, y1 - b, stroke.color, clip);
                self.fill_px_rect(x1 - b, y0 + b, x1, y1 - b, stroke.color, clip);
            } else {
                self.fill_px_rect(x0, y0, x1, y1, fill, clip);
            }
            return;
        }

        // Rounded path: signed-distance coverage, identical to the GPU shader.
        let center = (min + max) * 0.5;
        let half = (max - min) * 0.5;
        let inner_margin = max_radius(radius).max(bw) + 1.0;
        let x0 = (min.x.floor() as i32).max(clip[0]);
        let y0 = (min.y.floor() as i32).max(clip[1]);
        let x1 = (max.x.ceil() as i32).min(clip[2]);
        let y1 = (max.y.ceil() as i32).min(clip[3]);
        for y in y0..y1 {
            let py = y as f32 + 0.5;
            let row_inner = py > min.y + inner_margin && py < max.y - inner_margin;
            for x in x0..x1 {
                let px = x as f32 + 0.5;
                if row_inner && px > min.x + inner_margin && px < max.x - inner_margin {
                    self.blend(x, y, fill, 1.0, clip);
                    continue;
                }
                let d = sd_round_box(Vec2::new(px, py) - center, half, radius);
                let outer = (0.5 - d).clamp(0.0, 1.0);
                if outer <= 0.0 {
                    continue;
                }
                if bw > 0.0 {
                    let inner = (0.5 - (d + bw)).clamp(0.0, 1.0);
                    self.blend(x, y, fill, inner, clip);
                    self.blend(x, y, stroke.color, outer - inner, clip);
                } else {
                    self.blend(x, y, fill, outer, clip);
                }
            }
        }
    }


    fn draw_surface(
        &mut self,
        rect: Rect,
        fill_top: Color,
        fill_bottom: Color,
        radius: CornerRadius,
        stroke: ui_core::Stroke,
        rim_strength: f32,
    ) {
        let scale = self.scale;
        let clip = self.clip();
        let min = rect.min * scale;
        let max = rect.max * scale;
        if min.x >= max.x || min.y >= max.y { return; }
        let center = (min + max) * 0.5;
        let half = (max - min) * 0.5;
        let radii = scale_radius(radius, scale);
        let bw = if stroke.color.a > 0 { stroke.width.max(0.0) * scale } else { 0.0 };
        let x0 = (min.x.floor() as i32).max(clip[0]);
        let y0 = (min.y.floor() as i32).max(clip[1]);
        let x1 = (max.x.ceil() as i32).min(clip[2]);
        let y1 = (max.y.ceil() as i32).min(clip[3]);
        let lerp = |a: u8, b: u8, t: f32| -> u8 {
            (a as f32 + (b as f32 - a as f32) * t).round().clamp(0.0, 255.0) as u8
        };
        for y in y0..y1 {
            let py = y as f32 + 0.5;
            let t = ((py - min.y) / (max.y - min.y).max(1.0)).clamp(0.0, 1.0);
            let h = ((py - min.y) / 2.0).clamp(0.0, 1.0);
            let top_light = (1.0 - h * h * (3.0 - 2.0 * h)) * rim_strength.clamp(0.0, 1.0) * 0.12;
            let brighten = |v: u8| -> u8 {
                (v as f32 + (255.0 - v as f32) * top_light).round() as u8
            };
            let fill = Color::rgba(
                brighten(lerp(fill_top.r, fill_bottom.r, t)),
                brighten(lerp(fill_top.g, fill_bottom.g, t)),
                brighten(lerp(fill_top.b, fill_bottom.b, t)),
                lerp(fill_top.a, fill_bottom.a, t),
            );
            for x in x0..x1 {
                let point = Vec2::new(x as f32 + 0.5, py);
                let d = sd_round_box(point - center, half, radii);
                let outer = (0.5 - d).clamp(0.0, 1.0);
                if outer <= 0.0 { continue; }
                let inner = if bw > 0.0 { (0.5 - (d + bw)).clamp(0.0, 1.0) } else { outer };
                self.blend(x, y, fill, inner, clip);
                if bw > 0.0 { self.blend(x, y, stroke.color, outer - inner, clip); }
            }
        }
    }

    fn draw_soft_shadow(
        &mut self,
        rect: Rect,
        radius: CornerRadius,
        color: Color,
        offset: Vec2,
        blur: f32,
        spread: f32,
    ) {
        if color.a == 0 { return; }
        let spread = spread.clamp(-32.0, 32.0);
        let blur = blur.clamp(0.5, 64.0) * self.scale;
        let min = (rect.min + offset - Vec2::splat(spread)) * self.scale;
        let max = (rect.max + offset + Vec2::splat(spread)) * self.scale;
        if min.x >= max.x || min.y >= max.y { return; }
        let center = (min + max) * 0.5;
        let half = (max - min) * 0.5;
        let radius = CornerRadius {
            top_left: (radius.top_left + spread).max(0.0),
            top_right: (radius.top_right + spread).max(0.0),
            bottom_right: (radius.bottom_right + spread).max(0.0),
            bottom_left: (radius.bottom_left + spread).max(0.0),
        };
        let radii = scale_radius(radius, self.scale);
        let clip = self.clip();
        let pad = blur + 1.0;
        let x0 = ((min.x - pad).floor() as i32).max(clip[0]);
        let y0 = ((min.y - pad).floor() as i32).max(clip[1]);
        let x1 = ((max.x + pad).ceil() as i32).min(clip[2]);
        let y1 = ((max.y + pad).ceil() as i32).min(clip[3]);
        for y in y0..y1 {
            for x in x0..x1 {
                let d = sd_round_box(Vec2::new(x as f32 + 0.5, y as f32 + 0.5) - center, half, radii);
                let t = ((blur - d) / (2.0 * blur)).clamp(0.0, 1.0);
                let coverage = t * t * (3.0 - 2.0 * t);
                if coverage > 0.0 { self.blend(x, y, color, coverage, clip); }
            }
        }
    }

    fn draw_line(&mut self, a: Vec2, b: Vec2, width: f32, color: Color) {
        let s = self.scale;
        let clip = self.clip();
        let a = a * s;
        let b = b * s;
        let hw = (width * s * 0.5).max(0.5);
        let pad = hw + 1.0;
        let x0 = ((a.x.min(b.x) - pad).floor() as i32).max(clip[0]);
        let y0 = ((a.y.min(b.y) - pad).floor() as i32).max(clip[1]);
        let x1 = ((a.x.max(b.x) + pad).ceil() as i32).min(clip[2]);
        let y1 = ((a.y.max(b.y) + pad).ceil() as i32).min(clip[3]);
        for y in y0..y1 {
            for x in x0..x1 {
                let d = segment_distance(Vec2::new(x as f32 + 0.5, y as f32 + 0.5), a, b);
                let cov = (hw - d + 0.5).clamp(0.0, 1.0);
                if cov > 0.0 {
                    self.blend(x, y, color, cov, clip);
                }
            }
        }
    }

    fn blit_mask(&mut self, mask: &Mask, x: i32, y: i32, color: Color) {
        let clip = self.clip();
        for my in 0..mask.height as i32 {
            for mx in 0..mask.width as i32 {
                let c = mask.data[(my as u32 * mask.width + mx as u32) as usize];
                if c > 0 {
                    self.blend(x + mx, y + my, color, c as f32 / 255.0, clip);
                }
            }
        }
    }

    fn draw_text(&mut self, text: &str, pos: Vec2, size: f32, color: Color, face: Face) {
        let s = self.scale;
        let px = size * s;
        let Some(layout) = text::shape_face(text, px, face) else {
            self.draw_bitmap_text(text, pos, size, color, face == Face::Strong);
            return;
        };
        let origin_x = pos.x * s;
        let baseline = (pos.y * s + layout.cap_height).round();
        for g in &layout.glyphs {
            let key = GlyphKey::new(g.font, g.id, px);
            if let Some(mask) = self.glyphs.get(key) {
                let gx = ((origin_x + g.x).clamp(-100000.0, 100000.0).round() as i32)
                    .saturating_add(mask.left);
                let gy = ((baseline + g.y).clamp(-100000.0, 100000.0).round() as i32)
                    .saturating_add(mask.top);
                self.blit_mask(&mask, gx, gy, color);
            }
        }
    }

    /// Last-resort 5x7 bitmap text when no outline font could be loaded.
    fn draw_bitmap_text(&mut self, text: &str, pos: Vec2, size: f32, color: Color, strong: bool) {
        let clip = self.clip();
        let scale = ((size * self.scale) / 11.0).round().max(1.0) as i32;
        let mut cur_x = (pos.x * self.scale) as i32;
        let cur_y = (pos.y * self.scale) as i32;
        for ch in text.chars() {
            if let Some(glyph) = crate::cpu_font::glyph_for_char(ch) {
                for (col_idx, &col_byte) in glyph.iter().enumerate() {
                    for row_idx in 0..7 {
                        if (col_byte & (1 << row_idx)) != 0 {
                            let px = cur_x + col_idx as i32 * scale;
                            let py = cur_y + row_idx * scale;
                            let extra = if strong { 1 } else { 0 };
                            self.fill_px_rect(px, py, px + scale + extra, py + scale, color, clip);
                        }
                    }
                }
            }
            cur_x += 6 * scale;
        }
    }

    fn draw_icon(&mut self, id: &'static str, svg: &'static str, rect: Rect, color: Color) {
        let s = self.scale;
        let w = (rect.width() * s).round() as u32;
        let h = (rect.height() * s).round() as u32;
        if let Some(mask) = self.icons.get(
            IconKey {
                id,
                width: w,
                height: h,
            },
            svg,
        ) {
            let x = (rect.min.x * s).round() as i32;
            let y = (rect.min.y * s).round() as i32;
            self.blit_mask(&mask, x, y, color);
        }
    }
}

impl Default for CpuBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl RenderBackend for CpuBackend {
    fn kind(&self) -> BackendKind {
        BackendKind::SoftwareCpuSafeMode
    }

    fn set_scale_factor(&mut self, scale: f32) {
        if scale.is_finite() && scale > 0.0 {
            self.scale = scale;
        }
    }

    fn begin_frame(&mut self, width: u32, height: u32, clear_color: Color) {
        self.width = width;
        self.height = height;
        let opaque = 0xFF00_0000 | (clear_color.to_u32() & 0x00FF_FFFF);
        let count = (width as usize) * (height as usize);
        self.pixels.clear();
        self.pixels.resize(count, opaque);
        self.clip_stack.clear();
        self.stats.draw_calls = 0;
        self.stats.primitives_count = 0;
        self.stats.vertices = 0;
    }

    fn submit(&mut self, buffer: &CommandBuffer) -> RenderStats {
        let start = Instant::now();
        for cmd in buffer.commands() {
            match cmd {
                DrawCommand::Rect {
                    rect,
                    fill,
                    radius,
                    stroke,
                } => self.draw_rect(*rect, *fill, *radius, *stroke),
                DrawCommand::Surface { rect, fill_top, fill_bottom, radius, stroke, rim_strength } =>
                    self.draw_surface(*rect, *fill_top, *fill_bottom, *radius, *stroke, *rim_strength),
                DrawCommand::SoftShadow { rect, radius, color, offset, blur, spread } =>
                    self.draw_soft_shadow(*rect, *radius, *color, *offset, *blur, *spread),
                DrawCommand::Line { start, end, stroke } => {
                    self.draw_line(*start, *end, stroke.width, stroke.color)
                }
                DrawCommand::CubicBezier {
                    start,
                    ctrl1,
                    ctrl2,
                    end,
                    stroke,
                } => {
                    let pts = flatten_cubic(*start, *ctrl1, *ctrl2, *end);
                    for w in pts.windows(2) {
                        self.draw_line(w[0], w[1], stroke.width, stroke.color);
                    }
                }
                DrawCommand::Text {
                    text,
                    pos,
                    size,
                    color,
                    strong,
                } => self.draw_text(text, *pos, *size, *color, Face::from_strong(*strong)),
                DrawCommand::Code {
                    text,
                    pos,
                    size,
                    color,
                } => self.draw_text(text, *pos, *size, *color, Face::Mono),
                DrawCommand::Icon {
                    id,
                    svg,
                    rect,
                    color,
                } => self.draw_icon(id, svg, *rect, *color),
                DrawCommand::Image {
                    rect,
                    texture_id,
                    uv_rect,
                    tint,
                } => self.draw_image(*texture_id, *rect, *uv_rect, *tint),
                DrawCommand::ClipPush { rect } => {
                    let s = self.scale;
                    let parent = self.clip();
                    let c = [
                        ((rect.min.x * s).floor() as i32).max(parent[0]),
                        ((rect.min.y * s).floor() as i32).max(parent[1]),
                        ((rect.max.x * s).ceil() as i32).min(parent[2]),
                        ((rect.max.y * s).ceil() as i32).min(parent[3]),
                    ];
                    self.clip_stack.push(c);
                }
                DrawCommand::ClipPop => {
                    self.clip_stack.pop();
                }
            }
        }
        self.stats.draw_calls = 1;
        self.stats.primitives_count += buffer.len();
        self.stats.vertices = self.pixels.len();
        self.stats.frame_time_us = start.elapsed().as_micros() as u64;
        self.stats.backend = Some(BackendKind::SoftwareCpuSafeMode);
        self.stats
    }

    fn end_frame(&mut self) -> Result<RenderStats, String> {
        Ok(self.stats)
    }

    fn recover(&mut self) -> Result<(), String> {
        // CPU fallback is stateless with respect to drivers and never loses its device.
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ui_core::Stroke;

    fn px(cpu: &CpuBackend, x: u32, y: u32) -> u32 {
        cpu.pixels()[(y * cpu.width() + x) as usize]
    }

    #[test]
    fn cpu_backend_samples_registered_rgba_viewport_frames() {
        let mut cpu = CpuBackend::new();
        cpu.upload_rgba8_image(42, 2, 1, &[255, 0, 0, 255, 0, 255, 0, 255])
            .unwrap();
        cpu.begin_frame(8, 4, Color::BLACK);
        let mut draw = CommandBuffer::new();
        draw.push(DrawCommand::Image {
            rect: Rect::from_min_size(Vec2::ZERO, Vec2::new(8.0, 4.0)),
            texture_id: 42,
            uv_rect: Rect::from_min_max(Vec2::ZERO, Vec2::new(1.0, 1.0)),
            tint: Color::WHITE,
        });
        cpu.submit(&draw);
        assert_eq!(px(&cpu, 1, 2), Color::rgb(255, 0, 0).to_u32());
        assert_eq!(px(&cpu, 6, 2), Color::rgb(0, 255, 0).to_u32());
    }

    #[test]
    fn premium_surface_gradient_and_shadow_are_cpu_visible() {
        let mut cpu = CpuBackend::new();
        cpu.begin_frame(96, 96, Color::BLACK);
        let mut buf = CommandBuffer::new();
        let rect = Rect::from_min_size(Vec2::new(16.0, 16.0), Vec2::new(64.0, 48.0));
        buf.push(DrawCommand::SoftShadow {
            rect,
            radius: CornerRadius::same(9.0),
            color: Color::rgba(255, 0, 0, 160),
            offset: Vec2::new(0.0, 6.0),
            blur: 8.0,
            spread: 0.0,
        });
        buf.push(DrawCommand::Surface {
            rect,
            fill_top: Color::rgb(64, 56, 47),
            fill_bottom: Color::rgb(28, 25, 24),
            radius: CornerRadius::same(9.0),
            stroke: ui_core::Stroke::new(1.0, Color::rgb(95, 88, 72)),
            rim_strength: 0.65,
        });
        cpu.submit(&buf);
        assert_ne!(px(&cpu, 40, 22), px(&cpu, 40, 56), "gradient must vary by row");
        assert_ne!(px(&cpu, 40, 70), Color::BLACK.to_u32(), "soft shadow extends below surface");
        assert_eq!(px(&cpu, 2, 2), Color::BLACK.to_u32(), "shadow stays localized");
    }

    #[test]
    fn cpu_backend_rasterizes_rects_cleanly() {
        let mut cpu = CpuBackend::new();
        cpu.begin_frame(100, 100, Color::BLACK);
        let mut buf = CommandBuffer::new();
        buf.push(DrawCommand::Rect {
            rect: Rect::from_min_size(Vec2::new(10.0, 10.0), Vec2::new(20.0, 20.0)),
            fill: Color::WHITE,
            radius: CornerRadius::ZERO,
            stroke: Stroke::NONE,
        });
        cpu.submit(&buf);
        assert!(cpu.end_frame().is_ok());
        assert_eq!(px(&cpu, 15, 15), Color::WHITE.to_u32());
        assert_eq!(px(&cpu, 5, 5), Color::BLACK.to_u32());
    }

    #[test]
    fn rounded_corners_are_antialiased_and_clip_is_respected() {
        let mut cpu = CpuBackend::new();
        cpu.begin_frame(64, 64, Color::BLACK);
        let mut buf = CommandBuffer::new();
        buf.push(DrawCommand::ClipPush {
            rect: Rect::from_min_size(Vec2::ZERO, Vec2::new(32.0, 64.0)),
        });
        buf.push(DrawCommand::Rect {
            rect: Rect::from_min_size(Vec2::new(4.0, 4.0), Vec2::new(56.0, 56.0)),
            fill: Color::WHITE,
            radius: CornerRadius::same(12.0),
            stroke: Stroke::NONE,
        });
        buf.push(DrawCommand::ClipPop);
        cpu.submit(&buf);
        assert_eq!(px(&cpu, 4, 4), Color::BLACK.to_u32(), "corner cut away");
        let edge = px(&cpu, 6, 9) & 0xFF;
        assert!(edge > 0 && edge < 255, "anti-aliased corner, got {edge}");
        assert_eq!(px(&cpu, 20, 30), Color::WHITE.to_u32());
        assert_eq!(px(&cpu, 40, 30), Color::BLACK.to_u32(), "clipped half");
    }

    #[test]
    fn text_renders_real_glyph_coverage_at_dpi_scale() {
        let mut cpu = CpuBackend::new();
        cpu.set_scale_factor(2.0);
        cpu.begin_frame(400, 80, Color::BLACK);
        let mut buf = CommandBuffer::new();
        buf.push(DrawCommand::Text {
            text: "Kolpona কল্পনা".into(),
            pos: Vec2::new(4.0, 8.0),
            size: 16.0,
            color: Color::WHITE,
            strong: false,
        });
        cpu.submit(&buf);
        let lit: Vec<u32> = cpu
            .pixels()
            .iter()
            .map(|p| p & 0xFF)
            .filter(|&c| c > 0)
            .collect();
        assert!(lit.len() > 400, "text must paint pixels");
        assert!(
            lit.iter().any(|&c| c < 255),
            "glyph edges must be anti-aliased"
        );
    }
}
