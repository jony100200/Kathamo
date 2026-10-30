//! Kathamo Chat — Left Navigation Sidebar
//!
//! Renders application branding, New Chat CTA, live conversation search,
//! grouped conversation history with active selection states, and user profile.

use super::models::{ChatConversation, UiState};
use ui_core::{CornerRadius, Rect, Stroke, Vec2};
use ui_render::{CommandBuffer, DrawCommand};
use ui_themes::ThemePalette;
use ui_widgets::input::render_text_input;

#[allow(dead_code)]
pub struct SidebarRenderResult {
    pub clicked_new_chat: bool,
    pub clicked_conv_id: Option<String>,
    pub clicked_delete_conv_id: Option<String>,
    pub clicked_toggle_sidebar: bool,
    pub clicked_settings: bool,
}

pub fn render_sidebar(
    buf: &mut CommandBuffer,
    bounds: Rect,
    conversations: &[ChatConversation],
    active_conv_id: &str,
    ui: &UiState,
    pal: &ThemePalette,
) -> SidebarRenderResult {
    let result = SidebarRenderResult {
        clicked_new_chat: false,
        clicked_conv_id: None,
        clicked_delete_conv_id: None,
        clicked_toggle_sidebar: false,
        clicked_settings: false,
    };

    // Sidebar background & right border
    buf.push(DrawCommand::Rect {
        rect: bounds,
        fill: pal.canvas,
        radius: CornerRadius::ZERO,
        stroke: Stroke::NONE,
    });
    buf.push(DrawCommand::Line {
        start: Vec2::new(bounds.max.x, bounds.min.y),
        end: Vec2::new(bounds.max.x, bounds.max.y),
        stroke: Stroke::new(1.0, pal.border),
    });

    let is_collapsed = !ui.sidebar_expanded;
    let pad_x = if is_collapsed { 8.0 } else { 14.0 };
    let mut y = bounds.min.y + 12.0;

    // 1. Branding Header
    let header_rect = Rect::from_min_size(
        Vec2::new(bounds.min.x + pad_x, y),
        Vec2::new(bounds.width() - pad_x * 2.0, 32.0),
    );

    if is_collapsed {
        // Minimal brand icon
        buf.push(DrawCommand::Rect {
            rect: Rect::from_min_size(header_rect.min, Vec2::new(32.0, 32.0)),
            fill: pal.accent.with_alpha(35),
            radius: CornerRadius::same(6.0),
            stroke: Stroke::new(1.0, pal.accent),
        });
        buf.push(DrawCommand::Text {
            pos: Vec2::new(header_rect.min.x + 8.0, header_rect.min.y + 8.0),
            text: "✦".into(),
            color: pal.accent,
            size: 15.0,
            strong: true,
        });
    } else {
        // Full brand with collapse button
        buf.push(DrawCommand::Rect {
            rect: Rect::from_min_size(header_rect.min, Vec2::new(28.0, 28.0)),
            fill: pal.accent.with_alpha(30),
            radius: CornerRadius::same(6.0),
            stroke: Stroke::new(1.0, pal.accent.with_alpha(150)),
        });
        buf.push(DrawCommand::Text {
            pos: Vec2::new(header_rect.min.x + 8.0, header_rect.min.y + 6.0),
            text: "✦".into(),
            color: pal.accent,
            size: 14.0,
            strong: true,
        });
        buf.push(DrawCommand::Text {
            pos: Vec2::new(header_rect.min.x + 36.0, header_rect.min.y + 3.0),
            text: "Kathamo Chat".into(),
            color: pal.text,
            size: 14.0,
            strong: true,
        });
        buf.push(DrawCommand::Text {
            pos: Vec2::new(header_rect.min.x + 36.0, header_rect.min.y + 18.0),
            text: "Framework Showcase".into(),
            color: pal.text_muted,
            size: 10.0,
            strong: false,
        });

        // Sidebar collapse toggle icon
        let toggle_r = Rect::from_min_size(
            Vec2::new(bounds.max.x - 30.0, header_rect.min.y + 4.0),
            Vec2::new(20.0, 20.0),
        );
        buf.push(DrawCommand::Text {
            pos: Vec2::new(toggle_r.min.x + 4.0, toggle_r.min.y + 3.0),
            text: "◀".into(),
            color: pal.text_muted,
            size: 12.0,
            strong: false,
        });
    }

    y += 42.0;

    // 2. "New Chat" Button
    let new_btn_rect = Rect::from_min_size(
        Vec2::new(bounds.min.x + pad_x, y),
        Vec2::new(bounds.width() - pad_x * 2.0, 36.0),
    );

    buf.push(DrawCommand::Rect {
        rect: new_btn_rect,
        fill: pal.elevated,
        radius: CornerRadius::same(6.0),
        stroke: Stroke::new(1.0, pal.border),
    });

    if is_collapsed {
        buf.push(DrawCommand::Text {
            pos: Vec2::new(new_btn_rect.min.x + 14.0, new_btn_rect.min.y + 9.0),
            text: "+".into(),
            color: pal.accent,
            size: 16.0,
            strong: true,
        });
    } else {
        buf.push(DrawCommand::Text {
            pos: Vec2::new(new_btn_rect.min.x + 12.0, new_btn_rect.min.y + 10.0),
            text: "+  New Chat".into(),
            color: pal.text,
            size: 13.0,
            strong: true,
        });
        buf.push(DrawCommand::Text {
            pos: Vec2::new(new_btn_rect.max.x - 48.0, new_btn_rect.min.y + 11.0),
            text: "Ctrl+N".into(),
            color: pal.text_muted,
            size: 10.0,
            strong: false,
        });
    }

    y += 44.0;

    // 3. Search Box (Expanded Only)
    if !is_collapsed {
        let search_rect = Rect::from_min_size(
            Vec2::new(bounds.min.x + pad_x, y),
            Vec2::new(bounds.width() - pad_x * 2.0, 30.0),
        );
        render_text_input(
            buf,
            &ui.search_query,
            "🔍 Search chats...",
            search_rect,
            ui.search_focused,
            false,
            pal,
        );
        y += 38.0;
    }

    // 4. Conversation History List
    let footer_h = 56.0;
    let list_bottom = bounds.max.y - footer_h;
    let list_rect = Rect::from_min_max(
        Vec2::new(bounds.min.x, y),
        Vec2::new(bounds.max.x, list_bottom),
    );

    buf.push(DrawCommand::ClipPush { rect: list_rect });

    // Filter conversations by query
    let query_lower = ui.search_query.to_lowercase();
    let filtered_convs: Vec<&ChatConversation> = conversations
        .iter()
        .filter(|c| {
            if query_lower.is_empty() {
                true
            } else {
                c.title.to_lowercase().contains(&query_lower)
                    || c.subtitle.to_lowercase().contains(&query_lower)
            }
        })
        .collect();

    if !is_collapsed && !filtered_convs.is_empty() {
        buf.push(DrawCommand::Text {
            pos: Vec2::new(bounds.min.x + pad_x, y + 2.0),
            text: "RECENT CONVERSATIONS".into(),
            color: pal.text_muted.with_alpha(160),
            size: 10.0,
            strong: true,
        });
        y += 20.0;
    }

    for conv in &filtered_convs {
        let is_selected = conv.id == active_conv_id;
        let item_h = if is_collapsed { 36.0 } else { 48.0 };
        let item_r = Rect::from_min_size(
            Vec2::new(bounds.min.x + pad_x, y),
            Vec2::new(bounds.width() - pad_x * 2.0, item_h),
        );

        if item_r.min.y > list_bottom {
            break;
        }

        if is_selected {
            buf.push(DrawCommand::Rect {
                rect: item_r,
                fill: pal.elevated,
                radius: CornerRadius::same(6.0),
                stroke: Stroke::new(1.0, pal.accent.with_alpha(100)),
            });
            // Left active pill strip
            buf.push(DrawCommand::Rect {
                rect: Rect::from_min_size(
                    Vec2::new(item_r.min.x + 2.0, item_r.min.y + 8.0),
                    Vec2::new(3.0, item_h - 16.0),
                ),
                fill: pal.accent,
                radius: CornerRadius::same(1.5),
                stroke: Stroke::NONE,
            });
        }

        if is_collapsed {
            // Icon only
            buf.push(DrawCommand::Text {
                pos: Vec2::new(item_r.min.x + 14.0, item_r.min.y + 11.0),
                text: "💬".into(),
                color: if is_selected {
                    pal.accent
                } else {
                    pal.text_muted
                },
                size: 13.0,
                strong: is_selected,
            });
        } else {
            // Title
            let title_color = if is_selected {
                pal.text
            } else {
                pal.text.with_alpha(210)
            };
            buf.push(DrawCommand::Text {
                pos: Vec2::new(item_r.min.x + 12.0, item_r.min.y + 8.0),
                text: ui_render::fit_text(&conv.title, 12.5, is_selected, item_r.width() - 40.0),
                color: title_color,
                size: 12.5,
                strong: is_selected,
            });

            // Subtitle / preview
            buf.push(DrawCommand::Text {
                pos: Vec2::new(item_r.min.x + 12.0, item_r.min.y + 26.0),
                text: ui_render::fit_text(&conv.subtitle, 10.5, false, item_r.width() - 24.0),
                color: pal.text_muted,
                size: 10.5,
                strong: false,
            });

            // Time / Pinned badge
            if conv.is_pinned {
                buf.push(DrawCommand::Text {
                    pos: Vec2::new(item_r.max.x - 20.0, item_r.min.y + 8.0),
                    text: "📌".into(),
                    color: pal.accent,
                    size: 10.0,
                    strong: false,
                });
            }
        }

        y += item_h + 4.0;
    }

    buf.push(DrawCommand::ClipPop);

    // 5. User Profile Footer
    let footer_rect =
        Rect::from_min_max(Vec2::new(bounds.min.x, bounds.max.y - footer_h), bounds.max);

    buf.push(DrawCommand::Line {
        start: Vec2::new(bounds.min.x, footer_rect.min.y),
        end: Vec2::new(bounds.max.x, footer_rect.min.y),
        stroke: Stroke::new(1.0, pal.border),
    });

    let avatar_rect = Rect::from_min_size(
        Vec2::new(footer_rect.min.x + pad_x, footer_rect.min.y + 12.0),
        Vec2::new(32.0, 32.0),
    );

    // Avatar background
    buf.push(DrawCommand::Rect {
        rect: avatar_rect,
        fill: pal.accent.with_alpha(45),
        radius: CornerRadius::same(16.0),
        stroke: Stroke::new(1.5, pal.accent),
    });
    buf.push(DrawCommand::Text {
        pos: Vec2::new(avatar_rect.min.x + 8.0, avatar_rect.min.y + 8.0),
        text: "JD".into(),
        color: pal.text,
        size: 12.0,
        strong: true,
    });

    // Online status dot
    buf.push(DrawCommand::Rect {
        rect: Rect::from_min_size(
            Vec2::new(avatar_rect.max.x - 8.0, avatar_rect.max.y - 8.0),
            Vec2::new(8.0, 8.0),
        ),
        fill: pal.success,
        radius: CornerRadius::same(4.0),
        stroke: Stroke::new(1.0, pal.canvas),
    });

    if !is_collapsed {
        buf.push(DrawCommand::Text {
            pos: Vec2::new(avatar_rect.max.x + 10.0, footer_rect.min.y + 12.0),
            text: "Jonayed (Workspace)".into(),
            color: pal.text,
            size: 12.0,
            strong: true,
        });
        buf.push(DrawCommand::Text {
            pos: Vec2::new(avatar_rect.max.x + 10.0, footer_rect.min.y + 28.0),
            text: "Local Engine Ready".into(),
            color: pal.success,
            size: 10.0,
            strong: false,
        });

        // Settings gear button
        buf.push(DrawCommand::Text {
            pos: Vec2::new(footer_rect.max.x - 28.0, footer_rect.min.y + 18.0),
            text: "⚙".into(),
            color: pal.text_muted,
            size: 16.0,
            strong: false,
        });
    }

    result
}
