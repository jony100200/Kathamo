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
pub use text::{fit as fit_text, measure as measure_text, measure_face, Face, Mask};

#[cfg(test)]
mod shader_validation {
    #[test]
    fn studio_shader_parses_and_validates() {
        let module = naga::front::wgsl::parse_str(include_str!("shader.wgsl"))
            .expect("studio shader must have valid WGSL syntax");
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        )
        .validate(&module)
        .expect("studio shader must be valid for wgpu");
    }
}
