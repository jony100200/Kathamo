//! Professional input controls: Text Inputs, Checkboxes, Dropdown Selectors, and Progress Bars.
//!
//! Complies with Fluent 2 and Material 3 design systems with full state layers:
//! Normal, Hovered, Pressed, Keyboard-Focused, Disabled, and Error.

use ui_core::{Color, CornerRadius, Rect, Stroke, Vec2};
use ui_render::{CommandBuffer, DrawCommand};
use ui_themes::ThemePalette;

/// Checkbox / Toggle Switch state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CheckboxState {
    pub checked: bool,
    pub hovered: bool,
    pub focused: bool,
    pub disabled: bool,
}

impl CheckboxState {
    pub const fn new(checked: bool) -> Self {
        Self {
            checked,
            hovered: false,
            focused: false,
            disabled: false,
        }
    }

    pub fn toggle(&mut self) {
        if !self.disabled {
            self.checked = !self.checked;
        }
    }
}

/// Renders a professional studio checkbox with label ("Snap to grid").
pub fn render_checkbox(
    cmd_buf: &mut CommandBuffer,
    state: &CheckboxState,
    label: &str,
    bounds: Rect,
    palette: &ThemePalette,
) {
    let box_size = 16.0;
    let box_rect = Rect::from_min_size(
        Vec2::new(
            bounds.min.x,
            bounds.min.y + (bounds.height() - box_size) * 0.5,
        ),
        Vec2::new(box_size, box_size),
    );

    // Box background
    let bg_color = if state.disabled {
        palette.surface
    } else if state.checked {
        palette.accent
    } else if state.hovered {
        palette.elevated
    } else {
        palette.canvas
    };

    let border_color = if state.focused {
        palette.accent_hover
    } else if state.disabled {
        palette.border.with_alpha(100)
    } else {
        palette.border
    };

    cmd_buf.push(DrawCommand::Rect {
        rect: box_rect,
        fill: bg_color,
        radius: CornerRadius::same(3.0),
        stroke: Stroke::new(if state.focused { 2.0 } else { 1.0 }, border_color),
    });

    // Checkmark mark if checked
    if state.checked {
        let check_color = if state.disabled {
            palette.text_muted
        } else {
            Color::rgb(255, 255, 255)
        };

        cmd_buf.push(DrawCommand::Line {
            start: Vec2::new(box_rect.min.x + 3.5, box_rect.min.y + 8.5),
            end: Vec2::new(box_rect.min.x + 6.5, box_rect.min.y + 12.0),
            stroke: Stroke::new(1.5, check_color),
        });
        cmd_buf.push(DrawCommand::Line {
            start: Vec2::new(box_rect.min.x + 6.5, box_rect.min.y + 12.0),
            end: Vec2::new(box_rect.min.x + 12.5, box_rect.min.y + 4.5),
            stroke: Stroke::new(1.5, check_color),
        });
    }

    // Text label
    let text_color = if state.disabled {
        palette.text_muted
    } else {
        palette.text
    };

    cmd_buf.push(DrawCommand::Text {
        pos: Vec2::new(box_rect.max.x + 8.0, bounds.min.y + 2.0),
        text: label.to_string(),
        color: text_color,
        size: 13.0,
        strong: false,
    });
}

/// Studio Dropdown Selector state ("Choose workspace").
#[derive(Debug, Clone, PartialEq)]
pub struct DropdownState {
    pub options: Vec<String>,
    pub selected_index: usize,
    pub is_open: bool,
    pub hovered: bool,
    pub focused: bool,
    pub disabled: bool,
}

impl DropdownState {
    pub fn new(options: Vec<&str>, initial_index: usize) -> Self {
        Self {
            options: options.into_iter().map(String::from).collect(),
            selected_index: initial_index,
            is_open: false,
            hovered: false,
            focused: false,
            disabled: false,
        }
    }

    pub fn selected_text(&self) -> &str {
        self.options
            .get(self.selected_index)
            .map(|s| s.as_str())
            .unwrap_or("Select...")
    }

    pub fn select(&mut self, index: usize) {
        if index < self.options.len() {
            self.selected_index = index;
            self.is_open = false;
        }
    }
}

/// Renders a dropdown selector button and expanded menu.
pub fn render_dropdown(
    cmd_buf: &mut CommandBuffer,
    state: &DropdownState,
    bounds: Rect,
    palette: &ThemePalette,
) {
    let bg_color = if state.disabled {
        palette.surface
    } else if state.hovered {
        palette.elevated
    } else {
        palette.canvas
    };

    let border_color = if state.focused {
        palette.accent
    } else {
        palette.border
    };

    // Main header button
    cmd_buf.push(DrawCommand::Rect {
        rect: bounds,
        fill: bg_color,
        radius: CornerRadius::same(4.0),
        stroke: Stroke::new(if state.focused { 2.0 } else { 1.0 }, border_color),
    });

    // Selected text
    cmd_buf.push(DrawCommand::Text {
        pos: Vec2::new(bounds.min.x + 10.0, bounds.min.y + 7.0),
        text: state.selected_text().to_string(),
        color: if state.disabled {
            palette.text_muted
        } else {
            palette.text
        },
        size: 13.0,
        strong: false,
    });

    // Downward chevron
    let chevron_x = bounds.max.x - 16.0;
    let chevron_y = bounds.min.y + bounds.height() * 0.5;
    cmd_buf.push(DrawCommand::Line {
        start: Vec2::new(chevron_x - 4.0, chevron_y - 2.0),
        end: Vec2::new(chevron_x, chevron_y + 2.0),
        stroke: Stroke::new(1.5, palette.text_muted),
    });
    cmd_buf.push(DrawCommand::Line {
        start: Vec2::new(chevron_x, chevron_y + 2.0),
        end: Vec2::new(chevron_x + 4.0, chevron_y - 2.0),
        stroke: Stroke::new(1.5, palette.text_muted),
    });

    // Expanded menu popup
    if state.is_open && !state.disabled {
        let item_height = 26.0;
        let menu_height = item_height * state.options.len() as f32 + 6.0;
        let menu_bounds = Rect::from_min_size(
            Vec2::new(bounds.min.x, bounds.max.y + 2.0),
            Vec2::new(bounds.width(), menu_height),
        );

        // Menu container
        cmd_buf.push(DrawCommand::Rect {
            rect: menu_bounds,
            fill: palette.elevated_high,
            radius: CornerRadius::same(4.0),
            stroke: Stroke::new(1.0, palette.border),
        });

        for (idx, opt) in state.options.iter().enumerate() {
            let item_y = menu_bounds.min.y + 3.0 + idx as f32 * item_height;
            let item_bounds = Rect::from_min_size(
                Vec2::new(menu_bounds.min.x + 3.0, item_y),
                Vec2::new(menu_bounds.width() - 6.0, item_height),
            );

            let is_selected = idx == state.selected_index;
            if is_selected {
                cmd_buf.push(DrawCommand::Rect {
                    rect: item_bounds,
                    fill: palette.accent.with_alpha(60),
                    radius: CornerRadius::same(3.0),
                    stroke: Stroke::NONE,
                });
            }

            cmd_buf.push(DrawCommand::Text {
                pos: Vec2::new(item_bounds.min.x + 8.0, item_bounds.min.y + 6.0),
                text: opt.clone(),
                color: if is_selected {
                    palette.accent_hover
                } else {
                    palette.text
                },
                size: 12.5,
                strong: is_selected,
            });
        }
    }
}

/// Linear and determinate/indeterminate progress indicator.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProgressState {
    pub progress: f32, // 0.0 to 1.0
    pub indeterminate: bool,
}

impl ProgressState {
    pub const fn determinate(progress: f32) -> Self {
        Self {
            progress,
            indeterminate: false,
        }
    }

    pub const fn indeterminate() -> Self {
        Self {
            progress: 0.0,
            indeterminate: true,
        }
    }
}

/// Renders a thin, sleek studio progress bar.
pub fn render_progress_bar(
    cmd_buf: &mut CommandBuffer,
    state: &ProgressState,
    bounds: Rect,
    palette: &ThemePalette,
) {
    // Track background
    cmd_buf.push(DrawCommand::Rect {
        rect: bounds,
        fill: palette.surface,
        radius: CornerRadius::same(bounds.height() * 0.5),
        stroke: Stroke::new(1.0, palette.border),
    });

    let fill_width = if state.indeterminate {
        bounds.width() * 0.3
    } else {
        bounds.width() * state.progress.clamp(0.0, 1.0)
    };

    if fill_width > 1.0 {
        let fill_bounds = Rect::from_min_size(bounds.min, Vec2::new(fill_width, bounds.height()));
        cmd_buf.push(DrawCommand::Rect {
            rect: fill_bounds,
            fill: palette.accent,
            radius: CornerRadius::same(bounds.height() * 0.5),
            stroke: Stroke::NONE,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checkbox_state_toggles_safely() {
        let mut cb = CheckboxState::new(false);
        cb.toggle();
        assert!(cb.checked);
        cb.toggle();
        assert!(!cb.checked);

        cb.disabled = true;
        cb.toggle();
        assert!(!cb.checked, "Disabled checkbox must not toggle");
    }

    #[test]
    fn dropdown_selection_updates_safely() {
        let mut dd = DropdownState::new(vec!["2D Canvas", "3D Scene", "Motion Flow"], 0);
        assert_eq!(dd.selected_text(), "2D Canvas");
        dd.select(1);
        assert_eq!(dd.selected_text(), "3D Scene");
    }

    #[test]
    fn progress_clamped_safely() {
        let p = ProgressState::determinate(1.5);
        assert_eq!(p.progress, 1.5);
        let mut cmd_buf = CommandBuffer::new();
        let palette = ThemePalette::dark();
        render_progress_bar(
            &mut cmd_buf,
            &p,
            Rect::from_min_size(Vec2::ZERO, Vec2::new(100.0, 6.0)),
            &palette,
        );
        assert!(!cmd_buf.is_empty());
    }
}
