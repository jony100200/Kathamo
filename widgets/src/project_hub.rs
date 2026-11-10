//! A start screen for document apps: a title, a few big "start" cards with their own
//! actions, quick buttons (new, open, ...) and a grid of recent projects with covers.
//!
//! The hub is a pure view. The app supplies the cards and recent items each frame and
//! acts on the returned [`HubAction`]; it keeps the recent list and loads cover textures.

use ui_core::{Rect, Vec2};
use ui_themes::{RADIUS_CARD, SPACE_2, SPACE_3, SPACE_4, SPACE_6, SPACE_8};

use crate::studio::{centered, wrap_text, ButtonKind, Glyph, Studio};
use crate::studio_cards::card_grid;
use crate::type_scale::TypeRole;

/// One big way to start, with one or more actions (buttons).
#[derive(Debug, Clone, Copy)]
pub struct HubStart<'a> {
    pub glyph: Glyph,
    pub title: &'a str,
    pub body: &'a str,
    /// Button labels; the first is drawn as the primary action.
    pub actions: &'a [&'a str],
}

/// One recent project card.
#[derive(Debug, Clone, Copy)]
pub struct HubProject<'a> {
    pub title: &'a str,
    /// For example the last-opened date.
    pub caption: &'a str,
    /// For example the project's kind or last mode.
    pub badge: Option<&'a str>,
    /// A cover image texture already uploaded by the app, and its width / height.
    pub cover: Option<(u64, f32)>,
}

/// Everything the hub shows this frame.
#[derive(Debug, Clone, Copy)]
pub struct ProjectHub<'a> {
    pub title: &'a str,
    pub tagline: &'a str,
    pub starts: &'a [HubStart<'a>],
    /// Small buttons at the top right: (icon, label).
    pub quick: &'a [(Glyph, &'a str)],
    pub recent: &'a [HubProject<'a>],
    /// Shown instead of the grid when there are no recent projects.
    pub recent_empty: &'a str,
}

/// What was clicked in the hub this frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HubAction {
    /// Start card `card`, its action button `action`.
    Start { card: usize, action: usize },
    /// A recent project card.
    Recent(usize),
    /// A quick button.
    Quick(usize),
}

/// Content column: at most 1200 px wide, centred, with side margins.
fn column(rect: Rect) -> Rect {
    let w = (rect.width() - SPACE_8 * 2.0).clamp(0.0, 1200.0);
    let x = rect.center().x - w * 0.5;
    Rect::from_min_max(
        Vec2::new(x, rect.min.y + SPACE_8),
        Vec2::new(x + w, rect.max.y - SPACE_6),
    )
}

/// Draws the hub into `rect` and returns what was clicked.
pub fn project_hub(s: &mut Studio, rect: Rect, hub: &ProjectHub) -> Option<HubAction> {
    let p = s.palette;
    s.fill(rect, p.canvas, 0.0);
    let col = column(rect);
    let mut action = None;
    let display = TypeRole::Display.style();
    display.draw(s.buf, hub.title, col.min, p.text);
    let tagline = TypeRole::Body.style().at(14.0);
    tagline.draw(
        s.buf,
        hub.tagline,
        Vec2::new(col.min.x, col.min.y + 40.0),
        p.text_muted,
    );
    // Quick buttons, right-aligned on the title row.
    let mut x = col.max.x;
    for (i, (glyph, label)) in hub.quick.iter().enumerate().rev() {
        let w = crate::studio::button_width(label, 12.5) + 22.0;
        x -= w;
        let r = Rect::from_min_size(Vec2::new(x, col.min.y + 4.0), Vec2::new(w, 36.0));
        if s.button_with_icon(r, Some(*glyph), label, ButtonKind::Secondary)
            .clicked
        {
            action = Some(HubAction::Quick(i));
        }
        x -= SPACE_2;
    }
    let mut y = col.min.y + 40.0 + 20.0 + SPACE_6;
    let n = hub.starts.len().max(1) as f32;
    let card_w = ((col.width() - SPACE_4 * (n - 1.0)) / n).floor();
    let card_h = 176.0;
    for (i, start) in hub.starts.iter().enumerate() {
        let r = Rect::from_min_size(
            Vec2::new(col.min.x + i as f32 * (card_w + SPACE_4), y),
            Vec2::new(card_w, card_h),
        );
        if let Some(a) = start_card(s, r, start) {
            action = Some(HubAction::Start { card: i, action: a });
        }
    }
    y += card_h + SPACE_6;
    TypeRole::Section
        .style()
        .draw(s.buf, "Recent projects", Vec2::new(col.min.x, y), p.text);
    y += 20.0 + SPACE_3;
    let area = Rect::from_min_max(Vec2::new(col.min.x, y), col.max);
    if hub.recent.is_empty() {
        TypeRole::Body
            .style()
            .draw(s.buf, hub.recent_empty, area.min, p.text_muted);
        return action;
    }
    // Covers are wide; a short window gets flatter cards rather than cut-off ones.
    let mut cards = card_grid(area, hub.recent.len(), 200.0, 0.56, 52.0);
    if let Some(first) = cards.first().copied() {
        if first.height() > area.height() && first.width() > 0.0 {
            let aspect = ((area.height() - 52.0) / first.width()).max(0.3);
            cards = card_grid(area, hub.recent.len(), 200.0, aspect, 52.0);
        }
    }
    s.clip(area);
    for (i, (r, item)) in cards.iter().zip(hub.recent).enumerate() {
        if r.min.y > area.max.y {
            break;
        }
        let hit = s.thumbnail_card(
            *r,
            item.cover,
            crate::studio::glyphs::FOLDER,
            item.title,
            item.caption,
            item.badge,
        );
        if hit.clicked && area.contains(s.pointer.position) {
            action = Some(HubAction::Recent(i));
        }
    }
    s.unclip();
    action
}

/// One start card; returns the index of the clicked action button.
fn start_card(s: &mut Studio, rect: Rect, start: &HubStart) -> Option<usize> {
    let p = s.palette;
    let hovered = s.interact(rect).hovered;
    s.surface(rect, if hovered { 2 } else { 1 }, RADIUS_CARD);
    let pad = SPACE_6;
    let disc = Rect::from_min_size(rect.min + Vec2::splat(pad), Vec2::splat(48.0));
    s.outline(disc, p.elevated, p.border, 24.0);
    s.icon(
        start.glyph,
        centered(disc.center(), Vec2::splat(24.0)),
        p.accent,
    );
    let tx = disc.max.x + SPACE_4;
    let title = TypeRole::Title.style().at(20.0);
    title.draw(s.buf, start.title, Vec2::new(tx, disc.min.y), p.text);
    let body = TypeRole::Body.style();
    let mut y = disc.min.y + 30.0;
    for line in wrap_text(start.body, rect.max.x - pad - tx, body.size)
        .iter()
        .take(3)
    {
        body.draw(s.buf, line, Vec2::new(tx, y), p.text_muted);
        y += 19.0;
    }
    let mut x = tx;
    let by = rect.max.y - pad - 36.0;
    let mut picked = None;
    for (i, label) in start.actions.iter().enumerate() {
        let w = crate::studio::button_width(label, 12.5) + 12.0;
        let r = Rect::from_min_size(Vec2::new(x, by), Vec2::new(w, 36.0));
        let kind = if i == 0 {
            ButtonKind::Primary
        } else {
            ButtonKind::Secondary
        };
        if s.button(r, label, kind).clicked {
            picked = Some(i);
        }
        x += w + SPACE_2;
    }
    picked
}

#[cfg(test)]
mod tests {
    use super::*;
    use ui_platform::PointerState;
    use ui_render::{CommandBuffer, DrawCommand};
    use ui_themes::{ExtendedTokens, ThemeMode, ThemePalette};

    const STARTS: [HubStart; 2] = [
        HubStart {
            glyph: crate::studio::glyphs::SPHERE,
            title: "Make something",
            body: "From an image or from scratch.",
            actions: &["From an image", "From scratch"],
        },
        HubStart {
            glyph: crate::studio::glyphs::BRUSH,
            title: "Paint something",
            body: "Load a model and paint on it.",
            actions: &["Open a model"],
        },
    ];

    fn frame(pointer: PointerState, recent: &[HubProject]) -> (Option<HubAction>, CommandBuffer) {
        let mut buf = CommandBuffer::new();
        let hub = ProjectHub {
            title: "Studio",
            tagline: "Pick up where you left off.",
            starts: &STARTS,
            quick: &[(crate::studio::glyphs::PLUS, "New project")],
            recent,
            recent_empty: "Nothing yet.",
        };
        let mut s = Studio::new(
            &mut buf,
            ThemePalette::dark(),
            ExtendedTokens::for_mode(ThemeMode::Dark),
            &pointer,
        );
        let rect = Rect::from_min_size(Vec2::ZERO, Vec2::new(1280.0, 800.0));
        let action = project_hub(&mut s, rect, &hub);
        (action, buf)
    }

    fn label_pos(buf: &CommandBuffer, want: &str) -> Vec2 {
        buf.commands()
            .iter()
            .find_map(|c| match c {
                DrawCommand::Text { text, pos, .. } | DrawCommand::StyledText { text, pos, .. }
                    if text == want =>
                {
                    Some(*pos)
                }
                _ => None,
            })
            .unwrap_or_else(|| panic!("{want} not drawn"))
    }

    fn click(at: Vec2) -> PointerState {
        PointerState {
            position: at + Vec2::new(3.0, 6.0),
            primary_clicked: true,
            ..PointerState::default()
        }
    }

    #[test]
    fn start_actions_quick_buttons_and_recent_cards_report_clicks() {
        let recent = [HubProject {
            title: "Old project",
            caption: "Yesterday",
            badge: Some("Paint"),
            cover: None,
        }];
        let (idle, buf) = frame(PointerState::default(), &recent);
        assert_eq!(idle, None);
        let scratch = label_pos(&buf, "From scratch");
        let new = label_pos(&buf, "New project");
        let old = label_pos(&buf, "Old project");
        assert_eq!(
            frame(click(scratch), &recent).0,
            Some(HubAction::Start { card: 0, action: 1 })
        );
        assert_eq!(frame(click(new), &recent).0, Some(HubAction::Quick(0)));
        assert_eq!(frame(click(old), &recent).0, Some(HubAction::Recent(0)));
    }

    #[test]
    fn an_empty_recent_list_shows_the_hint() {
        let (_, buf) = frame(PointerState::default(), &[]);
        label_pos(&buf, "Nothing yet.");
    }
}
