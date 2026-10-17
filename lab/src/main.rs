//! Kolpona UI Lab: Native 60-144 FPS Hardware Workbench & Stress Test Harness.

mod app;
mod benchmark;
mod tabs;
mod window;

pub use app::UiLabApp;
pub use tabs::LabTab;
pub use window::LabWindowApp;

use ui_platform::winit::event_loop::{ControlFlow, EventLoop};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let is_benchmark = args.iter().any(|a| a == "--benchmark" || a == "-b");

    let app = UiLabApp::new();

    if is_benchmark {
        benchmark::run_headless_benchmark(app, 1000);
        return;
    }

    // Launch native interactive OS window, falling back to headless benchmark if no display is available
    match EventLoop::new() {
        Ok(event_loop) => {
            event_loop.set_control_flow(ControlFlow::Wait);
            let mut lab_window = LabWindowApp::new(app);
            if let Err(e) = event_loop.run_app(&mut lab_window) {
                eprintln!("EventLoop exited with error: {e}");
            }
        }
        Err(e) => {
            eprintln!(
                "⚠️ No graphical display server found ({e}). Falling back to headless benchmark mode."
            );
            benchmark::run_headless_benchmark(app, 1000);
        }
    }
}
