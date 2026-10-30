//! Conversational presentation components: message block, code block, and composer.

use ui_core::{Color, Rect, Vec2};
use ui_render::measure_text;
use ui_themes::{RADIUS_CARD, RADIUS_CONTROL, SPACE_2, SPACE_3};

use crate::studio::{
    centered, fit_text, glyphs, with_alpha, wrap_text, ButtonKind, Interaction, Studio,
};

#[inline]
fn r_min(x: f32, y: f32, w: f32, h: f32) -> Rect {
    Rect::from_min_size(Vec2::new(x, y), Vec2::new(w, h))
}

#[inline]
fn r_sq(x: f32, y: f32, s: f32) -> Rect {
    Rect::from_min_size(Vec2::new(x, y), Vec2::splat(s))
}

#[inline]
fn fit_txt(s: &mut Studio, t: &str, p: Vec2, w: f32, sz: f32, c: Color) -> f32 {
    s.text_fit(t, p, w, sz, c, false)
}

#[inline]
fn fit_bold(s: &mut Studio, t: &str, p: Vec2, w: f32, sz: f32, c: Color) -> f32 {
    s.text_fit(t, p, w, sz, c, true)
}

#[inline]
fn txt(s: &mut Studio, t: &str, p: Vec2, sz: f32, c: Color) {
    s.text(t, p, sz, c, false);
}

/// Role for conversational messages in a studio session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageRole {
    User,
    Model,
    Agent,
    System,
}

/// Output events emitted by the composer widget for one frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ComposerOut {
    pub send: bool,
    pub stop: bool,
    pub attach: bool,
}

/// Multi-line composer field with Send/Stop action, attach button, and focus ring.
pub fn composer(
    studio: &mut Studio,
    rect: Rect,
    text: &str,
    placeholder: &str,
    focused: bool,
    busy: bool,
) -> ComposerOut {
    let p = studio.palette;
    let edge = if focused {
        studio.tokens.focus
    } else {
        p.border
    };
    studio.outline(rect, p.surface, edge, RADIUS_CARD);

    let btn_h = 28.0;
    let btn_y = rect.max.y - SPACE_3 - btn_h;
    let attach_rect = r_sq(rect.min.x + SPACE_3, btn_y, btn_h);
    let attach_hit =
        studio.button_with_icon(attach_rect, Some(glyphs::PLUS), "", ButtonKind::Ghost);

    let act_w = 64.0;
    let act_rect = r_min(rect.max.x - SPACE_3 - act_w, btn_y, act_w, btn_h);
    let (send_clicked, stop_clicked) = if busy {
        let h =
            studio.button_with_icon(act_rect, Some(glyphs::STOP), "Stop", ButtonKind::Secondary);
        (false, h.clicked)
    } else {
        let h = studio.button(act_rect, "Send", ButtonKind::Primary);
        (h.clicked, false)
    };

    let text_x = rect.min.x + SPACE_3;
    let max_tw = (rect.width() - SPACE_3 * 2.0).max(0.0);
    let top_y = rect.min.y + SPACE_3;
    if !text.is_empty() {
        let mut cur_y = top_y;
        for line in wrap_text(text, max_tw, 13.0).iter().take(3) {
            fit_txt(studio, line, Vec2::new(text_x, cur_y), max_tw, 13.0, p.text);
            cur_y += 18.0;
        }
    } else {
        fit_txt(
            studio,
            placeholder,
            Vec2::new(text_x, top_y),
            max_tw,
            13.0,
            p.text_muted,
        );
    }

    ComposerOut {
        send: send_clicked,
        stop: stop_clicked,
        attach: attach_hit.clicked,
    }
}

/// Code block panel with language tag, mono lines, and copy action.
pub fn code_block(
    studio: &mut Studio,
    x: f32,
    y: f32,
    width: f32,
    lang: &str,
    lines: &[String],
) -> (f32, Interaction) {
    let header_h = 32.0;
    let line_h = 19.0;
    let total_h = header_h + lines.len().max(1) as f32 * line_h + 8.0;
    let panel_rect = r_min(x, y, width, total_h);

    studio.outline(
        panel_rect,
        studio.palette.canvas,
        studio.palette.border,
        RADIUS_CONTROL,
    );
    studio.line(
        Vec2::new(x, y + header_h),
        Vec2::new(x + width, y + header_h),
        studio.palette.border,
    );

    let copy_rect = r_min(x + width - SPACE_3 - 56.0, y + 4.0, 56.0, 24.0);
    let copy_hit = studio.button(copy_rect, "Copy", ButtonKind::Ghost);

    let lang_str = if lang.is_empty() { "text" } else { lang };
    let max_lang_w = (width - 56.0 - SPACE_3 * 3.0).max(0.0);
    fit_bold(
        studio,
        lang_str,
        Vec2::new(x + SPACE_3, y + 9.0),
        max_lang_w,
        11.0,
        studio.palette.text_muted,
    );

    let mut line_y = y + header_h + 6.0;
    let max_code_w = (width - SPACE_3 * 2.0).max(0.0);
    for line in lines {
        fit_txt(
            studio,
            line,
            Vec2::new(x + SPACE_3, line_y),
            max_code_w,
            12.0,
            studio.palette.text,
        );
        line_y += line_h;
    }
    (total_h, copy_hit)
}

/// Conversational message block with avatar, author header, wrapped bubble, and streaming dots.
pub fn message_block(
    studio: &mut Studio,
    x: f32,
    y: f32,
    width: f32,
    author: &str,
    role: MessageRole,
    time: &str,
    body: &str,
    streaming: bool,
) -> f32 {
    let is_user = role == MessageRole::User;
    let max_bubble_w = (width - 28.0 - SPACE_3 * 2.0).min(width * 0.82).max(120.0);
    let text_max_w = (max_bubble_w - SPACE_3 * 2.0).max(40.0);
    let lines = wrap_text(body, text_max_w, 13.0);

    let mut widest = if streaming { 36.0 } else { 0.0f32 };
    for l in &lines {
        widest = widest.max(measure_text(l, 13.0, false));
    }
    let bubble_w = (widest + SPACE_3 * 2.0).min(max_bubble_w).max(56.0);
    let line_h = 19.0;
    let stream_h = if streaming { 16.0 } else { 0.0 };
    let bubble_h =
        (lines.len().max(if streaming { 0 } else { 1 }) as f32 * line_h + stream_h + SPACE_2 * 2.0)
            .max(36.0);

    let (av_rect, bubble_x, hdr_x, right_align) = if is_user {
        let ar = r_sq(x + width - 28.0, y, 28.0);
        (ar, ar.min.x - SPACE_2 - bubble_w, ar.min.x - SPACE_2, true)
    } else {
        let ar = r_sq(x, y, 28.0);
        (ar, ar.max.x + SPACE_2, ar.max.x + SPACE_2, false)
    };
    studio.avatar(av_rect, author);

    let hdr_fit = fit_text(author, 160.0, 11.5, true);
    let time_fit = fit_text(time, 80.0, 10.5, false);
    let hdr_text = format!("{hdr_fit}  {time_fit}");
    let hdr_w = measure_text(&hdr_text, 11.5, false);
    let text_x = if right_align {
        (hdr_x - hdr_w).max(x)
    } else {
        hdr_x
    };
    txt(
        studio,
        &hdr_text,
        Vec2::new(text_x, y + 2.0),
        11.5,
        studio.palette.text_muted,
    );

    let bubble_y = y + 20.0;
    let bubble_rect = r_min(bubble_x, bubble_y, bubble_w, bubble_h);
    let (fill, edge) = if is_user {
        (
            with_alpha(studio.palette.accent, 28),
            with_alpha(studio.palette.accent, 75),
        )
    } else {
        (studio.palette.surface, studio.palette.border)
    };
    studio.outline(bubble_rect, fill, edge, RADIUS_CARD);

    let mut cur_y = bubble_y + SPACE_2;
    for line in &lines {
        txt(
            studio,
            line,
            Vec2::new(bubble_x + SPACE_3, cur_y),
            13.0,
            studio.palette.text,
        );
        cur_y += line_h;
    }
    if streaming {
        let dot_y = cur_y + 4.0;
        let dot_x = bubble_x + SPACE_3;
        for i in 0..3 {
            let dot_c = Vec2::new(dot_x + i as f32 * 8.0, dot_y);
            studio.fill(
                centered(dot_c, Vec2::splat(4.0)),
                studio.tokens.working,
                2.0,
            );
        }
    }
    (bubble_y + bubble_h - y) + SPACE_2
}
