//! Kathamo on a phone. One screen that exercises what a mobile app needs:
//! taps (counter button, switch), drags (slider), finger scrolling (list),
//! Bengali shaping, and the GPU/CPU renderer report.
//!
//! Android: `tools/android/build_apk.ps1` builds the APK around `android_main`.
//! Desktop preview: `cargo run -p kathamo-android-demo` (phone-sized window).

use kathamo::prelude::*;

const ROWS: usize = 40;
const ROW_H: f32 = 56.0;

pub struct MobileDemo {
    pub taps: u32,
    pub switch_on: bool,
    /// Slider position, 0..=1.
    pub level: f32,
    /// List scroll offset in logical px (0 = top).
    pub scroll: f32,
    pub renderer: String,
    dragging_slider: bool,
}

impl Default for MobileDemo {
    fn default() -> Self {
        Self {
            taps: 0,
            switch_on: true,
            level: 0.4,
            scroll: 0.0,
            renderer: "CPU renderer".into(),
            dragging_slider: false,
        }
    }
}

fn rect(x: f32, y: f32, w: f32, h: f32) -> Rect {
    Rect::from_min_size(Vec2::new(x, y), Vec2::new(w.max(1.0), h.max(1.0)))
}

fn text(buf: &mut CommandBuffer, value: &str, x: f32, y: f32, size: f32, color: Color, strong: bool) {
    buf.push(DrawCommand::Text { text: value.to_owned(), pos: Vec2::new(x, y), size, color, strong });
}

fn card(buf: &mut CommandBuffer, r: Rect, top: Color, bottom: Color, edge: Color, radius: f32) {
    buf.push(DrawCommand::Surface {
        rect: r,
        fill_top: top,
        fill_bottom: bottom,
        radius: CornerRadius::same(radius),
        stroke: Stroke::new(1.0, edge),
        rim_strength: 0.55,
    });
}

/// Screen regions, computed from the safe area (`ctx.safe_bounds()`) so the layout
/// fits any phone and stays clear of the status bar and gesture bar.
pub struct Layout {
    pub button: Rect,
    pub switch: Rect,
    pub slider: Rect,
    pub list: Rect,
}

impl Layout {
    pub fn of(bounds: Rect) -> Self {
        let pad = 16.0;
        let (x, y0) = (bounds.min.x + pad, bounds.min.y);
        let w = bounds.width() - pad * 2.0;
        let button = rect(x, y0 + 168.0, w, 64.0);
        let switch = rect(x, y0 + 248.0, w, 52.0);
        let slider = rect(x, y0 + 316.0, w, 60.0);
        let top = y0 + 392.0;
        let list = rect(x, top, w, (bounds.max.y - top - pad).max(ROW_H));
        Self { button, switch, slider, list }
    }
}

impl MobileDemo {
    fn max_scroll(list: Rect) -> f32 {
        (ROWS as f32 * ROW_H - list.height()).max(0.0)
    }

    fn set_level(&mut self, slider: Rect, x: f32) {
        let track = slider.width() - 32.0;
        self.level = ((x - slider.min.x - 16.0) / track).clamp(0.0, 1.0);
    }
}

impl KathamoApp for MobileDemo {
    fn title(&self) -> &str {
        "Kathamo"
    }

    fn default_size(&self) -> (u32, u32) {
        (412, 892)
    }

    fn prepare_gpu(&mut self, gpu: &mut GpuBackend) {
        self.renderer = gpu.adapter_summary();
    }

    fn update(&mut self, ctx: &mut AppContext) {
        let bounds = ctx.safe_bounds();
        let p = ctx.theme;
        let l = Layout::of(bounds);
        let (pw, ph) = ctx.physical_size;
        let scale = ctx.scale;
        let inset_top = bounds.min.y - ctx.bounds().min.y;
        let buf = &mut ctx.cmd_buf;

        let (x, y) = (bounds.min.x + 16.0, bounds.min.y);
        text(buf, "Kathamo", x, y + 20.0, 30.0, p.text, true);
        text(buf, "Native Rust UI, running on this device", x, y + 62.0, 13.0, p.text_muted, false);
        text(buf, "কথামো: বাংলা লেখা ঠিকঠাক দেখাচ্ছে", x, y + 86.0, 15.0, p.accent, false);
        text(buf, &self.renderer, x, y + 116.0, 11.0, p.text_muted, false);
        let info = format!("{pw} x {ph} px, scale {scale:.2}, safe inset top {inset_top:.0}");
        text(buf, &info, x, y + 136.0, 11.0, p.text_muted, false);

        let b = l.button;
        card(buf, b, p.accent, p.accent.with_alpha(200), p.accent_hover, 16.0);
        let label = format!("Tapped {} time{}", self.taps, if self.taps == 1 { "" } else { "s" });
        text(buf, &label, b.min.x + 20.0, b.min.y + 20.0, 18.0, p.canvas, true);

        let s = l.switch;
        card(buf, s, p.elevated, p.surface, p.border, 14.0);
        text(buf, "Switch", s.min.x + 16.0, s.min.y + 16.0, 15.0, p.text, false);
        let knob_track = rect(s.max.x - 70.0, s.min.y + 12.0, 54.0, 28.0);
        let on = self.switch_on;
        buf.push(DrawCommand::Rect {
            rect: knob_track,
            fill: if on { p.accent } else { p.border },
            radius: CornerRadius::same(14.0),
            stroke: Stroke::new(0.0, p.border),
        });
        let kx = if on { knob_track.max.x - 26.0 } else { knob_track.min.x + 2.0 };
        buf.push(DrawCommand::Rect {
            rect: rect(kx, knob_track.min.y + 2.0, 24.0, 24.0),
            fill: p.text,
            radius: CornerRadius::same(12.0),
            stroke: Stroke::new(0.0, p.border),
        });

        let sl = l.slider;
        card(buf, sl, p.elevated, p.surface, p.border, 14.0);
        text(buf, &format!("Level {:.0}%", self.level * 100.0), sl.min.x + 16.0, sl.min.y + 8.0, 13.0, p.text, false);
        let track = rect(sl.min.x + 16.0, sl.min.y + 38.0, sl.width() - 32.0, 6.0);
        buf.push(DrawCommand::Rect { rect: track, fill: p.border, radius: CornerRadius::same(3.0), stroke: Stroke::new(0.0, p.border) });
        let fill_w = track.width() * self.level;
        buf.push(DrawCommand::Rect { rect: rect(track.min.x, track.min.y, fill_w, 6.0), fill: p.accent, radius: CornerRadius::same(3.0), stroke: Stroke::new(0.0, p.border) });
        buf.push(DrawCommand::Rect {
            rect: rect(track.min.x + fill_w - 11.0, track.min.y - 8.0, 22.0, 22.0),
            fill: p.text,
            radius: CornerRadius::same(11.0),
            stroke: Stroke::new(0.0, p.border),
        });

        let list = l.list;
        card(buf, list, p.surface, p.surface, p.border, 14.0);
        buf.push(DrawCommand::ClipPush { rect: list });
        for i in 0..ROWS {
            let y = list.min.y + i as f32 * ROW_H - self.scroll;
            if y + ROW_H < list.min.y || y > list.max.y {
                continue;
            }
            if i > 0 {
                buf.push(DrawCommand::Line {
                    start: Vec2::new(list.min.x + 16.0, y),
                    end: Vec2::new(list.max.x - 16.0, y),
                    stroke: Stroke::new(1.0, p.border.with_alpha(120)),
                });
            }
            text(buf, &format!("Row {:02}", i + 1), list.min.x + 16.0, y + 12.0, 15.0, p.text, true);
            text(buf, "Drag with one finger to scroll", list.min.x + 16.0, y + 33.0, 11.0, p.text_muted, false);
        }
        buf.push(DrawCommand::ClipPop);
    }

    fn handle_mouse_down(&mut self, pos: Vec2, _b: PointerButton, ctx: &mut AppContext) -> bool {
        let l = Layout::of(ctx.safe_bounds());
        if l.button.contains(pos) {
            self.taps += 1;
            return true;
        }
        if l.switch.contains(pos) {
            self.switch_on = !self.switch_on;
            return true;
        }
        if l.slider.contains(pos) {
            self.dragging_slider = true;
            self.set_level(l.slider, pos.x);
            return true;
        }
        false
    }

    fn handle_mouse_move(&mut self, pos: Vec2, _delta: Vec2, ctx: &mut AppContext) -> bool {
        if self.dragging_slider {
            self.set_level(Layout::of(ctx.safe_bounds()).slider, pos.x);
            return true;
        }
        false
    }

    fn handle_mouse_up(&mut self, _pos: Vec2, _b: PointerButton, _ctx: &mut AppContext) -> bool {
        std::mem::take(&mut self.dragging_slider)
    }

    fn handle_scroll(&mut self, delta: Vec2, pos: Vec2, ctx: &mut AppContext) -> bool {
        let list = Layout::of(ctx.safe_bounds()).list;
        if !list.contains(pos) {
            return false;
        }
        // One scroll line = 20 logical px, matching the touch mapping in the runner.
        self.scroll = (self.scroll - delta.y * 20.0).clamp(0.0, Self::max_scroll(list));
        true
    }
}

#[cfg(target_os = "android")]
#[no_mangle]
fn android_main(app: AndroidApp) {
    if let Err(error) = run_android(MobileDemo::default(), app) {
        eprintln!("Kathamo demo stopped: {error}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn taps_switch_and_slider_respond() {
        let mut r = HeadlessAppRunner::new(MobileDemo::default());
        r.step_frame();
        let l = Layout::of(r.ctx.safe_bounds());
        r.click_at(l.button.center(), PointerButton::Primary);
        r.click_at(l.button.center(), PointerButton::Primary);
        assert_eq!(r.app.taps, 2);
        r.click_at(l.switch.center(), PointerButton::Primary);
        assert!(!r.app.switch_on);
        r.click_at(Vec2::new(l.slider.max.x - 16.0, l.slider.center().y), PointerButton::Primary);
        assert!((r.app.level - 1.0).abs() < 1e-3);
    }

    #[test]
    fn list_scroll_follows_finger_and_stops_at_ends() {
        let mut app = MobileDemo::default();
        let mut ctx = AppContext::new(412, 892, 1.0);
        let inside = Layout::of(ctx.safe_bounds()).list.center();
        // Finger moves up 100 px -> runner sends -5 lines -> content scrolls down 100 px.
        assert!(app.handle_scroll(Vec2::new(0.0, -5.0), inside, &mut ctx));
        assert_eq!(app.scroll, 100.0);
        app.handle_scroll(Vec2::new(0.0, 50.0), inside, &mut ctx);
        assert_eq!(app.scroll, 0.0);
        app.handle_scroll(Vec2::new(0.0, -10_000.0), inside, &mut ctx);
        assert_eq!(app.scroll, MobileDemo::max_scroll(Layout::of(ctx.safe_bounds()).list));
    }

    #[test]
    fn layout_moves_down_with_the_status_bar_and_stops_above_the_gesture_bar() {
        let mut ctx = AppContext::new(412, 892, 1.0);
        let plain = Layout::of(ctx.safe_bounds());
        ctx.safe_area = Rect::from_min_max(Vec2::new(0.0, 48.0), Vec2::new(412.0, 868.0));
        let inset = Layout::of(ctx.safe_bounds());
        assert_eq!(inset.button.min.y, plain.button.min.y + 48.0);
        assert_eq!(inset.list.max.y, 868.0 - 16.0);
        assert!(inset.list.height() >= ROW_H);
    }
}
