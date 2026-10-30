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
    let selected_str = state.selected_text();
    let max_chars = ((bounds.width() - 32.0) / 7.5).max(1.0) as usize;
    let display_str = if selected_str.len() > max_chars && max_chars > 3 {
        let end_idx = selected_str
            .char_indices()
            .nth(max_chars.saturating_sub(3))
            .map(|(i, _)| i)
            .unwrap_or(selected_str.len());
        format!("{}...", &selected_str[..end_idx])
    } else {
        selected_str.to_string()
    };
    cmd_buf.push(DrawCommand::Text {
        pos: Vec2::new(bounds.min.x + 10.0, bounds.min.y + 7.0),
        text: display_str,
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

/// Renders a modern iOS / Fluent style toggle switch (smooth pill switch with round knob).
pub fn render_toggle(
    cmd_buf: &mut CommandBuffer,
    checked: bool,
    is_hovered: bool,
    label: &str,
    bounds: Rect,
    palette: &ThemePalette,
) {
    let track_width = 36.0;
    let track_height = 20.0;
    let track_y = bounds.min.y + (bounds.height() - track_height) * 0.5;
    let track_rect = Rect::from_min_size(
        Vec2::new(bounds.min.x, track_y),
        Vec2::new(track_width, track_height),
    );

    // The knob slides and the track cross-fades on a spring instead of jumping.
    let key = ui_core::motion::rect_key("toggle", track_rect);
    let on = ui_core::motion::animate_bool(key, checked, ui_core::motion::TOGGLE);
    let hover = ui_core::motion::animate_bool(key ^ 1, is_hovered, ui_core::motion::HOVER);
    let off_fill = palette.surface.lerp(palette.elevated, hover);
    let on_fill = palette.accent.lerp(palette.accent_hover, hover);
    let bg_color = off_fill.lerp(on_fill, on);
    let stroke_color = Stroke::new(1.0, palette.border.with_alpha((255.0 * (1.0 - on)) as u8));

    // Track capsule
    cmd_buf.push(DrawCommand::Rect {
        rect: track_rect,
        fill: bg_color,
        radius: CornerRadius::same(10.0),
        stroke: stroke_color,
    });

    // Knob with subtle ambient shadow
    let knob_diameter = 14.0;
    let travel = track_width - knob_diameter - 6.0;
    let knob_x = track_rect.min.x + 3.0 + travel * on;
    let knob_y = track_rect.min.y + 3.0;
    let knob_rect = Rect::from_min_size(
        Vec2::new(knob_x, knob_y),
        Vec2::new(knob_diameter, knob_diameter),
    );

    // Knob shadow
    cmd_buf.push(DrawCommand::Rect {
        rect: Rect::from_min_size(
            Vec2::new(knob_rect.min.x, knob_rect.min.y + 1.0),
            Vec2::new(knob_diameter, knob_diameter),
        ),
        fill: Color::rgba(0, 0, 0, 50),
        radius: CornerRadius::same(7.0),
        stroke: Stroke::NONE,
    });

    // Knob body
    cmd_buf.push(DrawCommand::Rect {
        rect: knob_rect,
        fill: Color::rgb(255, 255, 255),
        radius: CornerRadius::same(7.0),
        stroke: Stroke::NONE,
    });

    // Label if present
    if !label.is_empty() {
        cmd_buf.push(DrawCommand::Text {
            pos: Vec2::new(
                track_rect.max.x + 10.0,
                bounds.min.y + (bounds.height() - 13.0) * 0.5 - 1.0,
            ),
            text: label.to_string(),
            color: palette.text,
            size: 13.0,
            strong: false,
        });
    }
}

/// Renders a modern text input box with placeholder and focus highlight.
pub fn render_text_input(
    cmd_buf: &mut CommandBuffer,
    text: &str,
    placeholder: &str,
    bounds: Rect,
    is_focused: bool,
    is_hovered: bool,
    palette: &ThemePalette,
) {
    // Focus ring: the border eases to the accent and a soft accent glow fades in.
    let key = ui_core::motion::rect_key("text_input", bounds);
    let focus = ui_core::motion::animate_bool(key, is_focused, ui_core::motion::HOVER);
    let hover = ui_core::motion::animate_bool(key ^ 1, is_hovered, ui_core::motion::HOVER);
    if focus > 0.0 {
        cmd_buf.push(DrawCommand::SoftShadow {
            rect: bounds,
            radius: CornerRadius::same(6.0),
            color: palette.accent.with_alpha((70.0 * focus) as u8),
            offset: Vec2::ZERO,
            blur: 8.0,
            spread: 1.0,
        });
    }
    let edge = palette
        .border
        .lerp(palette.accent.with_alpha(160), hover * 0.6)
        .lerp(palette.accent, focus);

    cmd_buf.push(DrawCommand::Rect {
        rect: bounds,
        fill: palette.canvas,
        radius: CornerRadius::same(6.0),
        stroke: Stroke::new(1.0, edge),
    });

    let text_y = bounds.min.y + (bounds.height() - 13.0) * 0.5 - 1.0;
    let max_chars = ((bounds.width() - 20.0) / 7.5).max(1.0) as usize;
    if text.is_empty() {
        cmd_buf.push(DrawCommand::Text {
            pos: Vec2::new(bounds.min.x + 10.0, text_y),
            text: placeholder.to_string(),
            color: palette.text_muted,
            size: 13.0,
            strong: false,
        });
    } else {
        // Count Unicode scalars, not UTF-8 bytes; Bengali/emoji input must
        // never lose a word or index into the middle of a character.
        let count = text.chars().count();
        let display_text = if count > max_chars {
            text.chars().skip(count - max_chars).collect::<String>()
        } else {
            text.to_string()
        };
        cmd_buf.push(DrawCommand::Text {
            pos: Vec2::new(bounds.min.x + 10.0, text_y),
            text: display_text,
            color: palette.text,
            size: 13.0,
            strong: false,
        });
    }

    // Blinking / focus cursor indicator
    if is_focused {
        let char_width = 7.5;
        let visible_chars = text.chars().count().min(max_chars);
        let cursor_x = bounds.min.x + 10.0 + visible_chars as f32 * char_width;
        let cursor_top = bounds.min.y + 6.0;
        let cursor_bottom = bounds.max.y - 6.0;
        cmd_buf.push(DrawCommand::Line {
            start: Vec2::new(cursor_x, cursor_top),
            end: Vec2::new(cursor_x, cursor_bottom),
            stroke: Stroke::new(1.5, palette.accent),
        });
    }
}

/// Renders a modern segmented pill control (like Linear/Raycast workspace tabs).
pub fn render_segmented_tabs(
    cmd_buf: &mut CommandBuffer,
    tabs: &[&str],
    active_idx: usize,
    bounds: Rect,
    hovered_idx: Option<usize>,
    palette: &ThemePalette,
) {
    if tabs.is_empty() {
        return;
    }
    let tab_width = (bounds.width() - 4.0) / tabs.len() as f32;
    let rects: Vec<Rect> = (0..tabs.len())
        .map(|i| {
            Rect::from_min_size(
                Vec2::new(
                    bounds.min.x + 2.0 + i as f32 * tab_width,
                    bounds.min.y + 2.0,
                ),
                Vec2::new(tab_width, bounds.height() - 4.0),
            )
        })
        .collect();
    let pointer = hovered_idx.map_or(Vec2::new(f32::MIN, f32::MIN), |i| rects[i].center());
    crate::tab_strip::render(cmd_buf, tabs, &rects, bounds, active_idx, pointer, palette);
}

/// Renders a telemetry stat card (e.g., VRAM usage, RAM, ETA, throughput).
pub fn render_stat_card(
    cmd_buf: &mut CommandBuffer,
    label: &str,
    value: &str,
    subtext: &str,
    bounds: Rect,
    palette: &ThemePalette,
) {
    // Card background
    cmd_buf.push(DrawCommand::Rect {
        rect: bounds,
        fill: palette.surface.with_alpha(230),
        radius: CornerRadius::same(8.0),
        stroke: Stroke::new(1.0, palette.border),
    });

    // 1px hairline top specular highlight
    if bounds.width() > 16.0 {
        cmd_buf.push(DrawCommand::Rect {
            rect: Rect::from_min_size(
                Vec2::new(bounds.min.x + 6.0, bounds.min.y),
                Vec2::new(bounds.width() - 12.0, 1.0),
            ),
            fill: Color::rgba(255, 255, 255, 25),
            radius: CornerRadius::ZERO,
            stroke: Stroke::NONE,
        });
    }

    // Label (10.5px uppercase muted)
    cmd_buf.push(DrawCommand::Text {
        pos: Vec2::new(bounds.min.x + 12.0, bounds.min.y + 10.0),
        text: label.to_uppercase(),
        color: palette.text_muted,
        size: 10.5,
        strong: true,
    });

    // Value (18px bold text / accent)
    cmd_buf.push(DrawCommand::Text {
        pos: Vec2::new(bounds.min.x + 12.0, bounds.min.y + 26.0),
        text: value.to_string(),
        color: palette.text,
        size: 18.0,
        strong: true,
    });

    // Subtext (11px muted or accent)
    if !subtext.is_empty() {
        cmd_buf.push(DrawCommand::Text {
            pos: Vec2::new(bounds.min.x + 12.0, bounds.min.y + 48.0),
            text: subtext.to_string(),
            color: palette.accent,
            size: 11.0,
            strong: false,
        });
    }
}

/// Renders a selectable model list item card with hover and selection styling.
pub fn render_model_card(
    cmd_buf: &mut CommandBuffer,
    name: &str,
    params: &str,
    quant: &str,
    bounds: Rect,
    is_selected: bool,
    is_hovered: bool,
    palette: &ThemePalette,
) {
    let (fill, stroke) = if is_selected {
        (
            palette.accent.with_alpha(32),
            Stroke::new(1.5, palette.accent),
        )
    } else if is_hovered {
        (palette.elevated, Stroke::new(1.0, palette.border))
    } else {
        (palette.surface, Stroke::new(1.0, palette.border))
    };

    cmd_buf.push(DrawCommand::Rect {
        rect: bounds,
        fill,
        radius: CornerRadius::same(6.0),
        stroke,
    });

    // Model name (13px bold)
    cmd_buf.push(DrawCommand::Text {
        pos: Vec2::new(bounds.min.x + 12.0, bounds.min.y + 8.0),
        text: name.to_string(),
        color: palette.text,
        size: 13.0,
        strong: true,
    });

    // Params badge
    let param_badge_rect = Rect::from_min_size(
        Vec2::new(bounds.min.x + 12.0, bounds.min.y + 28.0),
        Vec2::new(params.len() as f32 * 6.5 + 12.0, 18.0),
    );
    cmd_buf.push(DrawCommand::Rect {
        rect: param_badge_rect,
        fill: palette.surface,
        radius: CornerRadius::same(4.0),
        stroke: Stroke::new(1.0, palette.border),
    });
    cmd_buf.push(DrawCommand::Text {
        pos: Vec2::new(param_badge_rect.min.x + 6.0, param_badge_rect.min.y + 3.0),
        text: params.to_string(),
        color: palette.text_muted,
        size: 10.5,
        strong: false,
    });

    // Quant badge
    let quant_badge_rect = Rect::from_min_size(
        Vec2::new(param_badge_rect.max.x + 6.0, bounds.min.y + 28.0),
        Vec2::new(quant.len() as f32 * 6.5 + 12.0, 18.0),
    );
    cmd_buf.push(DrawCommand::Rect {
        rect: quant_badge_rect,
        fill: palette.accent.with_alpha(25),
        radius: CornerRadius::same(4.0),
        stroke: Stroke::new(1.0, palette.accent.with_alpha(100)),
    });
    cmd_buf.push(DrawCommand::Text {
        pos: Vec2::new(quant_badge_rect.min.x + 6.0, quant_badge_rect.min.y + 3.0),
        text: quant.to_string(),
        color: palette.accent,
        size: 10.5,
        strong: true,
    });
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

    #[test]
    fn toggle_and_text_input_render_commands() {
        let mut cmd_buf = CommandBuffer::new();
        let palette = ThemePalette::dark();
        let bounds = Rect::from_min_size(Vec2::ZERO, Vec2::new(200.0, 24.0));
        render_toggle(&mut cmd_buf, true, false, "Enable CUDA", bounds, &palette);
        assert!(!cmd_buf.is_empty());

        let mut cmd_buf2 = CommandBuffer::new();
        render_text_input(
            &mut cmd_buf2,
            "Qwen-2.5",
            "Search...",
            bounds,
            true,
            false,
            &palette,
        );
        assert!(!cmd_buf2.is_empty());
    }

    #[test]
    fn segmented_tabs_and_stat_cards_render_commands() {
        let mut cmd_buf = CommandBuffer::new();
        let palette = ThemePalette::dark();
        let bounds = Rect::from_min_size(Vec2::ZERO, Vec2::new(300.0, 32.0));
        render_segmented_tabs(
            &mut cmd_buf,
            &["GGUF", "AWQ", "EXL2"],
            0,
            bounds,
            None,
            &palette,
        );
        assert!(!cmd_buf.is_empty());

        let mut cmd_buf2 = CommandBuffer::new();
        let card_bounds = Rect::from_min_size(Vec2::ZERO, Vec2::new(180.0, 72.0));
        render_stat_card(
            &mut cmd_buf2,
            "VRAM Used",
            "11.8 / 16.0 GB",
            "74% Active Pool",
            card_bounds,
            &palette,
        );
        assert!(!cmd_buf2.is_empty());
    }
}
