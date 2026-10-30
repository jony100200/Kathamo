//! Studio workspace primitives: measured text, layered surfaces, rail items, pills,
//! avatars, list rows, buttons, meters and empty states.
//!
//! Widgets here are presentation only. They draw from theme tokens, report pointer
//! interaction, and never own application state.

use ui_core::{Color, CornerRadius, Rect, Stroke, Vec2};
use ui_platform::PointerState;
use ui_render::{measure_text, CommandBuffer, DrawCommand};
use ui_themes::{ExtendedTokens, ThemePalette, RADIUS_CARD, RADIUS_CONTROL, SPACE_2, SPACE_3};

/// A tintable monochrome SVG icon.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Glyph {
    pub id: &'static str,
    pub svg: &'static str,
}

macro_rules! glyph {
    ($name:ident, $file:literal) => {
        pub const $name: Glyph = Glyph {
            id: $file,
            svg: include_str!(concat!("../../assets/icons/", $file, ".svg")),
        };
    };
}

/// Icons used by workspace shells.
pub mod glyphs {
    use super::Glyph;
    glyph!(HOME, "home");
    glyph!(WORK, "layout-grid");
    glyph!(SESSIONS, "terminal");
    glyph!(LIBRARY, "stack-2");
    glyph!(ACTIVITY, "timeline");
    glyph!(SETTINGS, "settings");
    glyph!(SEARCH, "search");
    glyph!(PLUS, "plus");
    glyph!(FOLDER, "folder");
    glyph!(BRANCH, "git-branch");
    glyph!(CHECK, "check");
    glyph!(ALERT, "alert-triangle");
    glyph!(PLAY, "player-play");
    glyph!(STOP, "player-stop");
    glyph!(SPARKLES, "sparkles");
    glyph!(CPU, "cpu");
    glyph!(CHEVRON_DOWN, "chevron-down");
    glyph!(CHEVRON_RIGHT, "chevron-right");
    glyph!(FILE, "file-text");
    glyph!(CLOSE, "x");
    glyph!(INFO, "info-circle");
    glyph!(CLOCK, "clock");
    glyph!(CODE, "code");
    glyph!(LAYERS, "layers-intersect");
    glyph!(DESKTOP, "device-desktop");
    glyph!(REFRESH, "refresh");
    glyph!(CHAT, "message");
    glyph!(OFFICE, "users");
    glyph!(AGENT, "robot");
    glyph!(SUN, "sun");
    glyph!(MOON, "moon");
    glyph!(BELL, "bell");
    glyph!(SEND, "send");
    glyph!(ATTACH, "paperclip");
    glyph!(CHANNEL, "hash");
    glyph!(TASKS, "checklist");
    glyph!(IMPORT, "upload-cloud");
    glyph!(PHOTO, "photo");
    glyph!(COPY, "copy");
    glyph!(DOWNLOAD, "download");
    glyph!(TRASH, "trash");
}

/// Semantic tone for status pills, dots and meters. Always paired with a text label.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    Neutral,
    Good,
    Warning,
    Bad,
    Working,
    Info,
    Accent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonKind {
    Primary,
    Secondary,
    Ghost,
}

/// Pointer result for one widget this frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Interaction {
    pub hovered: bool,
    pub clicked: bool,
}

/// Painter bound to one frame's command buffer, palette and pointer.
pub struct Studio<'a> {
    pub buf: &'a mut CommandBuffer,
    pub palette: ThemePalette,
    pub tokens: ExtendedTokens,
    pub pointer: &'a PointerState,
}

/// Vertical text offset so a line of `size` px sits centred in a box.
pub fn text_center_y(rect: Rect, size: f32) -> f32 {
    rect.center().y - size * 0.36
}

/// Width of `label` drawn as a button of the given text size.
pub fn button_width(label: &str, size: f32) -> f32 {
    measure_text(label, size, true) + SPACE_3 * 2.0
}

impl<'a> Studio<'a> {
    pub fn new(
        buf: &'a mut CommandBuffer,
        palette: ThemePalette,
        tokens: ExtendedTokens,
        pointer: &'a PointerState,
    ) -> Self {
        Self {
            buf,
            palette,
            tokens,
            pointer,
        }
    }

    pub fn interact(&self, rect: Rect) -> Interaction {
        let hovered = rect.contains(self.pointer.position);
        Interaction {
            hovered,
            clicked: hovered && self.pointer.primary_clicked,
        }
    }

    pub fn tone_color(&self, tone: Tone) -> Color {
        match tone {
            Tone::Neutral => self.palette.text_muted,
            Tone::Good => self.palette.success,
            Tone::Warning => self.palette.warning,
            Tone::Bad => self.palette.danger,
            Tone::Working => self.tokens.working,
            Tone::Info => self.tokens.info,
            Tone::Accent => self.palette.accent,
        }
    }

    pub fn fill(&mut self, rect: Rect, color: Color, radius: f32) {
        self.buf.push(DrawCommand::Rect {
            rect,
            fill: color,
            radius: CornerRadius::same(radius),
            stroke: Stroke::new(0.0, Color::TRANSPARENT),
        });
    }

    pub fn outline(&mut self, rect: Rect, fill: Color, edge: Color, radius: f32) {
        self.buf.push(DrawCommand::Rect {
            rect,
            fill,
            radius: CornerRadius::same(radius),
            stroke: Stroke::new(1.0, edge),
        });
    }

    pub fn line(&mut self, from: Vec2, to: Vec2, color: Color) {
        self.buf.push(DrawCommand::Line {
            start: from,
            end: to,
            stroke: Stroke::new(1.0, color),
        });
    }

    pub fn clip(&mut self, rect: Rect) {
        self.buf.push(DrawCommand::ClipPush { rect });
    }

    pub fn unclip(&mut self) {
        self.buf.push(DrawCommand::ClipPop);
    }

    pub fn text(&mut self, value: &str, pos: Vec2, size: f32, color: Color, strong: bool) {
        self.buf.push(DrawCommand::Text {
            text: value.to_owned(),
            pos,
            size,
            color,
            strong,
        });
    }

    /// Draw text truncated with an ellipsis to fit `max_width`. Returns the drawn width.
    pub fn text_fit(
        &mut self,
        value: &str,
        pos: Vec2,
        max_width: f32,
        size: f32,
        color: Color,
        strong: bool,
    ) -> f32 {
        let fitted = fit_text(value, max_width, size, strong);
        let width = measure_text(&fitted, size, strong);
        self.text(&fitted, pos, size, color, strong);
        width
    }

    pub fn icon(&mut self, glyph: Glyph, rect: Rect, color: Color) {
        self.buf.push(DrawCommand::Icon {
            id: glyph.id,
            svg: glyph.svg,
            rect,
            color,
        });
    }

    /// Layered card surface: soft shadow, vertical gradient, hairline border, top rim light.
    pub fn surface(&mut self, rect: Rect, level: u8, radius: f32) {
        let top = self.palette.surface_at_elevation(level.saturating_add(1));
        let bottom = self.palette.surface_at_elevation(level);
        if level > 0 {
            self.buf.push(DrawCommand::SoftShadow {
                rect,
                radius: CornerRadius::same(radius),
                color: Color::rgba(0, 0, 0, 70),
                offset: Vec2::new(0.0, 4.0),
                blur: 14.0,
                spread: 0.0,
            });
        }
        self.buf.push(DrawCommand::Surface {
            rect,
            fill_top: mix(bottom, top, 0.55),
            fill_bottom: bottom,
            radius: CornerRadius::same(radius),
            stroke: Stroke::new(1.0, self.palette.border),
            rim_strength: 0.45,
        });
    }

    /// Clickable card. Hover lifts the surface one elevation level.
    pub fn card(&mut self, rect: Rect) -> Interaction {
        let hit = self.interact(rect);
        self.surface(rect, if hit.hovered { 2 } else { 1 }, RADIUS_CARD);
        hit
    }

    /// Small uppercase section label, e.g. "NEEDS YOU".
    pub fn section_label(&mut self, value: &str, pos: Vec2) {
        let upper = value.to_uppercase();
        self.text(&upper, pos, 10.5, self.palette.text_muted, true);
    }

    /// Status pill: coloured dot + text label. Returns its width.
    pub fn pill(&mut self, label: &str, pos: Vec2, tone: Tone) -> f32 {
        let size = 11.5;
        let width = measure_text(label, size, true) + 26.0;
        let rect = Rect::from_min_size(pos, Vec2::new(width, 22.0));
        let color = self.tone_color(tone);
        self.outline(rect, with_alpha(color, 26), with_alpha(color, 90), 11.0);
        self.fill(
            centered(
                Vec2::new(rect.min.x + 11.0, rect.center().y),
                Vec2::splat(6.0),
            ),
            color,
            3.0,
        );
        self.text(
            label,
            Vec2::new(rect.min.x + 19.0, text_center_y(rect, size)),
            size,
            color,
            true,
        );
        width
    }

    /// Circle with initials, tinted from a stable hash of the name.
    pub fn avatar(&mut self, rect: Rect, name: &str) {
        let tint = name_tint(name);
        let radius = rect.width().min(rect.height()) / 2.0;
        self.outline(rect, with_alpha(tint, 48), with_alpha(tint, 120), radius);
        let initials = initials(name);
        let size = (rect.height() * 0.38).max(9.0);
        let w = measure_text(&initials, size, true);
        self.text(
            &initials,
            Vec2::new(rect.center().x - w / 2.0, text_center_y(rect, size)),
            size,
            tint,
            true,
        );
    }

    pub fn button(&mut self, rect: Rect, label: &str, kind: ButtonKind) -> Interaction {
        self.button_with_icon(rect, None, label, kind)
    }

    pub fn button_with_icon(
        &mut self,
        rect: Rect,
        glyph: Option<Glyph>,
        label: &str,
        kind: ButtonKind,
    ) -> Interaction {
        let hit = self.interact(rect);
        let p = self.palette;
        let (fill, edge, ink) = match kind {
            ButtonKind::Primary => (
                if hit.hovered {
                    p.accent_hover
                } else {
                    p.accent
                },
                p.accent,
                self.tokens.on_accent,
            ),
            ButtonKind::Secondary => (
                if hit.hovered {
                    p.elevated_high
                } else {
                    p.elevated
                },
                p.border,
                p.text,
            ),
            ButtonKind::Ghost => (
                if hit.hovered {
                    p.elevated
                } else {
                    Color::TRANSPARENT
                },
                if hit.hovered {
                    p.border
                } else {
                    Color::TRANSPARENT
                },
                if hit.hovered { p.text } else { p.text_muted },
            ),
        };
        self.outline(rect, fill, edge, RADIUS_CONTROL);
        let size = 12.5;
        let text_w = measure_text(label, size, true);
        let icon_w = if glyph.is_some() { 16.0 + 6.0 } else { 0.0 };
        let mut x = rect.center().x - (text_w + icon_w) / 2.0;
        if let Some(g) = glyph {
            self.icon(
                g,
                Rect::from_min_size(Vec2::new(x, rect.center().y - 8.0), Vec2::splat(16.0)),
                ink,
            );
            x += icon_w;
        }
        self.text(
            label,
            Vec2::new(x, text_center_y(rect, size)),
            size,
            ink,
            true,
        );
        hit
    }

    /// Square icon-only button with a hover background.
    pub fn icon_button(&mut self, rect: Rect, glyph: Glyph, active: bool) -> Interaction {
        let hit = self.interact(rect);
        if active || hit.hovered {
            self.fill(rect, self.palette.elevated, RADIUS_CONTROL);
        }
        let color = if active || hit.hovered {
            self.palette.text
        } else {
            self.palette.text_muted
        };
        let s = 18.0;
        self.icon(glyph, centered(rect.center(), Vec2::splat(s)), color);
        hit
    }

    /// Navigation rail destination: icon above a short label, accent bar when active.
    pub fn rail_item(
        &mut self,
        rect: Rect,
        glyph: Glyph,
        label: &str,
        active: bool,
        badge: Option<usize>,
    ) -> Interaction {
        let hit = self.interact(rect);
        let p = self.palette;
        let tile = centered(
            Vec2::new(rect.center().x, rect.min.y + 20.0),
            Vec2::new(40.0, 32.0),
        );
        if active {
            self.fill(tile, with_alpha(p.accent, 34), RADIUS_CONTROL);
            self.fill(
                Rect::from_min_size(
                    Vec2::new(rect.min.x, tile.min.y + 6.0),
                    Vec2::new(3.0, tile.height() - 12.0),
                ),
                p.accent,
                1.5,
            );
        } else if hit.hovered {
            self.fill(tile, p.elevated, RADIUS_CONTROL);
        }
        let ink = if active {
            p.accent
        } else if hit.hovered {
            p.text
        } else {
            p.text_muted
        };
        self.icon(glyph, centered(tile.center(), Vec2::splat(19.0)), ink);
        let size = 10.5;
        let w = measure_text(label, size, active);
        self.text(
            label,
            Vec2::new(rect.center().x - w / 2.0, tile.max.y + 5.0),
            size,
            if active { p.text } else { p.text_muted },
            active,
        );
        if let Some(count) = badge.filter(|c| *c > 0) {
            let text = if count > 99 {
                "99+".to_string()
            } else {
                count.to_string()
            };
            let bw = measure_text(&text, 9.5, true) + 9.0;
            let br = Rect::from_min_size(
                Vec2::new(tile.max.x - bw + 6.0, tile.min.y - 4.0),
                Vec2::new(bw.max(16.0), 16.0),
            );
            self.fill(br, p.accent, 8.0);
            self.text(
                &text,
                Vec2::new(br.center().x - (bw - 9.0) / 2.0, text_center_y(br, 9.5)),
                9.5,
                self.tokens.on_accent,
                true,
            );
        }
        hit
    }

    /// Context-navigation or list row: optional icon, label, optional trailing text.
    pub fn nav_row(
        &mut self,
        rect: Rect,
        glyph: Option<Glyph>,
        label: &str,
        trailing: Option<&str>,
        active: bool,
    ) -> Interaction {
        let hit = self.interact(rect);
        let p = self.palette;
        if active {
            self.fill(rect, self.tokens.selection, RADIUS_CONTROL);
        } else if hit.hovered {
            self.fill(rect, p.elevated, RADIUS_CONTROL);
        }
        let ink = if active { p.text } else { p.text_muted };
        let mut x = rect.min.x + SPACE_2 + 2.0;
        if let Some(g) = glyph {
            self.icon(
                g,
                Rect::from_min_size(Vec2::new(x, rect.center().y - 8.0), Vec2::splat(16.0)),
                if active { p.accent } else { ink },
            );
            x += 24.0;
        }
        let size = 13.0;
        let trailing_w = trailing
            .map(|t| measure_text(t, 11.5, false) + SPACE_3)
            .unwrap_or(0.0);
        let max_w = (rect.max.x - x - trailing_w - SPACE_2).max(0.0);
        self.text_fit(
            label,
            Vec2::new(x, text_center_y(rect, size)),
            max_w,
            size,
            if active || hit.hovered { p.text } else { ink },
            active,
        );
        if let Some(t) = trailing {
            let w = measure_text(t, 11.5, false);
            self.text(
                t,
                Vec2::new(rect.max.x - SPACE_2 - w, text_center_y(rect, 11.5)),
                11.5,
                p.text_muted,
                false,
            );
        }
        hit
    }

    /// Horizontal meter. `None` renders "unavailable" instead of a fake zero.
    pub fn meter(&mut self, rect: Rect, label: &str, value: Option<(f32, f32)>, tone: Tone) {
        let p = self.palette;
        self.text(label, rect.min, 10.5, p.text_muted, true);
        let bar = Rect::from_min_size(
            Vec2::new(rect.min.x, rect.max.y - 4.0),
            Vec2::new(rect.width(), 4.0),
        );
        self.fill(bar, p.elevated, 2.0);
        let readout = match value {
            Some((used, total)) if total > 0.0 => {
                let frac = (used / total).clamp(0.0, 1.0);
                let mut filled = bar;
                filled.max.x = bar.min.x + bar.width() * frac;
                self.fill(filled, self.tone_color(tone), 2.0);
                format!("{used:.1} / {total:.0} GB")
            }
            _ => "unavailable".to_string(),
        };
        let w = measure_text(&readout, 10.5, false);
        self.text(
            &readout,
            Vec2::new(rect.max.x - w, rect.min.y),
            10.5,
            p.text_muted,
            false,
        );
    }

    /// Centred empty/offline state with an optional call to action.
    pub fn empty_state(
        &mut self,
        rect: Rect,
        glyph: Glyph,
        title: &str,
        body: &str,
        action: Option<&str>,
    ) -> Interaction {
        let p = self.palette;
        let c = rect.center();
        let top = c.y - 70.0;
        let disc = centered(Vec2::new(c.x, top + 22.0), Vec2::splat(48.0));
        self.outline(disc, p.elevated, p.border, 24.0);
        self.icon(glyph, centered(disc.center(), Vec2::splat(22.0)), p.accent);
        let tw = measure_text(title, 16.0, true);
        self.text(
            title,
            Vec2::new(c.x - tw / 2.0, top + 62.0),
            16.0,
            p.text,
            true,
        );
        let max_w = rect.width().min(420.0);
        let mut y = top + 88.0;
        for line in wrap_text(body, max_w, 13.0) {
            let lw = measure_text(&line, 13.0, false);
            self.text(
                &line,
                Vec2::new(c.x - lw / 2.0, y),
                13.0,
                p.text_muted,
                false,
            );
            y += 19.0;
        }
        match action {
            Some(label) => {
                let w = button_width(label, 12.5) + 8.0;
                let r = Rect::from_min_size(Vec2::new(c.x - w / 2.0, y + 12.0), Vec2::new(w, 34.0));
                self.button(r, label, ButtonKind::Secondary)
            }
            None => Interaction::default(),
        }
    }
}

/// Truncate `value` with an ellipsis so it fits `max_width` at `size`.
pub fn fit_text(value: &str, max_width: f32, size: f32, strong: bool) -> String {
    if measure_text(value, size, strong) <= max_width {
        return value.to_owned();
    }
    let chars: Vec<char> = value.chars().collect();
    let (mut lo, mut hi) = (0usize, chars.len());
    while lo < hi {
        let mid = (lo + hi + 1) / 2;
        let candidate: String = chars[..mid].iter().collect::<String>() + "…";
        if measure_text(&candidate, size, strong) <= max_width {
            lo = mid;
        } else {
            hi = mid - 1;
        }
    }
    if lo == 0 {
        return String::new();
    }
    chars[..lo].iter().collect::<String>().trim_end().to_owned() + "…"
}

/// Greedy word wrap by measured width.
pub fn wrap_text(value: &str, max_width: f32, size: f32) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in value.split_whitespace() {
        let candidate = if current.is_empty() {
            word.to_owned()
        } else {
            format!("{current} {word}")
        };
        if measure_text(&candidate, size, false) > max_width && !current.is_empty() {
            lines.push(std::mem::replace(&mut current, word.to_owned()));
        } else {
            current = candidate;
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

/// Up to two uppercase initials from a display name or id.
pub fn initials(name: &str) -> String {
    let parts: Vec<&str> = name
        .split(|c: char| c.is_whitespace() || c == '-' || c == '_' || c == '.')
        .filter(|p| !p.is_empty())
        .collect();
    let mut out: String = parts
        .iter()
        .take(2)
        .filter_map(|p| p.chars().next())
        .collect();
    if parts.len() == 1 {
        out = parts[0].chars().take(2).collect();
    }
    out.to_uppercase()
}

fn name_tint(name: &str) -> Color {
    const TINTS: [Color; 6] = [
        Color::rgb(0xE5, 0xA0, 0x73),
        Color::rgb(0x6F, 0xC3, 0xD6),
        Color::rgb(0x9F, 0x9B, 0xE6),
        Color::rgb(0x68, 0xC5, 0x9E),
        Color::rgb(0xE9, 0x8F, 0xA6),
        Color::rgb(0xC9, 0xB8, 0x6A),
    ];
    let hash = name
        .bytes()
        .fold(2166136261u32, |h, b| (h ^ b as u32).wrapping_mul(16777619));
    TINTS[hash as usize % TINTS.len()]
}

pub fn with_alpha(color: Color, alpha: u8) -> Color {
    color.with_alpha(alpha)
}

/// Rect of `size` centred on `center`.
pub fn centered(center: Vec2, size: Vec2) -> Rect {
    Rect::from_min_size(center - size * 0.5, size)
}

fn mix(a: Color, b: Color, t: f32) -> Color {
    let lerp = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color::rgba(
        lerp(a.r, b.r),
        lerp(a.g, b.g),
        lerp(a.b, b.b),
        lerp(a.a, b.a),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fit_text_truncates_with_ellipsis_and_keeps_short_text() {
        assert_eq!(fit_text("Ongon", 500.0, 13.0, false), "Ongon");
        let long = "claude-opus-reasoning-model-with-a-very-long-name";
        let fitted = fit_text(long, 80.0, 13.0, false);
        assert!(fitted.ends_with('…'));
        assert!(measure_text(&fitted, 13.0, false) <= 80.0);
        assert!(fit_text("বাংলা লেবেল", 20.0, 13.0, false).chars().count() < 11);
    }

    #[test]
    fn wrap_text_respects_width() {
        let lines = wrap_text("one two three four five six seven eight", 60.0, 13.0);
        assert!(lines.len() > 1);
        assert_eq!(lines.join(" "), "one two three four five six seven eight");
    }

    #[test]
    fn initials_are_stable() {
        assert_eq!(initials("Claude Code"), "CC");
        assert_eq!(initials("codex"), "CO");
        assert_eq!(initials("open-code"), "OC");
        assert_eq!(name_tint("codex"), name_tint("codex"));
    }

    #[test]
    fn clicks_only_register_inside_widget() {
        let mut buf = CommandBuffer::default();
        let pointer = PointerState {
            position: Vec2::new(10.0, 10.0),
            primary_clicked: true,
            ..PointerState::default()
        };
        let palette = ThemePalette::midnight_graphite();
        let tokens = ExtendedTokens::for_mode(ui_themes::ThemeMode::MidnightGraphite);
        let mut s = Studio::new(&mut buf, palette, tokens, &pointer);
        let inside = Rect::from_min_size(Vec2::ZERO, Vec2::new(40.0, 40.0));
        let outside = Rect::from_min_size(Vec2::new(50.0, 0.0), Vec2::new(40.0, 40.0));
        assert!(s.button(inside, "Go", ButtonKind::Primary).clicked);
        assert!(!s.button(outside, "No", ButtonKind::Ghost).clicked);
        assert!(s.meter_draws_unavailable_without_value());
    }

    impl Studio<'_> {
        fn meter_draws_unavailable_without_value(&mut self) -> bool {
            let before = self.buf.len();
            self.meter(
                Rect::from_min_size(Vec2::ZERO, Vec2::new(120.0, 24.0)),
                "VRAM",
                None,
                Tone::Working,
            );
            self.buf
                .commands()
                .iter()
                .skip(before)
                .any(|c| matches!(c, DrawCommand::Text { text, .. } if text == "unavailable"))
        }
    }
}
