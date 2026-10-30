//! Painting for the mascot: body, head feature, face and mood overlays.
//!
//! Everything is placed in body units around the rect centre (1.0 = half the
//! mascot size), so one layout serves 20 px chat avatars and 72 px hero cards.
//! Shapes are rounded rects (ovals) and round-capped lines (capsules).

use ui_core::{Color, CornerRadius, Rect, Stroke, Vec2};
use ui_render::DrawCommand;

use super::mascot::{EyeShape, Feature, Identity, Mood, Pose};
use crate::studio::{centered, Studio};

/// Eye / mouth ink. Dark on every body colour, in both themes.
const INK: Color = Color::rgb(0x26, 0x21, 0x2E);
const BLUSH: Color = Color::rgba(0xFF, 0x6F, 0x8A, 115);
const LEAF: Color = Color::rgb(0x5D, 0xBB, 0x63);
const BADGE: Color = Color::rgb(0xFF, 0xB5, 0x47);

struct Pen<'s, 'a> {
    s: &'s mut Studio<'a>,
    c: Vec2,
    u: f32,
}

impl Pen<'_, '_> {
    fn at(&self, x: f32, y: f32) -> Vec2 {
        self.c + Vec2::new(x * self.u, y * self.u)
    }

    fn oval(&mut self, x: f32, y: f32, w: f32, h: f32, color: Color) {
        let size = Vec2::new(w * self.u, h * self.u);
        let rect = centered(self.at(x, y), size);
        self.s.fill(rect, color, size.x.min(size.y) * 0.5);
    }

    fn width(&self, w: f32) -> f32 {
        (w * self.u).max(1.1)
    }

    fn line(&mut self, a: (f32, f32), b: (f32, f32), w: f32, color: Color) {
        let stroke = Stroke::new(self.width(w), color);
        self.s.buf.push(DrawCommand::Line {
            start: self.at(a.0, a.1),
            end: self.at(b.0, b.1),
            stroke,
        });
    }

    fn curve(&mut self, pts: [(f32, f32); 4], w: f32, color: Color) {
        let stroke = Stroke::new(self.width(w), color);
        self.s.buf.push(DrawCommand::CubicBezier {
            start: self.at(pts[0].0, pts[0].1),
            ctrl1: self.at(pts[1].0, pts[1].1),
            ctrl2: self.at(pts[2].0, pts[2].1),
            end: self.at(pts[3].0, pts[3].1),
            stroke,
        });
    }
}

/// Body box in units: centre x/y, width, height.
struct Body {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

impl Body {
    fn top(&self) -> f32 {
        self.y - self.h * 0.5
    }
}

pub(super) fn draw(s: &mut Studio, rect: Rect, id: &Identity, mood: Mood, p: &Pose) {
    let u = rect.width().min(rect.height()) * 0.5;
    let mut pen = Pen {
        s,
        c: rect.center(),
        u,
    };
    let body_color = if mood == Mood::Sleeping {
        id.body.lerp(Color::rgb(0x9A, 0x9A, 0xA6), 0.35)
    } else {
        id.body
    };
    let (w, h) = (1.56 * (1.0 + p.squash), 1.30 * (1.0 - p.squash));
    let body = Body {
        x: p.sway,
        y: 0.84 - p.lift - h * 0.5,
        w,
        h,
    };

    // Ground shadow shrinks as the body lifts off.
    let spread = (1.0 - p.lift * 2.2).clamp(0.4, 1.0);
    shadow(
        &mut pen,
        body.x,
        1.05 * spread * (1.0 + p.squash),
        0.35 + 0.65 * spread,
    );

    feature(&mut pen, id, &body, body_color, p.wiggle);
    if mood == Mood::NeedsYou {
        arm(&mut pen, &body, body_color, p.arm);
    }
    body_shape(&mut pen, &body, body_color);
    face(&mut pen, id, mood, &body, p);
    overlays(&mut pen, mood, &body, p);
}

/// Soft contact shadow: feathered so it reads as a shadow, not a plinth.
fn shadow(pen: &mut Pen, x: f32, w: f32, strength: f32) {
    let size = Vec2::new(w * pen.u, 0.10 * pen.u);
    let rect = centered(pen.at(x, 0.87), size);
    pen.s.buf.push(DrawCommand::SoftShadow {
        rect,
        radius: CornerRadius::same(size.y * 0.5),
        color: Color::rgba(0, 0, 0, (70.0 * strength) as u8),
        offset: Vec2::ZERO,
        blur: (0.10 * pen.u).max(1.5),
        spread: 0.0,
    });
}

fn darker(c: Color, t: f32) -> Color {
    c.lerp(Color::rgb(0x1E, 0x14, 0x24), t)
}

fn lighter(c: Color, t: f32) -> Color {
    c.lerp(Color::WHITE, t)
}

fn body_shape(pen: &mut Pen, b: &Body, color: Color) {
    let size = Vec2::new(b.w * pen.u, b.h * pen.u);
    let rect = centered(pen.at(b.x, b.y), size);
    let edge = Stroke::new((0.07 * pen.u).max(1.0), darker(color, 0.5));
    pen.s.buf.push(DrawCommand::Surface {
        rect,
        fill_top: lighter(color, 0.22),
        fill_bottom: darker(color, 0.06),
        radius: CornerRadius::same(size.y * 0.47),
        stroke: edge,
        rim_strength: 0.55,
    });
    // Soft sheen on the upper left: reads as a rounded, glossy toy.
    pen.oval(
        b.x - b.w * 0.24,
        b.top() + b.h * 0.2,
        0.34,
        0.16,
        Color::rgba(255, 255, 255, 70),
    );
}

/// Head decoration, drawn before the body so it tucks behind the head.
fn feature(pen: &mut Pen, id: &Identity, b: &Body, color: Color, wiggle: f32) {
    let (x, top) = (b.x, b.top());
    let dark = darker(color, 0.5);
    let inner = lighter(color, 0.45);
    match id.feature {
        Feature::RoundEars => {
            for side in [-1.0, 1.0] {
                let ex = x + side * b.w * 0.33;
                pen.oval(ex, top + 0.06, 0.54, 0.54, dark);
                pen.oval(ex, top + 0.06, 0.42, 0.42, color);
                pen.oval(ex, top + 0.08, 0.22, 0.22, inner);
            }
        }
        Feature::CatEars => {
            for side in [-1.0, 1.0] {
                let base = x + side * b.w * 0.30;
                let tip = (base + side * 0.10 + wiggle * 0.02, top - 0.34);
                for (wid, col) in [(0.30, dark), (0.20, color)] {
                    for dx in [-0.16, 0.0, 0.16] {
                        pen.line((base + dx, top + 0.16), tip, wid, col);
                    }
                }
                pen.line(
                    (base, top + 0.08),
                    (tip.0 - side * 0.02, tip.1 + 0.16),
                    0.10,
                    inner,
                );
            }
        }
        Feature::BunnyEars => {
            for side in [-1.0, 1.0] {
                let base = (x + side * 0.24, top + 0.14);
                let tip = (x + side * 0.40 + wiggle * 0.05, top - 0.52);
                pen.line(base, tip, 0.34, dark);
                pen.line(base, tip, 0.25, color);
                pen.line((base.0, base.1 - 0.06), (tip.0, tip.1 + 0.04), 0.10, inner);
            }
        }
        Feature::Antenna => {
            let tip = (x + wiggle * 0.12, top - 0.36);
            pen.line((x, top + 0.06), tip, 0.08, dark);
            pen.oval(tip.0, tip.1, 0.30, 0.30, darker(id.pop, 0.45));
            pen.oval(tip.0, tip.1, 0.22, 0.22, id.pop);
            pen.oval(
                tip.0 - 0.04,
                tip.1 - 0.04,
                0.08,
                0.08,
                Color::rgba(255, 255, 255, 200),
            );
        }
        Feature::Sprout => {
            let tip = (x + wiggle * 0.05, top - 0.24);
            pen.line((x, top + 0.04), tip, 0.08, darker(LEAF, 0.35));
            for side in [-1.0, 1.0] {
                let leaf = (tip.0 + side * 0.24, tip.1 - 0.10 - side * wiggle * 0.03);
                pen.line(tip, leaf, 0.22, darker(LEAF, 0.35));
                pen.line(tip, leaf, 0.14, LEAF);
            }
        }
        Feature::Tuft => {
            for (dx, dy, r) in [(-0.17, 0.04, 0.30), (0.17, 0.04, 0.30), (0.0, -0.06, 0.34)] {
                let tx = x + dx + wiggle * 0.03 * (1.0 + dy * 10.0);
                pen.oval(tx, top + dy, r + 0.07, r + 0.07, dark);
                pen.oval(tx, top + dy, r, r, id.pop);
            }
        }
    }
}

/// Raised waving arm on the right side, for NeedsYou.
fn arm(pen: &mut Pen, b: &Body, color: Color, raise: f32) {
    let shoulder = (b.x + b.w * 0.36, b.y + 0.02);
    let angle = 0.35 + 0.55 * (1.0 - raise);
    let hand = (
        shoulder.0 + 0.48 * angle.sin(),
        shoulder.1 - 0.48 * angle.cos(),
    );
    pen.line(shoulder, hand, 0.26, darker(color, 0.5));
    pen.line(shoulder, hand, 0.17, color);
    pen.oval(hand.0, hand.1, 0.26, 0.26, darker(color, 0.5));
    pen.oval(hand.0, hand.1, 0.18, 0.18, lighter(color, 0.15));
}

fn face(pen: &mut Pen, id: &Identity, mood: Mood, b: &Body, p: &Pose) {
    let look = Vec2::new(p.look.x.clamp(-1.0, 1.0), p.look.y.clamp(-1.0, 1.0));
    let fx = b.x + look.x * 0.10;
    let fy = b.y - b.h * 0.04 + look.y * 0.06;
    let gap = 0.31 * (1.0 + p.squash);
    for side in [-1.0, 1.0] {
        pen.oval(fx + side * (gap + 0.06), fy + 0.20, 0.24, 0.13, BLUSH);
    }
    for side in [-1.0, 1.0] {
        let ex = fx + side * gap;
        match mood {
            Mood::Error => {
                let r = 0.09;
                pen.line((ex - r, fy - r), (ex + r, fy + r), 0.075, INK);
                pen.line((ex - r, fy + r), (ex + r, fy - r), 0.075, INK);
            }
            _ if p.blink > 0.75 => {
                let r = 0.11;
                let pts = [
                    (ex - r, fy),
                    (ex - r * 0.5, fy + 0.08),
                    (ex + r * 0.5, fy + 0.08),
                    (ex + r, fy),
                ];
                pen.curve(pts, 0.065, INK);
            }
            _ => eye(pen, id.eyes, ex, fy, p.blink, look),
        }
    }
    mouth(pen, mood, fx, fy + 0.25);
}

fn eye(pen: &mut Pen, shape: EyeShape, x: f32, y: f32, blink: f32, look: Vec2) {
    let (w, h) = match shape {
        EyeShape::Round => (0.25, 0.29),
        EyeShape::Tall => (0.21, 0.34),
        EyeShape::Wide => (0.29, 0.24),
    };
    let h = h * (1.0 - 0.8 * blink);
    pen.oval(x, y, w, h, INK);
    if blink < 0.4 {
        let (hx, hy) = (x - w * 0.18 + look.x * 0.03, y - h * 0.2 + look.y * 0.02);
        pen.oval(hx, hy, w * 0.42, w * 0.42, Color::WHITE);
        pen.oval(
            x + w * 0.2,
            y + h * 0.24,
            w * 0.18,
            w * 0.18,
            Color::rgba(255, 255, 255, 180),
        );
    }
}

fn mouth(pen: &mut Pen, mood: Mood, x: f32, y: f32) {
    match mood {
        Mood::Ready => {
            let pts = [
                (x - 0.11, y - 0.02),
                (x - 0.05, y + 0.07),
                (x + 0.05, y + 0.07),
                (x + 0.11, y - 0.02),
            ];
            pen.curve(pts, 0.06, INK);
        }
        Mood::Working => {
            let pts = [
                (x - 0.07, y),
                (x - 0.03, y + 0.035),
                (x + 0.03, y + 0.035),
                (x + 0.07, y),
            ];
            pen.curve(pts, 0.06, INK);
        }
        Mood::NeedsYou => {
            pen.oval(x, y + 0.02, 0.17, 0.15, INK);
            pen.oval(x, y + 0.055, 0.09, 0.05, Color::rgb(0xFF, 0x8A, 0x9A));
        }
        Mood::Error => {
            let pts = [
                (x - 0.12, y + 0.04),
                (x - 0.04, y - 0.05),
                (x + 0.04, y + 0.09),
                (x + 0.12, y),
            ];
            pen.curve(pts, 0.055, INK);
        }
        Mood::Sleeping => pen.oval(x, y + 0.01, 0.08, 0.07, INK),
    }
}

fn overlays(pen: &mut Pen, mood: Mood, b: &Body, p: &Pose) {
    match mood {
        Mood::NeedsYou => {
            let (bx, by) = (b.x - 0.66, b.top() - 0.12 - 0.05 * p.float);
            pen.oval(bx, by, 0.50, 0.50, darker(BADGE, 0.55));
            pen.oval(bx, by, 0.42, 0.42, BADGE);
            pen.line((bx, by - 0.10), (bx, by + 0.03), 0.085, INK);
            pen.oval(bx, by + 0.115, 0.09, 0.09, INK);
        }
        Mood::Error => {
            let star = pen.s.palette.warning;
            for k in 0..3 {
                let a = p.spin + k as f32 * std::f32::consts::TAU / 3.0;
                let (sx, sy) = (b.x + 0.62 * a.cos(), b.top() - 0.06 + 0.16 * a.sin());
                let r = 0.09 + 0.02 * a.sin();
                // Dark rim first so the stars read over a yellow body too.
                for (w, c) in [(0.11, darker(star, 0.6)), (0.06, star)] {
                    pen.line((sx - r, sy), (sx + r, sy), w, c);
                    pen.line((sx, sy - r), (sx, sy + r), w, c);
                }
            }
        }
        Mood::Sleeping => {
            let ink = pen.s.palette.text_muted;
            for (k, size) in [(0.0, 0.26), (0.5, 0.18)] {
                let f = (p.float + k).fract();
                let alpha = (255.0 * (1.0 - f).min(f * 4.0).min(1.0)) as u8;
                let (zx, zy) = (b.x + 0.62 + 0.18 * f, b.top() - 0.02 - 0.55 * f);
                z_letter(pen, zx, zy, size * (0.75 + 0.25 * f), ink.with_alpha(alpha));
            }
        }
        Mood::Ready | Mood::Working => {}
    }
}

fn z_letter(pen: &mut Pen, x: f32, y: f32, s: f32, color: Color) {
    let (l, r, t, btm) = (x - s * 0.5, x + s * 0.5, y - s * 0.5, y + s * 0.5);
    pen.line((l, t), (r, t), 0.065, color);
    pen.line((r, t), (l, btm), 0.065, color);
    pen.line((l, btm), (r, btm), 0.065, color);
}
