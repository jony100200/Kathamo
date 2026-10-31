//! Motion: frame-rate-independent springs for hover, press, toggle and slide transitions.
//!
//! Widgets are immediate-mode and keep no state between frames, so animation state
//! lives here, keyed by a stable hash (usually widget kind + rect). The UI runs on one
//! thread, so the store is thread-local and widgets call [`animate`] with no extra
//! parameters threaded through every render function.
//!
//! The store starts in *instant* mode: [`animate`] returns the target immediately.
//! Headless tests and snapshots stay deterministic; the native runner switches it
//! off and calls [`begin_frame`] with the real frame time.
//!
//! The spring is the exact critically damped solution (no overshoot, no jitter at
//! any frame rate), from Daniel Holden, "Spring-It-On" (theorangeduck.com).

use std::cell::RefCell;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::time::Instant;

use crate::Rect;

/// Half-life in seconds: time to cover half the remaining distance.
pub const HOVER: f32 = 0.045;
pub const PRESS: f32 = 0.03;
pub const TOGGLE: f32 = 0.06;
pub const SLIDE: f32 = 0.08;

/// Largest step simulated per frame; a stalled frame must not teleport the spring.
const MAX_DT: f32 = 1.0 / 20.0;
const SETTLE_EPSILON: f32 = 1e-3;

/// One critically damped spring.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Spring {
    pub value: f32,
    pub velocity: f32,
}

impl Spring {
    pub fn at(value: f32) -> Self {
        Self {
            value,
            velocity: 0.0,
        }
    }

    /// Advances toward `target` by `dt` seconds.
    pub fn step(&mut self, target: f32, half_life: f32, dt: f32) {
        let y = 2.0 * std::f32::consts::LN_2 / half_life.max(1e-5);
        let j0 = self.value - target;
        let j1 = self.velocity + j0 * y;
        let decay = (-y * dt).exp();
        self.value = decay * (j0 + j1 * dt) + target;
        self.velocity = decay * (self.velocity - j1 * y * dt);
        if (self.value - target).abs() < SETTLE_EPSILON && self.velocity.abs() < SETTLE_EPSILON {
            *self = Self::at(target);
        }
    }

    pub fn is_settled(&self, target: f32) -> bool {
        self.value == target && self.velocity == 0.0
    }
}

struct Store {
    springs: HashMap<u64, (Spring, u64)>,
    frame: u64,
    dt: f32,
    instant: bool,
    reduced: bool,
    clock: f64,
    animating: bool,
    wake_at: Option<Instant>,
}

thread_local! {
    static STORE: RefCell<Store> = RefCell::new(Store {
        springs: HashMap::new(),
        frame: 0,
        dt: 0.0,
        instant: true,
        reduced: false,
        clock: 0.0,
        animating: false,
        wake_at: None,
    });
}

/// Instant mode snaps every value to its target (tests, snapshots, reduced motion).
pub fn set_instant(instant: bool) {
    STORE.with_borrow_mut(|s| s.instant = instant);
}

/// User preference: no ambient loops and no transitions. Springs snap to their
/// targets and [`ambient_running`] turns false, so idle animations draw a still pose.
pub fn set_reduced_motion(reduced: bool) {
    STORE.with_borrow_mut(|s| s.reduced = reduced);
}

pub fn reduced_motion() -> bool {
    STORE.with_borrow(|s| s.reduced)
}

/// Seconds of animated time since start: the sum of frame steps. Stays at 0 in
/// instant mode, so headless frames are deterministic.
pub fn clock() -> f32 {
    STORE.with_borrow(|s| s.clock as f32)
}

/// True when looping (time-driven) animation should play: a live runner and no
/// reduced-motion preference.
pub fn ambient_running() -> bool {
    STORE.with_borrow(|s| !s.instant && !s.reduced)
}

/// Starts a frame `dt` seconds after the previous one. Springs not touched last
/// frame are dropped, so closed widgets do not leak state.
pub fn begin_frame(dt: f32) {
    STORE.with_borrow_mut(|s| {
        s.frame += 1;
        s.dt = dt.clamp(0.0, MAX_DT);
        s.clock += s.dt as f64;
        s.animating = false;
        let keep_after = s.frame.saturating_sub(2);
        s.springs.retain(|_, (_, seen)| *seen >= keep_after);
    });
}

/// True while any spring touched this frame is still moving; the runner keeps
/// redrawing until it settles, then the app goes back to zero idle work.
pub fn is_animating() -> bool {
    STORE.with_borrow(|s| s.animating)
}

/// Returns the animated value for `key` moving toward `target`.
/// A key seen for the first time starts at its target (nothing animates in on load).
pub fn animate(key: u64, target: f32, half_life: f32) -> f32 {
    animate_from(key, target, target, half_life)
}

/// Like [`animate`], but a new key starts at `from`: use for enter transitions
/// such as a toast sliding in.
pub fn animate_from(key: u64, from: f32, target: f32, half_life: f32) -> f32 {
    STORE.with_borrow_mut(|s| {
        let (frame, dt, instant) = (s.frame, s.dt, s.instant || s.reduced);
        let (spring, seen) = s.springs.entry(key).or_insert((Spring::at(from), frame));
        *seen = frame;
        if instant {
            *spring = Spring::at(target);
        } else {
            spring.step(target, half_life, dt);
        }
        let value = spring.value;
        if !spring.is_settled(target) {
            s.animating = true;
        }
        value
    })
}

/// Asks for another frame without a spring.
pub fn request_frame() {
    STORE.with_borrow_mut(|s| s.animating = true);
}

/// Asks for one frame at `deadline` (earliest request wins), for time-based UI
/// such as a toast expiring while the user is idle. The app sleeps until then.
pub fn wake_at(deadline: Instant) {
    STORE.with_borrow_mut(|s| s.wake_at = Some(s.wake_at.map_or(deadline, |w| w.min(deadline))));
}

/// Takes the pending wake-up deadline; the native runner sleeps until it.
pub fn take_wake_deadline() -> Option<Instant> {
    STORE.with_borrow_mut(|s| s.wake_at.take())
}

/// 0..1 transition for an on/off state such as hover or checked.
pub fn animate_bool(key: u64, on: bool, half_life: f32) -> f32 {
    animate(key, if on { 1.0 } else { 0.0 }, half_life)
}

/// Stable key from a widget kind tag and its rectangle.
pub fn rect_key(tag: &str, rect: Rect) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    tag.hash(&mut h);
    for v in [rect.min.x, rect.min.y, rect.max.x, rect.max.y] {
        v.to_bits().hash(&mut h);
    }
    h.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spring_moves_partway_by_half_life_and_never_overshoots() {
        // From rest, the exact solution is 1 - e^(-2ln2)(1 + 2ln2) = 0.40 at one half-life.
        let mut s = Spring::at(0.0);
        let mut max = 0.0f32;
        for _ in 0..1000 {
            s.step(1.0, 0.1, 0.0001);
            max = max.max(s.value);
        }
        assert!(
            (s.value - 0.4034).abs() < 0.01,
            "value at half-life: {}",
            s.value
        );
        for _ in 0..100 {
            s.step(1.0, 0.1, 0.05);
            max = max.max(s.value);
        }
        assert!(max <= 1.0, "critically damped spring overshot: {max}");
        assert!(s.is_settled(1.0));
    }

    #[test]
    fn result_is_independent_of_frame_rate() {
        let (mut a, mut b) = (Spring::at(0.0), Spring::at(0.0));
        for _ in 0..6 {
            a.step(1.0, HOVER, 1.0 / 60.0);
        }
        for _ in 0..24 {
            b.step(1.0, HOVER, 1.0 / 240.0);
        }
        assert!(
            (a.value - b.value).abs() < 1e-4,
            "{} vs {}",
            a.value,
            b.value
        );
    }

    #[test]
    fn store_animates_then_settles_and_reports_activity() {
        set_instant(false);
        begin_frame(0.0);
        assert_eq!(animate_bool(7, false, HOVER), 0.0);
        begin_frame(1.0 / 60.0);
        let v = animate_bool(7, true, HOVER);
        assert!(v > 0.0 && v < 1.0, "mid-transition value {v}");
        assert!(is_animating());
        for _ in 0..120 {
            begin_frame(1.0 / 60.0);
            animate_bool(7, true, HOVER);
        }
        assert!(!is_animating());
        set_instant(true);
    }

    #[test]
    fn animate_from_enters_from_start_value() {
        set_instant(false);
        begin_frame(1.0 / 60.0);
        let v = animate_from(11, 0.0, 1.0, SLIDE);
        assert!(v > 0.0 && v < 0.5, "first enter frame {v}");
        assert!(is_animating());
        set_instant(true);
    }

    #[test]
    fn instant_mode_snaps_and_untouched_keys_are_dropped() {
        set_instant(true);
        begin_frame(0.016);
        assert_eq!(animate(9, 0.5, SLIDE), 0.5);
        assert!(!is_animating());
        for _ in 0..3 {
            begin_frame(0.016);
        }
        STORE.with_borrow(|s| assert!(!s.springs.contains_key(&9)));
    }

    #[test]
    fn clock_advances_with_frames_and_reduced_motion_stills_everything() {
        set_instant(false);
        let start = clock();
        begin_frame(0.02);
        begin_frame(0.02);
        assert!((clock() - start - 0.04).abs() < 1e-5);
        assert!(ambient_running());
        set_reduced_motion(true);
        assert!(!ambient_running());
        assert_eq!(
            animate_from(21, 0.0, 1.0, SLIDE),
            1.0,
            "reduced motion snaps"
        );
        set_reduced_motion(false);
        set_instant(true);
        assert!(!ambient_running(), "headless frames stay still");
    }
}
