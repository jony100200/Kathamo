//! Input abstractions: keyboard shortcuts, mouse/pointer states, and focus management.

use ui_core::Vec2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Modifiers {
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
    pub command: bool, // Meta/Win/Super
}

impl Modifiers {
    pub const NONE: Self = Self {
        ctrl: false,
        shift: false,
        alt: false,
        command: false,
    };
    pub const CTRL: Self = Self {
        ctrl: true,
        shift: false,
        alt: false,
        command: false,
    };
    pub const SHIFT: Self = Self {
        ctrl: false,
        shift: true,
        alt: false,
        command: false,
    };
    pub const ALT: Self = Self {
        ctrl: false,
        shift: false,
        alt: true,
        command: false,
    };

    pub fn matches(&self, other: &Self) -> bool {
        self.ctrl == other.ctrl
            && self.shift == other.shift
            && self.alt == other.alt
            && self.command == other.command
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Key {
    Character(char),
    Escape,
    Enter,
    Tab,
    Backspace,
    Delete,
    Space,
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    F(u8),
    Custom(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shortcut {
    pub modifiers: Modifiers,
    pub key: Key,
    pub description: String,
}

impl Shortcut {
    pub fn new(modifiers: Modifiers, key: Key, desc: impl Into<String>) -> Self {
        Self {
            modifiers,
            key,
            description: desc.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PointerButton {
    #[default]
    Primary, // Left
    Secondary, // Right
    Middle,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct PointerState {
    pub position: Vec2,
    pub primary_down: bool,
    pub secondary_down: bool,
    pub middle_down: bool,
    pub scroll_delta: Vec2,
    /// Single-frame click transition.
    pub primary_clicked: bool,
    /// Single-frame press-down transition.
    pub primary_pressed: bool,
    /// Single-frame release transition.
    pub primary_released: bool,
    /// Movement delta during drag.
    pub drag_delta: Vec2,
    /// Whether actively dragging.
    pub is_dragging: bool,
    /// Active drag capture target (e.g. splitter, canvas, scrubber).
    pub drag_capture: Option<String>,
    /// Double click occurred on this frame.
    pub double_clicked: bool,
    /// Last click position for double-click proximity.
    pub last_click_pos: Vec2,
    /// Last click frame index.
    pub last_click_frame: u64,
    /// Monotonic frame counter.
    pub frame_counter: u64,
    /// Whether transition tracking is active.
    pub transitions_active: bool,
    /// Secondary (Right) click transitions.
    pub secondary_clicked: bool,
    pub secondary_pressed: bool,
    pub secondary_released: bool,
    /// Middle click transitions.
    pub middle_clicked: bool,
    pub middle_pressed: bool,
    pub middle_released: bool,
}

impl PointerState {
    /// Returns true if a click occurred on this frame.
    pub fn is_clicked(&self) -> bool {
        if self.transitions_active {
            self.primary_clicked || self.primary_released
        } else {
            self.primary_down && !self.is_dragging
        }
    }

    /// Returns true if primary button is currently held down.
    pub fn is_down(&self) -> bool {
        self.primary_down
    }

    /// Captures the pointer for dragging by the specified element.
    pub fn capture_drag(&mut self, id: impl Into<String>) {
        self.drag_capture = Some(id.into());
    }

    /// Releases any active drag capture.
    pub fn release_drag(&mut self) {
        self.drag_capture = None;
    }

    /// Checks if drag is captured by the given element ID.
    pub fn is_captured_by(&self, id: &str) -> bool {
        self.drag_capture.as_deref() == Some(id)
    }

    /// Returns whether any element currently owns drag capture.
    pub fn has_drag_capture(&self) -> bool {
        self.drag_capture.is_some()
    }

    /// Resets transient per-frame flags before processing new frame inputs.
    pub fn step_frame(&mut self) {
        self.transitions_active = true;
        self.frame_counter = self.frame_counter.wrapping_add(1);
        self.primary_clicked = false;
        self.primary_pressed = false;
        self.primary_released = false;
        self.secondary_clicked = false;
        self.secondary_pressed = false;
        self.secondary_released = false;
        self.middle_clicked = false;
        self.middle_pressed = false;
        self.middle_released = false;
        self.double_clicked = false;
        self.drag_delta = Vec2::ZERO;
        self.scroll_delta = Vec2::ZERO;
    }

    /// Handles button press transition with double-click detection.
    pub fn on_button_down(&mut self, button: PointerButton) {
        self.transitions_active = true;
        match button {
            PointerButton::Primary => {
                if !self.primary_down {
                    self.primary_pressed = true;
                    self.primary_clicked = true;

                    // Double click detection: within 25 frames and 5 pixels
                    let pos_delta = (self.position - self.last_click_pos).length();
                    let frame_diff = self.frame_counter.saturating_sub(self.last_click_frame);
                    if frame_diff <= 25 && pos_delta <= 5.0 {
                        self.double_clicked = true;
                        self.last_click_frame = 0; // Reset after double click
                    } else {
                        self.last_click_frame = self.frame_counter;
                        self.last_click_pos = self.position;
                    }
                }
                self.primary_down = true;
            }
            PointerButton::Secondary => {
                if !self.secondary_down {
                    self.secondary_pressed = true;
                    self.secondary_clicked = true;
                }
                self.secondary_down = true;
            }
            PointerButton::Middle => {
                if !self.middle_down {
                    self.middle_pressed = true;
                    self.middle_clicked = true;
                }
                self.middle_down = true;
            }
        }
    }

    /// Handles button release transition.
    pub fn on_button_up(&mut self, button: PointerButton) {
        self.transitions_active = true;
        match button {
            PointerButton::Primary => {
                if self.primary_down {
                    self.primary_released = true;
                }
                self.primary_down = false;
                self.is_dragging = false;
                self.drag_capture = None;
            }
            PointerButton::Secondary => {
                if self.secondary_down {
                    self.secondary_released = true;
                }
                self.secondary_down = false;
            }
            PointerButton::Middle => {
                if self.middle_down {
                    self.middle_released = true;
                }
                self.middle_down = false;
            }
        }
    }

    /// Handles mouse movement and drag detection.
    pub fn on_move(&mut self, new_pos: Vec2) {
        let delta = new_pos - self.position;
        self.position = new_pos;
        if self.primary_down || self.secondary_down || self.middle_down {
            self.drag_delta = delta;
            if delta.length_sq() > 1.0 {
                self.is_dragging = true;
            }
        }
    }
}

/// Rich text editing and keyboard navigation events.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextInputEvent {
    Char(char),
    Text(String),
    Backspace,
    Delete,
    Enter,
    Escape,
    Tab,
    Backtab,
    Left,
    Right,
    Home,
    End,
    SelectAll,
    Copy,
    Cut,
    Paste(String),
}

/// Standard desktop IME (Input Method Editor) events for complex scripts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImeEvent {
    Preedit(String, Option<(usize, usize)>),
    Commit(String),
    Disabled,
}

/// Comprehensive focus ring manager supporting Tab order registration and cycling.
#[derive(Debug, Default)]
pub struct FocusManager {
    focused_id: Option<String>,
    focusable_registry: Vec<String>,
}

impl FocusManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn focus(&mut self, id: impl Into<String>) {
        self.focused_id = Some(id.into());
    }

    pub fn focus_id(&mut self, id: ui_core::ElementId) {
        self.focused_id = Some(format!("{}", id));
    }

    pub fn clear(&mut self) {
        self.focused_id = None;
    }

    pub fn is_focused(&self, id: &str) -> bool {
        self.focused_id.as_deref() == Some(id)
    }

    pub fn is_element_focused(&self, id: ui_core::ElementId) -> bool {
        let s = format!("{}", id);
        self.focused_id.as_deref() == Some(&s)
    }

    pub fn focused_id(&self) -> Option<&str> {
        self.focused_id.as_deref()
    }

    /// Registers a focusable target ID in tab navigation order.
    pub fn register_focusable(&mut self, id: impl Into<String>) {
        let s = id.into();
        if !self.focusable_registry.contains(&s) {
            self.focusable_registry.push(s);
        }
    }

    /// Clears the focusable registry at the start of a layout pass.
    pub fn clear_registry(&mut self) {
        self.focusable_registry.clear();
    }

    /// Cycles focus forwards or backwards through registered targets.
    pub fn cycle_focus(&mut self, backwards: bool) {
        if self.focusable_registry.is_empty() {
            return;
        }

        let current_idx = self
            .focused_id
            .as_ref()
            .and_then(|cur| self.focusable_registry.iter().position(|r| r == cur));

        let next_idx = match current_idx {
            Some(idx) => {
                if backwards {
                    if idx == 0 {
                        self.focusable_registry.len() - 1
                    } else {
                        idx - 1
                    }
                } else {
                    (idx + 1) % self.focusable_registry.len()
                }
            }
            None => {
                if backwards {
                    self.focusable_registry.len() - 1
                } else {
                    0
                }
            }
        };

        self.focused_id = Some(self.focusable_registry[next_idx].clone());
    }

    pub fn focus_next(&mut self) {
        self.cycle_focus(false);
    }

    pub fn focus_prev(&mut self) {
        self.cycle_focus(true);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shortcut_matches_modifiers_accurately() {
        let sc = Shortcut::new(Modifiers::CTRL, Key::Character('k'), "Command Palette");
        assert!(sc.modifiers.matches(&Modifiers::CTRL));
        assert!(!sc.modifiers.matches(&Modifiers::NONE));
    }

    #[test]
    fn focus_manager_tracks_active_element_and_cycles() {
        let mut fm = FocusManager::new();
        fm.register_focusable("input_a");
        fm.register_focusable("input_b");
        fm.register_focusable("input_c");

        assert_eq!(fm.focused_id(), None);
        fm.focus_next();
        assert_eq!(fm.focused_id(), Some("input_a"));
        fm.focus_next();
        assert_eq!(fm.focused_id(), Some("input_b"));
        fm.focus_prev();
        assert_eq!(fm.focused_id(), Some("input_a"));
        fm.focus_prev();
        assert_eq!(fm.focused_id(), Some("input_c"));
    }

    #[test]
    fn pointer_state_click_transitions_fire_only_once_per_press() {
        let mut ptr = PointerState::default();
        assert!(!ptr.is_clicked());
        assert!(!ptr.is_down());

        // Press down
        ptr.on_button_down(PointerButton::Primary);
        assert!(ptr.primary_down);
        assert!(ptr.primary_pressed);
        assert!(ptr.primary_clicked);
        assert!(ptr.is_clicked());

        // Next frame while still holding button down
        ptr.step_frame();
        assert!(ptr.primary_down); // Still down
        assert!(!ptr.primary_pressed); // No new press
        assert!(!ptr.primary_clicked); // No new click!
        assert!(!ptr.is_clicked()); // Zero repeated triggering on held press!

        // Release button
        ptr.on_button_up(PointerButton::Primary);
        assert!(!ptr.primary_down);
        assert!(ptr.primary_released);
        assert!(ptr.is_clicked()); // Activated on release

        // Following frame after release
        ptr.step_frame();
        assert!(!ptr.primary_down);
        assert!(!ptr.primary_released);
        assert!(!ptr.is_clicked());
    }

    #[test]
    fn pointer_drag_tracking_computes_delta_and_capture() {
        let mut ptr = PointerState::default();
        ptr.on_move(Vec2::new(10.0, 20.0));
        assert!(!ptr.is_dragging);

        ptr.on_button_down(PointerButton::Primary);
        ptr.capture_drag("splitter_1");
        assert!(ptr.is_captured_by("splitter_1"));
        assert!(ptr.has_drag_capture());

        ptr.on_move(Vec2::new(25.0, 20.0));
        assert!(ptr.is_dragging);
        assert_eq!(ptr.drag_delta, Vec2::new(15.0, 0.0));

        ptr.on_button_up(PointerButton::Primary);
        assert!(!ptr.has_drag_capture());
    }

    #[test]
    fn pointer_double_click_detection() {
        let mut ptr = PointerState::default();
        ptr.position = Vec2::new(100.0, 100.0);
        ptr.on_button_down(PointerButton::Primary);
        ptr.on_button_up(PointerButton::Primary);
        assert!(!ptr.double_clicked);

        // Advance 5 frames and click in almost the same spot
        for _ in 0..5 {
            ptr.step_frame();
        }
        ptr.position = Vec2::new(101.0, 100.0);
        ptr.on_button_down(PointerButton::Primary);
        assert!(ptr.double_clicked);
    }
}

