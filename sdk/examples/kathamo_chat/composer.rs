//! Kathamo Chat — Bottom Message Composer
//!
//! Anchored input area with rounded card design, dual-stroke focus highlight,
//! Send/Stop generation toggle, and clearly labeled demo attachment trigger.

use super::models::{StreamingState, UiState};
use ui_core::{Color, CornerRadius, Rect, Stroke, Vec2};
use ui_render::{CommandBuffer, DrawCommand};
use ui_themes::ThemePalette;
use ui_widgets::input::render_text_input;

#[allow(dead_code)]
pub struct ComposerRenderResult {
    pub clicked_send: bool,
    pub clicked_stop: bool,
    pub clicked_attach: bool,
    pub enter_pressed: bool,
}

pub fn render_composer(
    buf: &mut CommandBuffer,
    bounds: Rect,
    streaming: &StreamingState,
    ui: &UiState,
    pal: &ThemePalette,
) -> ComposerRenderResult {
    let result = ComposerRenderResult {
        clicked_send: false,
        clicked_stop: false,
        clicked_attach: false,
        enter_pressed: false,
    };

    // Card width centered (max 780px matching reading column)
    let card_w = (bounds.width() - 80.0).min(780.0).max(320.0);
    let card_x = bounds.min.x + (bounds.width() - card_w) * 0.5;
    let card_h = 76.0;
    let card_y = bounds.min.y + (bounds.height() - card_h) * 0.5 - 4.0;
    let card_rect = Rect::from_min_size(Vec2::new(card_x, card_y), Vec2::new(card_w, card_h));

    // Outer subtle card border & elevated fill
    let (border_col, border_w) = if ui.composer_focused {
        (pal.accent, 1.5)
    } else {
        (pal.border, 1.0)
    };

    buf.push(DrawCommand::Rect {
        rect: card_rect,
        fill: pal.canvas,
        radius: CornerRadius::same(12.0),
        stroke: Stroke::new(border_w, border_col),
    });

    // 1. Top row: Text input area
    let input_rect = Rect::from_min_size(
        Vec2::new(card_x + 12.0, card_y + 8.0),
        Vec2::new(card_w - 24.0, 32.0),
    );

    render_text_input(
        buf,
        &ui.composer_text,
        "Message Kathamo Chat... (Press Enter to send)",
        input_rect,
        ui.composer_focused,
        false,
        pal,
    );

    // 2. Bottom row: Toolbar (Attachment demo pill, Model tag, Send/Stop button)
    let tool_y = card_y + 44.0;

    // Attachment Button (Labeled Demo Control)
    let attach_r = Rect::from_min_size(Vec2::new(card_x + 12.0, tool_y), Vec2::new(110.0, 24.0));
    buf.push(DrawCommand::Rect {
        rect: attach_r,
        fill: pal.elevated,
        radius: CornerRadius::same(5.0),
        stroke: Stroke::new(1.0, pal.border),
    });
    buf.push(DrawCommand::Text {
        pos: Vec2::new(attach_r.min.x + 8.0, attach_r.min.y + 5.0),
        text: "📎 Attach (Demo)".into(),
        color: pal.text_muted,
        size: 10.5,
        strong: false,
    });

    // Model Info Pill
    let model_tag_r = Rect::from_min_size(
        Vec2::new(attach_r.max.x + 8.0, tool_y),
        Vec2::new(135.0, 24.0),
    );
    buf.push(DrawCommand::Text {
        pos: Vec2::new(model_tag_r.min.x + 4.0, model_tag_r.min.y + 5.0),
        text: "Kathamo Core • 8K ctx".into(),
        color: pal.text_muted.with_alpha(150),
        size: 10.0,
        strong: false,
    });

    // Send / Stop button on the right
    let btn_w = 78.0;
    let btn_h = 26.0;
    let btn_r = Rect::from_min_size(
        Vec2::new(card_x + card_w - btn_w - 12.0, tool_y - 1.0),
        Vec2::new(btn_w, btn_h),
    );

    if streaming.is_active {
        // Stop Generating Button
        buf.push(DrawCommand::Rect {
            rect: btn_r,
            fill: pal.danger,
            radius: CornerRadius::same(6.0),
            stroke: Stroke::NONE,
        });
        buf.push(DrawCommand::Text {
            pos: Vec2::new(btn_r.min.x + 10.0, btn_r.min.y + 6.0),
            text: "⏹ Stop".into(),
            color: Color::WHITE,
            size: 11.5,
            strong: true,
        });
    } else {
        // Send Button
        let can_send = !ui.composer_text.trim().is_empty();
        let (send_bg, send_fg) = if can_send {
            (pal.accent, pal.canvas)
        } else {
            (pal.elevated, pal.text_muted.with_alpha(120))
        };

        buf.push(DrawCommand::Rect {
            rect: btn_r,
            fill: send_bg,
            radius: CornerRadius::same(6.0),
            stroke: Stroke::NONE,
        });
        buf.push(DrawCommand::Text {
            pos: Vec2::new(btn_r.min.x + 12.0, btn_r.min.y + 6.0),
            text: "➤ Send".into(),
            color: send_fg,
            size: 11.5,
            strong: true,
        });
    }

    // Keyboard shortcut hint below card
    let hint_text = "Kathamo Chat can make mistakes. Check important info.";
    buf.push(DrawCommand::Text {
        pos: Vec2::new(card_x + (card_w - 290.0) * 0.5, bounds.max.y - 12.0),
        text: hint_text.into(),
        color: pal.text_muted.with_alpha(120),
        size: 9.5,
        strong: false,
    });

    result
}
