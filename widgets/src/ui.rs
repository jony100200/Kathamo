//! Unified Studio UI API for Kolpona, Chobi, Drisho, Jogot, Rupayon, and Pothik.
//!
//! Creative applications interact exclusively with this high-level interface:
//! ```rust,ignore
//! ui.panel("Layers", |ui| {
//!     ui.heading("Chobi Studio");
//!     if ui.button("Generate").clicked() {
//!         // Action
//!     }
//!     ui.icon(StudioIcon::Sparkles);
//! });
//! ```
//! All low-level windowing, wgpu rasterization, font shaping, vector icons,
//! and AccessKit accessibility nodes are coordinated behind replaceable adapters.

use ui_core::{CornerRadius, Rect, Stroke, Vec2};
use ui_platform::{AccessibilityNode, AccessibilityRole, AccessibilityTree, PointerState};
use ui_render::{CommandBuffer, DrawCommand};
use ui_themes::{BilingualLabel, FluentTypographyScale, ThemePalette};

use crate::controls::{render_badge, render_button, ButtonStyle, NumericScrubber};
use crate::icon_cache::{IconAtlas, StudioIcon};
use crate::input::{
    render_checkbox, render_dropdown, render_progress_bar, CheckboxState, DropdownState,
    ProgressState,
};

/// Interaction response returned by studio widgets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Response {
    pub clicked: bool,
    pub hovered: bool,
    pub changed: bool,
}

impl Response {
    pub fn clicked(self) -> bool {
        self.clicked
    }

    pub fn hovered(self) -> bool {
        self.hovered
    }

    pub fn changed(self) -> bool {
        self.changed
    }
}

/// Unified studio UI builder (Kathamo / Kolpona).
pub struct KolponaUi<'a> {
    pub cmd_buf: &'a mut CommandBuffer,
    pub icon_atlas: &'a mut IconAtlas,
    pub accessibility: Option<&'a mut AccessibilityTree>,
    pub palette: &'a ThemePalette,
    pub cursor: Vec2,
    pub bounds: Rect,
    pub pointer: PointerState,
    pub item_spacing: f32,
}

/// Canonical Kathamo UI builder type alias.
pub type KathamoUi<'a> = KolponaUi<'a>;

impl<'a> KolponaUi<'a> {
    pub fn new(
        cmd_buf: &'a mut CommandBuffer,
        icon_atlas: &'a mut IconAtlas,
        palette: &'a ThemePalette,
        bounds: Rect,
        pointer: PointerState,
    ) -> Self {
        let cursor = Vec2::new(bounds.min.x + 8.0, bounds.min.y + 8.0);
        Self {
            cmd_buf,
            icon_atlas,
            accessibility: None,
            palette,
            cursor,
            bounds,
            pointer,
            item_spacing: 6.0,
        }
    }

    pub fn with_accessibility(mut self, tree: &'a mut AccessibilityTree) -> Self {
        self.accessibility = Some(tree);
        self
    }

    /// Primary action button ("Generate").
    pub fn button(&mut self, text: &str) -> Response {
        self.button_styled(text, ButtonStyle::Primary, None)
    }

    /// Secondary outline button.
    pub fn button_secondary(&mut self, text: &str) -> Response {
        self.button_styled(text, ButtonStyle::Secondary, None)
    }

    /// Toolbar action button.
    pub fn button_toolbar(&mut self, icon: StudioIcon) -> Response {
        self.button_styled(icon.name(), ButtonStyle::Ghost, Some(icon))
    }

    /// Styled button with interactive states and accessibility node generation.
    pub fn button_styled(
        &mut self,
        text: &str,
        style: ButtonStyle,
        icon: Option<StudioIcon>,
    ) -> Response {
        let height = 30.0;
        let width = (text.len() as f32 * 8.5 + if icon.is_some() { 36.0 } else { 24.0 }).max(72.0);
        let rect = Rect::from_min_size(self.cursor, Vec2::new(width, height));

        let hovered = rect.contains(self.pointer.position);
        let clicked = hovered && self.pointer.primary_down;

        render_button(self.cmd_buf, rect, text, style, hovered, self.palette);

        if let Some(ic) = icon {
            let icon_rect = Rect::from_min_size(
                Vec2::new(rect.min.x + 6.0, rect.min.y + 7.0),
                Vec2::new(16.0, 16.0),
            );
            self.icon_atlas
                .render(self.cmd_buf, ic, icon_rect, self.palette.text);
        }

        // Accessibility bridge
        if let Some(tree) = &mut self.accessibility {
            let id = tree.alloc_id();
            let mut node = AccessibilityNode::new(id, AccessibilityRole::Button, text, rect);
            node.focused = hovered;
            let root = tree.root_id();
            tree.insert_child(root, node);
        }

        self.cursor.y += height + self.item_spacing;
        Response {
            clicked,
            hovered,
            changed: clicked,
        }
    }

    /// Vector icon rendered from cached Tabler vector atlas (accepts `StudioIcon` or string name like `"image"`).
    pub fn icon(&mut self, icon: impl crate::icon_cache::IntoStudioIcon) {
        let ic = icon.into_studio_icon();
        let size = 20.0;
        let rect = Rect::from_min_size(self.cursor, Vec2::new(size, size));
        self.icon_atlas
            .render(self.cmd_buf, ic, rect, self.palette.text);
        self.cursor.y += size + self.item_spacing;
    }

    /// Prominent studio section heading.
    pub fn heading(&mut self, text: &str) {
        let size = FluentTypographyScale::Section.pixels();
        self.cmd_buf.push(DrawCommand::Text {
            pos: self.cursor,
            text: text.to_string(),
            color: self.palette.text,
            size,
            strong: true,
        });

        if let Some(tree) = &mut self.accessibility {
            let id = tree.alloc_id();
            let rect = Rect::from_min_size(self.cursor, Vec2::new(self.bounds.width(), size + 4.0));
            let node = AccessibilityNode::new(id, AccessibilityRole::Heading, text, rect);
            let root = tree.root_id();
            tree.insert_child(root, node);
        }

        self.cursor.y += size + 8.0;
    }

    /// Bilingual studio heading (English + Bengali script, e.g. "Chobi (ছবি)").
    pub fn bilingual_heading(&mut self, label: &BilingualLabel) {
        let full_text = label.display_string(true);
        self.heading(&full_text);
    }

    /// Standard interface label (14px).
    pub fn label(&mut self, text: &str) {
        let size = FluentTypographyScale::Body.pixels();
        self.cmd_buf.push(DrawCommand::Text {
            pos: self.cursor,
            text: text.to_string(),
            color: self.palette.text_muted,
            size,
            strong: false,
        });
        self.cursor.y += size + self.item_spacing;
    }

    /// Studio checkbox ("Snap to grid").
    pub fn checkbox(&mut self, label: &str, checked: &mut bool) -> Response {
        let height = 22.0;
        let rect = Rect::from_min_size(self.cursor, Vec2::new(self.bounds.width() - 16.0, height));
        let hovered = rect.contains(self.pointer.position);
        let clicked = hovered && self.pointer.primary_down;

        if clicked {
            *checked = !*checked;
        }

        let mut state = CheckboxState::new(*checked);
        state.hovered = hovered;
        render_checkbox(self.cmd_buf, &state, label, rect, self.palette);

        self.cursor.y += height + self.item_spacing;
        Response {
            clicked,
            hovered,
            changed: clicked,
        }
    }

    /// Studio dropdown selector ("Choose workspace").
    pub fn dropdown(&mut self, _label: &str, selected: &mut usize, options: &[&str]) -> Response {
        let height = 28.0;
        let rect = Rect::from_min_size(self.cursor, Vec2::new(200.0, height));
        let hovered = rect.contains(self.pointer.position);
        let clicked = hovered && self.pointer.primary_down;

        let mut state = DropdownState::new(options.to_vec(), *selected);
        state.hovered = hovered;
        render_dropdown(self.cmd_buf, &state, rect, self.palette);

        if clicked {
            *selected = (*selected + 1) % options.len();
        }

        self.cursor.y += height + self.item_spacing;
        Response {
            clicked,
            hovered,
            changed: clicked,
        }
    }

    /// Progress indicator (determinate or indeterminate).
    pub fn progress(&mut self, value: f32) {
        let height = 6.0;
        let rect = Rect::from_min_size(self.cursor, Vec2::new(self.bounds.width() - 20.0, height));
        let state = ProgressState::determinate(value);
        render_progress_bar(self.cmd_buf, &state, rect, self.palette);
        self.cursor.y += height + self.item_spacing + 4.0;
    }

    /// Numeric drag scrubber.
    pub fn scrubber(&mut self, label: &str, value: &mut f32, min: f32, max: f32) -> Response {
        let scrubber = NumericScrubber::new(label, *value, min, max, 0.1);
        let rect = Rect::from_min_size(self.cursor, Vec2::new(self.bounds.width() - 20.0, 26.0));
        let hovered = rect.contains(self.pointer.position);
        scrubber.render(self.cmd_buf, rect, hovered, self.palette);
        self.cursor.y += 26.0 + self.item_spacing;
        Response {
            clicked: false,
            hovered,
            changed: false,
        }
    }

    /// 1px divider rule.
    pub fn separator(&mut self) {
        let rect = Rect::from_min_size(
            Vec2::new(self.bounds.min.x + 4.0, self.cursor.y + 2.0),
            Vec2::new(self.bounds.width() - 8.0, 1.0),
        );
        self.cmd_buf.push(DrawCommand::Rect {
            rect,
            fill: self.palette.border,
            radius: CornerRadius::ZERO,
            stroke: Stroke::NONE,
        });
        self.cursor.y += 6.0;
    }

    /// Semantic status badge.
    pub fn badge(&mut self, text: &str, is_success: bool) {
        render_badge(self.cmd_buf, self.cursor, text, is_success, self.palette);
        self.cursor.y += 20.0 + self.item_spacing;
    }

    /// Nested panel container ("Layers", "Properties", "Project Explorer").
    pub fn panel<F>(&mut self, title: &str, content: F)
    where
        F: FnOnce(&mut KolponaUi),
    {
        let panel_header_height = 28.0;
        let start_pos = self.cursor;

        // Draw panel header bar
        let header_rect = Rect::from_min_size(
            start_pos,
            Vec2::new(self.bounds.width() - 16.0, panel_header_height),
        );
        self.cmd_buf.push(DrawCommand::Rect {
            rect: header_rect,
            fill: self.palette.surface,
            radius: CornerRadius::same(4.0),
            stroke: Stroke::new(1.0, self.palette.border),
        });
        self.cmd_buf.push(DrawCommand::Text {
            pos: Vec2::new(header_rect.min.x + 8.0, header_rect.min.y + 6.0),
            text: title.to_string(),
            color: self.palette.text,
            size: 13.0,
            strong: true,
        });

        self.cursor.y += panel_header_height + 4.0;
        self.cursor.x += 8.0;

        // Render inner contents
        content(self);

        self.cursor.x -= 8.0;
        self.cursor.y += 6.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kolpona_ui_builder_evaluates_controls_and_advances_cursor() {
        let mut cmd_buf = CommandBuffer::new();
        let mut icon_atlas = IconAtlas::new();
        let palette = ThemePalette::dark();
        let bounds = Rect::from_min_size(Vec2::ZERO, Vec2::new(400.0, 600.0));
        let pointer = PointerState::default();

        let mut ui = KolponaUi::new(&mut cmd_buf, &mut icon_atlas, &palette, bounds, pointer);
        let start_y = ui.cursor.y;

        ui.panel("Layers", |ui| {
            ui.heading("Chobi Studio");
            let resp = ui.button("Generate");
            assert!(!resp.clicked());
            ui.icon("image");
            ui.icon(StudioIcon::Sparkles);

            let mut snap = true;
            ui.checkbox("Snap to grid", &mut snap);
            assert!(snap);
        });

        assert!(
            ui.cursor.y > start_y,
            "Cursor must advance monotonically down"
        );
        assert!(!cmd_buf.is_empty(), "Draw commands must be emitted");
    }
}
