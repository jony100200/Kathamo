//! Window configuration and lifecycle coordination.
//! Implements the Zero-White-Screen startup protocol:
//! 1. Window created with dark class brush or hidden flag.
//! 2. First frame rendered and presented to swapchain.
//! 3. Window revealed to the user only when valid pixels exist.

use ui_core::Color;

#[derive(Debug, Clone, PartialEq)]
pub struct WindowOptions {
    pub title: String,
    pub width: f32,
    pub height: f32,
    pub min_width: f32,
    pub min_height: f32,
    pub dark_titlebar: bool,
    pub background_brush: Color,
    pub hide_until_first_paint: bool,
}

impl Default for WindowOptions {
    fn default() -> Self {
        Self {
            title: "Kalponic Studio".into(),
            width: 1440.0,
            height: 900.0,
            min_width: 800.0,
            min_height: 500.0,
            dark_titlebar: true,
            background_brush: Color::rgb(15, 18, 25), // Studio dark theme canvas
            hide_until_first_paint: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WindowLifecycle {
    #[default]
    Initializing,
    FirstFrameReady,
    VisibleAndActive,
    Suspended,
}

/// Coordinates the first-frame presentation fence to eliminate white screen flashes.
pub struct LifecycleCoordinator {
    pub options: WindowOptions,
    pub state: WindowLifecycle,
    frames_presented: u64,
}

impl LifecycleCoordinator {
    pub fn new(options: WindowOptions) -> Self {
        Self {
            options,
            state: WindowLifecycle::Initializing,
            frames_presented: 0,
        }
    }

    /// Called by the rendering backend after a frame is committed to the swapchain.
    pub fn on_frame_presented(&mut self) -> bool {
        self.frames_presented += 1;
        if self.state == WindowLifecycle::Initializing {
            self.state = WindowLifecycle::FirstFrameReady;
            // Transition immediately to visible
            self.state = WindowLifecycle::VisibleAndActive;
            true // Signals to show the window now!
        } else {
            false
        }
    }

    pub fn is_visible(&self) -> bool {
        self.state == WindowLifecycle::VisibleAndActive
    }

    pub fn frames_presented(&self) -> u64 {
        self.frames_presented
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lifecycle_coordinator_signals_visibility_only_after_first_frame() {
        let mut coord = LifecycleCoordinator::new(WindowOptions::default());
        assert!(!coord.is_visible());
        assert_eq!(coord.frames_presented(), 0);

        let reveal = coord.on_frame_presented();
        assert!(reveal, "Must signal window reveal on first frame");
        assert!(coord.is_visible());
        assert_eq!(coord.frames_presented(), 1);

        let subsequent = coord.on_frame_presented();
        assert!(
            !subsequent,
            "Subsequent frames must not trigger reveal again"
        );
        assert_eq!(coord.frames_presented(), 2);
    }
}
