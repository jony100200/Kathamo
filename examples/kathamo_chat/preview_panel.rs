//! Kathamo Chat — Collapsible Code & Document Artifact Preview Panel
//!
//! Side panel inspired by ChatGPT Canvas and Claude Artifacts. Displays code
//! snippets and markdown specifications beside the main conversation area.

use super::models::{CodeArtifact, UiState};
use ui_core::{CornerRadius, Rect, Stroke, Vec2};
use ui_render::{CommandBuffer, DrawCommand};
use ui_themes::ThemePalette;

#[allow(dead_code)]
pub struct PreviewPanelRenderResult {
    pub clicked_close: bool,
    pub clicked_copy_all: bool,
}

pub fn render_preview_panel(
    buf: &mut CommandBuffer,
    bounds: Rect,
    artifact: Option<&CodeArtifact>,
    ui: &UiState,
    pal: &ThemePalette,
) -> PreviewPanelRenderResult {
    let result = PreviewPanelRenderResult {
        clicked_close: false,
        clicked_copy_all: false,
    };

    // Panel background & left border
    buf.push(DrawCommand::Rect {
        rect: bounds,
        fill: pal.canvas,
        radius: CornerRadius::ZERO,
        stroke: Stroke::NONE,
    });
    buf.push(DrawCommand::Line {
        start: Vec2::new(bounds.min.x, bounds.min.y),
        end: Vec2::new(bounds.min.x, bounds.max.y),
        stroke: Stroke::new(1.0, pal.border),
    });

    let pad_x = 14.0;

    // 1. Header Bar
    let header_h = 44.0;
    let header_rect = Rect::from_min_size(bounds.min, Vec2::new(bounds.width(), header_h));

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

    let title_text = artifact
        .map(|a| a.title.as_str())
        .unwrap_or("Code Artifact");
    let lang_text = artifact
        .map(|a| a.language.to_uppercase())
        .unwrap_or_else(|| "TEXT".into());

    buf.push(DrawCommand::Text {
        pos: Vec2::new(header_rect.min.x + pad_x, header_rect.min.y + 13.0),
        text: title_text.into(),
        color: pal.text,
        size: 13.0,
        strong: true,
    });

    // Language badge
    let badge_rect = Rect::from_min_size(
        Vec2::new(header_rect.min.x + pad_x + 130.0, header_rect.min.y + 11.0),
        Vec2::new(56.0, 20.0),
    );
    buf.push(DrawCommand::Rect {
        rect: badge_rect,
        fill: pal.accent.with_alpha(35),
        radius: CornerRadius::same(4.0),
        stroke: Stroke::new(1.0, pal.accent.with_alpha(100)),
    });
    buf.push(DrawCommand::Text {
        pos: Vec2::new(badge_rect.min.x + 8.0, badge_rect.min.y + 4.0),
        text: lang_text,
        color: pal.accent,
        size: 10.0,
        strong: true,
    });

    // Copy All button
    let copy_btn_rect = Rect::from_min_size(
        Vec2::new(header_rect.max.x - 90.0, header_rect.min.y + 10.0),
        Vec2::new(60.0, 24.0),
    );
    buf.push(DrawCommand::Rect {
        rect: copy_btn_rect,
        fill: pal.canvas,
        radius: CornerRadius::same(4.0),
        stroke: Stroke::new(1.0, pal.border),
    });
    buf.push(DrawCommand::Text {
        pos: Vec2::new(copy_btn_rect.min.x + 6.0, copy_btn_rect.min.y + 5.0),
        text: "📋 Copy".into(),
        color: pal.text_muted,
        size: 10.5,
        strong: false,
    });

    // Close button "✕"
    let close_btn_rect = Rect::from_min_size(
        Vec2::new(header_rect.max.x - 24.0, header_rect.min.y + 12.0),
        Vec2::new(16.0, 16.0),
    );
    buf.push(DrawCommand::Text {
        pos: Vec2::new(close_btn_rect.min.x, close_btn_rect.min.y + 1.0),
        text: "✕".into(),
        color: pal.text_muted,
        size: 12.0,
        strong: false,
    });

    // 2. Code Viewer Body
    let body_rect = Rect::from_min_max(Vec2::new(bounds.min.x, header_rect.max.y), bounds.max);

    buf.push(DrawCommand::ClipPush { rect: body_rect });

    if let Some(art) = artifact {
        let line_h = 20.0;
        let mut y = body_rect.min.y + 12.0 - ui.preview_scroll_y.max(0.0);

        for (i, line) in art.code.lines().enumerate() {
            if y + line_h < body_rect.min.y {
                y += line_h;
                continue;
            }
            if y > body_rect.max.y {
                break;
            }

            // Line number
            buf.push(DrawCommand::Text {
                pos: Vec2::new(body_rect.min.x + 12.0, y),
                text: format!("{:>3}", i + 1),
                color: pal.text_muted.with_alpha(100),
                size: 11.5,
                strong: false,
            });

            // Syntax coloring
            let color = if line.trim().starts_with("//") || line.trim().starts_with("#") {
                pal.text_muted
            } else if line.contains("pub fn ") || line.contains("impl ") || line.contains("struct ")
            {
                pal.accent
            } else if line.contains("\"") {
                pal.warning
            } else {
                pal.text
            };

            buf.push(DrawCommand::Text {
                pos: Vec2::new(body_rect.min.x + 46.0, y),
                text: line.to_string(),
                color,
                size: 11.5,
                strong: false,
            });

            y += line_h;
        }
    } else {
        // Empty state when no artifact selected
        buf.push(DrawCommand::Text {
            pos: Vec2::new(body_rect.min.x + 20.0, body_rect.min.y + 40.0),
            text: "No artifact selected.".into(),
            color: pal.text_muted,
            size: 12.0,
            strong: false,
        });
        buf.push(DrawCommand::Text {
            pos: Vec2::new(body_rect.min.x + 20.0, body_rect.min.y + 60.0),
            text: "Click 'View in Panel' on any assistant code snippet.".into(),
            color: pal.text_muted.with_alpha(140),
            size: 11.0,
            strong: false,
        });
    }

    buf.push(DrawCommand::ClipPop);

    result
}
