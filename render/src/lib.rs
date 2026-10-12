//! `ui-render`: Real `wgpu` GPU pipeline and safe-mode CPU fallback rasterizer,
//! sharing one Unicode text stack (rustybuzz + ab_glyph) and one SVG icon pipeline (resvg).

pub mod backend;
pub mod command;
pub mod cpu_backend;
pub mod cpu_font;
pub mod gpu_backend;
pub mod shapes;
pub mod soft_present;
pub mod text;
pub mod vector;

pub use backend::{BackendKind, RenderBackend, RenderStats};
pub use command::{CommandBuffer, DrawCommand};
pub use cpu_backend::CpuBackend;
pub use gpu_backend::{GpuBackend, SharedWindow};
pub use soft_present::SoftwarePresenter;
pub use text::{measure as measure_text, Mask};
