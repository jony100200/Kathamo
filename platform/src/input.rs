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

    pub fn clear(&mut self) {
        self.focused_id = None;
    }

    pub fn is_focused(&self, id: &str) -> bool {
        self.focused_id.as_deref() == Some(id)
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
}
