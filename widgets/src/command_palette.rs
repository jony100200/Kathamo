//! Command palette (Ctrl+K) quick launcher and shortcut directory.

use crate::theme::ThemePalette;
use ui_core::{CornerRadius, Rect, Stroke, Vec2};
use ui_render::{CommandBuffer, DrawCommand};

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
    pub selected_index: usize,
    pub commands: Vec<PaletteCommand>,
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
        }
    }
}

impl CommandPalette {
    pub fn toggle(&mut self) {
        self.is_open = !self.is_open;
        if self.is_open {
            self.query.clear();
            self.selected_index = 0;
        }
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
        let dialog_w = 540.0;
        let dialog_h = 360.0;
        let dialog_rect = Rect::from_min_size(
            Vec2::new((screen_size.x - dialog_w) * 0.5, screen_size.y * 0.2),
            Vec2::new(dialog_w, dialog_h),
        );

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
            Vec2::new(dialog_w - 32.0, 38.0),
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

        // List results
        let filtered = self.filtered_commands();
        let mut y = search_rect.max.y + 12.0;
        for (i, cmd) in filtered.iter().enumerate().take(6) {
            let row_rect = Rect::from_min_size(
                Vec2::new(dialog_rect.min.x + 16.0, y),
                Vec2::new(dialog_w - 32.0, 34.0),
            );

            let is_sel = i == self.selected_index;
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

            y += 38.0;
        }
    }
}
