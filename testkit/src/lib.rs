//! `ui-testkit`: Automated benchmarking, frame percentiles, startup timing, and screenshot capture.

pub mod benchmark;
pub mod screenshot;
pub mod timer;

pub use benchmark::{BenchmarkRecorder, FrameMetrics};
pub use screenshot::ScreenshotCapture;
pub use timer::StartupTimer;
