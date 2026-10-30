//! Headless GPU snapshots: render any `KathamoApp` offscreen and save a PNG.
//!
//! `run_native` switches to this mode when `KATHAMO_SNAPSHOT=<file.png>` is set,
//! so every app can be screenshotted for visual review without a desktop session.
//! Motion is settled before capture so images are deterministic.

use std::path::Path;

use ui_render::{GpuBackend, RenderBackend};
use ui_testkit::ScreenshotCapture;

use crate::app::{HeadlessAppRunner, KathamoApp};

/// Frames rendered before capture, so lazily-built layouts and fonts settle.
const WARMUP_FRAMES: usize = 3;

/// Renders `app` on an offscreen GPU target and writes the frame to `path` as PNG.
pub fn snapshot_png<A: KathamoApp>(app: A, path: &Path) -> Result<(), String> {
    ui_core::motion::set_instant(true);
    let mut runner = HeadlessAppRunner::new(app);
    for _ in 0..WARMUP_FRAMES {
        runner.step_frame();
    }
    let (width, height) = runner.ctx.physical_size;
    let mut gpu = GpuBackend::new_headless(width, height)?;
    gpu.set_scale_factor(runner.ctx.scale);
    runner.app.prepare_gpu(&mut gpu);
    for (id, image) in &runner.ctx.images {
        gpu.upload_rgba8_image(*id, image.width, image.height, &image.rgba)?;
    }
    gpu.begin_frame(width, height, runner.ctx.theme.canvas);
    gpu.submit(&runner.ctx.cmd_buf);
    gpu.end_frame()?;
    let pixels = gpu.read_pixels()?;
    ScreenshotCapture::save_png(path, &pixels, width, height).map_err(|e| e.to_string())?;
    eprintln!(
        "[Kathamo] snapshot {}x{} on {} -> {}",
        width,
        height,
        gpu.adapter_summary(),
        path.display()
    );
    Ok(())
}
