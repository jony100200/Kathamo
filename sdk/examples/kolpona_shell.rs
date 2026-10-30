//! Kolpona native studio shell. UI-only showcase; no agent backend is simulated.
//! Native: cargo run -p kathamo --example kolpona_shell -- --native
//! Smoke test: cargo run -p kathamo --example kolpona_shell

use kathamo::prelude::*;

const APPS: [(&str, &str); 6] = [
    ("Ongon", "Agent workspace"),
    ("Rupayon", "Asset factory"),
    ("Ojon", "Model workshop"),
    ("Chobi", "Image studio"),
    ("Drisho", "Motion studio"),
    ("Pothik", "Knowledge studio"),
];
const ACCENTS: [Color; 6] = [
    Color::rgb(217, 169, 108),
    Color::rgb(115, 186, 163),
    Color::rgb(165, 150, 207),
    Color::rgb(218, 147, 137),
    Color::rgb(119, 164, 197),
    Color::rgb(197, 180, 122),
];
const ICONS: [&str; 6] = [
    include_str!("../../assets/icons/sparkles.svg"),
    include_str!("../../assets/icons/layers-intersect.svg"),
    include_str!("../../assets/icons/cpu.svg"),
    include_str!("../../assets/icons/photo.svg"),
    include_str!("../../assets/icons/movie.svg"),
    include_str!("../../assets/icons/search.svg"),
];

#[derive(Default)]
struct KolponaShell {
    selected: usize,
    details_hidden: bool,
}

fn rect(x: f32, y: f32, w: f32, h: f32) -> Rect {
    Rect::from_min_size(Vec2::new(x, y), Vec2::new(w.max(1.0), h.max(1.0)))
}

fn text(buf: &mut CommandBuffer, value: &str, x: f32, y: f32, size: f32, color: Color, strong: bool) {
    buf.push(DrawCommand::Text {
        text: value.to_owned(),
        pos: Vec2::new(x, y),
        size,
        color,
        strong,
    });
}

fn surface(buf: &mut CommandBuffer, r: Rect, top: Color, bottom: Color, edge: Color, radius: f32) {
    buf.push(DrawCommand::Surface {
        rect: r,
        fill_top: top,
        fill_bottom: bottom,
        radius: CornerRadius::same(radius),
        stroke: Stroke::new(1.0, edge),
        rim_strength: 0.55,
    });
}

fn icon(buf: &mut CommandBuffer, id: &'static str, svg: &'static str, r: Rect, color: Color) {
    buf.push(DrawCommand::Icon { id, svg, rect: r, color });
}

impl KolponaShell {
    fn has_inspector(&self, bounds: Rect) -> bool {
        !self.details_hidden && bounds.width() >= 1220.0
    }

    fn work_rect(&self, bounds: Rect) -> Rect {
        let aside = if self.has_inspector(bounds) { 348.0 } else { 38.0 };
        rect(108.0, 107.0, bounds.width() - 108.0 - aside, bounds.height() - 140.0)
    }

    fn card_rects(&self, bounds: Rect) -> [Rect; 6] {
        let work = self.work_rect(bounds);
        let gap = 14.0;
        let width = (work.width() - gap * 2.0) / 3.0;
        std::array::from_fn(|idx| {
            let col = idx % 3;
            let row = idx / 3;
            rect(
                work.min.x + col as f32 * (width + gap),
                work.min.y + 312.0 + row as f32 * 177.0,
                width,
                162.0,
            )
        })
    }

    fn inspector_toggle(bounds: Rect) -> Rect {
        rect(bounds.max.x - 181.0, 20.0, 146.0, 35.0)
    }
}

impl KathamoApp for KolponaShell {
    fn title(&self) -> &str { "Kolpona | Creative Workspace" }
    fn default_size(&self) -> (u32, u32) { (1510, 920) }

    fn update(&mut self, ctx: &mut AppContext) {
        let bounds = ctx.bounds();
        let p = ctx.theme;
        let work = self.work_rect(bounds);
        let buf = &mut ctx.cmd_buf;

        surface(buf, rect(0.0, 0.0, bounds.width(), 75.0),
            p.surface, p.canvas, p.border.with_alpha(95), 0.0);
        surface(buf, rect(0.0, 75.0, 75.0, bounds.height() - 75.0),
            p.surface, p.canvas, p.border.with_alpha(95), 0.0);

        surface(buf, rect(15.0, 16.0, 44.0, 44.0),
            p.accent, Color::rgb(176, 123, 75), p.accent_hover, 13.0);
        text(buf, "K", 31.0, 25.0, 22.0, p.canvas, true);
        text(buf, "KOLPONA", 102.0, 18.0, 18.0, p.text, true);
        text(buf, "CREATIVE OPERATING WORKSPACE", 102.0, 43.0, 10.0, p.text_muted, false);

        let rail: [(&str, &str, &str); 4] = [
            ("home", include_str!("../../assets/icons/home.svg"), "Home"),
            ("apps", include_str!("../../assets/icons/layout-grid.svg"), "Apps"),
            ("create", include_str!("../../assets/icons/bolt.svg"), "Create"),
            ("system", include_str!("../../assets/icons/settings.svg"), "System"),
        ];
        for (idx, (id, svg, name)) in rail.iter().enumerate() {
            let y = 108.0 + idx as f32 * 83.0;
            if idx == 0 {
                surface(buf, rect(10.0, y - 7.0, 55.0, 59.0), p.elevated, p.surface, p.border, 12.0);
            }
            icon(buf, id, svg, rect(26.0, y, 23.0, 23.0),
                if idx == 0 { p.accent } else { p.text_muted });
            text(buf, name, 15.0, y + 31.0, 10.0, p.text_muted, false);
        }
        let toggle = Self::inspector_toggle(bounds);
        surface(buf, toggle, p.elevated, p.surface, p.border, 10.0);
        text(buf, if self.details_hidden { "Show details" } else { "Hide details" },
            toggle.min.x + 13.0, toggle.min.y + 10.0, 11.0, p.text, true);

        text(buf, "Your creative workspace", work.min.x, work.min.y, 27.0, p.text, true);
        text(buf, "Move between your tools and projects without terminal clutter.",
            work.min.x, work.min.y + 44.0, 12.0, p.text_muted, false);

        let banner = rect(work.min.x, work.min.y + 82.0, work.width(), 148.0);
        buf.push(DrawCommand::SoftShadow {
            rect: banner,
            radius: CornerRadius::same(16.0),
            color: Color::rgba(0, 0, 0, 74),
            offset: Vec2::new(0.0, 5.0),
            blur: 14.0,
            spread: 0.0,
        });
        surface(buf, banner, Color::rgb(59, 48, 40),
            Color::rgb(37, 38, 37), Color::rgb(93, 75, 57), 16.0);
        text(buf, "BUILT FOR CREATORS", banner.min.x + 23.0,
            banner.min.y + 18.0, 11.0, p.accent_hover, true);
        text(buf, "Make something worth finishing.", banner.min.x + 23.0,
            banner.min.y + 51.0, 22.0, p.text, true);
        text(buf, "One native workspace. Six focused studio applications.",
            banner.min.x + 23.0, banner.min.y + 101.0, 12.0, p.text_muted, false);

        text(buf, "APPLICATIONS", work.min.x, work.min.y + 273.0, 11.0, p.text_muted, true);
        for (i, card) in self.card_rects(bounds).iter().enumerate() {
            let selected = i == self.selected;
            if selected {
                buf.push(DrawCommand::SoftShadow {
                    rect: *card,
                    radius: CornerRadius::same(14.0),
                    color: Color::rgba(0, 0, 0, 90),
                    offset: Vec2::new(0.0, 5.0),
                    blur: 12.0,
                    spread: 0.0,
                });
            }
            surface(buf, *card, if selected { p.elevated_high } else { p.elevated },
                p.surface, if selected { ACCENTS[i].with_alpha(145) } else { p.border }, 14.0);
            surface(buf, rect(card.min.x + 17.0, card.min.y + 16.0, 45.0, 45.0),
                p.elevated_high, p.surface, ACCENTS[i].with_alpha(125), 12.0);
            icon(buf, APPS[i].0, ICONS[i],
                rect(card.min.x + 28.0, card.min.y + 27.0, 23.0, 23.0), ACCENTS[i]);
            text(buf, APPS[i].0, card.min.x + 17.0, card.min.y + 80.0, 17.0, p.text, true);
            text(buf, APPS[i].1, card.min.x + 17.0, card.min.y + 115.0, 11.0, p.text_muted, false);
            if selected {
                text(buf, "ACTIVE CARD", card.max.x - 102.0, card.min.y + 28.0,
                    9.0, ACCENTS[i], true);
            }
        }

        text(buf, "NATIVE UI PREVIEW", work.min.x, bounds.max.y - 36.0,
            10.0, p.text_muted, true);
        text(buf, "No agent or application service connected", work.min.x + 146.0,
            bounds.max.y - 36.0, 10.0, p.text_muted, false);

        if self.has_inspector(bounds) {
            let x = bounds.max.x - 324.0;
            let panel = rect(x, 101.0, 291.0, bounds.height() - 132.0);
            surface(buf, panel, p.elevated, p.surface, p.border, 16.0);
            text(buf, "WORKSPACE INSPECTOR", x + 20.0, 124.0, 11.0, p.text_muted, true);
            surface(buf, rect(x + 20.0, 173.0, 56.0, 56.0),
                p.elevated_high, p.surface, ACCENTS[self.selected].with_alpha(145), 14.0);
            icon(buf, APPS[self.selected].0, ICONS[self.selected],
                rect(x + 35.0, 188.0, 25.0, 25.0), ACCENTS[self.selected]);
            text(buf, APPS[self.selected].0, x + 20.0, 250.0, 23.0, p.text, true);
            text(buf, APPS[self.selected].1, x + 20.0, 287.0, 12.0, p.text_muted, false);
            surface(buf, rect(x + 19.0, 325.0, 253.0, 87.0),
                p.elevated_high, p.surface, p.border, 11.0);
            text(buf, "CURRENT SELECTION", x + 32.0, 342.0, 10.0, p.accent, true);
            text(buf, APPS[self.selected].0, x + 32.0, 369.0, 16.0, p.text, true);
            text(buf, "STATUS", x + 20.0, 451.0, 10.0, p.text_muted, true);
            text(buf, "No active session in this preview", x + 20.0, 481.0, 12.0, p.text, false);
            text(buf, "SERVICES", x + 20.0, 528.0, 10.0, p.text_muted, true);
            text(buf, "Not connected to Kolpona", x + 20.0, 557.0, 12.0, p.text, false);
        }
    }

    fn handle_mouse_down(&mut self, pos: Vec2, _button: PointerButton, ctx: &mut AppContext) -> bool {
        if Self::inspector_toggle(ctx.bounds()).contains(pos) {
            self.details_hidden = !self.details_hidden;
            return true;
        }
        for (idx, card) in self.card_rects(ctx.bounds()).iter().enumerate() {
            if card.contains(pos) {
                self.selected = idx;
                return true;
            }
        }
        false
    }
}

fn main() {
    if std::env::args().any(|arg| arg == "--native") {
        run_native(KolponaShell::default()).expect("Kolpona shell could not launch");
        return;
    }
    let mut runner = HeadlessAppRunner::new(KolponaShell::default());
    assert!(runner.step_frame() > 0);
    let card = runner.app.card_rects(runner.ctx.bounds())[3];
    runner.click_at(card.center(), PointerButton::Primary);
    assert_eq!(runner.app.selected, 3);
    let toggle = KolponaShell::inspector_toggle(runner.ctx.bounds());
    runner.click_at(toggle.center(), PointerButton::Primary);
    assert!(runner.app.details_hidden);
    assert!(runner.step_frame() > 0);
    println!("Kolpona native shell: layout and selection smoke test passed");
}
