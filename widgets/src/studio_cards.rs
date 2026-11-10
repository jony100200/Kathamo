//! Card-shaped studio widgets: a progress row, a file drop card for empty states, a
//! thumbnail card, a responsive card grid and a tooltip with a shortcut hint.
//!
//! Like the rest of [`Studio`], these draw from theme tokens, report pointer interaction
//! and own no application state.

use ui_core::{motion, Color, CornerRadius, Rect, Stroke, Vec2};
use ui_render::DrawCommand;
use ui_themes::{RADIUS_CARD, RADIUS_CONTROL, SPACE_2, SPACE_3, SPACE_4, SPACE_6};

use crate::studio::{centered, fit_text, wrap_text, ButtonKind, Glyph, Interaction, Studio};
use crate::type_scale::TypeRole;

/// What a drop card's buttons reported this frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DropCardResponse {
    /// The main button (for example "Browse") was clicked.
    pub browse: bool,
    /// The secondary button (for example "Try a sample") was clicked.
    pub secondary: bool,
}

/// Text and buttons of a [`Studio::drop_card`].
#[derive(Debug, Clone, Copy)]
pub struct DropCard<'a> {
    pub glyph: Glyph,
    pub title: &'a str,
    pub body: &'a str,
    pub browse: &'a str,
    pub secondary: Option<&'a str>,
    /// Files are being dragged over the window: highlight the card as a drop target.
    pub hovering_files: bool,
}

/// Lays out `count` cards in rows across `area`: as many columns as fit at `min_width`,
/// stretched to fill the row, each `aspect` (height / width) plus `footer` pixels tall.
pub fn card_grid(area: Rect, count: usize, min_width: f32, aspect: f32, footer: f32) -> Vec<Rect> {
    let gap = SPACE_4;
    let cols = (((area.width() + gap) / (min_width + gap)).floor() as usize).max(1);
    let w = ((area.width() - gap * (cols as f32 - 1.0)) / cols as f32).floor();
    let h = (w * aspect + footer).round();
    (0..count)
        .map(|i| {
            let (row, col) = (i / cols, i % cols);
            Rect::from_min_size(
                Vec2::new(
                    area.min.x + col as f32 * (w + gap),
                    area.min.y + row as f32 * (h + gap),
                ),
                Vec2::new(w, h),
            )
        })
        .collect()
}

impl Studio<'_> {
    /// A labelled progress bar: `label` on the left, `value` (for example "3 of 12") in
    /// monospace on the right, a bar below. `fraction: None` draws a moving indeterminate
    /// bar (still under reduced motion or in tests).
    pub fn progress_row(&mut self, rect: Rect, label: &str, fraction: Option<f32>, value: &str) {
        let p = self.palette;
        let label_style = TypeRole::Caption.style();
        let value_style = TypeRole::Numeric.style().at(11.0);
        let vw = value_style.measure(value);
        let room = (rect.width() - vw - SPACE_3).max(0.0);
        let shown = fit_text(label, room, label_style.size, false);
        label_style.draw(self.buf, &shown, rect.min, p.text_muted);
        value_style.draw(
            self.buf,
            value,
            Vec2::new(rect.max.x - vw, rect.min.y + 1.0),
            p.text,
        );
        let track = Rect::from_min_size(
            Vec2::new(rect.min.x, rect.max.y - 4.0),
            Vec2::new(rect.width(), 4.0),
        );
        self.fill(track, p.elevated, 2.0);
        let filled = match fraction {
            Some(f) => {
                let f = motion::animate(
                    motion::rect_key("progress_row", rect),
                    f.clamp(0.0, 1.0),
                    motion::SLIDE,
                );
                Rect::from_min_size(track.min, Vec2::new(track.width() * f, 4.0))
            }
            None => {
                let span = track.width() * 0.3;
                let phase = if motion::ambient_running() {
                    motion::request_frame();
                    (motion::clock() * 0.8).fract()
                } else {
                    0.35
                };
                let x0 = track.min.x + (track.width() + span) * phase - span;
                Rect::from_min_max(
                    Vec2::new(x0.max(track.min.x), track.min.y),
                    Vec2::new((x0 + span).min(track.max.x), track.max.y),
                )
            }
        };
        if filled.width() > 0.5 {
            self.fill(filled, p.accent, 2.0);
        }
    }

    /// Empty-state card that invites a file: a dashed outline, a large icon, a title, a
    /// short body and up to two buttons. Highlights while files hover over the window.
    pub fn drop_card(&mut self, rect: Rect, card: DropCard) -> DropCardResponse {
        let p = self.palette;
        let glow = motion::animate_bool(
            motion::rect_key("drop_card", rect),
            card.hovering_files,
            motion::HOVER,
        );
        let fill = p.surface.lerp(p.elevated, 0.4 + glow * 0.6);
        self.buf.push(DrawCommand::Rect {
            rect,
            fill,
            radius: CornerRadius::same(RADIUS_CARD),
            stroke: Stroke::NONE,
        });
        let edge = p.border.lerp(p.accent, glow);
        dashed_outline(self, rect.expand(-1.0), edge);
        let c = rect.center();
        let title_style = TypeRole::Title.style();
        let body_style = TypeRole::Body.style();
        let mut lines = wrap_text(
            card.body,
            (rect.width() - SPACE_6 * 2.0).min(420.0),
            body_style.size,
        );
        // A short card gets a smaller disc and only the body lines that fit.
        let room = rect.height() - SPACE_4 * 2.0;
        let full = |disc: f32, n: usize| {
            disc + SPACE_4 + 24.0 + SPACE_2 + n as f32 * 20.0 + SPACE_6 + 36.0
        };
        let disc_side = if full(64.0, lines.len()) <= room {
            64.0
        } else {
            40.0
        };
        while !lines.is_empty() && full(disc_side, lines.len()) > room {
            lines.pop();
        }
        let block = full(disc_side, lines.len());
        let mut y = (c.y - block * 0.5).max(rect.min.y + SPACE_4);
        let disc = centered(Vec2::new(c.x, y + disc_side * 0.5), Vec2::splat(disc_side));
        self.outline(
            disc,
            p.elevated.lerp(p.accent, glow * 0.25),
            p.border,
            disc_side * 0.5,
        );
        self.icon(
            card.glyph,
            centered(disc.center(), Vec2::splat(disc_side * 0.5)),
            p.accent,
        );
        y += disc_side + SPACE_4;
        let tw = title_style.measure(card.title);
        title_style.draw(self.buf, card.title, Vec2::new(c.x - tw * 0.5, y), p.text);
        y += 24.0 + SPACE_2;
        for line in &lines {
            let lw = body_style.measure(line);
            body_style.draw(self.buf, line, Vec2::new(c.x - lw * 0.5, y), p.text_muted);
            y += 20.0;
        }
        y += SPACE_6;
        let bw = crate::studio::button_width(card.browse, 12.5) + 28.0;
        let sw = card
            .secondary
            .map_or(0.0, |s| crate::studio::button_width(s, 12.5) + 8.0);
        let total = bw + if sw > 0.0 { sw + SPACE_3 } else { 0.0 };
        let mut x = c.x - total * 0.5;
        let browse = Rect::from_min_size(Vec2::new(x, y), Vec2::new(bw, 36.0));
        let mut out = DropCardResponse {
            browse: self
                .button_with_icon(
                    browse,
                    Some(crate::studio::glyphs::FOLDER),
                    card.browse,
                    ButtonKind::Primary,
                )
                .clicked,
            secondary: false,
        };
        x += bw + SPACE_3;
        if let Some(label) = card.secondary {
            let r = Rect::from_min_size(Vec2::new(x, y), Vec2::new(sw, 36.0));
            out.secondary = self.button(r, label, ButtonKind::Secondary).clicked;
        }
        out
    }

    /// A clickable thumbnail card: an image (`texture`: id and width / height, cropped to
    /// fill), or a placeholder icon, a title, a caption and an optional small badge over the
    /// image's top-left corner.
    pub fn thumbnail_card(
        &mut self,
        rect: Rect,
        texture: Option<(u64, f32)>,
        placeholder: Glyph,
        title: &str,
        caption: &str,
        badge: Option<&str>,
    ) -> Interaction {
        let hit = self.interact(rect);
        let p = self.palette;
        let lift = motion::animate_bool(
            motion::rect_key("thumb_card", rect),
            hit.hovered,
            motion::HOVER,
        );
        self.surface(rect, if lift > 0.5 { 2 } else { 1 }, RADIUS_CARD);
        if lift > 0.01 {
            self.buf.push(DrawCommand::Rect {
                rect,
                fill: Color::TRANSPARENT,
                radius: CornerRadius::same(RADIUS_CARD),
                stroke: Stroke::new(1.0, p.accent.with_alpha((lift * 160.0) as u8)),
            });
        }
        let image = Rect::from_min_max(
            rect.min + Vec2::splat(SPACE_2),
            Vec2::new(rect.max.x - SPACE_2, rect.max.y - 52.0),
        );
        self.fill(image, p.canvas, RADIUS_CONTROL);
        match texture {
            Some((id, aspect)) => self.buf.push(DrawCommand::Image {
                rect: image,
                texture_id: id,
                uv_rect: cover_uv(aspect, image.width() / image.height().max(1.0)),
                tint: Color::WHITE,
            }),
            None => self.icon(
                placeholder,
                centered(image.center(), Vec2::splat(32.0)),
                p.text_muted,
            ),
        }
        if let Some(text) = badge {
            let style = TypeRole::Caption.style();
            let w = style.measure(text) + SPACE_3;
            let r = Rect::from_min_size(image.min + Vec2::splat(SPACE_2), Vec2::new(w, 20.0));
            self.fill(r, p.canvas.with_alpha(210), 10.0);
            style.draw(
                self.buf,
                text,
                Vec2::new(r.min.x + SPACE_3 * 0.5, r.min.y + style.centre_offset(20.0)),
                p.text,
            );
        }
        let title_style = TypeRole::Label.style();
        let room = rect.width() - SPACE_3 * 2.0;
        let x = rect.min.x + SPACE_3;
        let shown = fit_text(title, room, title_style.size, true);
        title_style.draw(self.buf, &shown, Vec2::new(x, rect.max.y - 44.0), p.text);
        let cap_style = TypeRole::Caption.style();
        let cap = fit_text(caption, room, cap_style.size, false);
        cap_style.draw(
            self.buf,
            &cap,
            Vec2::new(x, rect.max.y - 24.0),
            p.text_muted,
        );
        hit
    }

    /// A tooltip for `target` that fades in after the pointer rests on it for about half a
    /// second. `shortcut` (for example "Ctrl+S") is drawn after the text in monospace.
    /// Draw it last so it sits above the rest of the frame.
    pub fn tooltip(&mut self, target: Rect, text: &str, shortcut: Option<&str>, viewport: Rect) {
        let hovered = target.contains(self.pointer.position) && !self.pointer.primary_down;
        // A slow spring doubles as the hover delay: nothing shows until it is nearly settled.
        let t = motion::animate_bool(motion::rect_key("tooltip", target), hovered, 0.12);
        let alpha = ((t - 0.85) / 0.15).clamp(0.0, 1.0);
        if alpha <= 0.0 {
            return;
        }
        let p = self.palette;
        let text_style = TypeRole::Caption.style().at(12.0);
        let key_style = TypeRole::Numeric.style().at(11.0);
        let tw = text_style.measure(text);
        let kw = shortcut.map_or(0.0, |k| key_style.measure(k) + SPACE_2 * 2.0 + SPACE_2);
        let size = Vec2::new(tw + kw + SPACE_2 * 2.0, 28.0);
        let mut min = Vec2::new(target.center().x - size.x * 0.5, target.max.y + 6.0);
        if min.y + size.y > viewport.max.y - SPACE_2 {
            min.y = target.min.y - size.y - 6.0;
        }
        min.x = min.x.clamp(
            viewport.min.x + SPACE_2,
            (viewport.max.x - size.x - SPACE_2).max(viewport.min.x + SPACE_2),
        );
        let rect = Rect::from_min_size(min, size);
        let a = |c: Color| c.with_alpha((c.a as f32 * alpha) as u8);
        self.buf.push(DrawCommand::SoftShadow {
            rect,
            radius: CornerRadius::same(6.0),
            color: Color::rgba(0, 0, 0, (110.0 * alpha) as u8),
            offset: Vec2::new(0.0, 3.0),
            blur: 10.0,
            spread: 0.0,
        });
        self.outline(rect, a(p.elevated_high), a(p.border), 6.0);
        let y = rect.min.y + text_style.centre_offset(size.y);
        text_style.draw(
            self.buf,
            text,
            Vec2::new(rect.min.x + SPACE_2, y),
            a(p.text),
        );
        if let Some(key) = shortcut {
            let kx = rect.min.x + SPACE_2 + tw + SPACE_2;
            let key_rect = Rect::from_min_size(
                Vec2::new(kx, rect.min.y + 5.0),
                Vec2::new(kw - SPACE_2, 18.0),
            );
            self.outline(key_rect, a(p.surface), a(p.border), 4.0);
            let ky = key_rect.min.y + key_style.centre_offset(18.0);
            key_style.draw(
                self.buf,
                key,
                Vec2::new(key_rect.min.x + SPACE_2, ky),
                a(p.text_muted),
            );
        }
    }
}

/// The centred part of an image of aspect `source` (width / height) that fills a box of
/// aspect `target`, as a UV rectangle.
pub fn cover_uv(source: f32, target: f32) -> Rect {
    if !(source > 0.0 && target > 0.0) {
        return Rect::from_min_size(Vec2::ZERO, Vec2::splat(1.0));
    }
    if source > target {
        let w = target / source;
        Rect::from_min_size(Vec2::new((1.0 - w) * 0.5, 0.0), Vec2::new(w, 1.0))
    } else {
        let h = source / target;
        Rect::from_min_size(Vec2::new(0.0, (1.0 - h) * 0.5), Vec2::new(1.0, h))
    }
}

/// A dashed rounded-corner outline (straight dashes on the sides, corners left open).
fn dashed_outline(s: &mut Studio, rect: Rect, color: Color) {
    let (dash, gap, inset) = (8.0, 6.0, RADIUS_CARD);
    let mut run = |from: Vec2, to: Vec2| {
        let len = (to - from).length();
        let dir = (to - from) * (1.0 / len.max(1e-3));
        let mut t = 0.0;
        while t < len {
            let end = (t + dash).min(len);
            s.buf.push(DrawCommand::Line {
                start: from + dir * t,
                end: from + dir * end,
                stroke: Stroke::new(1.5, color),
            });
            t += dash + gap;
        }
    };
    run(
        Vec2::new(rect.min.x + inset, rect.min.y),
        Vec2::new(rect.max.x - inset, rect.min.y),
    );
    run(
        Vec2::new(rect.min.x + inset, rect.max.y),
        Vec2::new(rect.max.x - inset, rect.max.y),
    );
    run(
        Vec2::new(rect.min.x, rect.min.y + inset),
        Vec2::new(rect.min.x, rect.max.y - inset),
    );
    run(
        Vec2::new(rect.max.x, rect.min.y + inset),
        Vec2::new(rect.max.x, rect.max.y - inset),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use ui_platform::PointerState;
    use ui_render::CommandBuffer;
    use ui_themes::{ExtendedTokens, ThemeMode, ThemePalette};

    fn studio<'a>(buf: &'a mut CommandBuffer, pointer: &'a PointerState) -> Studio<'a> {
        Studio::new(
            buf,
            ThemePalette::dark(),
            ExtendedTokens::for_mode(ThemeMode::Dark),
            pointer,
        )
    }

    #[test]
    fn a_short_drop_card_keeps_its_buttons_inside() {
        let rect = Rect::from_min_size(Vec2::ZERO, Vec2::new(600.0, 200.0));
        let card = DropCard {
            glyph: crate::studio::glyphs::PHOTO,
            title: "Drop an image",
            body: "A long explanation that wraps onto more than one line in a narrow card,                    and then some more words so it needs a third line too.",
            browse: "Browse",
            secondary: None,
            hovering_files: false,
        };
        let mut buf = CommandBuffer::new();
        let idle = PointerState::default();
        studio(&mut buf, &idle).drop_card(rect, card);
        for c in buf.commands() {
            if let DrawCommand::Rect { rect: r, .. } = c {
                assert!(r.max.y <= rect.max.y + 0.5, "{r:?} spills out of {rect:?}");
            }
        }
    }

    #[test]
    fn cover_uv_crops_the_long_side_and_keeps_the_centre() {
        let wide = cover_uv(2.0, 1.0);
        assert!((wide.width() - 0.5).abs() < 1e-6 && (wide.min.x - 0.25).abs() < 1e-6);
        let tall = cover_uv(1.0, 2.0);
        assert!((tall.height() - 0.5).abs() < 1e-6 && (tall.min.y - 0.25).abs() < 1e-6);
        assert_eq!(cover_uv(0.0, 1.0).width(), 1.0);
    }

    #[test]
    fn card_grid_fills_rows_and_never_overflows() {
        let area = Rect::from_min_size(Vec2::new(10.0, 20.0), Vec2::new(1000.0, 800.0));
        let cards = card_grid(area, 7, 220.0, 0.75, 52.0);
        assert_eq!(cards.len(), 7);
        let cols = cards.iter().filter(|r| r.min.y == cards[0].min.y).count();
        assert_eq!(cols, 4);
        for r in &cards {
            assert!(
                r.min.x >= area.min.x && r.max.x <= area.max.x + 0.5,
                "{r:?}"
            );
            assert!(r.width() >= 220.0);
        }
        // A narrow area still gets one full-width column.
        let narrow = card_grid(
            Rect::from_min_size(Vec2::ZERO, Vec2::new(100.0, 100.0)),
            2,
            220.0,
            1.0,
            0.0,
        );
        assert_eq!(narrow[1].min.x, 0.0);
    }

    #[test]
    fn drop_card_reports_its_buttons() {
        let rect = Rect::from_min_size(Vec2::ZERO, Vec2::new(600.0, 420.0));
        let card = DropCard {
            glyph: crate::studio::glyphs::PHOTO,
            title: "Drop an image",
            body: "or browse for one",
            browse: "Browse",
            secondary: Some("Try a sample"),
            hovering_files: false,
        };
        let mut buf = CommandBuffer::new();
        let idle = PointerState::default();
        assert_eq!(
            studio(&mut buf, &idle).drop_card(rect, card),
            DropCardResponse::default()
        );
        // Find the buttons from the drawn labels and click each.
        let label_x = |buf: &CommandBuffer, want: &str| {
            buf.commands().iter().find_map(|c| match c {
                DrawCommand::Text { text, pos, .. } if text == want => Some(*pos),
                _ => None,
            })
        };
        let browse_at = label_x(&buf, "Browse").unwrap() + Vec2::new(4.0, 6.0);
        let sample_at = label_x(&buf, "Try a sample").unwrap() + Vec2::new(4.0, 6.0);
        for (at, browse) in [(browse_at, true), (sample_at, false)] {
            let pointer = PointerState {
                position: at,
                primary_clicked: true,
                ..PointerState::default()
            };
            let mut buf = CommandBuffer::new();
            let out = studio(&mut buf, &pointer).drop_card(rect, card);
            assert_eq!((out.browse, out.secondary), (browse, !browse));
        }
    }

    #[test]
    fn progress_row_and_thumbnail_card_draw() {
        let mut buf = CommandBuffer::new();
        let pointer = PointerState {
            position: Vec2::new(50.0, 50.0),
            ..PointerState::default()
        };
        let mut s = studio(&mut buf, &pointer);
        s.progress_row(
            Rect::from_min_size(Vec2::ZERO, Vec2::new(300.0, 24.0)),
            "Rendering",
            Some(0.5),
            "32 / 64 spp",
        );
        s.progress_row(
            Rect::from_min_size(Vec2::new(0.0, 30.0), Vec2::new(300.0, 24.0)),
            "Scanning",
            None,
            "",
        );
        let card = Rect::from_min_size(Vec2::ZERO, Vec2::new(200.0, 200.0));
        assert!(
            s.thumbnail_card(
                card,
                None,
                crate::studio::glyphs::FOLDER,
                "Brick wall",
                "Today",
                Some("Material")
            )
            .hovered
        );
        let texts: Vec<String> = buf
            .commands()
            .iter()
            .filter_map(|c| match c {
                DrawCommand::StyledText { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect();
        for want in ["Rendering", "32 / 64 spp", "Brick wall", "Material"] {
            assert!(
                texts.iter().any(|t| t == want),
                "{want} missing in {texts:?}"
            );
        }
    }

    #[test]
    fn tooltip_shows_with_its_shortcut_while_hovered() {
        let target = Rect::from_min_size(Vec2::new(100.0, 100.0), Vec2::new(32.0, 32.0));
        let viewport = Rect::from_min_size(Vec2::ZERO, Vec2::new(800.0, 600.0));
        let mut buf = CommandBuffer::new();
        let away = PointerState::default();
        studio(&mut buf, &away).tooltip(target, "Save", Some("Ctrl+S"), viewport);
        assert!(buf.is_empty());
        let over = PointerState {
            position: target.center(),
            ..PointerState::default()
        };
        studio(&mut buf, &over).tooltip(target, "Save", Some("Ctrl+S"), viewport);
        assert!(buf
            .commands()
            .iter()
            .any(|c| matches!(c, DrawCommand::StyledText { text, .. } if text == "Ctrl+S")));
    }
}
