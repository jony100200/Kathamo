//! Renders a contact sheet of agent mascots (every mood, several agents, three
//! sizes, both studio themes) with the CPU backend and saves it as a PNG for review.
//!
//! Output: `$KATHAMO_MASCOT_PREVIEW` if set, else `D:/KSAppDev/_work/sprint/mascot_preview.png`
//! when that folder exists, else the system temp dir. A 2x (HiDPI) copy is written
//! next to it as `*_2x.png`.

use std::path::PathBuf;

use ui_core::{Color, Rect, Vec2};
use ui_platform::PointerState;
use ui_render::{CommandBuffer, CpuBackend, RenderBackend};
use ui_testkit::ScreenshotCapture;
use ui_themes::{ExtendedTokens, ThemeMode, ThemePalette};
use ui_widgets::studio::{Studio, Tone};
use ui_widgets::studio_parts::{agent_card, mascot, Mood};

const MOODS: [Mood; 5] = [
    Mood::Ready,
    Mood::Working,
    Mood::NeedsYou,
    Mood::Error,
    Mood::Sleeping,
];
const AGENTS: [&str; 8] = [
    "codex", "hermes", "kilo", "opencode", "qwen", "cline", "claude", "grok",
];
const SIZES: [f32; 3] = [32.0, 44.0, 72.0];
const PANEL_W: f32 = 1240.0;
const PANEL_H: f32 = 400.0;

fn output_path() -> PathBuf {
    if let Ok(p) = std::env::var("KATHAMO_MASCOT_PREVIEW") {
        return PathBuf::from(p);
    }
    let sprint = PathBuf::from("D:/KSAppDev/_work/sprint");
    let dir = if sprint.is_dir() {
        sprint
    } else {
        std::env::temp_dir()
    };
    dir.join("mascot_preview.png")
}

/// One theme panel: per size a row of the five moods, then eight agents at Ready.
fn panel(buf: &mut CommandBuffer, origin: Vec2, mode: ThemeMode, palette: ThemePalette) {
    // The pointer rests on the last 72 px mascot to show the hover reaction.
    let pointer = PointerState {
        position: origin + Vec2::new(PANEL_W - 112.0, 16.0 + 32.0 + 22.0 + 44.0 + 22.0 + 20.0),
        ..PointerState::default()
    };
    let mut s = Studio::new(buf, palette, ExtendedTokens::for_mode(mode), &pointer);
    s.fill(
        Rect::from_min_size(origin, Vec2::new(PANEL_W, PANEL_H)),
        palette.canvas,
        0.0,
    );
    let mut y = origin.y + 16.0;
    for size in SIZES {
        let mut x = origin.x + 16.0;
        for (i, mood) in MOODS.iter().enumerate() {
            let rect = Rect::from_min_size(Vec2::new(x, y), Vec2::splat(size));
            mascot(&mut s, rect, AGENTS[i], *mood, 0.9);
            x += size + 18.0;
        }
        x += 24.0;
        for name in AGENTS {
            let rect = Rect::from_min_size(Vec2::new(x, y), Vec2::splat(size));
            mascot(&mut s, rect, name, Mood::Ready, 0.9);
            x += size + 14.0;
        }
        y += size + 22.0;
    }
    let tones = [Tone::Good, Tone::Working, Tone::Warning, Tone::Bad];
    let labels = ["Ready", "Working", "Needs you", "Error"];
    for (i, name) in ["Codex", "Hermes", "Kilo", "OpenCode"].iter().enumerate() {
        let rect = Rect::from_min_size(
            Vec2::new(origin.x + 16.0 + i as f32 * 258.0, y),
            Vec2::new(246.0, 92.0),
        );
        agent_card(
            &mut s,
            rect,
            name,
            "local",
            labels[i],
            tones[i],
            "Waiting for a task",
        );
    }
}

fn render(scale: f32) -> (Vec<u32>, u32, u32) {
    let mut buf = CommandBuffer::default();
    panel(
        &mut buf,
        Vec2::ZERO,
        ThemeMode::MidnightGraphite,
        ThemePalette::midnight_graphite(),
    );
    panel(
        &mut buf,
        Vec2::new(0.0, PANEL_H),
        ThemeMode::WarmPaper,
        ThemePalette::warm_paper(),
    );
    let (w, h) = ((PANEL_W * scale) as u32, (PANEL_H * 2.0 * scale) as u32);
    let mut cpu = CpuBackend::new();
    cpu.set_scale_factor(scale);
    cpu.begin_frame(w, h, Color::BLACK);
    cpu.submit(&buf);
    cpu.end_frame().expect("cpu frame");
    (cpu.pixels().to_vec(), w, h)
}

#[test]
fn mascot_contact_sheet_renders_and_is_saved() {
    let (pixels, w, h) = render(1.0);
    // Real pixels: the mascot area must not be a flat canvas.
    let canvas = ThemePalette::midnight_graphite().canvas.to_u32() & 0xFF_FFFF;
    let row = 16 + 16; // middle of the first 32 px row
    let drawn = (0..w as usize)
        .filter(|x| pixels[row * w as usize + x] & 0xFF_FFFF != canvas)
        .count();
    assert!(drawn > 200, "only {drawn} mascot pixels on the first row");

    let path = output_path();
    ScreenshotCapture::save_png(&path, &pixels, w, h).expect("write preview");
    let (px2, w2, h2) = render(2.0);
    let path2 = path.with_file_name(format!(
        "{}_2x.png",
        path.file_stem().unwrap().to_string_lossy()
    ));
    ScreenshotCapture::save_png(&path2, &px2, w2, h2).expect("write 2x preview");
    println!("mascot preview: {}", path.display());
}
