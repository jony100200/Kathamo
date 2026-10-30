//! Card components: model card, agent card, and task card.

use ui_core::{motion, Color, Rect, Vec2};
use ui_render::measure_text;
use ui_themes::{RADIUS_CARD, SPACE_2, SPACE_3};

use super::controls::chip;
use super::mascot::{mascot, Mood};
use crate::studio::{
    fit_text, initials, name_tint, text_center_y, with_alpha, Interaction, Studio, Tone,
};

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

/// Task card with status pill, metadata, title, and assignee avatar.
pub fn task_card(
    studio: &mut Studio,
    rect: Rect,
    title: &str,
    assignee: &str,
    state_label: &str,
    tone: Tone,
    meta: &str,
) -> Interaction {
    let hit = studio.card(rect);
    let top_y = rect.min.y + SPACE_3;
    let pill_w = studio.pill(state_label, Vec2::new(rect.min.x + SPACE_3, top_y), tone);
    let max_meta_w = (rect.max.x - (rect.min.x + SPACE_3 + pill_w + SPACE_2) - SPACE_3).max(0.0);
    let meta_fit = fit_text(meta, max_meta_w, 11.0, false);
    let mw = measure_text(&meta_fit, 11.0, false);
    txt(
        studio,
        &meta_fit,
        Vec2::new(rect.max.x - SPACE_3 - mw, top_y + 4.0),
        11.0,
        studio.palette.text_muted,
    );

    let title_y = top_y + 28.0;
    let max_title_w = (rect.width() - SPACE_3 * 2.0).max(0.0);
    fit_bold(
        studio,
        title,
        Vec2::new(rect.min.x + SPACE_3, title_y),
        max_title_w,
        13.0,
        studio.palette.text,
    );

    let av_rect = r_sq(rect.min.x + SPACE_3, rect.max.y - SPACE_3 - 20.0, 20.0);
    studio.avatar(av_rect, assignee);
    let name_x = av_rect.max.x + SPACE_2;
    let max_nw = (rect.max.x - name_x - SPACE_3).max(0.0);
    fit_txt(
        studio,
        assignee,
        Vec2::new(name_x, text_center_y(av_rect, 11.5)),
        max_nw,
        11.5,
        studio.palette.text_muted,
    );
    hit
}

/// Agent card: animated mascot (mood follows the readiness tone), source tag,
/// readiness status and summary detail.
pub fn agent_card(
    studio: &mut Studio,
    rect: Rect,
    name: &str,
    source_label: &str,
    readiness_label: &str,
    readiness_tone: Tone,
    detail: &str,
) -> Interaction {
    let mood = Mood::from_tone(readiness_tone);
    agent_card_with_mood(
        studio,
        rect,
        name,
        source_label,
        readiness_label,
        readiness_tone,
        mood,
        detail,
    )
}

/// Agent card whose mascot mood is chosen by the caller, for states the tone
/// alone cannot tell apart (a usage limit warns in the pill but the mascot sleeps).
pub fn agent_card_with_mood(
    studio: &mut Studio,
    rect: Rect,
    name: &str,
    source_label: &str,
    readiness_label: &str,
    readiness_tone: Tone,
    mood: Mood,
    detail: &str,
) -> Interaction {
    let hit = studio.card(rect);
    let top_y = rect.min.y + SPACE_3;
    let av_rect = r_sq(rect.min.x + SPACE_3, top_y, 36.0);
    mascot(studio, av_rect, name, mood, motion::clock());

    let pill_w = measure_text(readiness_label, 11.5, true) + 26.0;
    let pill_pos = Vec2::new(rect.max.x - SPACE_3 - pill_w, top_y);
    studio.pill(readiness_label, pill_pos, readiness_tone);

    let name_x = av_rect.max.x + SPACE_2;
    let avail_w = (pill_pos.x - name_x - SPACE_2).max(0.0);
    let chip_w = measure_text(source_label, 10.5, false) + 14.0;
    let max_nw = (avail_w - chip_w - SPACE_2).max(20.0);
    let drawn_nw = fit_bold(
        studio,
        name,
        Vec2::new(name_x, top_y + 2.0),
        max_nw,
        13.5,
        studio.palette.text,
    );
    chip(
        studio,
        Vec2::new(name_x + drawn_nw + SPACE_2, top_y + 1.0),
        source_label,
        Tone::Neutral,
    );

    let detail_y = top_y + 36.0 + SPACE_2;
    let max_det_w = (rect.width() - SPACE_3 * 2.0).max(0.0);
    fit_txt(
        studio,
        detail,
        Vec2::new(rect.min.x + SPACE_3, detail_y),
        max_det_w,
        12.0,
        studio.palette.text_muted,
    );
    hit
}

/// Model card with monogram tile, metadata chips, and status pill.
pub fn model_card(
    studio: &mut Studio,
    rect: Rect,
    name: &str,
    family: &str,
    size_label: &str,
    quant: &str,
    state_label: &str,
    state_tone: Tone,
    selected: bool,
) -> Interaction {
    let hit = studio.interact(rect);
    studio.surface(rect, if hit.hovered { 2 } else { 1 }, RADIUS_CARD);
    if selected {
        studio.outline(
            rect,
            with_alpha(studio.palette.accent, 20),
            studio.palette.accent,
            RADIUS_CARD,
        );
    }
    let top_y = rect.min.y + SPACE_3;
    let tile_rect = r_sq(rect.min.x + SPACE_3, top_y, 38.0);
    let tint = name_tint(name);
    studio.outline(tile_rect, with_alpha(tint, 40), with_alpha(tint, 110), 8.0);
    let inits = initials(name);
    let inits_w = measure_text(&inits, 14.0, true);
    txt_bold(
        studio,
        &inits,
        Vec2::new(
            tile_rect.center().x - inits_w / 2.0,
            text_center_y(tile_rect, 14.0),
        ),
        14.0,
        tint,
    );

    let pill_w = measure_text(state_label, 11.5, true) + 26.0;
    let pill_pos = Vec2::new(rect.max.x - SPACE_3 - pill_w, top_y);
    studio.pill(state_label, pill_pos, state_tone);

    let name_x = tile_rect.max.x + SPACE_2;
    let max_nw = (pill_pos.x - name_x - SPACE_2).max(0.0);
    fit_bold(
        studio,
        name,
        Vec2::new(name_x, top_y + 2.0),
        max_nw,
        13.5,
        studio.palette.text,
    );
    fit_txt(
        studio,
        family,
        Vec2::new(name_x, top_y + 20.0),
        max_nw,
        11.5,
        studio.palette.text_muted,
    );

    let chips_y = top_y + 38.0 + SPACE_2;
    let mut chip_x = rect.min.x + SPACE_3;
    chip_x += chip(
        studio,
        Vec2::new(chip_x, chips_y),
        size_label,
        Tone::Neutral,
    ) + SPACE_2;
    chip(studio, Vec2::new(chip_x, chips_y), quant, Tone::Neutral);
    hit
}
