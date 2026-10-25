//! Declarative, Composable Layout Engine for Kathamo SDK.

use ui_core::*;
use ui_platform::PointerState;
use ui_render::command::DrawCommand;
use ui_render::CommandBuffer;
use ui_themes::ThemePalette;

/// Layout direction for declarative containers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutDirection {
    Vertical,
    Horizontal,
}

/// Fluent declarative UI builder that eliminates manual coordinate math.
pub struct LayoutBuilder<'a> {
    pub bounds: Rect,
    pub cursor: Vec2,
    pub direction: LayoutDirection,
    pub spacing: f32,
    pub padding: f32,
    pub cmd_buf: &'a mut CommandBuffer,
    pub ptr: &'a PointerState,
    pub palette: &'a ThemePalette,
}

impl<'a> LayoutBuilder<'a> {
    pub fn new(
        bounds: Rect,
        cmd_buf: &'a mut CommandBuffer,
        ptr: &'a PointerState,
        palette: &'a ThemePalette,
    ) -> Self {
        let padding = 8.0;
        let cursor = Vec2::new(bounds.min.x + padding, bounds.min.y + padding);
        Self {
            bounds,
            cursor,
            direction: LayoutDirection::Vertical,
            spacing: 6.0,
            padding,
            cmd_buf,
            ptr,
            palette,
        }
    }

    /// Customize spacing without changing the global theme.
    pub fn with_spacing(mut self, spacing: f32) -> Self {
        self.spacing = spacing.max(0.0);
        self
    }

    /// Customize container padding and update its content origin.
    pub fn with_padding(mut self, padding: f32) -> Self {
        self.padding = padding.max(0.0);
        self.cursor = self.bounds.min + Vec2::splat(self.padding);
        self
    }

    /// Use a horizontal or vertical item flow.
    pub fn with_direction(mut self, direction: LayoutDirection) -> Self {
        self.direction = direction;
        self
    }

    /// Draw a weighted, responsive multi-column row.
    /// Each child receives its own bounds, cursor, and independent item layout.
    pub fn flex_columns(
        &mut self,
        height: f32,
        weights: &[f32],
        gap: f32,
        mut build: impl FnMut(usize, &mut LayoutBuilder<'_>),
    ) {
        if weights.is_empty() { return; }
        let gap = gap.max(0.0);
        let row_width = (self.bounds.max.x - self.padding - self.cursor.x).max(0.0);
        let usable = (row_width - gap * (weights.len().saturating_sub(1) as f32)).max(0.0);
        let total: f32 = weights.iter().map(|w| w.max(0.0)).sum();
        let total = if total > 0.0 { total } else { weights.len() as f32 };
        let mut x = self.cursor.x;
        let start_y = self.cursor.y;
        for (index, weight) in weights.iter().enumerate() {
            let w = usable * if *weight > 0.0 { *weight / total } else if total == weights.len() as f32 { 1.0 / total } else { 0.0 };
            let rect = Rect::from_min_size(Vec2::new(x, start_y), Vec2::new(w, height.max(0.0)));
            self.cmd_buf.push(DrawCommand::ClipPush { rect });
            {
                let mut child = LayoutBuilder::new(rect, &mut *self.cmd_buf, self.ptr, self.palette)
                    .with_spacing(self.spacing)
                    .with_padding(self.padding.min(w * 0.5));
                build(index, &mut child);
            }
            self.cmd_buf.push(DrawCommand::ClipPop);
            x += w + gap;
        }
        self.cursor.y = start_y + height.max(0.0) + self.spacing;
    }

    /// Add empty spacing between items.
    pub fn space(&mut self, amount: f32) {
        match self.direction {
            LayoutDirection::Vertical => self.cursor.y += amount,
            LayoutDirection::Horizontal => self.cursor.x += amount,
        }
    }

    /// Render a subtle horizontal divider line.
    pub fn separator(&mut self) {
        let sep_rect = match self.direction {
            LayoutDirection::Vertical => {
                let r = Rect::from_min_size(
                    Vec2::new(self.bounds.min.x + self.padding, self.cursor.y + 2.0),
                    Vec2::new(self.bounds.width() - (self.padding * 2.0), 1.0),
                );
                self.cursor.y += 6.0;
                r
            }
            LayoutDirection::Horizontal => {
                let r = Rect::from_min_size(
                    Vec2::new(self.cursor.x + 2.0, self.bounds.min.y + self.padding),
                    Vec2::new(1.0, self.bounds.height() - (self.padding * 2.0)),
                );
                self.cursor.x += 6.0;
                r
            }
        };

        self.cmd_buf.push(DrawCommand::Rect {
            rect: sep_rect,
            fill: self.palette.border.with_alpha(120),
            radius: CornerRadius::ZERO,
            stroke: Stroke::NONE,
        });
    }

    /// Section heading.
    pub fn heading(&mut self, text: &str) {
        self.cmd_buf.push(DrawCommand::Text {
            text: text.to_string(),
            pos: self.cursor,
            size: 13.0,
            color: self.palette.text,
            strong: true,
        });
        self.cursor.y += 20.0;
    }

    /// Text label.
    pub fn label(&mut self, text: &str) {
        self.cmd_buf.push(DrawCommand::Text {
            text: text.to_string(),
            pos: self.cursor,
            size: 11.5,
            color: self.palette.text_muted,
            strong: false,
        });
        match self.direction {
            LayoutDirection::Vertical => self.cursor.y += 18.0,
            LayoutDirection::Horizontal => self.cursor.x += text.len() as f32 * 7.0 + 8.0,
        }
    }

    /// Clickable push button. Returns true on click.
    pub fn button(&mut self, text: &str) -> bool {
        let btn_w = text.len() as f32 * 7.5 + 24.0;
        let btn_h = 24.0;
        let rect = Rect::from_min_size(self.cursor, Vec2::new(btn_w, btn_h));
        let hovered = rect.contains(self.ptr.position);
        let clicked = hovered && self.ptr.primary_clicked;

        self.cmd_buf.push(DrawCommand::Rect {
            rect,
            fill: if clicked {
                self.palette.elevated_high
            } else if hovered {
                self.palette.elevated
            } else {
                self.palette.surface
            },
            radius: CornerRadius::same(4.0),
            stroke: Stroke::new(1.0, self.palette.border),
        });

        self.cmd_buf.push(DrawCommand::Text {
            text: text.to_string(),
            pos: Vec2::new(rect.min.x + 10.0, rect.min.y + 5.0),
            size: 11.0,
            color: self.palette.text,
            strong: false,
        });

        match self.direction {
            LayoutDirection::Vertical => self.cursor.y += btn_h + self.spacing,
            LayoutDirection::Horizontal => self.cursor.x += btn_w + self.spacing,
        }

        clicked
    }

    /// Toggle switch. Returns true if value changed.
    pub fn toggle(&mut self, label: &str, checked: &mut bool) -> bool {
        let row_h = 22.0;
        let toggle_w = 34.0;
        let toggle_h = 16.0;

        let row_rect = Rect::from_min_size(self.cursor, Vec2::new(self.bounds.width() - (self.padding * 2.0), row_h));
        let toggle_rect = Rect::from_min_size(Vec2::new(row_rect.max.x - toggle_w, row_rect.min.y + 3.0), Vec2::new(toggle_w, toggle_h));

        let clicked = toggle_rect.contains(self.ptr.position) && self.ptr.primary_clicked;
        if clicked {
            *checked = !*checked;
        }

        // Label
        self.cmd_buf.push(DrawCommand::Text {
            text: label.to_string(),
            pos: Vec2::new(row_rect.min.x, row_rect.min.y + 3.0),
            size: 11.5,
            color: self.palette.text,
            strong: false,
        });

        // Toggle track
        self.cmd_buf.push(DrawCommand::Rect {
            rect: toggle_rect,
            fill: if *checked { self.palette.accent } else { self.palette.surface },
            radius: CornerRadius::same(8.0),
            stroke: Stroke::new(1.0, self.palette.border),
        });

        // Toggle knob
        let knob_x = if *checked { toggle_rect.max.x - 14.0 } else { toggle_rect.min.x + 2.0 };
        self.cmd_buf.push(DrawCommand::Rect {
            rect: Rect::from_min_size(Vec2::new(knob_x, toggle_rect.min.y + 2.0), Vec2::new(12.0, 12.0)),
            fill: Color::WHITE,
            radius: CornerRadius::same(6.0),
            stroke: Stroke::NONE,
        });

        self.cursor.y += row_h + self.spacing;
        clicked
    }

    /// High-precision numeric scrubber with horizontal mouse drag.
    pub fn scrubber(&mut self, label: &str, value: &mut f32, min: f32, max: f32, step: f32) -> bool {
        let row_h = 24.0;
        let row_rect = Rect::from_min_size(self.cursor, Vec2::new(self.bounds.width() - (self.padding * 2.0), row_h));

        // Label (left half)
        self.cmd_buf.push(DrawCommand::Text {
            text: label.to_string(),
            pos: Vec2::new(row_rect.min.x, row_rect.min.y + 5.0),
            size: 11.5,
            color: self.palette.text_muted,
            strong: false,
        });

        // Scrubber box (right half)
        let box_w = 100.0;
        let box_rect = Rect::from_min_size(Vec2::new(row_rect.max.x - box_w, row_rect.min.y), Vec2::new(box_w, row_h));
        let hovered = box_rect.contains(self.ptr.position);
        let mut changed = false;

        if hovered && self.ptr.primary_down && self.ptr.drag_delta.x.abs() > 0.0 {
            *value = (*value + (self.ptr.drag_delta.x * step)).clamp(min, max);
            changed = true;
        }

        self.cmd_buf.push(DrawCommand::Rect {
            rect: box_rect,
            fill: if hovered { self.palette.elevated } else { self.palette.surface },
            radius: CornerRadius::same(4.0),
            stroke: Stroke::new(1.0, self.palette.border),
        });

        // Value text
        self.cmd_buf.push(DrawCommand::Text {
            text: format!("{:.2}", *value),
            pos: Vec2::new(box_rect.min.x + 8.0, box_rect.min.y + 5.0),
            size: 11.0,
            color: self.palette.text,
            strong: false,
        });

        self.cursor.y += row_h + self.spacing;
        changed
    }

    /// Nested horizontal layout scope.
    pub fn horizontal<F>(&mut self, f: F)
    where
        F: FnOnce(&mut LayoutBuilder),
    {
        let original_cursor = self.cursor;
        self.direction = LayoutDirection::Horizontal;
        f(self);
        self.direction = LayoutDirection::Vertical;
        self.cursor = Vec2::new(original_cursor.x, self.cursor.y + 24.0 + self.spacing);
    }

    /// Nested container card with background and border.
    pub fn card<F>(&mut self, f: F)
    where
        F: FnOnce(&mut LayoutBuilder),
    {
        let start_y = self.cursor.y;
        let card_w = self.bounds.width() - (self.padding * 2.0);
        
        self.cursor.y += self.padding;
        f(self);
        let end_y = self.cursor.y + self.padding;
        let card_h = end_y - start_y;

        let card_rect = Rect::from_min_size(Vec2::new(self.bounds.min.x + self.padding, start_y), Vec2::new(card_w, card_h));

        self.cmd_buf.push(DrawCommand::Rect {
            rect: card_rect,
            fill: self.palette.elevated,
            radius: CornerRadius::same(6.0),
            stroke: Stroke::new(1.0, self.palette.border),
        });

        self.cursor.y = end_y + self.spacing;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_declarative_layout_builder_emits_commands() {
        let bounds = Rect::from_min_size(Vec2::ZERO, Vec2::new(300.0, 500.0));
        let mut cmd_buf = CommandBuffer::new();
        let ptr = PointerState::default();
        let palette = ThemePalette::dark();

        let mut ui = LayoutBuilder::new(bounds, &mut cmd_buf, &ptr, &palette);
        ui.heading("Settings");
        ui.label("General options");
        ui.separator();
        let _ = ui.button("Apply");
        let mut active = true;
        let _ = ui.toggle("Enable GPU", &mut active);
        let mut speed = 1.0;
        let _ = ui.scrubber("Speed", &mut speed, 0.0, 10.0, 0.1);

        assert!(!cmd_buf.commands().is_empty());
    }
}
