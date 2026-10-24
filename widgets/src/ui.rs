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
    pub dragging: bool,
    pub activated: bool,
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

    pub fn dragging(self) -> bool {
        self.dragging
    }

    pub fn activated(self) -> bool {
        self.activated || self.clicked
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

    /// Evaluates whether an activation click occurred within the given rectangle.
    #[inline]
    pub fn is_clicked_in_rect(&self, rect: Rect) -> bool {
        rect.contains(self.pointer.position) && self.pointer.is_clicked()
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
        let clicked = self.is_clicked_in_rect(rect);

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
            dragging: false,
            activated: clicked,
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

        let width = match self.direction {
            Direction::Horizontal => (text.len() as f32 * 9.5 + 8.0).max(40.0),
            Direction::Vertical => self.bounds.width(),
        };
        self.advance(width, size + 8.0);
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
        let width = match self.direction {
            Direction::Horizontal => (text.len() as f32 * 7.5 + 4.0).max(30.0),
            Direction::Vertical => self.bounds.width(),
        };
        self.advance(width, size + self.item_spacing);
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
        let clicked = self.is_clicked_in_rect(rect);

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
            dragging: false,
            activated: clicked,
        }
    }

    /// Studio dropdown selector ("Choose workspace"). Safe against empty options.
    pub fn dropdown(&mut self, _label: &str, selected: &mut usize, options: &[&str]) -> Response {
        let height = 28.0;
        let width = match self.direction {
            Direction::Horizontal => {
                let max_len = options.iter().map(|s| s.len()).max().unwrap_or(10);
                (max_len as f32 * 8.0 + 36.0).clamp(200.0, 420.0)
            }
            Direction::Vertical => (self.bounds.width() - 16.0).max(160.0),
        };
        let rect = Rect::from_min_size(self.cursor, Vec2::new(width, height));
        let hovered = rect.contains(self.pointer.position);
        let clicked = self.is_clicked_in_rect(rect);

        if options.is_empty() {
            let empty_opts = vec!["(Empty)"];
            let mut state = DropdownState::new(empty_opts, 0);
            state.hovered = hovered;
            render_dropdown(self.cmd_buf, &state, rect, self.palette);
            self.advance(width, height);
            return Response {
                clicked,
                hovered,
                changed: false,
                dragging: false,
                activated: clicked,
            };
        }

        let clamped_idx = (*selected).min(options.len() - 1);
        let mut state = DropdownState::new(options.to_vec(), clamped_idx);
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
            dragging: false,
            activated: clicked,
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

    /// Numeric drag scrubber with interactive drag modification.
    pub fn scrubber(&mut self, label: &str, value: &mut f32, min: f32, max: f32) -> Response {
        let width = self.bounds.width() - 20.0;
        let rect = Rect::from_min_size(self.cursor, Vec2::new(width, 26.0));
        let hovered = rect.contains(self.pointer.position);
        let mut changed = false;
        let dragging = hovered && self.pointer.primary_down;
        if dragging && self.pointer.drag_delta.x.abs() > 0.05 {
            let step = (max - min) * 0.005;
            let delta = self.pointer.drag_delta.x * step;
            let new_val = (*value + delta).clamp(min, max);
            if (new_val - *value).abs() > 1e-4 {
                *value = new_val;
                changed = true;
            }
        }

        let scrubber = NumericScrubber::new(label, *value, min, max, 0.1);
        scrubber.render(self.cmd_buf, rect, hovered, self.palette);
        self.advance(width, 26.0);
        Response {
            clicked: false,
            hovered,
            changed,
            dragging,
            activated: changed,
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
        let clicked = self.is_clicked_in_rect(rect);

        if clicked {
            *checked = !*checked;
        }

        crate::input::render_toggle(self.cmd_buf, *checked, hovered, label, rect, self.palette);
        self.advance(width, height);
        Response {
            clicked,
            hovered,
            changed: clicked,
            dragging: false,
            activated: clicked,
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
        let clicked = self.is_clicked_in_rect(rect);

        crate::input::render_text_input(
            self.cmd_buf,
            text,
            placeholder,
            rect,
            hovered || clicked,
            hovered,
            self.palette,
        );
        self.advance(width, height);
        Response {
            clicked,
            hovered,
            changed: false,
            dragging: false,
            activated: clicked,
        }
    }

    /// Interactive text input backed by persistent TextEditBuffer.
    pub fn text_input_buffered(
        &mut self,
        buffer: &mut ui_core::TextEditBuffer,
        placeholder: &str,
    ) -> Response {
        let height = 28.0;
        let width = match self.direction {
            Direction::Horizontal => 180.0,
            Direction::Vertical => (self.bounds.width() - 16.0).max(100.0),
        };
        let rect = Rect::from_min_size(self.cursor, Vec2::new(width, height));
        let hovered = rect.contains(self.pointer.position);
        let clicked = self.is_clicked_in_rect(rect);

        if clicked {
            buffer.set_focused(true);
        } else if self.pointer.is_clicked() && !hovered {
            buffer.set_focused(false);
        }

        crate::input::render_text_input(
            self.cmd_buf,
            buffer.as_str(),
            placeholder,
            rect,
            buffer.is_focused(),
            hovered,
            self.palette,
        );
        self.advance(width, height);
        Response {
            clicked,
            hovered,
            changed: false,
            dragging: false,
            activated: clicked,
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
        let clicked = self.is_clicked_in_rect(rect);

        // Render input field with offset text
        crate::input::render_text_input(
            self.cmd_buf,
            text,
            placeholder,
            rect,
            hovered || clicked,
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
            dragging: false,
            activated: clicked,
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
        let clicked = self.is_clicked_in_rect(rect);

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
            dragging: false,
            activated: clicked,
        }
    }

    /// Live telemetry stat card (e.g. VRAM, RAM, throughput, ETA).
    pub fn stat_card(&mut self, label: &str, value: &str, subtext: &str) {
        let height = 68.0;
        let width = match self.direction {
            Direction::Horizontal => (label.len().max(value.len()) as f32 * 10.5 + 32.0).clamp(210.0, 290.0),
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
        let clicked = self.is_clicked_in_rect(rect);

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
            dragging: false,
            activated: clicked,
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

    /// Modern collapsible panel container with toggle chevron (essential for AI agent sidebars).
    pub fn collapsible_panel<F, R>(&mut self, title: &str, is_open: &mut bool, content: F) -> Response
    where
        F: FnOnce(&mut KolponaUi) -> R,
    {
        let header_height = 28.0;
        let start_pos = self.cursor;
        let header_rect = Rect::from_min_size(
            start_pos,
            Vec2::new(self.bounds.width() - 16.0, header_height),
        );
        let hovered = header_rect.contains(self.pointer.position);
        let clicked = self.is_clicked_in_rect(header_rect);

        if clicked {
            *is_open = !*is_open;
        }

        let bg_fill = if hovered {
            self.palette.elevated
        } else {
            self.palette.surface
        };

        self.cmd_buf.push(DrawCommand::Rect {
            rect: header_rect,
            fill: bg_fill,
            radius: CornerRadius::same(4.0),
            stroke: Stroke::new(1.0, self.palette.border),
        });

        // Toggle indicator (chevron > or v)
        let indicator = if *is_open { "v" } else { ">" };
        self.cmd_buf.push(DrawCommand::Text {
            pos: Vec2::new(header_rect.min.x + 8.0, header_rect.min.y + 6.0),
            text: indicator.to_string(),
            color: self.palette.accent,
            size: 12.0,
            strong: true,
        });

        self.cmd_buf.push(DrawCommand::Text {
            pos: Vec2::new(header_rect.min.x + 22.0, header_rect.min.y + 6.0),
            text: title.to_string(),
            color: self.palette.text,
            size: 13.0,
            strong: true,
        });

        self.advance(header_rect.width(), header_height);

        if *is_open {
            self.cursor.x += 8.0;
            content(self);
            self.cursor.x -= 8.0;
            self.cursor.y += 6.0;
        }

        Response {
            clicked,
            hovered,
            changed: clicked,
            dragging: false,
            activated: clicked,
        }
    }

    /// Interactive audio waveform strip with draggable playhead (essential for KS-MotionFlow narration).
    pub fn audio_waveform(&mut self, samples: &[f32], playhead: &mut f32, height: f32) -> Response {
        let width = self.bounds.width() - 16.0;
        let rect = Rect::from_min_size(self.cursor, Vec2::new(width, height));
        let hovered = rect.contains(self.pointer.position);
        let clicked = self.is_clicked_in_rect(rect);
        let dragging = hovered && self.pointer.primary_down;

        if clicked || dragging {
            let rel_x = (self.pointer.position.x - rect.min.x) / rect.width().max(1.0);
            *playhead = rel_x.clamp(0.0, 1.0);
        }

        crate::timeline::render_audio_waveform(self.cmd_buf, samples, *playhead, rect, self.palette);
        self.advance(width, height);

        Response {
            clicked,
            hovered,
            changed: clicked || dragging,
            dragging,
            activated: clicked,
        }
    }

    /// High-density RangeSlider for bounded dual-thumb selection.
    pub fn range_slider(
        &mut self,
        label: Option<&str>,
        current_min: &mut f32,
        current_max: &mut f32,
        range_min: f32,
        range_max: f32,
    ) -> Response {
        let mut slider = crate::range_slider::RangeSlider::new(*current_min, *current_max, range_min, range_max);
        if let Some(lbl) = label {
            slider = slider.label(lbl);
        }
        slider.show(self, current_min, current_max)
    }

    /// Hierarchical breadcrumbs navigation trail.
    pub fn breadcrumbs(
        &mut self,
        items: &[crate::breadcrumbs::BreadcrumbItem],
    ) -> (Response, Option<usize>) {
        crate::breadcrumbs::Breadcrumbs::new(items).show(self)
    }

    /// Viewport-clamped floating tooltip.
    pub fn tooltip(
        &mut self,
        text: &str,
        target_rect: Rect,
        viewport: Rect,
    ) {
        crate::tooltip::TooltipOverlay::new(text, target_rect).show(self, viewport);
    }

    /// High-density Accordion component.
    pub fn accordion(
        &mut self,
        accordion: &mut crate::accordion::Accordion,
    ) -> (Response, Option<&'static str>) {
        let is_clicked = self.pointer.is_clicked();
        let clicked_id = accordion.render_headers(
            self.cmd_buf,
            self.icon_atlas,
            self.bounds,
            self.pointer.position,
            is_clicked,
            self.palette,
        );
        let total_h = accordion.total_height();
        self.advance(self.bounds.width() - 16.0, total_h);
        (
            Response {
                clicked: clicked_id.is_some(),
                hovered: self.bounds.contains(self.pointer.position),
                changed: clicked_id.is_some(),
                dragging: false,
                activated: clicked_id.is_some(),
            },
            clicked_id,
        )
    }

    /// Procedural polar HSV ColorWheel with value brightness scrubber.
    pub fn color_wheel(
        &mut self,
        wheel: &mut crate::color_picker::ColorWheel,
        color: &mut crate::color_picker::HsvColor,
    ) -> Response {
        let is_clicked = self.pointer.is_clicked();
        let center = Vec2::new(self.cursor.x + wheel.radius + 12.0, self.cursor.y + wheel.radius + 12.0);
        let changed = wheel.render(
            self.cmd_buf,
            center,
            color,
            self.pointer.position,
            self.pointer.primary_down,
            is_clicked,
            self.palette,
        );
        let total_height = wheel.radius * 2.0 + 70.0;
        self.advance(self.bounds.width() - 16.0, total_height);
        Response {
            clicked: is_clicked,
            hovered: true,
            changed,
            dragging: self.pointer.primary_down,
            activated: is_clicked,
        }
    }

    /// High-density NavigationRail shell primitive.
    pub fn navigation_rail(
        &mut self,
        rail: &crate::navigation_rail::NavigationRail,
        bounds: Rect,
    ) -> (Response, Option<&'static str>) {
        let is_clicked = self.pointer.is_clicked();
        let clicked_id = rail.render(
            self.cmd_buf,
            self.icon_atlas,
            bounds,
            self.pointer.position,
            is_clicked,
            self.palette,
        );
        (
            Response {
                clicked: clicked_id.is_some(),
                hovered: bounds.contains(self.pointer.position),
                changed: clicked_id.is_some(),
                dragging: false,
                activated: clicked_id.is_some(),
            },
            clicked_id,
        )
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

    #[test]
    fn collapsible_panel_and_audio_waveform() {
        let mut cmd_buf = CommandBuffer::new();
        let mut icon_atlas = IconAtlas::new();
        let palette = ThemePalette::dark();
        let bounds = Rect::from_min_size(Vec2::ZERO, Vec2::new(600.0, 600.0));
        let pointer = PointerState::default();

        let mut ui = KathamoUi::new(&mut cmd_buf, &mut icon_atlas, &palette, bounds, pointer);

        let mut is_open = true;
        let resp = ui.collapsible_panel("AI Agent Visual Planner", &mut is_open, |panel| {
            panel.label("Agent thinking...");
        });
        assert!(!resp.clicked());

        let samples = vec![0.2, 0.5, 0.9, 0.4, 0.1, 0.8];
        let mut playhead = 0.35;
        let wave_resp = ui.audio_waveform(&samples, &mut playhead, 48.0);
        assert!(!wave_resp.clicked());
        assert!(!cmd_buf.is_empty());
    }

    #[test]
    fn empty_dropdown_is_safe_and_never_panics() {
        let mut cmd_buf = CommandBuffer::new();
        let mut icon_atlas = IconAtlas::new();
        let palette = ThemePalette::dark();
        let bounds = Rect::from_min_size(Vec2::ZERO, Vec2::new(400.0, 400.0));
        let pointer = PointerState::default();

        let mut ui = KathamoUi::new(&mut cmd_buf, &mut icon_atlas, &palette, bounds, pointer);
        let mut selected = 0;
        let empty_options: &[&str] = &[];

        // Must not divide-by-zero or panic
        let resp = ui.dropdown("Empty", &mut selected, empty_options);
        assert!(!resp.clicked());
        assert_eq!(selected, 0);
    }

    #[test]
    fn held_click_triggers_once_with_transition_pointer() {
        let mut cmd_buf = CommandBuffer::new();
        let mut icon_atlas = IconAtlas::new();
        let palette = ThemePalette::dark();
        let bounds = Rect::from_min_size(Vec2::ZERO, Vec2::new(400.0, 400.0));

        let mut pointer = PointerState::default();
        pointer.position = Vec2::new(20.0, 20.0);
        // Frame 1: press down
        pointer.on_button_down(ui_platform::PointerButton::Primary);

        let mut ui1 = KathamoUi::new(&mut cmd_buf, &mut icon_atlas, &palette, bounds, pointer);
        let resp1 = ui1.button("Action");
        assert!(resp1.clicked(), "Button must trigger on first click frame");

        // Frame 2: button still held down
        pointer.step_frame();
        assert!(pointer.primary_down);
        let mut cmd_buf2 = CommandBuffer::new();
        let mut ui2 = KathamoUi::new(&mut cmd_buf2, &mut icon_atlas, &palette, bounds, pointer);
        let resp2 = ui2.button("Action");
        assert!(!resp2.clicked(), "Button must NOT repeatedly trigger while held down");
    }

    #[test]
    fn interactive_scrubber_drag_modifies_value() {
        let mut cmd_buf = CommandBuffer::new();
        let mut icon_atlas = IconAtlas::new();
        let palette = ThemePalette::dark();
        let bounds = Rect::from_min_size(Vec2::ZERO, Vec2::new(400.0, 400.0));

        let mut pointer = PointerState::default();
        pointer.position = Vec2::new(20.0, 20.0);
        pointer.primary_down = true;
        pointer.drag_delta = Vec2::new(10.0, 0.0); // dragging to the right

        let mut ui = KathamoUi::new(&mut cmd_buf, &mut icon_atlas, &palette, bounds, pointer);
        let mut value = 0.5f32;
        let resp = ui.scrubber("Exposure", &mut value, 0.0, 1.0);
        assert!(resp.changed);
        assert!(value > 0.5f32);
    }

    #[test]
    fn convenience_methods_workstation_components() {
        let mut cmd_buf = CommandBuffer::new();
        let mut icon_atlas = IconAtlas::new();
        let palette = ThemePalette::dark();
        let bounds = Rect::from_min_size(Vec2::ZERO, Vec2::new(600.0, 800.0));
        let pointer = PointerState::default();

        let mut ui = KathamoUi::new(&mut cmd_buf, &mut icon_atlas, &palette, bounds, pointer);

        // Range slider
        let mut min_val = 0.2;
        let mut max_val = 0.8;
        let slider_resp = ui.range_slider(Some("Roughness"), &mut min_val, &mut max_val, 0.0, 1.0);
        assert!(!slider_resp.clicked());

        // Breadcrumbs
        let items = [
            crate::breadcrumbs::BreadcrumbItem::new("Library"),
            crate::breadcrumbs::BreadcrumbItem::new("Textures"),
            crate::breadcrumbs::BreadcrumbItem::new("Wood_01"),
        ];
        let (bc_resp, clicked_crumb) = ui.breadcrumbs(&items);
        assert_eq!(clicked_crumb, None);
        assert!(!bc_resp.clicked());

        // Tooltip
        let target = Rect::from_min_size(Vec2::new(100.0, 100.0), Vec2::new(50.0, 30.0));
        ui.tooltip("Inspect Mesh", target, bounds);

        // Accordion
        let mut acc = crate::accordion::Accordion::new(true)
            .add_item(crate::accordion::AccordionItemState::new("trans", "Transform", true))
            .add_item(crate::accordion::AccordionItemState::new("mat", "Material", false));
        let (acc_resp, toggled_id) = ui.accordion(&mut acc);
        assert_eq!(toggled_id, None);
        assert!(!acc_resp.clicked());

        // ColorWheel
        let mut wheel = crate::color_picker::ColorWheel::new(40.0);
        let mut hsv = crate::color_picker::HsvColor::default();
        let color_resp = ui.color_wheel(&mut wheel, &mut hsv);
        assert!(!color_resp.clicked());

        assert!(!cmd_buf.is_empty());
    }
}


