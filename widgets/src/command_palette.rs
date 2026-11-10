//! Command palette (Ctrl+K) quick launcher and shortcut directory.
//!
//! The palette is driven by input: the host forwards keys, text, pointer moves and clicks, and
//! scroll to it. Enter or a click picks a command, and the host reads its id with
//! [`CommandPalette::take_chosen`].

use crate::theme::ThemePalette;
use ui_core::{CornerRadius, Rect, Stroke, Vec2};
use ui_platform::{Key, PointerButton};
use ui_render::{CommandBuffer, DrawCommand};

const DIALOG_W: f32 = 540.0;
const DIALOG_H: f32 = 360.0;
/// Result rows shown at once. Longer result lists scroll.
pub const PALETTE_VISIBLE_ROWS: usize = 6;
const ROW_STEP: f32 = 38.0;
const ROW_H: f32 = 34.0;

#[derive(Debug, Clone, PartialEq)]
pub struct PaletteCommand {
    pub id: String,
    pub title: String,
    pub category: String,
    pub shortcut: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CommandPalette {
    pub is_open: bool,
    pub query: String,
    /// Index into [`CommandPalette::filtered_commands`].
    pub selected_index: usize,
    pub commands: Vec<PaletteCommand>,
    /// Index into the filtered list of the first visible row.
    scroll: usize,
    /// Id of the command picked by Enter or a click, until taken with `take_chosen`.
    chosen: Option<String>,
}

impl Default for CommandPalette {
    fn default() -> Self {
        Self {
            is_open: false,
            query: String::new(),
            selected_index: 0,
            commands: vec![
                PaletteCommand {
                    id: "new_project".into(),
                    title: "Create New Project...".into(),
                    category: "File".into(),
                    shortcut: Some("Ctrl+N".into()),
                },
                PaletteCommand {
                    id: "open_project".into(),
                    title: "Open Existing Project...".into(),
                    category: "File".into(),
                    shortcut: Some("Ctrl+O".into()),
                },
                PaletteCommand {
                    id: "toggle_gpu_safe".into(),
                    title: "Toggle GPU Safe Mode Fallback".into(),
                    category: "System".into(),
                    shortcut: Some("F12".into()),
                },
                PaletteCommand {
                    id: "export_asset".into(),
                    title: "Export Sprite Sheet...".into(),
                    category: "Assets".into(),
                    shortcut: Some("Ctrl+E".into()),
                },
            ],
            scroll: 0,
            chosen: None,
        }
    }
}

/// Dialog rectangle for a screen of `screen_size`. Rendering and hit-testing both use it.
fn dialog_rect(screen_size: Vec2) -> Rect {
    Rect::from_min_size(
        Vec2::new((screen_size.x - DIALOG_W) * 0.5, screen_size.y * 0.2),
        Vec2::new(DIALOG_W, DIALOG_H),
    )
}

/// Rectangle of the visible result row `row` (0-based within the window).
fn row_rect(dialog: Rect, row: usize) -> Rect {
    let top = dialog.min.y + 16.0 + 38.0 + 12.0 + row as f32 * ROW_STEP;
    Rect::from_min_size(
        Vec2::new(dialog.min.x + 16.0, top),
        Vec2::new(DIALOG_W - 32.0, ROW_H),
    )
}

impl CommandPalette {
    pub fn toggle(&mut self) {
        self.is_open = !self.is_open;
        if self.is_open {
            self.query.clear();
            self.selected_index = 0;
            self.scroll = 0;
        }
    }

    /// Closes the palette without choosing anything.
    pub fn close(&mut self) {
        self.is_open = false;
    }

    /// The id picked by Enter or a click since the last call, if any. Taking it clears it.
    pub fn take_chosen(&mut self) -> Option<String> {
        self.chosen.take()
    }

    pub fn filtered_commands(&self) -> Vec<&PaletteCommand> {
        if self.query.trim().is_empty() {
            self.commands.iter().collect()
        } else {
            let q = self.query.to_lowercase();
            self.commands
                .iter()
                .filter(|c| {
                    c.title.to_lowercase().contains(&q) || c.category.to_lowercase().contains(&q)
                })
                .collect()
        }
    }

    /// Keeps the selection and the scroll window valid for the current result count.
    fn clamp_selection(&mut self) {
        let count = self.filtered_commands().len();
        if count == 0 {
            self.selected_index = 0;
            self.scroll = 0;
            return;
        }
        self.selected_index = self.selected_index.min(count - 1);
        self.scroll = self.scroll.min(count.saturating_sub(PALETTE_VISIBLE_ROWS));
        if self.selected_index < self.scroll {
            self.scroll = self.selected_index;
        } else if self.selected_index >= self.scroll + PALETTE_VISIBLE_ROWS {
            self.scroll = self.selected_index + 1 - PALETTE_VISIBLE_ROWS;
        }
    }

    /// Handles a key press. Returns true when the palette is open, because an open palette
    /// consumes every key.
    pub fn handle_key_down(&mut self, key: &Key) -> bool {
        if !self.is_open {
            return false;
        }
        match key {
            Key::Escape => self.close(),
            Key::Enter => {
                let chosen = self
                    .filtered_commands()
                    .get(self.selected_index)
                    .map(|c| c.id.clone());
                if chosen.is_some() {
                    self.chosen = chosen;
                    self.close();
                }
            }
            Key::ArrowDown => {
                let count = self.filtered_commands().len();
                if count > 0 {
                    self.selected_index = (self.selected_index + 1).min(count - 1);
                }
                self.clamp_selection();
            }
            Key::ArrowUp => {
                self.selected_index = self.selected_index.saturating_sub(1);
                self.clamp_selection();
            }
            Key::Backspace => {
                self.query.pop();
                self.selected_index = 0;
                self.scroll = 0;
                self.clamp_selection();
            }
            _ => {}
        }
        true
    }

    /// Appends typed text to the query. Returns true when the palette is open.
    pub fn handle_text_input(&mut self, text: &str) -> bool {
        if !self.is_open {
            return false;
        }
        self.query.extend(text.chars().filter(|c| !c.is_control()));
        self.selected_index = 0;
        self.scroll = 0;
        self.clamp_selection();
        true
    }

    /// The filtered index of the result row under `pos`, if any row is visible there.
    fn result_at(&self, pos: Vec2, screen_size: Vec2) -> Option<usize> {
        let dialog = dialog_rect(screen_size);
        let count = self.filtered_commands().len();
        let visible = count.saturating_sub(self.scroll).min(PALETTE_VISIBLE_ROWS);
        (0..visible)
            .find(|&row| row_rect(dialog, row).contains(pos))
            .map(|row| self.scroll + row)
    }

    /// Moves the selection to the result under the pointer. Returns true when open.
    pub fn handle_pointer_move(&mut self, pos: Vec2, screen_size: Vec2) -> bool {
        if !self.is_open {
            return false;
        }
        if let Some(index) = self.result_at(pos, screen_size) {
            self.selected_index = index;
        }
        true
    }

    /// A click picks the result under the pointer. A click outside the dialog dismisses the
    /// palette. Returns true when the palette is open.
    pub fn handle_pointer_down(
        &mut self,
        pos: Vec2,
        button: PointerButton,
        screen_size: Vec2,
    ) -> bool {
        if !self.is_open {
            return false;
        }
        if button != PointerButton::Primary {
            return true;
        }
        if let Some(index) = self.result_at(pos, screen_size) {
            self.chosen = self.filtered_commands().get(index).map(|c| c.id.clone());
            self.close();
        } else if !dialog_rect(screen_size).contains(pos) {
            self.close();
        }
        true
    }

    /// Scrolls the result list by wheel or trackpad delta (positive y moves up the list).
    /// Returns true when the palette is open.
    pub fn handle_scroll(&mut self, delta: Vec2) -> bool {
        if !self.is_open {
            return false;
        }
        let count = self.filtered_commands().len();
        if delta.y > 0.0 {
            self.scroll = self.scroll.saturating_sub(1);
        } else if delta.y < 0.0 {
            self.scroll = (self.scroll + 1).min(count.saturating_sub(PALETTE_VISIBLE_ROWS));
        }
        // Keep the selection inside the visible window.
        if count > 0 {
            let last_visible = (self.scroll + PALETTE_VISIBLE_ROWS).min(count) - 1;
            self.selected_index = self.selected_index.clamp(self.scroll, last_visible);
        }
        true
    }

    pub fn render(&self, buf: &mut CommandBuffer, screen_size: Vec2, palette: &ThemePalette) {
        if !self.is_open {
            return;
        }

        // Dim overlay
        let full_screen = Rect::from_min_size(Vec2::ZERO, screen_size);
        buf.push(DrawCommand::Rect {
            rect: full_screen,
            fill: ui_core::Color::BLACK.with_alpha(140),
            radius: CornerRadius::ZERO,
            stroke: Stroke::NONE,
        });

        // Dialog box centered
        let dialog_rect = dialog_rect(screen_size);

        buf.push(DrawCommand::SoftShadow {
            rect: dialog_rect,
            radius: CornerRadius::same(10.0),
            color: ui_core::Color::rgba(0, 0, 0, 140),
            offset: Vec2::new(0.0, 12.0),
            blur: 32.0,
            spread: 0.0,
        });
        // Frosted glass: blurred backdrop + translucent tint, then the hairline border.
        buf.push(DrawCommand::Backdrop {
            rect: dialog_rect,
            radius: CornerRadius::same(10.0),
            tint: palette.surface.with_alpha(205),
        });
        buf.push(DrawCommand::Rect {
            rect: dialog_rect,
            fill: ui_core::Color::TRANSPARENT,
            radius: CornerRadius::same(10.0),
            stroke: Stroke::new(1.0, palette.border),
        });

        // Search input bar
        let search_rect = Rect::from_min_size(
            Vec2::new(dialog_rect.min.x + 16.0, dialog_rect.min.y + 16.0),
            Vec2::new(DIALOG_W - 32.0, 38.0),
        );
        buf.push(DrawCommand::Rect {
            rect: search_rect,
            fill: palette.elevated,
            radius: CornerRadius::same(6.0),
            stroke: Stroke::new(1.0, palette.accent),
        });

        let search_text = if self.query.is_empty() {
            "Type a command or action... (Ctrl+K to dismiss)"
        } else {
            &self.query
        };
        buf.push(DrawCommand::Text {
            text: search_text.to_string(),
            pos: Vec2::new(search_rect.min.x + 12.0, search_rect.min.y + 11.0),
            size: 13.0,
            color: if self.query.is_empty() {
                palette.text_muted
            } else {
                palette.text
            },
            strong: false,
        });

        // List results, windowed by the scroll offset
        let filtered = self.filtered_commands();
        if filtered.is_empty() {
            buf.push(DrawCommand::Text {
                text: "No matching commands".to_string(),
                pos: Vec2::new(
                    dialog_rect.min.x + 26.0,
                    row_rect(dialog_rect, 0).min.y + 9.0,
                ),
                size: 12.5,
                color: palette.text_muted,
                strong: false,
            });
            return;
        }
        let scroll = self.scroll.min(filtered.len().saturating_sub(1));
        for (row, cmd) in filtered
            .iter()
            .enumerate()
            .skip(scroll)
            .take(PALETTE_VISIBLE_ROWS)
        {
            let row_rect = row_rect(dialog_rect, row - scroll);

            let is_sel = row == self.selected_index;
            buf.push(DrawCommand::Rect {
                rect: row_rect,
                fill: if is_sel {
                    palette.accent.with_alpha(40)
                } else {
                    ui_core::Color::TRANSPARENT
                },
                radius: CornerRadius::same(6.0),
                stroke: Stroke::NONE,
            });

            buf.push(DrawCommand::Text {
                text: format!("{}: {}", cmd.category, cmd.title),
                pos: Vec2::new(row_rect.min.x + 10.0, row_rect.min.y + 9.0),
                size: 12.5,
                color: palette.text,
                strong: is_sel,
            });

            if let Some(ref sc) = cmd.shortcut {
                buf.push(DrawCommand::Text {
                    text: sc.clone(),
                    pos: Vec2::new(row_rect.max.x - 70.0, row_rect.min.y + 9.0),
                    size: 11.5,
                    color: palette.text_muted,
                    strong: false,
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCREEN: Vec2 = Vec2::new(1280.0, 800.0);

    fn palette_with(count: usize) -> CommandPalette {
        let mut p = CommandPalette {
            commands: (0..count)
                .map(|i| PaletteCommand {
                    id: format!("cmd_{i}"),
                    title: format!("Command {i}"),
                    category: "Test".into(),
                    shortcut: None,
                })
                .collect(),
            ..CommandPalette::default()
        };
        p.toggle();
        p
    }

    fn row_center(row: usize) -> Vec2 {
        row_rect(dialog_rect(SCREEN), row).center()
    }

    #[test]
    fn typing_filters_results_and_enter_emits_the_selected_id() {
        let mut p = CommandPalette::default();
        p.toggle();
        assert!(p.handle_text_input("export"));
        assert_eq!(p.filtered_commands().len(), 1);

        assert!(p.handle_key_down(&Key::Enter));
        assert_eq!(p.take_chosen().as_deref(), Some("export_asset"));
        assert!(!p.is_open, "choosing closes the palette");
        assert_eq!(p.take_chosen(), None, "the choice is taken once");
    }

    #[test]
    fn arrows_move_the_selection_and_clamp_at_the_ends() {
        let mut p = CommandPalette::default();
        p.toggle();
        p.handle_key_down(&Key::ArrowDown);
        p.handle_key_down(&Key::ArrowDown);
        assert_eq!(p.selected_index, 2);
        for _ in 0..10 {
            p.handle_key_down(&Key::ArrowDown);
        }
        assert_eq!(p.selected_index, 3, "clamped to the last result");
        for _ in 0..10 {
            p.handle_key_down(&Key::ArrowUp);
        }
        assert_eq!(p.selected_index, 0);
        p.handle_key_down(&Key::ArrowDown);
        p.handle_key_down(&Key::Enter);
        assert_eq!(p.take_chosen().as_deref(), Some("open_project"));
    }

    #[test]
    fn escape_closes_without_choosing_and_keys_are_consumed_only_while_open() {
        let mut p = CommandPalette::default();
        assert!(
            !p.handle_key_down(&Key::ArrowDown),
            "closed palette ignores keys"
        );
        assert!(!p.handle_text_input("x"));
        p.toggle();
        assert!(p.handle_key_down(&Key::Escape));
        assert!(!p.is_open);
        assert_eq!(p.take_chosen(), None);
    }

    #[test]
    fn backspace_widens_the_results_again() {
        let mut p = CommandPalette::default();
        p.toggle();
        p.handle_text_input("zzz");
        assert!(p.filtered_commands().is_empty());
        p.handle_key_down(&Key::Enter);
        assert_eq!(
            p.take_chosen(),
            None,
            "Enter with no results chooses nothing"
        );
        for _ in 0..3 {
            p.handle_key_down(&Key::Backspace);
        }
        assert_eq!(p.filtered_commands().len(), 4);
    }

    #[test]
    fn hover_selects_and_click_chooses_the_row_under_the_pointer() {
        let mut p = CommandPalette::default();
        p.toggle();
        assert!(p.handle_pointer_move(row_center(2), SCREEN));
        assert_eq!(p.selected_index, 2);

        assert!(p.handle_pointer_down(row_center(2), PointerButton::Primary, SCREEN));
        assert_eq!(p.take_chosen().as_deref(), Some("toggle_gpu_safe"));
        assert!(!p.is_open);
    }

    #[test]
    fn click_outside_dismisses_and_secondary_click_does_nothing() {
        let mut p = CommandPalette::default();
        p.toggle();
        assert!(p.handle_pointer_down(Vec2::new(5.0, 5.0), PointerButton::Secondary, SCREEN));
        assert!(p.is_open);
        assert!(p.handle_pointer_down(Vec2::new(5.0, 5.0), PointerButton::Primary, SCREEN));
        assert!(!p.is_open);
        assert_eq!(p.take_chosen(), None);
    }

    #[test]
    fn long_lists_scroll_with_the_selection_and_the_wheel() {
        let mut p = palette_with(10);
        for _ in 0..PALETTE_VISIBLE_ROWS {
            p.handle_key_down(&Key::ArrowDown);
        }
        // Selection is row 6, one past the first window, so the window scrolled by one.
        assert_eq!(p.selected_index, PALETTE_VISIBLE_ROWS);
        assert_eq!(p.scroll, 1);

        p.handle_key_down(&Key::Enter);
        assert_eq!(p.take_chosen().as_deref(), Some("cmd_6"));

        p.toggle();
        p.handle_scroll(Vec2::new(0.0, -1.0));
        p.handle_scroll(Vec2::new(0.0, -1.0));
        p.handle_scroll(Vec2::new(0.0, -1.0));
        p.handle_scroll(Vec2::new(0.0, -1.0));
        p.handle_scroll(Vec2::new(0.0, -1.0));
        assert_eq!(p.scroll, 4, "the window stops at the end of the list");
        assert!(p.selected_index >= p.scroll);
        // Clicking the first visible row picks the first item in the window.
        p.handle_pointer_down(row_center(0), PointerButton::Primary, SCREEN);
        assert_eq!(p.take_chosen().as_deref(), Some("cmd_4"));
    }

    #[test]
    fn render_draws_only_the_visible_window() {
        let mut p = palette_with(10);
        p.scroll = 4;
        p.selected_index = 4;
        let mut buf = CommandBuffer::default();
        p.render(&mut buf, SCREEN, &ThemePalette::dark());
        let texts: Vec<String> = buf
            .commands()
            .iter()
            .filter_map(|c| match c {
                DrawCommand::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect();
        assert!(texts.iter().any(|t| t == "Test: Command 4"));
        assert!(texts.iter().any(|t| t == "Test: Command 9"));
        assert!(!texts.iter().any(|t| t == "Test: Command 3"));
    }
}
