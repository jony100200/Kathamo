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

use ui_core::{CornerRadius, Direction, Rect, SizeConstraint, Stroke, Vec2};
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
    pub direction: Direction,
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
            direction: Direction::Vertical,
        }
    }

    pub fn with_accessibility(mut self, tree: &'a mut AccessibilityTree) -> Self {
        self.accessibility = Some(tree);
        self
    }

    /// Advances the layout cursor based on current layout direction (vertical or horizontal).
    pub fn advance(&mut self, width: f32, height: f32) {
        match self.direction {
            Direction::Vertical => {
                self.cursor.y += height + self.item_spacing;
            }
            Direction::Horizontal => {
                self.cursor.x += width + self.item_spacing;
            }
        }
    }

    /// Primary action button ("Generate", "Quantize").
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

        self.advance(width, height);
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
        self.advance(size, size);
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

        self.advance(self.bounds.width(), size + 8.0);
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
        self.advance(self.bounds.width(), size + self.item_spacing);
    }

    /// High-contrast video subtitle with subpixel outline (essential for Cholpo and video overlays).
    pub fn subtitle(&mut self, text: &str) {
        let size = 20.0;
        let pos = self.cursor;
        self.cmd_buf.push_text_with_outline(
            text,
            pos,
            size,
            ui_core::Color::rgb(255, 255, 255),
            ui_core::Color::rgba(0, 0, 0, 220),
            1.5,
            true,
        );
        self.advance(self.bounds.width(), size + 10.0);
    }

    /// Cinematic title card with drop shadow (for story chapters, video intros, and section headers).
    pub fn title_card(&mut self, title: &str, subtitle: &str) {
        let height = 72.0;
        let card_rect =
            Rect::from_min_size(self.cursor, Vec2::new(self.bounds.width() - 16.0, height));

        // Semi-translucent dark cinematic backdrop
        self.cmd_buf.push(DrawCommand::Rect {
            rect: card_rect,
            fill: self.palette.canvas.with_alpha(200),
            radius: CornerRadius::same(8.0),
            stroke: Stroke::new(1.0, self.palette.border),
        });

        // 1px hairline top specular highlight
        self.cmd_buf.push(DrawCommand::Rect {
            rect: Rect::from_min_size(
                Vec2::new(card_rect.min.x + 8.0, card_rect.min.y),
                Vec2::new(card_rect.width() - 16.0, 1.0),
            ),
            fill: ui_core::Color::rgba(255, 255, 255, 25),
            radius: CornerRadius::ZERO,
            stroke: Stroke::NONE,
        });

        // Title with shadow
        self.cmd_buf.push_text_with_shadow(
            title,
            Vec2::new(card_rect.min.x + 16.0, card_rect.min.y + 14.0),
            20.0,
            self.palette.text,
            ui_core::Color::rgba(0, 0, 0, 160),
            Vec2::new(1.0, 1.5),
            true,
        );

        // Subtitle
        if !subtitle.is_empty() {
            self.cmd_buf.push(DrawCommand::Text {
                text: subtitle.to_string(),
                pos: Vec2::new(card_rect.min.x + 16.0, card_rect.min.y + 42.0),
                size: 12.0,
                color: self.palette.accent,
                strong: false,
            });
        }

        self.advance(card_rect.width(), height + self.item_spacing);
    }

    /// Studio checkbox ("Snap to grid").
    pub fn checkbox(&mut self, label: &str, checked: &mut bool) -> Response {
        let height = 22.0;
        let width = self.bounds.width() - 16.0;
        let rect = Rect::from_min_size(self.cursor, Vec2::new(width, height));
        let hovered = rect.contains(self.pointer.position);
        let clicked = hovered && self.pointer.primary_down;

        if clicked {
            *checked = !*checked;
        }

        let mut state = CheckboxState::new(*checked);
        state.hovered = hovered;
        render_checkbox(self.cmd_buf, &state, label, rect, self.palette);

        self.advance(width, height);
        Response {
            clicked,
            hovered,
            changed: clicked,
        }
    }

    /// Studio dropdown selector ("Choose workspace").
    pub fn dropdown(&mut self, _label: &str, selected: &mut usize, options: &[&str]) -> Response {
        let height = 28.0;
        let width = 200.0;
        let rect = Rect::from_min_size(self.cursor, Vec2::new(width, height));
        let hovered = rect.contains(self.pointer.position);
        let clicked = hovered && self.pointer.primary_down;

        let mut state = DropdownState::new(options.to_vec(), *selected);
        state.hovered = hovered;
        render_dropdown(self.cmd_buf, &state, rect, self.palette);

        if clicked {
            *selected = (*selected + 1) % options.len();
        }

        self.advance(width, height);
        Response {
            clicked,
            hovered,
            changed: clicked,
        }
    }

    /// Progress indicator (determinate or indeterminate).
    pub fn progress(&mut self, value: f32) {
        let height = 6.0;
        let width = self.bounds.width() - 20.0;
        let rect = Rect::from_min_size(self.cursor, Vec2::new(width, height));
        let state = ProgressState::determinate(value);
        render_progress_bar(self.cmd_buf, &state, rect, self.palette);
        self.advance(width, height + 4.0);
    }

    /// Numeric drag scrubber.
    pub fn scrubber(&mut self, label: &str, value: &mut f32, min: f32, max: f32) -> Response {
        let scrubber = NumericScrubber::new(label, *value, min, max, 0.1);
        let width = self.bounds.width() - 20.0;
        let rect = Rect::from_min_size(self.cursor, Vec2::new(width, 26.0));
        let hovered = rect.contains(self.pointer.position);
        scrubber.render(self.cmd_buf, rect, hovered, self.palette);
        self.advance(width, 26.0);
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
        let width = (text.len() as f32 * 7.5 + 16.0).max(60.0);
        self.advance(width, 20.0);
    }

    /// Modern iOS / Fluent toggle switch ("CUDA Acceleration", "Safetensors mode").
    pub fn toggle(&mut self, label: &str, checked: &mut bool) -> Response {
        let height = 22.0;
        let width = if label.is_empty() {
            38.0
        } else {
            (label.len() as f32 * 8.0 + 48.0).max(120.0)
        };
        let rect = Rect::from_min_size(self.cursor, Vec2::new(width, height));
        let hovered = rect.contains(self.pointer.position);
        let clicked = hovered && self.pointer.primary_down;

        if clicked {
            *checked = !*checked;
        }

        crate::input::render_toggle(self.cmd_buf, *checked, hovered, label, rect, self.palette);
        self.advance(width, height);
        Response {
            clicked,
            hovered,
            changed: clicked,
        }
    }

    /// Modern text input field.
    pub fn text_input(&mut self, text: &mut String, placeholder: &str) -> Response {
        let height = 28.0;
        let width = match self.direction {
            Direction::Horizontal => 180.0,
            Direction::Vertical => (self.bounds.width() - 16.0).max(100.0),
        };
        let rect = Rect::from_min_size(self.cursor, Vec2::new(width, height));
        let hovered = rect.contains(self.pointer.position);
        let clicked = hovered && self.pointer.primary_down;

        crate::input::render_text_input(
            self.cmd_buf,
            text,
            placeholder,
            rect,
            hovered,
            hovered,
            self.palette,
        );
        self.advance(width, height);
        Response {
            clicked,
            hovered,
            changed: false,
        }
    }

    /// Modern search input with integrated vector Search icon.
    pub fn search_bar(&mut self, text: &mut String, placeholder: &str) -> Response {
        let height = 28.0;
        let width = match self.direction {
            Direction::Horizontal => 220.0,
            Direction::Vertical => (self.bounds.width() - 16.0).max(100.0),
        };
        let rect = Rect::from_min_size(self.cursor, Vec2::new(width, height));
        let hovered = rect.contains(self.pointer.position);
        let clicked = hovered && self.pointer.primary_down;

        // Render input field with offset text
        crate::input::render_text_input(
            self.cmd_buf,
            text,
            placeholder,
            rect,
            hovered,
            hovered,
            self.palette,
        );

        // Vector Search icon inside left padding
        let icon_rect = Rect::from_min_size(
            Vec2::new(rect.min.x + 8.0, rect.min.y + 7.0),
            Vec2::new(14.0, 14.0),
        );
        self.icon_atlas
            .render(self.cmd_buf, StudioIcon::Search, icon_rect, self.palette.text_muted);

        self.advance(width, height);
        Response {
            clicked,
            hovered,
            changed: false,
        }
    }

    /// Segmented pill tabs (like Linear / Raycast switcher).
    pub fn tabs(&mut self, tabs: &[&str], selected: &mut usize) -> Response {
        let height = 28.0;
        let width = match self.direction {
            Direction::Horizontal => (tabs.len() as f32 * 75.0 + 8.0).max(120.0),
            Direction::Vertical => (self.bounds.width() - 16.0).max(120.0),
        };
        let rect = Rect::from_min_size(self.cursor, Vec2::new(width, height));
        let hovered = rect.contains(self.pointer.position);
        let clicked = hovered && self.pointer.primary_down;

        let tab_width = (width - 4.0) / tabs.len().max(1) as f32;
        let mut hovered_idx = None;
        if hovered {
            let rel_x = (self.pointer.position.x - rect.min.x - 2.0).max(0.0);
            let idx = (rel_x / tab_width) as usize;
            if idx < tabs.len() {
                hovered_idx = Some(idx);
                if clicked && *selected != idx {
                    *selected = idx;
                }
            }
        }

        crate::input::render_segmented_tabs(
            self.cmd_buf,
            tabs,
            *selected,
            rect,
            hovered_idx,
            self.palette,
        );
        self.advance(width, height);
        Response {
            clicked,
            hovered,
            changed: clicked,
        }
    }

    /// Live telemetry stat card (e.g. VRAM, RAM, throughput, ETA).
    pub fn stat_card(&mut self, label: &str, value: &str, subtext: &str) {
        let height = 68.0;
        let width = match self.direction {
            Direction::Horizontal => 150.0,
            Direction::Vertical => (self.bounds.width() - 16.0).max(120.0),
        };
        let rect = Rect::from_min_size(self.cursor, Vec2::new(width, height));
        crate::input::render_stat_card(self.cmd_buf, label, value, subtext, rect, self.palette);
        self.advance(width, height);
    }

    /// Selectable model list item card.
    pub fn model_card(
        &mut self,
        name: &str,
        params: &str,
        quant: &str,
        is_selected: bool,
    ) -> Response {
        let height = 52.0;
        let width = (self.bounds.width() - 16.0).max(140.0);
        let rect = Rect::from_min_size(self.cursor, Vec2::new(width, height));
        let hovered = rect.contains(self.pointer.position);
        let clicked = hovered && self.pointer.primary_down;

        crate::input::render_model_card(
            self.cmd_buf,
            name,
            params,
            quant,
            rect,
            is_selected,
            hovered,
            self.palette,
        );
        self.advance(width, height);
        Response {
            clicked,
            hovered,
            changed: clicked,
        }
    }

    /// Live status pill with glowing dot indicator.
    pub fn status_pill(&mut self, text: &str, is_active: bool) {
        let color = if is_active {
            self.palette.success
        } else {
            self.palette.text_muted
        };
        crate::controls::render_status_pill(self.cmd_buf, self.cursor, text, color, self.palette);
        let width = (text.len() as f32 * 7.0 + 28.0).max(54.0);
        self.advance(width, 22.0);
    }

    /// Declarative horizontal row layout of given height.
    pub fn horizontal<F, R>(&mut self, height: f32, f: F) -> R
    where
        F: FnOnce(&mut KolponaUi) -> R,
    {
        let start_x = self.cursor.x;
        let start_y = self.cursor.y;
        let available_w = (self.bounds.max.x - start_x).max(10.0);
        let row_bounds =
            Rect::from_min_size(Vec2::new(start_x, start_y), Vec2::new(available_w, height));

        let mut row_ui = KolponaUi {
            cmd_buf: self.cmd_buf,
            icon_atlas: self.icon_atlas,
            accessibility: self.accessibility.as_deref_mut(),
            palette: self.palette,
            cursor: Vec2::new(start_x, start_y),
            bounds: row_bounds,
            pointer: self.pointer,
            item_spacing: self.item_spacing,
            direction: Direction::Horizontal,
        };

        let result = f(&mut row_ui);
        self.cursor.y += height + self.item_spacing;
        result
    }

    /// Layout alias for `horizontal`.
    pub fn row<F, R>(&mut self, height: f32, f: F) -> R
    where
        F: FnOnce(&mut KolponaUi) -> R,
    {
        self.horizontal(height, f)
    }

    /// Declarative vertical column/box layout.
    pub fn vertical<F, R>(&mut self, width: f32, f: F) -> R
    where
        F: FnOnce(&mut KolponaUi) -> R,
    {
        let start_x = self.cursor.x;
        let start_y = self.cursor.y;
        let available_h = (self.bounds.max.y - start_y).max(10.0);
        let col_bounds =
            Rect::from_min_size(Vec2::new(start_x, start_y), Vec2::new(width, available_h));

        let mut col_ui = KolponaUi {
            cmd_buf: self.cmd_buf,
            icon_atlas: self.icon_atlas,
            accessibility: self.accessibility.as_deref_mut(),
            palette: self.palette,
            cursor: Vec2::new(start_x, start_y),
            bounds: col_bounds,
            pointer: self.pointer,
            item_spacing: self.item_spacing,
            direction: Direction::Vertical,
        };

        let result = f(&mut col_ui);
        self.cursor.x += width + self.item_spacing;
        result
    }

    /// Solves and renders multi-column layout using size constraints (fixed / flex / clamped).
    pub fn columns<F>(&mut self, height: f32, constraints: &[SizeConstraint], mut f: F)
    where
        F: FnMut(usize, &mut KolponaUi),
    {
        let start_x = self.cursor.x;
        let start_y = self.cursor.y;
        let available_w = (self.bounds.max.x - start_x).max(10.0);
        let container_bounds =
            Rect::from_min_size(Vec2::new(start_x, start_y), Vec2::new(available_w, height));

        let rects = ui_core::layout::resolve_stack_layout(
            container_bounds,
            Direction::Horizontal,
            self.item_spacing,
            constraints,
        );

        for (idx, rect) in rects.into_iter().enumerate() {
            let mut col_ui = KolponaUi {
                cmd_buf: self.cmd_buf,
                icon_atlas: self.icon_atlas,
                accessibility: self.accessibility.as_deref_mut(),
                palette: self.palette,
                cursor: Vec2::new(rect.min.x + 4.0, rect.min.y + 4.0),
                bounds: rect,
                pointer: self.pointer,
                item_spacing: self.item_spacing,
                direction: Direction::Vertical,
            };
            f(idx, &mut col_ui);
        }

        self.cursor.y += height + self.item_spacing;
    }

    /// Multi-column layout using relative flex weights (e.g. `&[1.0, 2.0]`).
    pub fn columns_flex<F>(&mut self, height: f32, weights: &[f32], f: F)
    where
        F: FnMut(usize, &mut KolponaUi),
    {
        let constraints: Vec<SizeConstraint> =
            weights.iter().map(|&w| SizeConstraint::Flex(w)).collect();
        self.columns(height, &constraints, f);
    }

    /// Solid elevated card container with 1px border.
    pub fn card<F, R>(&mut self, height: f32, f: F) -> R
    where
        F: FnOnce(&mut KolponaUi) -> R,
    {
        let start_x = self.cursor.x;
        let start_y = self.cursor.y;
        let available_w = (self.bounds.max.x - start_x - 4.0).max(10.0);
        let card_rect =
            Rect::from_min_size(Vec2::new(start_x, start_y), Vec2::new(available_w, height));

        self.cmd_buf.push(DrawCommand::Rect {
            rect: card_rect,
            fill: self.palette.surface,
            radius: CornerRadius::same(8.0),
            stroke: Stroke::new(1.0, self.palette.border),
        });

        let inner_bounds = Rect::from_min_max(
            card_rect.min + Vec2::splat(8.0),
            card_rect.max - Vec2::splat(8.0),
        );

        let mut inner_ui = KolponaUi {
            cmd_buf: self.cmd_buf,
            icon_atlas: self.icon_atlas,
            accessibility: self.accessibility.as_deref_mut(),
            palette: self.palette,
            cursor: inner_bounds.min,
            bounds: inner_bounds,
            pointer: self.pointer,
            item_spacing: self.item_spacing,
            direction: Direction::Vertical,
        };

        let result = f(&mut inner_ui);
        self.cursor.y += height + self.item_spacing;
        result
    }

    /// Linear / Raycast style glassmorphic card with specular top highlight.
    pub fn glass_card<F, R>(&mut self, height: f32, f: F) -> R
    where
        F: FnOnce(&mut KolponaUi) -> R,
    {
        let start_x = self.cursor.x;
        let start_y = self.cursor.y;
        let available_w = (self.bounds.max.x - start_x - 4.0).max(10.0);
        let card_rect =
            Rect::from_min_size(Vec2::new(start_x, start_y), Vec2::new(available_w, height));

        crate::controls::render_glass_card(self.cmd_buf, card_rect, self.palette, 8.0);

        let inner_bounds = Rect::from_min_max(
            card_rect.min + Vec2::splat(10.0),
            card_rect.max - Vec2::splat(10.0),
        );

        let mut inner_ui = KolponaUi {
            cmd_buf: self.cmd_buf,
            icon_atlas: self.icon_atlas,
            accessibility: self.accessibility.as_deref_mut(),
            palette: self.palette,
            cursor: inner_bounds.min,
            bounds: inner_bounds,
            pointer: self.pointer,
            item_spacing: self.item_spacing,
            direction: Direction::Vertical,
        };

        let result = f(&mut inner_ui);
        self.cursor.y += height + self.item_spacing;
        result
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

    #[test]
    fn declarative_horizontal_and_columns_layout() {
        let mut cmd_buf = CommandBuffer::new();
        let mut icon_atlas = IconAtlas::new();
        let palette = ThemePalette::dark();
        let bounds = Rect::from_min_size(Vec2::ZERO, Vec2::new(800.0, 600.0));
        let pointer = PointerState::default();

        let mut ui = KathamoUi::new(&mut cmd_buf, &mut icon_atlas, &palette, bounds, pointer);

        // Row
        ui.horizontal(36.0, |row| {
            row.button("Quantize");
            row.button_secondary("Cancel");
            row.status_pill("Ready", true);
        });

        // Columns split
        ui.columns(
            200.0,
            &[SizeConstraint::Fixed(240.0), SizeConstraint::Flex(1.0)],
            |idx, col| {
                if idx == 0 {
                    col.heading("Models");
                    let mut q = String::new();
                    col.search_bar(&mut q, "Search models...");
                } else {
                    col.heading("Configuration");
                    let mut cuda = true;
                    col.toggle("Enable CUDA", &mut cuda);
                }
            },
        );

        assert!(!cmd_buf.is_empty());
    }

    #[test]
    fn container_cards_and_stat_cards() {
        let mut cmd_buf = CommandBuffer::new();
        let mut icon_atlas = IconAtlas::new();
        let palette = ThemePalette::dark();
        let bounds = Rect::from_min_size(Vec2::ZERO, Vec2::new(600.0, 600.0));
        let pointer = PointerState::default();

        let mut ui = KathamoUi::new(&mut cmd_buf, &mut icon_atlas, &palette, bounds, pointer);

        ui.glass_card(120.0, |card| {
            card.stat_card("VRAM Pool", "11.8 / 16.0 GB", "74% Active");
            let mut tab = 0;
            card.tabs(&["GGUF", "AWQ", "EXL2"], &mut tab);
        });

        assert!(!cmd_buf.is_empty());
    }
}

