use super::types::PresetNode;
use ui_core::Vec2;
use ui_platform::Key;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SearchMenuState {
    pub is_open: bool,
    pub world_pos: Vec2,
    pub query: String,
    pub selected_index: usize,
}

impl SearchMenuState {
    pub fn open(&mut self, world_pos: Vec2) {
        self.is_open = true;
        self.world_pos = world_pos;
        self.query.clear();
        self.selected_index = 0;
    }

    pub fn close(&mut self) {
        self.is_open = false;
        self.query.clear();
        self.selected_index = 0;
    }

    pub fn filter_presets(&self) -> Vec<PresetNode> {
        let q = self.query.trim().to_lowercase();
        PresetNode::all()
            .iter()
            .copied()
            .filter(|p: &PresetNode| {
                if q.is_empty() {
                    return true;
                }
                let title = p.title().to_lowercase();
                let cat = p.category().to_lowercase();
                let desc = p.search_description().to_lowercase();
                title.contains(&q) || cat.contains(&q) || desc.contains(&q)
            })
            .collect()
    }

    pub fn handle_key_down(&mut self, key: Key) -> Option<PresetNode> {
        if !self.is_open {
            return None;
        }

        let filtered = self.filter_presets();
        let count = filtered.len();

        match key {
            Key::Escape => {
                self.close();
                None
            }
            Key::ArrowUp => {
                if count > 0 {
                    self.selected_index = (self.selected_index + count - 1) % count;
                }
                None
            }
            Key::ArrowDown => {
                if count > 0 {
                    self.selected_index = (self.selected_index + 1) % count;
                }
                None
            }
            Key::Enter => {
                if let Some(preset) = filtered.get(self.selected_index).copied() {
                    self.close();
                    Some(preset)
                } else {
                    None
                }
            }
            Key::Backspace => {
                self.query.pop();
                self.selected_index = 0;
                None
            }
            _ => None,
        }
    }
}
