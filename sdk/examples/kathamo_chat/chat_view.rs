//! Kathamo Chat — Central Conversation View
//!
//! Renders the scrollable message stream, welcome empty state, Markdown text,
//! syntax-highlighted code cards, action buttons, and animated streaming cursor.

use super::models::{ChatConversation, ChatMessageItem, CodeArtifact, StreamingState, UiState};
use ui_core::{CornerRadius, Rect, Stroke, Vec2};
use ui_render::{CommandBuffer, DrawCommand};
use ui_themes::ThemePalette;
use ui_widgets::rich_text::ChatRole;

pub struct ChatViewRenderResult {
    pub clicked_starter_prompt: Option<String>,
    pub clicked_copy_msg: Option<String>,
    pub clicked_regenerate: bool,
    pub clicked_view_artifact: Option<CodeArtifact>,
}

pub fn render_chat_view(
    buf: &mut CommandBuffer,
    bounds: Rect,
    active_conv: Option<&ChatConversation>,
    streaming: &StreamingState,
    ui: &UiState,
    pal: &ThemePalette,
) -> ChatViewRenderResult {
    let mut result = ChatViewRenderResult {
        clicked_starter_prompt: None,
        clicked_copy_msg: None,
        clicked_regenerate: false,
        clicked_view_artifact: None,
    };

    // Viewport background
    buf.push(DrawCommand::Rect {
        rect: bounds,
        fill: pal.surface,
        radius: CornerRadius::ZERO,
        stroke: Stroke::NONE,
    });

    buf.push(DrawCommand::ClipPush { rect: bounds });

    // Optimal reading column width (centered, max 780px)
    let col_w = (bounds.width() - 80.0).min(780.0).max(320.0);
    let col_x = bounds.min.x + (bounds.width() - col_w) * 0.5;

    let messages = active_conv.map(|c| &c.messages[..]).unwrap_or(&[]);

    if messages.is_empty() && !streaming.is_active {
        // Render Empty State Welcome Screen
        render_welcome_screen(buf, bounds, col_x, col_w, pal, &mut result);
    } else {
        // Render Message Stream
        let scroll_y = ui.scroll_y.clamp(0.0, 10000.0);
        let mut y = bounds.min.y + 24.0 - scroll_y;

        for msg in messages {
            let is_streaming_this = streaming.is_active && streaming.message_id == msg.id;
            let display_content = if is_streaming_this {
                let count = streaming.current_char_index;
                streaming
                    .full_response
                    .chars()
                    .take(count)
                    .collect::<String>()
            } else {
                msg.content.clone()
            };

            let rendered_h = render_message_item(
                buf,
                Vec2::new(col_x, y),
                col_w,
                msg,
                &display_content,
                is_streaming_this,
                bounds,
                pal,
                &mut result,
            );

            y += rendered_h + 24.0;
        }

        // Render scrollbar if content exceeds viewport
        let content_h = y + ui.scroll_y.max(0.0) - bounds.min.y;
        if content_h > bounds.height() {
            let scrollbar_w = 6.0;
            let scrollbar_h = ((bounds.height() / content_h) * bounds.height()).max(30.0);
            let scroll_ratio = ui.scroll_y.max(0.0) / (content_h - bounds.height()).max(1.0);
            let scrollbar_y = bounds.min.y + scroll_ratio * (bounds.height() - scrollbar_h);

            buf.push(DrawCommand::Rect {
                rect: Rect::from_min_size(
                    Vec2::new(bounds.max.x - scrollbar_w - 4.0, scrollbar_y),
                    Vec2::new(scrollbar_w, scrollbar_h),
                ),
                fill: pal.accent.with_alpha(80),
                radius: CornerRadius::same(3.0),
                stroke: Stroke::NONE,
            });
        }
    }

    buf.push(DrawCommand::ClipPop);

    result
}

/// Renders the ChatGPT / Claude-style welcome empty state.
fn render_welcome_screen(
    buf: &mut CommandBuffer,
    bounds: Rect,
    col_x: f32,
    col_w: f32,
    pal: &ThemePalette,
    _result: &mut ChatViewRenderResult,
) {
    let center_y = bounds.min.y + bounds.height() * 0.35;

    // Center Logo Pill
    let logo_size = 54.0;
    let logo_rect = Rect::from_min_size(
        Vec2::new(col_x + (col_w - logo_size) * 0.5, center_y - 80.0),
        Vec2::new(logo_size, logo_size),
    );

    buf.push(DrawCommand::Rect {
        rect: logo_rect,
        fill: pal.accent.with_alpha(35),
        radius: CornerRadius::same(14.0),
        stroke: Stroke::new(1.5, pal.accent),
    });
    buf.push(DrawCommand::Text {
        pos: Vec2::new(logo_rect.min.x + 16.0, logo_rect.min.y + 14.0),
        text: "✦".into(),
        color: pal.accent,
        size: 26.0,
        strong: true,
    });

    // Heading
    buf.push(DrawCommand::Text {
        pos: Vec2::new(col_x + (col_w - 280.0) * 0.5, center_y - 12.0),
        text: "How can I help you today?".into(),
        color: pal.text,
        size: 20.0,
        strong: true,
    });

    // Subtitle
    buf.push(DrawCommand::Text {
        pos: Vec2::new(col_x + (col_w - 360.0) * 0.5, center_y + 16.0),
        text: "Kathamo Native Desktop AI Interaction Showcase".into(),
        color: pal.text_muted,
        size: 13.0,
        strong: false,
    });

    // 4 Starter Prompt Cards in 2x2 grid
    let card_w = (col_w - 16.0) * 0.5;
    let card_h = 72.0;
    let starters = [
        (
            "⚡ Write Rust SIMD Vectorizer",
            "High-throughput concurrent algorithms",
        ),
        (
            "🎨 Design Cyberpunk Level Layout",
            "2.5D parallax layers and lighting pipelines",
        ),
        (
            "📝 Plan Sci-Fi Narrative Arc",
            "Factions, lore and environmental worldbuilding",
        ),
        ("বাংলায় কথোপকথন শুরু করুন", "বহুভাষিক টেক্সট শেপিং এবং ফন্ট রেন্ডারিং"),
    ];

    let start_y = center_y + 54.0;
    for (i, (title, desc)) in starters.iter().enumerate() {
        let row = i / 2;
        let col = i % 2;
        let card_x = col_x + col as f32 * (card_w + 16.0);
        let card_y = start_y + row as f32 * (card_h + 12.0);
        let card_rect = Rect::from_min_size(Vec2::new(card_x, card_y), Vec2::new(card_w, card_h));

        buf.push(DrawCommand::Rect {
            rect: card_rect,
            fill: pal.elevated,
            radius: CornerRadius::same(8.0),
            stroke: Stroke::new(1.0, pal.border),
        });

        buf.push(DrawCommand::Text {
            pos: Vec2::new(card_x + 14.0, card_y + 14.0),
            text: title.to_string(),
            color: pal.text,
            size: 13.0,
            strong: true,
        });

        buf.push(DrawCommand::Text {
            pos: Vec2::new(card_x + 14.0, card_y + 36.0),
            text: desc.to_string(),
            color: pal.text_muted,
            size: 11.0,
            strong: false,
        });
    }
}

/// Renders a single User or Assistant message card.
fn render_message_item(
    buf: &mut CommandBuffer,
    pos: Vec2,
    col_w: f32,
    msg: &ChatMessageItem,
    content: &str,
    is_streaming: bool,
    viewport: Rect,
    pal: &ThemePalette,
    _result: &mut ChatViewRenderResult,
) -> f32 {
    let mut total_h = 0.0;
    let is_user = msg.role == ChatRole::User;

    // Header: Avatar, Name, Timestamp
    let avatar_size = 24.0;
    let (avatar_text, name_text, avatar_color) = if is_user {
        ("👤", "You", pal.accent)
    } else {
        ("🤖", "Kathamo Assistant", pal.success)
    };

    buf.push(DrawCommand::Text {
        pos: Vec2::new(pos.x, pos.y + 2.0),
        text: avatar_text.into(),
        color: avatar_color,
        size: 15.0,
        strong: false,
    });

    buf.push(DrawCommand::Text {
        pos: Vec2::new(pos.x + avatar_size + 6.0, pos.y + 2.0),
        text: name_text.into(),
        color: pal.text,
        size: 13.0,
        strong: true,
    });

    buf.push(DrawCommand::Text {
        pos: Vec2::new(pos.x + avatar_size + 140.0, pos.y + 4.0),
        text: msg.timestamp.clone(),
        color: pal.text_muted,
        size: 10.5,
        strong: false,
    });

    total_h += 30.0;

    // Body Container
    let _body_bg = if is_user { pal.elevated } else { pal.surface };

    let (rendered_content_h, has_code) = render_rich_content(
        buf,
        Vec2::new(pos.x + 8.0, pos.y + total_h),
        col_w - 16.0,
        content,
        is_streaming,
        viewport,
        pal,
    );

    total_h += rendered_content_h + 8.0;

    // Action Bar (for Assistant messages)
    if !is_user && !is_streaming {
        let action_y = pos.y + total_h;
        let btn_w = 68.0;
        let btn_h = 24.0;

        // Copy button
        let copy_r = Rect::from_min_size(Vec2::new(pos.x + 8.0, action_y), Vec2::new(btn_w, btn_h));
        buf.push(DrawCommand::Rect {
            rect: copy_r,
            fill: pal.elevated,
            radius: CornerRadius::same(4.0),
            stroke: Stroke::new(1.0, pal.border),
        });
        buf.push(DrawCommand::Text {
            pos: Vec2::new(copy_r.min.x + 8.0, copy_r.min.y + 5.0),
            text: "📋 Copy".into(),
            color: pal.text_muted,
            size: 10.5,
            strong: false,
        });

        // Regenerate button
        let regen_r = Rect::from_min_size(
            Vec2::new(copy_r.max.x + 8.0, action_y),
            Vec2::new(90.0, btn_h),
        );
        buf.push(DrawCommand::Rect {
            rect: regen_r,
            fill: pal.elevated,
            radius: CornerRadius::same(4.0),
            stroke: Stroke::new(1.0, pal.border),
        });
        buf.push(DrawCommand::Text {
            pos: Vec2::new(regen_r.min.x + 8.0, regen_r.min.y + 5.0),
            text: "🔄 Regenerate".into(),
            color: pal.text_muted,
            size: 10.5,
            strong: false,
        });

        // Artifact Panel Button (if artifact or code is attached)
        if msg.artifact.is_some() || has_code {
            let art_r = Rect::from_min_size(
                Vec2::new(regen_r.max.x + 8.0, action_y),
                Vec2::new(125.0, btn_h),
            );
            buf.push(DrawCommand::Rect {
                rect: art_r,
                fill: pal.accent.with_alpha(30),
                radius: CornerRadius::same(4.0),
                stroke: Stroke::new(1.0, pal.accent.with_alpha(100)),
            });
            buf.push(DrawCommand::Text {
                pos: Vec2::new(art_r.min.x + 8.0, art_r.min.y + 5.0),
                text: "👁 View in Panel".into(),
                color: pal.accent,
                size: 10.5,
                strong: true,
            });
        }

        total_h += 32.0;
    }

    total_h
}

/// Parses and renders Markdown lines, headings, bullets, and syntax code cards.
fn render_rich_content(
    buf: &mut CommandBuffer,
    pos: Vec2,
    max_w: f32,
    content: &str,
    is_streaming: bool,
    viewport: Rect,
    pal: &ThemePalette,
) -> (f32, bool) {
    let mut y = pos.y;
    let mut in_code_block = false;
    let mut code_lang = String::new();
    let mut code_lines: Vec<&str> = Vec::new();
    let mut has_code_block = false;

    let chars_per_line = (max_w / 7.6).floor().max(1.0) as usize;

    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with("```") {
            if in_code_block {
                // End code block: render code card
                let code_h = render_code_card(
                    buf,
                    Vec2::new(pos.x, y),
                    max_w,
                    &code_lang,
                    &code_lines,
                    viewport,
                    pal,
                );
                y += code_h + 10.0;
                code_lines.clear();
                in_code_block = false;
            } else {
                // Start code block
                in_code_block = true;
                has_code_block = true;
                code_lang = trimmed[3..].trim().to_uppercase();
                if code_lang.is_empty() {
                    code_lang = "CODE".into();
                }
            }
            continue;
        }

        if in_code_block {
            code_lines.push(line);
            continue;
        }

        if trimmed.is_empty() {
            y += 10.0;
            continue;
        }

        // Headings & text
        if let Some(h) = trimmed.strip_prefix("### ") {
            buf.push(DrawCommand::Text {
                pos: Vec2::new(pos.x, y),
                text: h.into(),
                color: pal.text,
                size: 15.0,
                strong: true,
            });
            y += 24.0;
        } else if let Some(h) = trimmed.strip_prefix("## ") {
            buf.push(DrawCommand::Text {
                pos: Vec2::new(pos.x, y),
                text: h.into(),
                color: pal.text,
                size: 17.0,
                strong: true,
            });
            y += 28.0;
        } else if let Some(h) = trimmed.strip_prefix("# ") {
            buf.push(DrawCommand::Text {
                pos: Vec2::new(pos.x, y),
                text: h.into(),
                color: pal.text,
                size: 20.0,
                strong: true,
            });
            y += 32.0;
        } else {
            let (prefix, text, col, is_bullet) = if let Some(b) = trimmed
                .strip_prefix("- ")
                .or_else(|| trimmed.strip_prefix("* "))
            {
                ("• ", b, pal.text, true)
            } else if let Some(q) = trimmed.strip_prefix("> ") {
                ("▏ ", q, pal.text_muted, false)
            } else {
                ("", line, pal.text, false)
            };

            let clean_text = format!("{}{}", prefix, text.replace("**", "").replace("`", ""));
            let wrapped = wrap_text(&clean_text, chars_per_line);
            for row in wrapped {
                buf.push(DrawCommand::Text {
                    pos: Vec2::new(pos.x + if is_bullet { 6.0 } else { 0.0 }, y),
                    text: row,
                    color: col,
                    size: 13.0,
                    strong: false,
                });
                y += 19.0;
            }
        }
    }

    // If currently streaming inside an open code block
    if in_code_block && !code_lines.is_empty() {
        let code_h = render_code_card(
            buf,
            Vec2::new(pos.x, y),
            max_w,
            &code_lang,
            &code_lines,
            viewport,
            pal,
        );
        y += code_h + 10.0;
    }

    // Typewriter cursor pill if streaming
    if is_streaming {
        buf.push(DrawCommand::Rect {
            rect: Rect::from_min_size(Vec2::new(pos.x, y), Vec2::new(8.0, 15.0)),
            fill: pal.accent,
            radius: CornerRadius::same(1.5),
            stroke: Stroke::NONE,
        });
        y += 20.0;
    }

    (y - pos.y, has_code_block)
}

/// Renders a dark, syntax-highlighted code block card with copy action.
fn render_code_card(
    buf: &mut CommandBuffer,
    pos: Vec2,
    max_w: f32,
    lang: &str,
    lines: &[&str],
    _viewport: Rect,
    pal: &ThemePalette,
) -> f32 {
    let header_h = 28.0;
    let line_h = 18.0;
    let card_h = header_h + 12.0 + (lines.len().max(1) as f32 * line_h);
    let card_rect = Rect::from_min_size(pos, Vec2::new(max_w, card_h));

    // Card background
    buf.push(DrawCommand::Rect {
        rect: card_rect,
        fill: pal.canvas,
        radius: CornerRadius::same(6.0),
        stroke: Stroke::new(1.0, pal.border),
    });

    // Card header bar
    let header_rect = Rect::from_min_size(pos, Vec2::new(max_w, header_h));
    buf.push(DrawCommand::Rect {
        rect: header_rect,
        fill: pal.elevated,
        radius: CornerRadius::ZERO,
        stroke: Stroke::NONE,
    });
    buf.push(DrawCommand::Line {
        start: Vec2::new(header_rect.min.x, header_rect.max.y),
        end: Vec2::new(header_rect.max.x, header_rect.max.y),
        stroke: Stroke::new(1.0, pal.border),
    });

    // Language label
    buf.push(DrawCommand::Text {
        pos: Vec2::new(header_rect.min.x + 12.0, header_rect.min.y + 7.0),
        text: lang.to_string(),
        color: pal.text_muted,
        size: 10.5,
        strong: true,
    });

    // Copy Code button in header
    buf.push(DrawCommand::Text {
        pos: Vec2::new(header_rect.max.x - 75.0, header_rect.min.y + 7.0),
        text: "📋 Copy code".into(),
        color: pal.accent,
        size: 10.5,
        strong: false,
    });

    // Code lines with syntax coloring
    let mut code_y = header_rect.max.y + 8.0;
    for (i, line) in lines.iter().enumerate() {
        // Line number
        buf.push(DrawCommand::Text {
            pos: Vec2::new(card_rect.min.x + 12.0, code_y),
            text: format!("{:>2}", i + 1),
            color: pal.text_muted.with_alpha(100),
            size: 11.5,
            strong: false,
        });

        // Syntax colored text
        let color = if line.trim().starts_with("//") || line.trim().starts_with("#") {
            pal.text_muted
        } else if line.contains("pub fn ") || line.contains("impl ") || line.contains("struct ") {
            pal.accent
        } else if line.contains("\"") {
            pal.warning
        } else {
            pal.text
        };

        buf.push(DrawCommand::Text {
            pos: Vec2::new(card_rect.min.x + 40.0, code_y),
            text: line.to_string(),
            color,
            size: 11.5,
            strong: false,
        });

        code_y += line_h;
    }

    card_h
}

/// Helper function to wrap text neatly into lines.
fn wrap_text(text: &str, max_chars: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        if current.chars().count() + word.chars().count() + 1 > max_chars {
            if !current.is_empty() {
                lines.push(current.clone());
                current.clear();
            }
        }
        if !current.is_empty() {
            current.push(' ');
        }
        current.push_str(word);
    }
    if !current.is_empty() {
        lines.push(current);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}
