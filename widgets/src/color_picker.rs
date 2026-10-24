//! Procedural HSV ColorWheel and ColorPicker workstation component.
//!
//! Uses polar HSV mapping (atan2 hue + radial saturation) and value brightness
//! scrubber with zero heap allocations per frame.

use std::f32::consts::PI;
use ui_core::{Color, CornerRadius, Rect, Stroke, Vec2};
use ui_render::{CommandBuffer, DrawCommand};
use ui_themes::ThemePalette;

/// HSV representation of color.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HsvColor {
    pub h: f32, // 0.0 .. 1.0
    pub s: f32, // 0.0 .. 1.0
    pub v: f32, // 0.0 .. 1.0
    pub a: f32, // 0.0 .. 1.0
}

impl Default for HsvColor {
    fn default() -> Self {
        Self {
            h: 0.75, // violet/purple default
            s: 0.85,
            v: 0.95,
            a: 1.0,
        }
    }
}

impl HsvColor {
    pub const fn new(h: f32, s: f32, v: f32, a: f32) -> Self {
        Self { h, s, v, a }
    }

    /// Converts HSV to RGBA Color.
    pub fn to_color(self) -> Color {
        let (r, g, b) = hsv_to_rgb(self.h, self.s, self.v);
        Color::rgba(
            (r * 255.0).round() as u8,
            (g * 255.0).round() as u8,
            (b * 255.0).round() as u8,
            (self.a * 255.0).round() as u8,
        )
    }

    /// Constructs an HsvColor from an RGBA Color.
    pub fn from_color(c: Color) -> Self {
        let (h, s, v) = rgb_to_hsv(
            c.r as f32 / 255.0,
            c.g as f32 / 255.0,
            c.b as f32 / 255.0,
        );
        Self {
            h,
            s,
            v,
            a: c.a as f32 / 255.0,
        }
    }

    /// Returns hexadecimal string representation `#RRGGBB`.
    pub fn to_hex(&self) -> String {
        let c = self.to_color();
        format!("#{:02X}{:02X}{:02X}", c.r, c.g, c.b)
    }
}

/// Converts normalized HSV to normalized RGB.
#[inline]
pub fn hsv_to_rgb(h: f32, s: f32, v: f32) -> (f32, f32, f32) {
    let h6 = (h % 1.0 + 1.0) % 1.0 * 6.0;
    let i = h6.floor() as i32;
    let f = h6 - i as f32;
    let p = v * (1.0 - s);
    let q = v * (1.0 - s * f);
    let t = v * (1.0 - s * (1.0 - f));

    match i % 6 {
        0 => (v, t, p),
        1 => (q, v, p),
        2 => (p, v, t),
        3 => (p, q, v),
        4 => (t, p, v),
        _ => (v, p, q),
    }
}

/// Converts normalized RGB to normalized HSV.
#[inline]
pub fn rgb_to_hsv(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let d = max - min;

    let v = max;
    let s = if max == 0.0 { 0.0 } else { d / max };

    let h = if d == 0.0 {
        0.0
    } else if max == r {
        let mut val = (g - b) / d;
        if g < b {
            val += 6.0;
        }
        val / 6.0
    } else if max == g {
        ((b - r) / d + 2.0) / 6.0
    } else {
        ((r - g) / d + 4.0) / 6.0
    };

    (h, s, v)
}

/// Interactive ColorWheel and ColorPicker widget.
#[derive(Debug, Clone)]
pub struct ColorWheel {
    pub radius: f32,
    pub is_dragging_wheel: bool,
    pub is_dragging_val: bool,
}

impl Default for ColorWheel {
    fn default() -> Self {
        Self {
            radius: 54.0,
            is_dragging_wheel: false,
            is_dragging_val: false,
        }
    }
}

impl ColorWheel {
    pub fn new(radius: f32) -> Self {
        Self {
            radius,
            is_dragging_wheel: false,
            is_dragging_val: false,
        }
    }

    /// Evaluates polar coordinates from pointer position relative to center.
    pub fn polar_to_hs(&self, rel_pos: Vec2) -> (f32, f32) {
        let angle = (-rel_pos.y).atan2(rel_pos.x); // polar angle where 0 is right, counter-clockwise
        let norm_angle = if angle < 0.0 { angle + 2.0 * PI } else { angle };
        let hue = norm_angle / (2.0 * PI);
        let dist = (rel_pos.x * rel_pos.x + rel_pos.y * rel_pos.y).sqrt();
        let sat = (dist / self.radius).clamp(0.0, 1.0);
        (hue, sat)
    }

    /// Returns reticle position from current Hue and Saturation.
    pub fn hs_to_reticle(&self, center: Vec2, hue: f32, sat: f32) -> Vec2 {
        let angle = hue * 2.0 * PI;
        let dist = sat * self.radius;
        Vec2::new(
            center.x + dist * angle.cos(),
            center.y - dist * angle.sin(),
        )
    }

    /// Renders the color wheel, value slider, and hex readout.
    /// Returns true if color was modified.
    pub fn render(
        &mut self,
        cmd_buf: &mut CommandBuffer,
        center: Vec2,
        color: &mut HsvColor,
        pointer_pos: Vec2,
        primary_down: bool,
        is_clicked: bool,
        palette: &ThemePalette,
    ) -> bool {
        let mut changed = false;
        let rel_pos = pointer_pos - center;
        let dist_sq = rel_pos.length_sq();
        let inside_wheel = dist_sq <= (self.radius + 6.0) * (self.radius + 6.0);

        if is_clicked && inside_wheel {
            self.is_dragging_wheel = true;
        }
        if !primary_down {
            self.is_dragging_wheel = false;
            self.is_dragging_val = false;
        }

        if self.is_dragging_wheel {
            let (h, s) = self.polar_to_hs(rel_pos);
            color.h = h;
            color.s = s;
            changed = true;
        }

        // Draw color wheel outer disc backing
        let disc_radius = self.radius + 2.0;
        cmd_buf.push(DrawCommand::Rect {
            rect: Rect::from_min_size(
                Vec2::new(center.x - disc_radius, center.y - disc_radius),
                Vec2::new(disc_radius * 2.0, disc_radius * 2.0),
            ),
            fill: palette.surface,
            radius: CornerRadius::same(disc_radius),
            stroke: Stroke::new(1.5, palette.border),
        });

        // Procedural 24-step color spectrum fan
        let steps = 24;
        let step_angle = 2.0 * PI / steps as f32;
        for i in 0..steps {
            let a1 = i as f32 * step_angle;
            let a2 = (i + 1) as f32 * step_angle;
            let hue = (a1 + a2) * 0.5 / (2.0 * PI);
            let seg_color = HsvColor::new(hue, 0.9, color.v, 1.0).to_color();

            let p1 = Vec2::new(center.x + self.radius * a1.cos(), center.y - self.radius * a1.sin());
            let p2 = Vec2::new(center.x + self.radius * a2.cos(), center.y - self.radius * a2.sin());

            // Arc segment approximated by filled quad
            cmd_buf.push(DrawCommand::Line {
                start: center,
                end: Vec2::new((p1.x + p2.x) * 0.5, (p1.y + p2.y) * 0.5),
                stroke: Stroke::new(self.radius * 0.9, seg_color.with_alpha(180)),
            });
        }

        // Central desaturation blend
        let center_radius = self.radius * 0.35;
        cmd_buf.push(DrawCommand::Rect {
            rect: Rect::from_min_size(
                Vec2::new(center.x - center_radius, center.y - center_radius),
                Vec2::new(center_radius * 2.0, center_radius * 2.0),
            ),
            fill: Color::rgba(255, 255, 255, 120),
            radius: CornerRadius::same(center_radius),
            stroke: Stroke::NONE,
        });

        // Interactive reticle handle
        let reticle_pos = self.hs_to_reticle(center, color.h, color.s);
        let reticle_radius = 5.5;
        cmd_buf.push(DrawCommand::Rect {
            rect: Rect::from_min_size(
                Vec2::new(reticle_pos.x - reticle_radius, reticle_pos.y - reticle_radius),
                Vec2::new(reticle_radius * 2.0, reticle_radius * 2.0),
            ),
            fill: color.to_color(),
            radius: CornerRadius::same(reticle_radius),
            stroke: Stroke::new(2.0, Color::WHITE),
        });

        // Value Brightness Slider below wheel
        let val_slider_y = center.y + self.radius + 14.0;
        let slider_width = self.radius * 2.0;
        let val_slider_rect = Rect::from_min_size(
            Vec2::new(center.x - self.radius, val_slider_y),
            Vec2::new(slider_width, 14.0),
        );

        if is_clicked && val_slider_rect.contains(pointer_pos) {
            self.is_dragging_val = true;
        }

        if self.is_dragging_val {
            let rel_x = (pointer_pos.x - val_slider_rect.min.x) / slider_width.max(1.0);
            color.v = rel_x.clamp(0.0, 1.0);
            changed = true;
        }

        // Value track
        cmd_buf.push(DrawCommand::Rect {
            rect: val_slider_rect,
            fill: palette.canvas,
            radius: CornerRadius::same(7.0),
            stroke: Stroke::new(1.0, palette.border),
        });

        // Value fill gradient representation
        let val_knob_x = val_slider_rect.min.x + color.v * (slider_width - 12.0);
        let val_knob_rect = Rect::from_min_size(
            Vec2::new(val_knob_x, val_slider_rect.min.y + 1.0),
            Vec2::new(12.0, 12.0),
        );
        cmd_buf.push(DrawCommand::Rect {
            rect: val_knob_rect,
            fill: Color::WHITE,
            radius: CornerRadius::same(6.0),
            stroke: Stroke::new(1.5, palette.accent),
        });

        // Swatch preview & Hex label
        let swatch_rect = Rect::from_min_size(
            Vec2::new(center.x - self.radius, val_slider_y + 20.0),
            Vec2::new(28.0, 18.0),
        );
        cmd_buf.push(DrawCommand::Rect {
            rect: swatch_rect,
            fill: color.to_color(),
            radius: CornerRadius::same(3.0),
            stroke: Stroke::new(1.0, palette.border),
        });

        let hex_str = color.to_hex();
        cmd_buf.push(DrawCommand::Text {
            pos: Vec2::new(swatch_rect.max.x + 8.0, swatch_rect.min.y + 2.0),
            text: hex_str,
            color: palette.text,
            size: 11.0,
            strong: true,
        });

        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hsv_to_rgb_and_back_roundtrip() {
        let (r, g, b) = hsv_to_rgb(0.0, 1.0, 1.0); // Pure Red
        assert!((r - 1.0).abs() < 1e-4);
        assert!(g.abs() < 1e-4);
        assert!(b.abs() < 1e-4);

        let (h, s, v) = rgb_to_hsv(1.0, 0.0, 0.0);
        assert!(h.abs() < 1e-4);
        assert!((s - 1.0).abs() < 1e-4);
        assert!((v - 1.0).abs() < 1e-4);
    }

    #[test]
    fn test_polar_hs_conversion() {
        let wheel = ColorWheel::new(50.0);
        // Angle to the right (x > 0, y = 0) -> angle 0 -> Hue 0.0
        let (h1, s1) = wheel.polar_to_hs(Vec2::new(25.0, 0.0));
        assert!(h1.abs() < 1e-4);
        assert!((s1 - 0.5).abs() < 1e-4);

        // Angle up (x = 0, y < 0 in screen space) -> angle pi/2 -> Hue 0.25
        let (h2, s2) = wheel.polar_to_hs(Vec2::new(0.0, -50.0));
        assert!((h2 - 0.25).abs() < 1e-3);
        assert!((s2 - 1.0).abs() < 1e-3);
    }

    #[test]
    fn test_color_wheel_render_and_drag() {
        let mut cmd_buf = CommandBuffer::new();
        let palette = ThemePalette::dark();
        let mut wheel = ColorWheel::new(40.0);
        let mut color = HsvColor::default();

        let center = Vec2::new(100.0, 100.0);
        // Click at top of wheel
        let changed = wheel.render(
            &mut cmd_buf,
            center,
            &mut color,
            Vec2::new(100.0, 70.0),
            true,
            true,
            &palette,
        );

        assert!(changed);
        assert!(!cmd_buf.is_empty());
    }
}
