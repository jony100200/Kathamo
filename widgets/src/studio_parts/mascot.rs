//! Procedural agent mascot: a small round character drawn from plain draw commands.
//!
//! Identity (body colour, head feature, eye shape, animation phase) comes from a
//! stable hash of the name, so every agent is recognisable at a glance. The mood
//! picks the play: Ready breathes and blinks, Working hops and glances around,
//! NeedsYou waves with a "!" badge, Error is dizzy, Sleeping snoozes with a "z".
//!
//! Time is a parameter, so frames are deterministic. While a live runner shows a
//! mascot it asks for one frame per animation tick through `motion::wake_at` (not a
//! continuous redraw). Reduced motion or headless (instant) mode draws a still pose.

use std::f32::consts::{PI, TAU};
use std::time::{Duration, Instant};

use ui_core::{motion, Color, Rect, Vec2};

use super::mascot_draw;
use crate::studio::{Interaction, Studio, Tone};

/// What the agent is doing; drives the animation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Mood {
    Ready,
    Working,
    NeedsYou,
    Error,
    Sleeping,
}

impl Mood {
    /// Mood matching a status tone, for cards that only know a tone.
    pub fn from_tone(tone: Tone) -> Self {
        match tone {
            Tone::Working => Mood::Working,
            Tone::Warning => Mood::NeedsYou,
            Tone::Bad => Mood::Error,
            Tone::Neutral => Mood::Sleeping,
            Tone::Good | Tone::Info | Tone::Accent => Mood::Ready,
        }
    }

    /// Seconds between animation frames: busy moods tick faster than idle ones.
    fn tick(self) -> f32 {
        match self {
            Mood::Working | Mood::NeedsYou | Mood::Error => 1.0 / 30.0,
            Mood::Ready | Mood::Sleeping => 1.0 / 20.0,
        }
    }
}

/// Head decoration that makes a mascot recognisable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Feature {
    RoundEars,
    CatEars,
    BunnyEars,
    Antenna,
    Sprout,
    Tuft,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EyeShape {
    Round,
    Tall,
    Wide,
}

/// Stable per-name identity.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Identity {
    pub body: Color,
    /// Antenna bulb / accent colour that pairs with the body.
    pub pop: Color,
    pub feature: Feature,
    pub eyes: EyeShape,
    /// 0..1 offset so agents on one screen do not blink in sync.
    pub phase: f32,
}

/// Colour on the HSL wheel (all channels 0..1).
fn hsl(hue: f32, sat: f32, light: f32) -> Color {
    let chroma = (1.0 - (2.0 * light - 1.0).abs()) * sat;
    let channel = |n: f32| {
        let k = (n + hue.rem_euclid(1.0) * 12.0) % 12.0;
        let v = light - chroma * 0.5 * (k - 3.0).min(9.0 - k).clamp(-1.0, 1.0);
        (v.clamp(0.0, 1.0) * 255.0).round() as u8
    };
    Color::rgb(channel(0.0), channel(8.0), channel(4.0))
}

/// Body colour for a hue: mid-light and saturated so the dark eyes read on it and
/// it reads on dark and paper backgrounds. Yellow-greens look brighter at equal
/// lightness, so they are pulled down a little.
fn body_color(hue: f32) -> Color {
    let glare = (std::f32::consts::TAU * (hue - 0.2)).cos().max(0.0);
    hsl(hue, 0.68 - 0.1 * glare, 0.64 - 0.1 * glare)
}

const FEATURES: [Feature; 6] = [
    Feature::RoundEars,
    Feature::CatEars,
    Feature::BunnyEars,
    Feature::Antenna,
    Feature::Sprout,
    Feature::Tuft,
];

/// Identity from a stable FNV-1a hash of the name (case-insensitive).
pub fn identity(name: &str) -> Identity {
    let mut h = 2166136261u32;
    for b in name.trim().bytes() {
        h = (h ^ b.to_ascii_lowercase() as u32).wrapping_mul(16777619);
    }
    // murmur3 finaliser: similar names must not share a look.
    h ^= h >> 16;
    h = h.wrapping_mul(0x85EB_CA6B);
    h ^= h >> 13;
    h = h.wrapping_mul(0xC2B2_AE35);
    h ^= h >> 16;
    let hue = (h & 0xFFF) as f32 / 4096.0;
    Identity {
        body: body_color(hue),
        pop: hsl(hue + 0.45, 0.95, 0.66),
        feature: FEATURES[((h >> 12) & 0xFF) as usize % FEATURES.len()],
        eyes: [EyeShape::Round, EyeShape::Tall, EyeShape::Wide][((h >> 20) & 0xFF) as usize % 3],
        phase: (h >> 24) as f32 / 255.0,
    }
}

/// One frame of the animation, in body units (1.0 = half the mascot size).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Pose {
    /// Upward offset of the body.
    pub lift: f32,
    /// Positive = wider and shorter, negative = taller.
    pub squash: f32,
    /// Sideways offset of the body.
    pub sway: f32,
    /// Gaze direction, each axis -1..1.
    pub look: Vec2,
    /// 0 = eyes open, 1 = closed.
    pub blink: f32,
    /// Waving arm, 0..1 (NeedsYou).
    pub arm: f32,
    /// 0..1 loop for floating overlays ("z", badge bob).
    pub float: f32,
    /// Orbit angle of the dizzy stars (Error).
    pub spin: f32,
    /// -1..1 sway of antenna / ears.
    pub wiggle: f32,
}

/// Pose for `mood` at `time` seconds; `None` gives the still (reduced-motion) pose.
pub fn pose(mood: Mood, time: Option<f32>, phase: f32) -> Pose {
    let Some(t) = time else {
        return still_pose(mood);
    };
    let t = t + phase * 11.0;
    let wave = |period: f32| (TAU * t / period).sin();
    let blink = blink_at(t, 3.6 + phase * 1.8);
    match mood {
        Mood::Ready => Pose {
            squash: 0.035 * wave(3.0),
            lift: 0.02 * wave(3.0).max(0.0),
            look: Vec2::new(0.45 * glance(t / 6.5), 0.0),
            blink,
            wiggle: 0.35 * wave(2.6),
            ..Pose::default()
        },
        Mood::Working => {
            let hop = (PI * (t / 0.62).fract()).sin();
            Pose {
                lift: 0.16 * hop,
                squash: 0.11 * (1.0 - hop).powi(6) - 0.05 * hop,
                look: Vec2::new(glance(t / 1.9), -0.25),
                blink: blink_at(t, 2.4 + phase),
                wiggle: wave(0.62),
                ..Pose::default()
            }
        }
        Mood::NeedsYou => Pose {
            lift: 0.05 * wave(1.4).abs(),
            look: Vec2::new(0.0, -0.45),
            blink,
            arm: 0.5 + 0.5 * wave(0.75),
            float: 0.5 + 0.5 * wave(1.4),
            wiggle: 0.5 * wave(1.4),
            ..Pose::default()
        },
        Mood::Error => Pose {
            sway: 0.07 * wave(1.3),
            squash: 0.02 * wave(0.65),
            spin: TAU * (t / 1.4).fract(),
            wiggle: wave(1.3),
            ..Pose::default()
        },
        Mood::Sleeping => Pose {
            squash: 0.05 * wave(4.2),
            blink: 1.0,
            float: (t / 2.8).fract(),
            ..Pose::default()
        },
    }
}

/// Still poses still say which mood it is (raised arm, X eyes, closed eyes, "z").
fn still_pose(mood: Mood) -> Pose {
    match mood {
        Mood::Ready => Pose::default(),
        Mood::Working => Pose {
            lift: 0.08,
            look: Vec2::new(0.7, -0.25),
            ..Pose::default()
        },
        Mood::NeedsYou => Pose {
            look: Vec2::new(0.0, -0.45),
            arm: 1.0,
            float: 0.5,
            ..Pose::default()
        },
        Mood::Error => Pose {
            spin: 0.6,
            ..Pose::default()
        },
        Mood::Sleeping => Pose {
            blink: 1.0,
            float: 0.3,
            ..Pose::default()
        },
    }
}

/// Smooth side-to-side glance that rests at each side: a squashed sine.
fn glance(cycles: f32) -> f32 {
    let s = (TAU * cycles).sin();
    (s * 1.8).clamp(-1.0, 1.0)
}

/// Quick blink once per `period` seconds: 0 open, 1 closed, 0.16 s long.
fn blink_at(t: f32, period: f32) -> f32 {
    const LEN: f32 = 0.16;
    let x = t.rem_euclid(period);
    if x < LEN {
        1.0 - ((x / LEN) * 2.0 - 1.0).abs()
    } else {
        0.0
    }
}

/// Draws the mascot for `name` inside `rect` (square works best; it fits the
/// shorter side) at `time_secs`, and reports pointer interaction. Hovering makes it
/// hop and look at the cursor. Pass [`motion::clock`] as the time in a live app.
pub fn mascot(
    studio: &mut Studio,
    rect: Rect,
    name: &str,
    mood: Mood,
    time_secs: f32,
) -> Interaction {
    let hit = studio.interact(rect);
    let id = identity(name);
    let still = motion::reduced_motion();
    let mut p = pose(mood, (!still).then_some(time_secs), id.phase);

    let hover = motion::animate_bool(motion::rect_key("mascot", rect), hit.hovered, 0.07);
    if hover > 0.0 {
        let unit = rect.width().min(rect.height()) * 0.5;
        let to = (studio.pointer.position - rect.center()) * (1.0 / unit.max(1.0));
        let len = to.length().max(1.0);
        let gaze = Vec2::new(to.x / len, to.y / len);
        p.look = p.look + (gaze - p.look) * hover;
        p.lift += 0.12 * hover;
    }

    mascot_draw::draw(studio, rect, &id, mood, &p);

    if motion::ambient_running() {
        motion::wake_at(Instant::now() + Duration::from_secs_f32(mood.tick()));
    }
    hit
}

#[cfg(test)]
#[path = "mascot_tests.rs"]
mod tests;
