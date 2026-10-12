//! Unified rendering backend interface and telemetry statistics.

use crate::command::CommandBuffer;
use ui_core::Color;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendKind {
    HardwareGpu,
    SoftwareCpuSafeMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct RenderStats {
    pub backend: Option<BackendKind>,
    /// Real GPU draw calls issued (one per scissor batch) or 1 for the CPU rasterizer.
    pub draw_calls: usize,
    pub primitives_count: usize,
    /// Triangles/vertices actually generated for the frame (GPU) or pixels touched (CPU).
    pub vertices: usize,
    /// CPU time spent tessellating/rasterizing and encoding the frame, in microseconds.
    pub frame_time_us: u64,
}

/// A rendering backend. Coordinates in draw commands are **logical** points;
/// `begin_frame` receives the **physical** pixel size of the target and the
/// scale factor maps between the two (DPI awareness).
pub trait RenderBackend: Send {
    fn kind(&self) -> BackendKind;

    /// Sets the logical→physical scale factor (1.0 = 96 DPI, 2.0 = HiDPI).
    fn set_scale_factor(&mut self, _scale: f32) {}

    fn begin_frame(&mut self, width: u32, height: u32, clear_color: Color);

    fn submit(&mut self, buffer: &CommandBuffer) -> RenderStats;

    /// Finishes the frame: for the GPU this executes the draw and presents to the
    /// swapchain (or offscreen target). Errors mean the frame was not shown.
    fn end_frame(&mut self) -> Result<RenderStats, String>;

    /// Recovers from GPU device loss or resets graphics pipelines.
    fn recover(&mut self) -> Result<(), String>;
}
