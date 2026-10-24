//! Headless benchmarking, automated performance gate acceptance, and visual QA capture.

use crate::app::UiLabApp;
use crate::tabs::LabTab;
use ui_core::*;
use ui_render::BackendKind;
use ui_testkit::{FrameMetrics, ScreenshotCapture};

impl UiLabApp {
    /// Automated stress benchmark mode: renders frames under heavy load and reports statistics.
    pub fn run_benchmark_suite(&mut self, viewport: Rect, num_frames: usize) -> FrameMetrics {
        println!(
            "⚡ RUNNING UI LAB BENCHMARK SUITE ({} frames)...",
            num_frames
        );
        self.benchmark.clear();

        // Cycle through all tabs during benchmark to measure all workloads
        let tabs = [
            LabTab::CreativeWorkspace,
            LabTab::OjonWorkstation,
            LabTab::DockingAndOverview,
            LabTab::DesignSystemAndIcons,
            LabTab::GodotAnchorsAndMaterial3,
            LabTab::InfiniteCanvas,
            LabTab::NodeEditor,
            LabTab::Timeline,
            LabTab::VirtualizedList100k,
        ];

        for i in 0..num_frames {
            self.current_tab = tabs[i % tabs.len()];
            if self.current_tab == LabTab::VirtualizedList100k {
                // Exercise Android kinetic fling physics on 100k list
                if i % 40 == 0 {
                    self.virtual_list.fling(800.0);
                }
                self.virtual_list.tick_physics(viewport.height());
            } else if self.current_tab == LabTab::InfiniteCanvas {
                self.canvas.pan_by(Vec2::new(1.5, 0.5));
            }
            let _ = self.render_frame(viewport);
        }

        self.benchmark.compute_metrics()
    }
}

/// Automated stress benchmark and visual QA execution.
pub fn run_headless_benchmark(mut app: UiLabApp, num_frames: usize) {
    let viewport = Rect::from_min_size(Vec2::ZERO, Vec2::new(1440.0, 900.0));

    // Initialize headless GPU specifically for benchmark execution
    app.init_headless_gpu(1440, 900);

    // Render warm initial frame on Design System & Icons tab into CPU rasterizer for visual QA
    app.current_tab = LabTab::DesignSystemAndIcons;
    app.active_backend = BackendKind::SoftwareCpuSafeMode;
    let stats = app
        .render_frame(viewport)
        .expect("CPU warm visual QA frame must render successfully");
    println!(
        "   Rendered Frame #1 (Design System): {} commands, Backend: {:?}",
        stats.primitives_count, app.active_backend
    );

    let appdata = std::env::var("USERPROFILE").unwrap_or_else(|_| ".".into());
    let screenshot_path = format!(
        "{}/.gemini/antigravity-ide/brain/dae2600e-869f-431f-ad94-20f29860011c/ui_lab_frame.ppm",
        appdata
    );
    let _ = ScreenshotCapture::save_ppm(
        &screenshot_path,
        app.cpu.pixels(),
        app.cpu.width(),
        app.cpu.height(),
    );
    println!("   Visual QA Frame Saved: {}", screenshot_path);

    // Render Ojon Workstation tab into CPU rasterizer for visual QA
    app.current_tab = LabTab::OjonWorkstation;
    let ojon_stats = app
        .render_frame(viewport)
        .expect("CPU warm visual QA frame for Ojon must render successfully");
    println!(
        "   Rendered Frame #2 (Ojon Studio): {} commands, Backend: {:?}",
        ojon_stats.primitives_count, app.active_backend
    );

    let ojon_screenshot_path = format!(
        "{}/.gemini/antigravity-ide/brain/dae2600e-869f-431f-ad94-20f29860011c/ojon_workstation_frame.ppm",
        appdata
    );
    let _ = ScreenshotCapture::save_ppm(
        &ojon_screenshot_path,
        app.cpu.pixels(),
        app.cpu.width(),
        app.cpu.height(),
    );
    println!("   Ojon Workstation Visual QA Frame Saved: {}", ojon_screenshot_path);

    // Run benchmark suite on Hardware GPU if available, or CPU safe mode
    if app.gpu.is_some() {
        app.active_backend = BackendKind::HardwareGpu;
    } else {
        println!("ℹ️ Running benchmark suite on CPU Safe Mode (no GPU adapter present)");
        app.active_backend = BackendKind::SoftwareCpuSafeMode;
    }
    let metrics = app.run_benchmark_suite(viewport, num_frames);
    println!("------------------------------------------------------------");
    println!(
        "📊 PERFORMANCE ACCEPTANCE REPORT ({} Frames @ 1440x900)",
        num_frames
    );
    println!("   p50 Frame Time:   {:.2} ms", metrics.p50_ms);
    println!("   p90 Frame Time:   {:.2} ms", metrics.p90_ms);
    println!(
        "   p95 Frame Time:   {:.2} ms (Target Gate: <=16.7ms)",
        metrics.p95_ms
    );
    println!("   p99 Frame Time:   {:.2} ms", metrics.p99_ms);
    println!("   Max Frame Time:   {:.2} ms", metrics.max_ms);
    println!("   Stalls (>16.7ms): {}", metrics.stalls_over_16ms);
    println!("   Stalls (>33.3ms): {}", metrics.stalls_over_33ms);
    println!(
        "   Gate Verdict:     {}",
        if metrics.p95_ms <= 16.7 {
            "✅ PASS (SOLID 60Hz BUDGET)"
        } else {
            "❌ FAIL"
        }
    );
    println!("------------------------------------------------------------");

    // Test GPU Device Loss Recovery explicitly if GPU is present
    if let Some(gpu) = &mut app.gpu {
        println!("🧪 TESTING GPU DEVICE LOSS RECOVERY & CPU SAFE-MODE FALLBACK...");
        app.active_backend = BackendKind::HardwareGpu;
        gpu.simulate_device_loss();
        let _ = app.render_frame(viewport);
        println!(
            "   Device Loss Simulated -> Fallback Activated: {:?}",
            app.active_backend
        );
        assert_eq!(app.active_backend, BackendKind::SoftwareCpuSafeMode);
        println!("   GPU Recovery Verdict: ✅ PASS (Seamless fallback)");
        println!("------------------------------------------------------------");
    }
}
