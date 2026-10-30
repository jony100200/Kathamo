//! Kathamo Chat — Top Application Bar
//!
//! Renders active conversation title, switchable AI model selector,
//! context token telemetry, and side-panel view toggle.

use super::models::{ChatConversation, ChatModel, UiState};
use ui_core::{CornerRadius, Rect, Stroke, Vec2};
use ui_render::{CommandBuffer, DrawCommand};
use ui_themes::ThemePalette;

#[allow(dead_code)]
pub struct TopBarRenderResult {
    pub clicked_model_select: bool,
    pub clicked_toggle_preview: bool,
    pub clicked_clear_chat: bool,
}

pub fn render_top_bar(
    buf: &mut CommandBuffer,
    bounds: Rect,
    active_conv: Option<&ChatConversation>,
    selected_model: ChatModel,
    ui: &UiState,
    pal: &ThemePalette,
) -> TopBarRenderResult {
    let result = TopBarRenderResult {
        clicked_model_select: false,
        clicked_toggle_preview: false,
        clicked_clear_chat: false,
    };

    // Background & bottom border
    buf.push(DrawCommand::Rect {
        rect: bounds,
        fill: pal.canvas,
        radius: CornerRadius::ZERO,
        stroke: Stroke::NONE,
    });
    buf.push(DrawCommand::Line {
        start: Vec2::new(bounds.min.x, bounds.max.y),
        end: Vec2::new(bounds.max.x, bounds.max.y),
        stroke: Stroke::new(1.0, pal.border),
    });

    let pad_x = 18.0;
    let center_y = bounds.min.y + bounds.height() * 0.5;

    // 1. Left: Active Conversation Title
    let title_text = active_conv
        .map(|c| c.title.as_str())
        .unwrap_or("New Conversation");
    buf.push(DrawCommand::Text {
        pos: Vec2::new(bounds.min.x + pad_x, center_y - 8.0),
        text: title_text.into(),
        color: pal.text,
        size: 14.5,
        strong: true,
    });

    // 2. Center: Model Selector Pill
    let model_pill_w = 210.0;
    let model_pill_h = 30.0;
    let model_pill_rect = Rect::from_min_size(
        Vec2::new(
            bounds.min.x + (bounds.width() - model_pill_w) * 0.5,
            center_y - model_pill_h * 0.5,
        ),
        Vec2::new(model_pill_w, model_pill_h),
    );

    buf.push(DrawCommand::Rect {
        rect: model_pill_rect,
        fill: pal.elevated,
        radius: CornerRadius::same(15.0),
        stroke: Stroke::new(1.0, pal.border),
    });

    // Sparkle icon
    buf.push(DrawCommand::Text {
        pos: Vec2::new(model_pill_rect.min.x + 10.0, model_pill_rect.min.y + 7.0),
        text: "⚡".into(),
        color: pal.accent,
        size: 13.0,
        strong: false,
    });

    // Model name
    buf.push(DrawCommand::Text {
        pos: Vec2::new(model_pill_rect.min.x + 28.0, model_pill_rect.min.y + 7.0),
        text: selected_model.short_name().into(),
        color: pal.text,
        size: 12.0,
        strong: true,
    });

    // Model badge tag
    buf.push(DrawCommand::Text {
        pos: Vec2::new(model_pill_rect.min.x + 115.0, model_pill_rect.min.y + 8.0),
        text: selected_model.badge().into(),
        color: pal.text_muted,
        size: 10.0,
        strong: false,
    });

    // Dropdown arrow
    buf.push(DrawCommand::Text {
        pos: Vec2::new(model_pill_rect.max.x - 18.0, model_pill_rect.min.y + 8.0),
        text: "▼".into(),
        color: pal.text_muted,
        size: 9.0,
        strong: false,
    });

    // 3. Right: Action Buttons (Telemetry, Artifact Preview, Clear)
    let right_x = bounds.max.x - pad_x;

    // Artifacts / Code Preview toggle button
    let preview_btn_w = 110.0;
    let preview_btn_rect = Rect::from_min_size(
        Vec2::new(right_x - preview_btn_w, center_y - 14.0),
        Vec2::new(preview_btn_w, 28.0),
    );

    let (prev_bg, prev_stroke, prev_fg) = if ui.preview_panel_open {
        (
            pal.accent.with_alpha(40),
            Stroke::new(1.0, pal.accent),
            pal.accent,
        )
    } else {
        (pal.elevated, Stroke::new(1.0, pal.border), pal.text)
    };

    buf.push(DrawCommand::Rect {
        rect: preview_btn_rect,
        fill: prev_bg,
        radius: CornerRadius::same(6.0),
        stroke: prev_stroke,
    });
    buf.push(DrawCommand::Text {
        pos: Vec2::new(preview_btn_rect.min.x + 10.0, preview_btn_rect.min.y + 7.0),
        text: "</> Artifacts".into(),
        color: prev_fg,
        size: 11.5,
        strong: ui.preview_panel_open,
    });

    // Telemetry badge
    let telem_rect = Rect::from_min_size(
        Vec2::new(preview_btn_rect.min.x - 130.0, center_y - 12.0),
        Vec2::new(120.0, 24.0),
    );
    buf.push(DrawCommand::Text {
        pos: Vec2::new(telem_rect.min.x + 4.0, telem_rect.min.y + 5.0),
        text: "3,420 tok • 118 t/s".into(),
        color: pal.success,
        size: 10.5,
        strong: false,
    });

    result
}

/// Renders the floating model switcher dropdown menu if open.
pub fn render_model_dropdown(
    buf: &mut CommandBuffer,
    top_bar_bounds: Rect,
    current_model: ChatModel,
    pal: &ThemePalette,
) {
    let menu_w = 260.0;
    let menu_h = 130.0;
    let menu_rect = Rect::from_min_size(
        Vec2::new(
            top_bar_bounds.min.x + (top_bar_bounds.width() - menu_w) * 0.5,
            top_bar_bounds.max.y + 4.0,
        ),
        Vec2::new(menu_w, menu_h),
    );

    // Dropdown shadow & card background
    buf.push(DrawCommand::Rect {
        rect: menu_rect,
        fill: pal.elevated,
        radius: CornerRadius::same(8.0),
        stroke: Stroke::new(1.0, pal.border),
    });

    let mut y = menu_rect.min.y + 8.0;
    for model in ChatModel::all() {
        let is_selected = model == current_model;
        let item_rect = Rect::from_min_size(
            Vec2::new(menu_rect.min.x + 6.0, y),
            Vec2::new(menu_w - 12.0, 36.0),
        );

        if is_selected {
            buf.push(DrawCommand::Rect {
                rect: item_rect,
                fill: pal.accent.with_alpha(30),
                radius: CornerRadius::same(5.0),
                stroke: Stroke::new(1.0, pal.accent.with_alpha(120)),
            });
        }

        buf.push(DrawCommand::Text {
            pos: Vec2::new(item_rect.min.x + 10.0, item_rect.min.y + 6.0),
            text: model.display_name().into(),
            color: if is_selected { pal.accent } else { pal.text },
            size: 12.0,
            strong: is_selected,
        });

        buf.push(DrawCommand::Text {
            pos: Vec2::new(item_rect.min.x + 10.0, item_rect.min.y + 20.0),
            text: model.badge().into(),
            color: pal.text_muted,
            size: 9.5,
            strong: false,
        });

        y += 40.0;
    }
}
