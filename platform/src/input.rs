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

#[derive(Debug, Clone, Copy, PartialEq, Default)]
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
    /// Whether transition tracking is active (on_button_down/up or step_frame was called).
    pub transitions_active: bool,
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

    /// Resets transient per-frame flags before processing new frame inputs.
    pub fn step_frame(&mut self) {
        self.transitions_active = true;
        self.primary_clicked = false;
        self.primary_pressed = false;
        self.primary_released = false;
        self.drag_delta = Vec2::ZERO;
        self.scroll_delta = Vec2::ZERO;
    }

    /// Handles button press transition.
    pub fn on_button_down(&mut self, button: PointerButton) {
        self.transitions_active = true;
        match button {
            PointerButton::Primary => {
                if !self.primary_down {
                    self.primary_pressed = true;
                    self.primary_clicked = true;
                }
                self.primary_down = true;
            }
            PointerButton::Secondary => self.secondary_down = true,
            PointerButton::Middle => self.middle_down = true,
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
            }
            PointerButton::Secondary => self.secondary_down = false,
            PointerButton::Middle => self.middle_down = false,
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

/// Simple focus ring manager.
#[derive(Debug, Default)]
pub struct FocusManager {
    focused_id: Option<String>,
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
    fn focus_manager_tracks_active_element() {
        let mut fm = FocusManager::new();
        assert!(!fm.is_focused("search_box"));
        fm.focus("search_box");
        assert!(fm.is_focused("search_box"));
        fm.clear();
        assert!(!fm.is_focused("search_box"));
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
    fn pointer_drag_tracking_computes_delta() {
        let mut ptr = PointerState::default();
        ptr.on_move(Vec2::new(10.0, 20.0));
        assert!(!ptr.is_dragging);

        ptr.on_button_down(PointerButton::Primary);
        ptr.on_move(Vec2::new(25.0, 20.0));
        assert!(ptr.is_dragging);
        assert_eq!(ptr.drag_delta, Vec2::new(15.0, 0.0));
    }
}

