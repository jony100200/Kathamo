//! Controls and row primitives: chips, search field, segmented switcher, and channel row.

use ui_core::{Color, Rect, Vec2};
use ui_render::measure_text;
use ui_themes::{RADIUS_CONTROL, SPACE_2, SPACE_3};

use crate::studio::{fit_text, glyphs, text_center_y, with_alpha, Interaction, Studio, Tone};

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

#[inline]
fn txt_bold(s: &mut Studio, t: &str, p: Vec2, sz: f32, c: Color) {
    s.text(t, p, sz, c, true);
}

/// Small neutral metadata chip, distinct from a status pill.
pub fn chip(studio: &mut Studio, pos: Vec2, label: &str, tone: Tone) -> f32 {
    let size = 10.5;
    let width = measure_text(label, size, false) + 14.0;
    let rect = Rect::from_min_size(pos, Vec2::new(width, 18.0));
    let color = studio.tone_color(tone);
    let (fill, edge) = match tone {
        Tone::Neutral => (studio.palette.elevated, studio.palette.border),
        _ => (with_alpha(color, 24), with_alpha(color, 80)),
    };
    let ink = if tone == Tone::Neutral {
        studio.palette.text_muted
    } else {
        color
    };
    studio.outline(rect, fill, edge, 4.0);
    txt(
        studio,
        label,
        Vec2::new(rect.min.x + 7.0, text_center_y(rect, size)),
        size,
        ink,
    );
    width
}

/// Single-line search input with search icon, placeholder, and focus ring.
pub fn search_field(
    studio: &mut Studio,
    rect: Rect,
    text: &str,
    placeholder: &str,
    focused: bool,
) -> Interaction {
    let hit = studio.interact(rect);
    let p = studio.palette;
    let edge = if focused {
        studio.tokens.focus
    } else if hit.hovered {
        p.accent_hover
    } else {
        p.border
    };
    studio.outline(rect, p.surface_at_elevation(0), edge, RADIUS_CONTROL);
    let icon_r = r_sq(rect.min.x + 8.0, rect.center().y - 8.0, 16.0);
    studio.icon(
        glyphs::SEARCH,
        icon_r,
        if focused { p.accent } else { p.text_muted },
    );
    let text_x = rect.min.x + 30.0;
    let max_w = (rect.max.x - text_x - 8.0).max(0.0);
    let y = text_center_y(rect, 12.5);
    let (body, color) = if !text.is_empty() {
        (text, p.text)
    } else {
        (placeholder, p.text_muted)
    };
    fit_txt(studio, body, Vec2::new(text_x, y), max_w, 12.5, color);
    hit
}

/// Tab switcher segmented control returning the clicked tab index if clicked this frame.
pub fn segmented(
    studio: &mut Studio,
    rect: Rect,
    labels: &[&str],
    selected: usize,
) -> Option<usize> {
    if labels.is_empty() {
        return None;
    }
    studio.outline(
        rect,
        studio.palette.canvas,
        studio.palette.border,
        RADIUS_CONTROL,
    );
    let seg_w = (rect.width() - 4.0).max(0.0) / labels.len() as f32;
    let seg_h = (rect.height() - 4.0).max(0.0);
    let mut clicked = None;
    for (i, label) in labels.iter().enumerate() {
        let seg_r = r_min(
            rect.min.x + 2.0 + i as f32 * seg_w,
            rect.min.y + 2.0,
            seg_w,
            seg_h,
        );
        let hit = studio.interact(seg_r);
        if hit.clicked {
            clicked = Some(i);
        }
        let is_sel = i == selected;
        if is_sel {
            studio.fill(seg_r, studio.palette.elevated, RADIUS_CONTROL - 2.0);
        } else if hit.hovered {
            studio.fill(
                seg_r,
                with_alpha(studio.palette.elevated, 120),
                RADIUS_CONTROL - 2.0,
            );
        }
        let color = if is_sel || hit.hovered {
            studio.palette.text
        } else {
            studio.palette.text_muted
        };
        let fitted = fit_text(label, (seg_w - 8.0).max(0.0), 12.0, is_sel);
        let tx = seg_r.center().x - measure_text(&fitted, 12.0, is_sel) / 2.0;
        let pos = Vec2::new(tx, text_center_y(seg_r, 12.0));
        if is_sel {
            txt_bold(studio, &fitted, pos, 12.0, color);
        } else {
            txt(studio, &fitted, pos, 12.0, color);
        }
    }
    clicked
}

/// Channel or session list row with unread count and active accent bar.
pub fn channel_row(
    studio: &mut Studio,
    rect: Rect,
    name: &str,
    unread: usize,
    active: bool,
) -> Interaction {
    let hit = studio.interact(rect);
    let p = studio.palette;
    if active {
        studio.fill(rect, studio.tokens.selection, RADIUS_CONTROL);
        let bar = r_min(
            rect.min.x,
            rect.min.y + 6.0,
            3.0,
            (rect.height() - 12.0).max(0.0),
        );
        studio.fill(bar, p.accent, 1.5);
    } else if hit.hovered {
        studio.fill(rect, p.elevated, RADIUS_CONTROL);
    }
    let hash_x = rect.min.x + SPACE_3;
    let hash_c = if active { p.accent } else { p.text_muted };
    txt_bold(
        studio,
        "#",
        Vec2::new(hash_x, text_center_y(rect, 13.0)),
        13.0,
        hash_c,
    );
    let badge_w = if unread > 0 {
        let text = if unread > 99 {
            "99+".to_string()
        } else {
            unread.to_string()
        };
        let bw = (measure_text(&text, 10.0, true) + 10.0).max(18.0);
        let br = r_min(rect.max.x - SPACE_3 - bw, rect.center().y - 9.0, bw, 18.0);
        studio.fill(br, p.accent, 9.0);
        let tx = br.center().x - measure_text(&text, 10.0, true) / 2.0;
        txt_bold(
            studio,
            &text,
            Vec2::new(tx, text_center_y(br, 10.0)),
            10.0,
            studio.tokens.on_accent,
        );
        bw + SPACE_2
    } else {
        0.0
    };
    let name_x = hash_x + 16.0;
    let max_w = (rect.max.x - name_x - SPACE_3 - badge_w).max(0.0);
    let ink = if active || hit.hovered {
        p.text
    } else {
        p.text_muted
    };
    let pos = Vec2::new(name_x, text_center_y(rect, 13.0));
    if active {
        fit_bold(studio, name, pos, max_w, 13.0, ink);
    } else {
        fit_txt(studio, name, pos, max_w, 13.0, ink);
    }
    hit
}
