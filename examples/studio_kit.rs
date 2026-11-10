//! Studio kit tour: the project hub, a file drop card, progress rows, a view badge, an axis
//! gizmo, a tooltip with a shortcut and the role type scale.
//! cargo run -p kathamo --example studio_kit -- --native
//! cargo run -p kathamo --example studio_kit   (headless check)

use kathamo::prelude::*;
use kathamo::themes::{ExtendedTokens, ThemeMode};
use kathamo::widgets::studio::glyphs;
use kathamo::widgets::{
    axis_gizmo, project_hub, view_badge, ButtonKind, Corner, DropCard, HubAction, HubProject,
    HubStart, ProjectHub, Studio, TypeRole, AXIS_VIEWS,
};

#[derive(Default)]
struct KitTour {
    /// 0 = hub, 1 = widgets.
    page: usize,
    yaw: f32,
    pitch: f32,
    files_over: bool,
    log: String,
}

const STARTS: [HubStart; 2] = [
    HubStart {
        glyph: glyphs::SPHERE,
        title: "Make a thing",
        body: "Start from a picture, a preset or a blank page.",
        actions: &["From a picture", "From scratch"],
    },
    HubStart {
        glyph: glyphs::BRUSH,
        title: "Paint a thing",
        body: "Open a model and paint straight onto it.",
        actions: &["Open a model"],
    },
];

impl KitTour {
    fn hub(&mut self, s: &mut Studio, rect: Rect) {
        let recent = [
            HubProject {
                title: "Courtyard",
                caption: "Today",
                badge: Some("Make"),
                cover: None,
            },
            HubProject {
                title: "Old crate",
                caption: "3 days ago",
                badge: Some("Paint"),
                cover: None,
            },
        ];
        let hub = ProjectHub {
            title: "Studio Kit",
            tagline: "Everything on this page is a reusable Kathamo widget.",
            starts: &STARTS,
            quick: &[
                (glyphs::FOLDER_PLUS, "New project"),
                (glyphs::FOLDER_OPEN, "Open"),
            ],
            recent: &recent,
            recent_empty: "No projects yet.",
        };
        match project_hub(s, rect, &hub) {
            Some(HubAction::Start { card: 0, .. }) => self.page = 1,
            Some(action) => self.log = format!("{action:?}"),
            None => {}
        }
    }

    fn widgets(&mut self, s: &mut Studio, rect: Rect) {
        let p = s.palette;
        s.fill(rect, p.canvas, 0.0);
        let mut y = rect.min.y + 24.0;
        for role in TypeRole::ALL {
            let style = role.style();
            style.draw(
                s.buf,
                &format!("{role:?} 0123456789"),
                Vec2::new(24.0, y),
                p.text,
            );
            y += style.size + 12.0;
        }
        let card = Rect::from_min_size(Vec2::new(24.0, y + 16.0), Vec2::new(520.0, 360.0));
        let out = s.drop_card(
            card,
            DropCard {
                glyph: glyphs::PHOTO_PLUS,
                title: "Drop an image",
                body: "Or browse for one. A sample works too.",
                browse: "Browse",
                secondary: Some("Try a sample"),
                hovering_files: self.files_over,
            },
        );
        if out.browse || out.secondary {
            self.log = format!("{out:?}");
        }
        let col = Rect::from_min_size(Vec2::new(580.0, rect.min.y + 24.0), Vec2::new(360.0, 24.0));
        s.progress_row(col, "Rendering", Some(0.4), "26 / 64 spp");
        let row2 = Rect::from_min_size(col.min + Vec2::new(0.0, 40.0), col.size());
        s.progress_row(row2, "Scanning folders", None, "");
        let view = Rect::from_min_size(Vec2::new(580.0, 120.0), Vec2::new(420.0, 300.0));
        s.surface(view, 0, 8.0);
        view_badge(s.buf, view, Corner::TopRight, "64 spp · GPU", true, &p);
        let centre = Vec2::new(view.min.x + 56.0, view.min.y + 56.0);
        let over = axis_gizmo(
            s.buf,
            centre,
            40.0,
            self.yaw,
            self.pitch,
            s.pointer.position,
            &p,
        );
        if let (Some(i), true) = (over, s.pointer.primary_clicked) {
            self.yaw = AXIS_VIEWS[i].yaw;
            self.pitch = AXIS_VIEWS[i].pitch;
        }
        let back = Rect::from_min_size(Vec2::new(580.0, 440.0), Vec2::new(120.0, 36.0));
        if s.button_with_icon(back, Some(glyphs::HOME), "Hub", ButtonKind::Secondary)
            .clicked
        {
            self.page = 0;
        }
        s.tooltip(back, "Back to the hub", Some("Esc"), rect);
    }
}

impl KathamoApp for KitTour {
    fn title(&self) -> &str {
        "Kathamo studio kit"
    }

    fn update(&mut self, ctx: &mut AppContext) {
        ctx.theme = ThemePalette::midnight_graphite();
        let rect = ctx.bounds();
        let tokens = ExtendedTokens::for_mode(ThemeMode::MidnightGraphite);
        let pointer = ctx.pointer.clone();
        let mut s = Studio::new(&mut ctx.cmd_buf, ctx.theme, tokens, &pointer);
        if self.page == 0 {
            self.hub(&mut s, rect);
        } else {
            self.widgets(&mut s, rect);
        }
    }

    fn handle_files_hovered(&mut self, _paths: Vec<std::path::PathBuf>, _ctx: &mut AppContext) {
        self.files_over = true;
    }

    fn handle_files_hover_cancelled(&mut self, _ctx: &mut AppContext) {
        self.files_over = false;
    }

    fn handle_files_dropped(
        &mut self,
        paths: Vec<std::path::PathBuf>,
        _ctx: &mut AppContext,
    ) -> bool {
        self.files_over = false;
        self.log = format!("dropped {}", paths.len());
        true
    }
}

fn main() {
    if std::env::args().any(|arg| arg == "--native") {
        run_native(KitTour {
            yaw: 0.6,
            pitch: 0.4,
            ..KitTour::default()
        })
        .expect("studio kit example failed");
        return;
    }
    let mut runner = HeadlessAppRunner::new(KitTour::default());
    assert!(runner.step_frame() > 0);
    runner.app.page = 1;
    assert!(runner.step_frame() > 0);
    assert!(runner.drop_files(&["photo.png".into()]));
    assert_eq!(runner.app.log, "dropped 1");
    println!("studio kit: hub and widget pages render, drops arrive");
}
