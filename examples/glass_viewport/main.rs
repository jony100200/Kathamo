//! Kathamo Glass Viewport — real 3D under real frosted glass.
//!
//! The viewport is an actual wgpu 3D render (own pipeline, depth buffer, 4x MSAA,
//! lit meshes, emissive orbs, grid floor) shown through Kathamo's zero-copy external
//! texture path. Every panel floating over it is a `DrawCommand::Backdrop`: the frame
//! underneath is blurred live, so moving the camera moves what you see through the glass.
//!
//! Drag to orbit, scroll to zoom. Run:
//! `cargo run --release -p kathamo --example glass_viewport`

mod scene;

use std::time::Instant;

use kathamo::prelude::*;
use kathamo::widgets::tab_strip;
use scene::{Camera, FrameInput, Scene, Sky, OBJECTS};
use ui_core::motion;

const VIEWPORT_ID: u64 = 0x6C61_7373; // texture id of the 3D render
const CAMERAS: [&str; 4] = ["Orbit", "Front", "Top", "Side"];
const SKIES: [&str; 3] = ["Dusk", "Night", "Day"];

struct GlassViewport {
    scene: Option<Scene>,
    adapter: String,
    physical: (u32, u32),
    last_frame: Option<Instant>,
    frame_ms: f32,
    time: f32,
    target: Camera,
    camera_preset: usize,
    sky: usize,
    visible: [bool; 6],
    auto_rotate: bool,
    animate: bool,
    dragging: bool,
    toasts: NotificationHost,
}

/// Floating panel rectangles, shared by drawing and hit-testing.
struct Panels {
    title: Rect,
    cameras: Vec<Rect>,
    camera_strip: Rect,
    skies: Vec<Rect>,
    sky_strip: Rect,
    scene: Rect,
    view: Rect,
    stats: Rect,
}

impl Panels {
    fn new(b: Rect) -> Self {
        let probe = tab_strip::tab_rects(&CAMERAS, Vec2::ZERO, 30.0);
        let w = tab_strip::strip_bounds(&probe, Vec2::ZERO, 30.0).width();
        let cam_origin = Vec2::new(b.center().x - w * 0.5, b.min.y + 22.0);
        let cameras = tab_strip::tab_rects(&CAMERAS, cam_origin, 30.0);
        let probe = tab_strip::tab_rects(&SKIES, Vec2::ZERO, 30.0);
        let sw = tab_strip::strip_bounds(&probe, Vec2::ZERO, 30.0).width();
        let sky_origin = Vec2::new(b.max.x - sw - 20.0, b.min.y + 22.0);
        let skies = tab_strip::tab_rects(&SKIES, sky_origin, 30.0);
        Self {
            title: Rect::from_min_size(b.min + Vec2::new(20.0, 16.0), Vec2::new(262.0, 46.0)),
            camera_strip: tab_strip::strip_bounds(&cameras, cam_origin, 30.0),
            cameras,
            sky_strip: tab_strip::strip_bounds(&skies, sky_origin, 30.0),
            skies,
            scene: Rect::from_min_size(b.min + Vec2::new(20.0, 82.0), Vec2::new(262.0, 252.0)),
            view: Rect::from_min_size(Vec2::new(b.max.x - 282.0, b.min.y + 82.0), Vec2::new(262.0, 128.0)),
            stats: Rect::from_min_size(
                Vec2::new(b.center().x - 330.0, b.max.y - 58.0),
                Vec2::new(660.0, 38.0),
            ),
        }
    }

    fn contains(&self, p: Vec2) -> bool {
        [self.title, self.camera_strip, self.sky_strip, self.scene, self.view, self.stats]
            .iter()
            .any(|r| r.contains(p))
    }

    fn scene_row(&self, i: usize) -> Rect {
        Rect::from_min_size(
            self.scene.min + Vec2::new(16.0, 44.0 + i as f32 * 33.0),
            Vec2::new(self.scene.width() - 32.0, 26.0),
        )
    }

    fn view_row(&self, i: usize) -> Rect {
        Rect::from_min_size(
            self.view.min + Vec2::new(16.0, 44.0 + i as f32 * 36.0),
            Vec2::new(self.view.width() - 32.0, 26.0),
        )
    }
}

/// Frosted glass card: soft shadow, live blur of the 3D view, hairline rim.
fn glass(buf: &mut CommandBuffer, rect: Rect, radius: f32, palette: &ThemePalette) {
    let r = CornerRadius::same(radius);
    buf.push(DrawCommand::SoftShadow {
        rect,
        radius: r,
        color: Color::rgba(0, 0, 0, 90),
        offset: Vec2::new(0.0, 10.0),
        blur: 28.0,
        spread: 0.0,
    });
    buf.push(DrawCommand::Backdrop {
        rect,
        radius: r,
        tint: palette.surface.with_alpha(105),
    });
    buf.push(DrawCommand::Rect {
        rect,
        fill: Color::TRANSPARENT,
        radius: r,
        stroke: Stroke::new(1.0, Color::rgba(255, 255, 255, 34)),
    });
}

fn text(buf: &mut CommandBuffer, s: impl Into<String>, pos: Vec2, size: f32, color: Color, strong: bool) {
    buf.push(DrawCommand::Text {
        text: s.into(),
        pos,
        size,
        color,
        strong,
    });
}

impl GlassViewport {
    fn new() -> Self {
        let mut toasts = NotificationHost::new();
        toasts.info("Real 3D under real glass", "Drag to orbit · scroll to zoom · panels blur live");
        Self {
            scene: None,
            adapter: String::new(),
            physical: (1, 1),
            last_frame: None,
            frame_ms: 0.0,
            time: 0.0,
            target: Camera {
                yaw: 0.6,
                pitch: 0.32,
                distance: 9.5,
            },
            camera_preset: 0,
            sky: 0,
            visible: [true; 6],
            auto_rotate: true,
            animate: true,
            dragging: false,
            toasts,
        }
    }

    fn set_camera(&mut self, i: usize) {
        self.camera_preset = i;
        let (yaw, pitch) = match i {
            1 => (0.0, 0.12),
            2 => (self.target.yaw, 1.35),
            3 => (std::f32::consts::FRAC_PI_2, 0.1),
            _ => (self.target.yaw, 0.32),
        };
        self.target.yaw = yaw;
        self.target.pitch = pitch;
        self.auto_rotate = i == 0 && self.auto_rotate;
    }

    /// Camera follows its target on springs: presets and drags glide instead of jumping.
    fn camera(&self) -> Camera {
        Camera {
            yaw: motion::animate(0xCA01, self.target.yaw, motion::SLIDE * 1.6),
            pitch: motion::animate(0xCA02, self.target.pitch, motion::SLIDE * 1.6),
            distance: motion::animate(0xCA03, self.target.distance, motion::SLIDE),
        }
    }

    fn tick(&mut self) {
        let now = Instant::now();
        if let Some(last) = self.last_frame {
            let dt = (now - last).as_secs_f32().min(0.1);
            self.frame_ms = self.frame_ms * 0.9 + dt * 1000.0 * 0.1;
            if self.animate {
                self.time += dt;
            }
            if self.auto_rotate && !self.dragging {
                self.target.yaw += dt * 0.25;
            }
        }
        self.last_frame = Some(now);
        self.toasts.tick();
    }

    fn handle_clicks(&mut self, p: &Panels, ptr: &PointerState) {
        if !ptr.primary_clicked {
            return;
        }
        let pos = ptr.position;
        if let Some(i) = tab_strip::hit(&p.cameras, pos) {
            self.set_camera(i);
        }
        if let Some(i) = tab_strip::hit(&p.skies, pos) {
            self.sky = i;
        }
        for i in 0..OBJECTS.len() {
            if p.scene_row(i).contains(pos) {
                self.visible[i] = !self.visible[i];
            }
        }
        if p.view_row(0).contains(pos) {
            self.auto_rotate = !self.auto_rotate;
        }
        if p.view_row(1).contains(pos) {
            self.animate = !self.animate;
        }
    }

    fn draw_panels(&self, ctx: &mut AppContext, p: &Panels) {
        let pal = ctx.theme;
        let ptr = ctx.pointer.position;
        let buf = &mut ctx.cmd_buf;

        glass(buf, p.title, 12.0, &pal);
        text(buf, "Kathamo · Glass Viewport", p.title.min + Vec2::new(16.0, 8.0), 13.0, pal.text, true);
        text(buf, "real wgpu 3D · live backdrop blur", p.title.min + Vec2::new(16.0, 27.0), 10.5, pal.text_muted, false);

        glass(buf, p.camera_strip.expand(4.0), 11.0, &pal);
        tab_strip::render(buf, &CAMERAS, &p.cameras, p.camera_strip, self.camera_preset, ptr, &pal);
        glass(buf, p.sky_strip.expand(4.0), 11.0, &pal);
        tab_strip::render(buf, &SKIES, &p.skies, p.sky_strip, self.sky, ptr, &pal);

        glass(buf, p.scene, 14.0, &pal);
        text(buf, "SCENE", p.scene.min + Vec2::new(16.0, 16.0), 11.0, pal.text_muted, true);
        let tris = self.scene.as_ref().map_or([0; 6], |s| s.triangles);
        for (i, name) in OBJECTS.iter().enumerate() {
            let row = p.scene_row(i);
            let label = format!("{name}  ·  {} tris", tris[i]);
            ui_widgets::input::render_toggle(buf, self.visible[i], row.contains(ptr), &label, row, &pal);
        }

        glass(buf, p.view, 14.0, &pal);
        text(buf, "VIEW", p.view.min + Vec2::new(16.0, 16.0), 11.0, pal.text_muted, true);
        let rows = [("Auto-rotate camera", self.auto_rotate), ("Animate scene", self.animate)];
        for (i, (label, on)) in rows.into_iter().enumerate() {
            let row = p.view_row(i);
            ui_widgets::input::render_toggle(buf, on, row.contains(ptr), label, row, &pal);
        }

        glass(buf, p.stats, 12.0, &pal);
        let visible_tris: u32 = (0..6).filter(|&i| self.visible[i]).map(|i| tris[i]).sum();
        let mb = self.scene.as_ref().map_or(0, |s| s.target_bytes()) as f32 / (1024.0 * 1024.0);
        let live = self.auto_rotate || self.animate;
        let fps = if live && self.frame_ms > 0.0 {
            format!("{:.1} ms/frame", self.frame_ms)
        } else {
            "idle · 0 frames".to_string()
        };
        let stats = format!(
            "{fps}   ·   {visible_tris} triangles   ·   {}×{} 4× MSAA ({mb:.0} MB)   ·   {}",
            self.physical.0, self.physical.1, self.adapter
        );
        let stats = ui_render::fit_text(&stats, 11.0, false, p.stats.width() - 32.0);
        text(buf, stats, p.stats.min + Vec2::new(16.0, 12.0), 11.0, pal.text, false);

        let screen = ctx.bounds();
        self.toasts.render(&mut ctx.cmd_buf, screen, &pal);
    }
}

impl KathamoApp for GlassViewport {
    fn title(&self) -> &str {
        "Kathamo Glass Viewport"
    }

    fn default_size(&self) -> (u32, u32) {
        (1440, 900)
    }

    fn update(&mut self, ctx: &mut AppContext) {
        self.tick();
        self.physical = ctx.physical_size;
        let bounds = ctx.bounds();
        let panels = Panels::new(bounds);
        let ptr = ctx.pointer.clone();
        self.handle_clicks(&panels, &ptr);
        ctx.cmd_buf.push(DrawCommand::Image {
            rect: bounds,
            texture_id: VIEWPORT_ID,
            uv_rect: Rect::from_min_max(Vec2::ZERO, Vec2::new(1.0, 1.0)),
            tint: Color::WHITE,
        });
        self.draw_panels(ctx, &panels);
    }

    fn prepare_gpu(&mut self, gpu: &mut GpuBackend) {
        if self.scene.is_none() {
            self.scene = Some(Scene::new(gpu.device()));
            self.adapter = gpu.adapter_summary();
        }
        let frame = FrameInput {
            time: self.time,
            camera: self.camera(),
            sky: [Sky::Dusk, Sky::Night, Sky::Day][self.sky],
            visible: self.visible,
        };
        let Some(scene) = self.scene.as_mut() else { return };
        if let Some(view) = scene.ensure_size(gpu.device(), self.physical) {
            gpu.register_external_view(VIEWPORT_ID, view);
        }
        scene.render(gpu.device(), gpu.queue(), &frame);
    }

    fn handle_mouse_down(&mut self, pos: Vec2, button: PointerButton, ctx: &mut AppContext) -> bool {
        self.dragging = button == PointerButton::Primary && !Panels::new(ctx.bounds()).contains(pos);
        true
    }

    fn handle_mouse_move(&mut self, _pos: Vec2, delta: Vec2, ctx: &mut AppContext) -> bool {
        if self.dragging && ctx.pointer.is_down() {
            self.camera_preset = 0;
            self.target.yaw -= delta.x * 0.008;
            self.target.pitch = (self.target.pitch + delta.y * 0.006).clamp(-0.05, 1.45);
        }
        true
    }

    fn handle_mouse_up(&mut self, _pos: Vec2, _button: PointerButton, _ctx: &mut AppContext) -> bool {
        self.dragging = false;
        true
    }

    fn handle_scroll(&mut self, delta: Vec2, _pos: Vec2, _ctx: &mut AppContext) -> bool {
        self.target.distance = (self.target.distance * (1.0 - delta.y.clamp(-3.0, 3.0) * 0.08)).clamp(4.0, 22.0);
        true
    }

    fn wants_continuous_redraw(&self) -> bool {
        self.auto_rotate || self.animate
    }
}

fn main() {
    if let Err(e) = run_native(GlassViewport::new()) {
        eprintln!("Kathamo Glass Viewport failed: {e}");
    }
}
