//! Hardware GPU rendering backend built on `wgpu` (DX12 / Vulkan / Metal / GL).
//!
//! This is a real renderer, not a command counter:
//! * owns a `wgpu::Instance`, `Adapter`, `Device` and `Queue`;
//! * renders either into a window **surface/swapchain** (interactive) or into an
//!   **offscreen texture** (headless benchmarks, CI, screenshot comparison);
//! * tessellates every `DrawCommand` into vertices on the CPU, uploads them, and
//!   draws them with one pipeline (`shader.wgsl`) in scissor-clipped batches;
//! * rasterizes shaped glyphs and SVG icons once into an R8 atlas texture;
//! * detects genuine device loss (driver reset, `Device::destroy`, validation
//!   failure) and recreates adapter, device, pipeline and surface in `recover()`.

use crate::backend::{BackendKind, RenderBackend, RenderStats};
use crate::command::{CommandBuffer, DrawCommand};
use crate::shapes::flatten_cubic;
use crate::text::{self, GlyphKey, Mask};
use crate::vector::{IconKey, IconRasterCache};
use bytemuck::{Pod, Zeroable};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;
use ui_core::{Color, CornerRadius, Rect, Stroke, Vec2};

/// Any OS window usable as a render target (e.g. `Arc<winit::window::Window>`).
pub type SharedWindow = Arc<dyn wgpu::WindowHandle>;

const ATLAS_SIZE: u32 = 2048;

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct Vertex {
    pos: [f32; 2],
    shape: [f32; 4],
    radii: [f32; 4],
    color: [f32; 4],
    color2: [f32; 4],
    extra: [f32; 2],
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct Globals {
    viewport: [f32; 2],
    aa: f32,
    srgb_target: f32,
}

#[derive(Debug, Clone, Copy)]
struct Batch {
    scissor: [u32; 4],
    start: u32,
    end: u32,
}

#[derive(Debug, Clone, Copy)]
struct Slot {
    x: u32,
    y: u32,
    w: u32,
    h: u32,
    left: i32,
    top: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum AtlasKey {
    Glyph(GlyphKey),
    Icon(IconKey),
}

/// Simple shelf packer for the glyph/icon atlas.
#[derive(Debug, Default)]
struct ShelfPacker {
    x: u32,
    y: u32,
    row_h: u32,
}

impl ShelfPacker {
    fn alloc(&mut self, w: u32, h: u32) -> Option<(u32, u32)> {
        let (w, h) = (w + 1, h + 1); // 1px gutter against sampling bleed
        if w > ATLAS_SIZE || h > ATLAS_SIZE {
            return None;
        }
        if self.x + w > ATLAS_SIZE {
            self.x = 0;
            self.y += self.row_h;
            self.row_h = 0;
        }
        if self.y + h > ATLAS_SIZE {
            return None;
        }
        let pos = (self.x, self.y);
        self.x += w;
        self.row_h = self.row_h.max(h);
        Some(pos)
    }
}

enum Target {
    Surface {
        surface: wgpu::Surface<'static>,
        config: wgpu::SurfaceConfiguration,
    },
    Offscreen {
        texture: wgpu::Texture,
        view: wgpu::TextureView,
    },
}

/// Device-dependent GPU objects; rebuilt wholesale on recovery.
struct DeviceState {
    adapter_info: wgpu::AdapterInfo,
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    uniform_buf: wgpu::Buffer,
    atlas_tex: wgpu::Texture,
    vbuf: wgpu::Buffer,
    ibuf: wgpu::Buffer,
    format: wgpu::TextureFormat,
}

pub struct GpuBackend {
    instance: wgpu::Instance,
    window: Option<SharedWindow>,
    target: Option<Target>,
    dev: DeviceState,
    lost: Arc<AtomicBool>,
    vsync: bool,

    width: u32,
    height: u32,
    scale: f32,
    clear_color: Color,

    vertices: Vec<Vertex>,
    indices: Vec<u32>,
    batches: Vec<Batch>,
    clip_stack: Vec<Rect>,

    packer: ShelfPacker,
    slots: HashMap<AtlasKey, Option<Slot>>,
    uploads: Vec<(Slot, Arc<Mask>)>,
    atlas_full: bool,
    icons: IconRasterCache,

    stats: RenderStats,
}

fn create_instance() -> wgpu::Instance {
    wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::util::backend_bits_from_env()
            .unwrap_or(wgpu::Backends::PRIMARY | wgpu::Backends::GL),
        ..Default::default()
    })
}

fn color_f32(c: Color) -> [f32; 4] {
    c.to_f32_rgba()
}

fn srgb_to_linear(v: f64) -> f64 {
    if v <= 0.04045 {
        v / 12.92
    } else {
        ((v + 0.055) / 1.055).powf(2.4)
    }
}

impl DeviceState {
    fn create(
        instance: &wgpu::Instance,
        surface: Option<&wgpu::Surface<'static>>,
        lost: &Arc<AtomicBool>,
    ) -> Result<Self, String> {
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: false,
            compatible_surface: surface,
        }))
        .or_else(|| {
            pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::LowPower,
                force_fallback_adapter: true,
                compatible_surface: surface,
            }))
        })
        .ok_or_else(|| "no compatible GPU adapter found".to_string())?;

        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("kolpona-ui-device"),
                required_features: wgpu::Features::empty(),
                required_limits:
                    wgpu::Limits::downlevel_webgl2_defaults().using_resolution(adapter.limits()),
                memory_hints: wgpu::MemoryHints::MemoryUsage,
            },
            None,
        ))
        .map_err(|e| format!("request_device failed: {e}"))?;

        // Never panic on GPU errors: flag the device as lost so the app can fall back.
        let flag = lost.clone();
        device.on_uncaptured_error(Box::new(move |e| {
            eprintln!("[kolpona-gpu] uncaptured GPU error: {e}");
            flag.store(true, Ordering::SeqCst);
        }));
        let flag = lost.clone();
        device.set_device_lost_callback(move |reason, msg| {
            eprintln!("[kolpona-gpu] device lost ({reason:?}): {msg}");
            flag.store(true, Ordering::SeqCst);
        });

        let format = match surface {
            Some(s) => {
                let caps = s.get_capabilities(&adapter);
                caps.formats
                    .iter()
                    .copied()
                    .find(|f| !f.is_srgb())
                    .or_else(|| caps.formats.first().copied())
                    .ok_or_else(|| "surface reports no supported formats".to_string())?
            }
            None => wgpu::TextureFormat::Rgba8Unorm,
        };

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("kolpona-ui-shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("kolpona-ui-bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let uniform_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("kolpona-ui-globals"),
            size: std::mem::size_of::<Globals>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let atlas_tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("kolpona-ui-atlas"),
            size: wgpu::Extent3d {
                width: ATLAS_SIZE,
                height: ATLAS_SIZE,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let atlas_view = atlas_tex.create_view(&wgpu::TextureViewDescriptor::default());
        // Glyph quads are pixel-aligned 1:1 with physical pixels, so nearest is exact.
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("kolpona-ui-sampler"),
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("kolpona-ui-bg"),
            layout: &bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&atlas_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("kolpona-ui-layout"),
            bind_group_layouts: &[&bgl],
            push_constant_ranges: &[],
        });

        let attributes = wgpu::vertex_attr_array![
            0 => Float32x2, 1 => Float32x4, 2 => Float32x4,
            3 => Float32x4, 4 => Float32x4, 5 => Float32x2
        ];
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("kolpona-ui-pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                compilation_options: Default::default(),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Vertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &attributes,
                }],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let vbuf = Self::make_buffer(&device, 1 << 16, wgpu::BufferUsages::VERTEX);
        let ibuf = Self::make_buffer(&device, 1 << 15, wgpu::BufferUsages::INDEX);

        Ok(Self {
            adapter_info: adapter.get_info(),
            device,
            queue,
            pipeline,
            bind_group,
            uniform_buf,
            atlas_tex,
            vbuf,
            ibuf,
            format,
        })
    }

    fn make_buffer(device: &wgpu::Device, size: u64, usage: wgpu::BufferUsages) -> wgpu::Buffer {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("kolpona-ui-geometry"),
            size,
            usage: usage | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }
}

impl GpuBackend {
    /// Creates a GPU backend that presents into an OS window's swapchain.
    pub fn new_for_window(window: SharedWindow, width: u32, height: u32) -> Result<Self, String> {
        Self::init(Some(window), width, height)
    }

    /// Creates a GPU backend that renders into an offscreen texture (no window).
    pub fn new_headless(width: u32, height: u32) -> Result<Self, String> {
        Self::init(None, width, height)
    }

    /// Safely attempts to create a headless GPU backend, returning Err instead of panicking if no adapter is found.
    pub fn try_new() -> Result<Self, String> {
        Self::new_headless(1440, 900)
    }

    /// Convenience default constructor (headless 1440x900).
    pub fn new() -> Self {
        Self::new_headless(1440, 900).unwrap_or_else(|e| {
            panic!("failed to initialize GPU backend: {e}");
        })
    }
}

impl Default for GpuBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl GpuBackend {
    fn init(window: Option<SharedWindow>, width: u32, height: u32) -> Result<Self, String> {
        let instance = create_instance();
        let lost = Arc::new(AtomicBool::new(false));
        let (dev, target) =
            Self::create_device_and_target(&instance, window.as_ref(), &lost, width, height, true)?;
        Ok(Self {
            instance,
            window,
            target: Some(target),
            dev,
            lost,
            vsync: true,
            width,
            height,
            scale: 1.0,
            clear_color: Color::rgb(15, 18, 25),
            vertices: Vec::with_capacity(16 * 1024),
            indices: Vec::with_capacity(24 * 1024),
            batches: Vec::new(),
            clip_stack: Vec::new(),
            packer: ShelfPacker::default(),
            slots: HashMap::new(),
            uploads: Vec::new(),
            atlas_full: false,
            icons: IconRasterCache::new(),
            stats: RenderStats {
                backend: Some(BackendKind::HardwareGpu),
                ..Default::default()
            },
        })
    }

    fn create_device_and_target(
        instance: &wgpu::Instance,
        window: Option<&SharedWindow>,
        lost: &Arc<AtomicBool>,
        width: u32,
        height: u32,
        vsync: bool,
    ) -> Result<(DeviceState, Target), String> {
        match window {
            Some(win) => {
                let surface = instance
                    .create_surface(win.clone())
                    .map_err(|e| format!("create_surface failed: {e}"))?;
                let dev = DeviceState::create(instance, Some(&surface), lost)?;
                let config = wgpu::SurfaceConfiguration {
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                    format: dev.format,
                    width: width.max(1),
                    height: height.max(1),
                    present_mode: if vsync {
                        wgpu::PresentMode::AutoVsync
                    } else {
                        wgpu::PresentMode::AutoNoVsync
                    },
                    desired_maximum_frame_latency: 2,
                    alpha_mode: wgpu::CompositeAlphaMode::Auto,
                    view_formats: vec![],
                };
                surface.configure(&dev.device, &config);
                Ok((dev, Target::Surface { surface, config }))
            }
            None => {
                let dev = DeviceState::create(instance, None, lost)?;
                let (texture, view) = Self::make_offscreen(&dev, width, height);
                Ok((dev, Target::Offscreen { texture, view }))
            }
        }
    }

    fn make_offscreen(
        dev: &DeviceState,
        width: u32,
        height: u32,
    ) -> (wgpu::Texture, wgpu::TextureView) {
        let texture = dev.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("kolpona-ui-offscreen"),
            size: wgpu::Extent3d {
                width: width.max(1),
                height: height.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: dev.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        (texture, view)
    }

    /// Human readable adapter description, e.g. "NVIDIA GeForce RTX 3060 (Dx12)".
    pub fn adapter_summary(&self) -> String {
        let info = &self.dev.adapter_info;
        format!("{} ({:?}, {:?})", info.name, info.backend, info.device_type)
    }

    pub fn is_headless(&self) -> bool {
        matches!(self.target, Some(Target::Offscreen { .. }))
    }

    pub fn is_device_lost(&self) -> bool {
        self.lost.load(Ordering::SeqCst)
    }

    /// Enables or disables vsync for window presentation (takes effect on next resize/configure).
    pub fn set_vsync(&mut self, vsync: bool) {
        self.vsync = vsync;
        if let Some(Target::Surface { surface, config }) = &mut self.target {
            config.present_mode = if vsync {
                wgpu::PresentMode::AutoVsync
            } else {
                wgpu::PresentMode::AutoNoVsync
            };
            surface.configure(&self.dev.device, config);
        }
    }

    /// Forces a **real** device loss by destroying the wgpu device (equivalent to a
    /// driver reset / TDR from the application's point of view).
    pub fn simulate_device_loss(&mut self) {
        self.dev.device.destroy();
        self.lost.store(true, Ordering::SeqCst);
    }

    /// Number of glyph/icon rasters resident in the atlas.
    pub fn atlas_entries(&self) -> usize {
        self.slots.values().filter(|s| s.is_some()).count()
    }

    fn resize_target(&mut self, width: u32, height: u32) {
        match &mut self.target {
            Some(Target::Surface { surface, config }) => {
                if width > 0 && height > 0 && (config.width != width || config.height != height) {
                    config.width = width;
                    config.height = height;
                    surface.configure(&self.dev.device, config);
                }
            }
            Some(Target::Offscreen { .. }) => {
                let (texture, view) = Self::make_offscreen(&self.dev, width, height);
                self.target = Some(Target::Offscreen { texture, view });
            }
            None => {}
        }
    }

    fn reset_atlas(&mut self) {
        self.packer = ShelfPacker::default();
        self.slots.clear();
        self.uploads.clear();
        self.atlas_full = false;
    }

    // ---------------------------------------------------------------- tessellation

    fn current_scissor(&self) -> [u32; 4] {
        let s = self.scale;
        let full = Rect::from_min_size(
            Vec2::ZERO,
            Vec2::new(self.width as f32 / s, self.height as f32 / s),
        );
        let r = self.clip_stack.last().copied().unwrap_or(full);
        let x0 = (r.min.x * s).floor().clamp(0.0, self.width as f32) as u32;
        let y0 = (r.min.y * s).floor().clamp(0.0, self.height as f32) as u32;
        let x1 = (r.max.x * s).ceil().clamp(0.0, self.width as f32) as u32;
        let y1 = (r.max.y * s).ceil().clamp(0.0, self.height as f32) as u32;
        [x0, y0, x1.saturating_sub(x0), y1.saturating_sub(y0)]
    }

    fn start_batch(&mut self) {
        let start = self.indices.len() as u32;
        let scissor = self.current_scissor();
        if let Some(last) = self.batches.last_mut() {
            if last.end == last.start {
                last.scissor = scissor;
                return;
            }
        }
        self.batches.push(Batch {
            scissor,
            start,
            end: start,
        });
    }

    fn push_quad(&mut self, corners: [Vec2; 4], template: Vertex, uvs: Option<[[f32; 2]; 4]>) {
        let base = self.vertices.len() as u32;
        for (i, c) in corners.iter().enumerate() {
            let mut v = template;
            v.pos = [c.x, c.y];
            if let Some(uv) = uvs {
                v.shape = [uv[i][0], uv[i][1], 0.0, 0.0];
            }
            self.vertices.push(v);
        }
        self.indices
            .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
        if let Some(b) = self.batches.last_mut() {
            b.end = self.indices.len() as u32;
        }
    }

    fn tess_rect(&mut self, rect: Rect, fill: Color, radius: CornerRadius, stroke: Stroke) {
        let bw = if stroke.color.a > 0 {
            stroke.width.max(0.0)
        } else {
            0.0
        };
        if fill.a == 0 && bw == 0.0 {
            return;
        }
        let aa = 1.0 / self.scale;
        let min = rect.min - Vec2::splat(aa);
        let max = rect.max + Vec2::splat(aa);
        let center = rect.center();
        let half = rect.size() * 0.5;
        let template = Vertex {
            pos: [0.0; 2],
            shape: [center.x, center.y, half.x, half.y],
            radii: [
                radius.top_left,
                radius.top_right,
                radius.bottom_right,
                radius.bottom_left,
            ],
            color: color_f32(fill),
            color2: color_f32(stroke.color),
            extra: [bw, 0.0],
        };
        self.push_quad(
            [min, Vec2::new(max.x, min.y), max, Vec2::new(min.x, max.y)],
            template,
            None,
        );
    }

    fn tess_image(&mut self, rect: Rect, uv_rect: Rect, tint: Color) {
        if tint.a == 0 {
            return;
        }
        let aa = 1.0 / self.scale;
        let min = rect.min - Vec2::splat(aa);
        let max = rect.max + Vec2::splat(aa);
        let center = rect.center();
        let half = rect.size() * 0.5;
        let template = Vertex {
            pos: [0.0; 2],
            shape: [center.x, center.y, half.x, half.y],
            radii: [0.0; 4],
            color: color_f32(tint),
            color2: [0.0; 4],
            extra: [0.0, 0.0],
        };
        self.push_quad(
            [min, Vec2::new(max.x, min.y), max, Vec2::new(min.x, max.y)],
            template,
            Some([
                [uv_rect.min.x, uv_rect.min.y],
                [uv_rect.max.x, uv_rect.min.y],
                [uv_rect.max.x, uv_rect.max.y],
                [uv_rect.min.x, uv_rect.max.y],
            ]),
        );
    }

    fn tess_line(&mut self, a: Vec2, b: Vec2, width: f32, color: Color) {
        if color.a == 0 {
            return;
        }
        let aa = 1.0 / self.scale;
        let hw = (width * 0.5).max(0.5 * aa);
        let delta = b - a;
        let len = delta.length();
        let dir = if len > 1e-4 {
            delta * (1.0 / len)
        } else {
            Vec2::new(1.0, 0.0)
        };
        let n = Vec2::new(-dir.y, dir.x);
        let ext = hw + aa;
        let a2 = a - dir * ext;
        let b2 = b + dir * ext;
        let template = Vertex {
            pos: [0.0; 2],
            shape: [a.x, a.y, b.x, b.y],
            radii: [hw, 0.0, 0.0, 0.0],
            color: color_f32(color),
            color2: [0.0; 4],
            extra: [0.0, 2.0],
        };
        self.push_quad(
            [a2 + n * ext, b2 + n * ext, b2 - n * ext, a2 - n * ext],
            template,
            None,
        );
    }

    fn ensure_slot(
        &mut self,
        key: AtlasKey,
        mask: impl FnOnce(&mut Self) -> Option<Arc<Mask>>,
    ) -> Option<Slot> {
        if let Some(slot) = self.slots.get(&key) {
            return *slot;
        }
        let slot = match mask(self) {
            None => None,
            Some(m) => match self.packer.alloc(m.width, m.height) {
                Some((x, y)) => {
                    let slot = Slot {
                        x,
                        y,
                        w: m.width,
                        h: m.height,
                        left: m.left,
                        top: m.top,
                    };
                    self.uploads.push((slot, m));
                    Some(slot)
                }
                None => {
                    self.atlas_full = true;
                    return None;
                }
            },
        };
        self.slots.insert(key, slot);
        slot
    }

    fn push_mask_quad(&mut self, slot: Slot, phys_x: f32, phys_y: f32, color: Color) {
        let s = self.scale;
        let min = Vec2::new(phys_x / s, phys_y / s);
        let max = Vec2::new((phys_x + slot.w as f32) / s, (phys_y + slot.h as f32) / s);
        let inv = 1.0 / ATLAS_SIZE as f32;
        let (u0, v0) = (slot.x as f32 * inv, slot.y as f32 * inv);
        let (u1, v1) = (
            (slot.x + slot.w) as f32 * inv,
            (slot.y + slot.h) as f32 * inv,
        );
        let template = Vertex {
            pos: [0.0; 2],
            shape: [0.0; 4],
            radii: [0.0; 4],
            color: color_f32(color),
            color2: [0.0; 4],
            extra: [0.0, 1.0],
        };
        self.push_quad(
            [min, Vec2::new(max.x, min.y), max, Vec2::new(min.x, max.y)],
            template,
            Some([[u0, v0], [u1, v0], [u1, v1], [u0, v1]]),
        );
    }

    fn tess_text(&mut self, text_str: &str, pos: Vec2, size: f32, color: Color, strong: bool) {
        if color.a == 0 {
            return;
        }
        let s = self.scale;
        let px = size * s;
        let Some(layout) = text::shape(text_str, px, strong) else {
            self.tess_bitmap_text(text_str, pos, size, color);
            return;
        };
        let origin_x = pos.x * s;
        let baseline = (pos.y * s + layout.cap_height).round();
        for g in &layout.glyphs {
            let key = GlyphKey::new(g.font, g.id, px);
            let slot = self.ensure_slot(AtlasKey::Glyph(key), |_| {
                text::rasterize_glyph(key).map(Arc::new)
            });
            if let Some(slot) = slot {
                let x = (origin_x + g.x).round() + slot.left as f32;
                let y = (baseline + g.y).round() + slot.top as f32;
                self.push_mask_quad(slot, x, y, color);
            }
        }
    }

    /// Fallback when no outline fonts are installed: 5x7 bitmap glyphs as solid quads.
    fn tess_bitmap_text(&mut self, text_str: &str, pos: Vec2, size: f32, color: Color) {
        let cell = (size / 11.0).round().max(1.0);
        let mut x = pos.x;
        for ch in text_str.chars() {
            if let Some(glyph) = crate::cpu_font::glyph_for_char(ch) {
                for (col, &bits) in glyph.iter().enumerate() {
                    for row in 0..7 {
                        if bits & (1 << row) != 0 {
                            let r = Rect::from_min_size(
                                Vec2::new(x + col as f32 * cell, pos.y + row as f32 * cell),
                                Vec2::splat(cell),
                            );
                            self.tess_rect(r, color, CornerRadius::ZERO, Stroke::NONE);
                        }
                    }
                }
            }
            x += 6.0 * cell;
        }
    }

    fn tess_icon(&mut self, id: &'static str, svg: &'static str, rect: Rect, color: Color) {
        let s = self.scale;
        let key = IconKey {
            id,
            width: (rect.width() * s).round() as u32,
            height: (rect.height() * s).round() as u32,
        };
        let slot = self.ensure_slot(AtlasKey::Icon(key), |this| this.icons.get(key, svg));
        if let Some(slot) = slot {
            self.push_mask_quad(
                slot,
                (rect.min.x * s).round(),
                (rect.min.y * s).round(),
                color,
            );
        }
    }

    fn tessellate(&mut self, buffer: &CommandBuffer) {
        self.vertices.clear();
        self.indices.clear();
        self.batches.clear();
        self.clip_stack.clear();
        self.start_batch();
        for cmd in buffer.commands() {
            match cmd {
                DrawCommand::Rect {
                    rect,
                    fill,
                    radius,
                    stroke,
                } => self.tess_rect(*rect, *fill, *radius, *stroke),
                DrawCommand::Line { start, end, stroke } => {
                    self.tess_line(*start, *end, stroke.width, stroke.color)
                }
                DrawCommand::CubicBezier {
                    start,
                    ctrl1,
                    ctrl2,
                    end,
                    stroke,
                } => {
                    let pts = flatten_cubic(*start, *ctrl1, *ctrl2, *end);
                    for w in pts.windows(2) {
                        self.tess_line(w[0], w[1], stroke.width, stroke.color);
                    }
                }
                DrawCommand::Text {
                    text,
                    pos,
                    size,
                    color,
                    strong,
                } => self.tess_text(text, *pos, *size, *color, *strong),
                DrawCommand::Icon {
                    id,
                    svg,
                    rect,
                    color,
                } => self.tess_icon(id, svg, *rect, *color),
                DrawCommand::Image {
                    rect,
                    texture_id: _,
                    uv_rect,
                    tint,
                } => self.tess_image(*rect, *uv_rect, *tint),
                DrawCommand::ClipPush { rect } => {
                    let r = match self.clip_stack.last() {
                        Some(p) => Rect::from_min_max(
                            Vec2::new(rect.min.x.max(p.min.x), rect.min.y.max(p.min.y)),
                            Vec2::new(rect.max.x.min(p.max.x), rect.max.y.min(p.max.y)),
                        ),
                        None => *rect,
                    };
                    self.clip_stack.push(r);
                    self.start_batch();
                }
                DrawCommand::ClipPop => {
                    self.clip_stack.pop();
                    self.start_batch();
                }
            }
        }
    }

    // ---------------------------------------------------------------- execution

    fn ensure_capacity(&mut self) {
        let vbytes = (self.vertices.len() * std::mem::size_of::<Vertex>()) as u64;
        if vbytes > self.dev.vbuf.size() {
            let size = vbytes.next_power_of_two();
            self.dev.vbuf =
                DeviceState::make_buffer(&self.dev.device, size, wgpu::BufferUsages::VERTEX);
        }
        let ibytes = (self.indices.len() * 4) as u64;
        if ibytes > self.dev.ibuf.size() {
            let size = ibytes.next_power_of_two();
            self.dev.ibuf =
                DeviceState::make_buffer(&self.dev.device, size, wgpu::BufferUsages::INDEX);
        }
    }

    fn upload_atlas(&mut self) {
        for (slot, mask) in self.uploads.drain(..) {
            self.dev.queue.write_texture(
                wgpu::ImageCopyTexture {
                    texture: &self.dev.atlas_tex,
                    mip_level: 0,
                    origin: wgpu::Origin3d {
                        x: slot.x,
                        y: slot.y,
                        z: 0,
                    },
                    aspect: wgpu::TextureAspect::All,
                },
                &mask.data,
                wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(slot.w),
                    rows_per_image: Some(slot.h),
                },
                wgpu::Extent3d {
                    width: slot.w,
                    height: slot.h,
                    depth_or_array_layers: 1,
                },
            );
        }
    }

    fn encode_pass(&self, encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView) {
        let srgb = self.dev.format.is_srgb();
        let c = self.clear_color.to_f32_rgba();
        let conv = |v: f32| -> f64 {
            if srgb {
                srgb_to_linear(v as f64)
            } else {
                v as f64
            }
        };
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("kolpona-ui-pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: conv(c[0]),
                        g: conv(c[1]),
                        b: conv(c[2]),
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        if self.indices.is_empty() {
            return;
        }
        pass.set_pipeline(&self.dev.pipeline);
        pass.set_bind_group(0, &self.dev.bind_group, &[]);
        let vbytes = (self.vertices.len() * std::mem::size_of::<Vertex>()) as u64;
        let ibytes = (self.indices.len() * 4) as u64;
        pass.set_vertex_buffer(0, self.dev.vbuf.slice(..vbytes));
        pass.set_index_buffer(self.dev.ibuf.slice(..ibytes), wgpu::IndexFormat::Uint32);
        for b in &self.batches {
            if b.end <= b.start || b.scissor[2] == 0 || b.scissor[3] == 0 {
                continue;
            }
            pass.set_scissor_rect(b.scissor[0], b.scissor[1], b.scissor[2], b.scissor[3]);
            pass.draw_indexed(b.start..b.end, 0, 0..1);
        }
    }

    /// Reads back the last offscreen frame as 0xAARRGGBB pixels (headless mode only).
    pub fn read_pixels(&mut self) -> Result<Vec<u32>, String> {
        let Some(Target::Offscreen { texture, .. }) = &self.target else {
            return Err("read_pixels is only available for headless (offscreen) targets".into());
        };
        if self.is_device_lost() {
            return Err("GPU device lost".into());
        }
        let (w, h) = (self.width.max(1), self.height.max(1));
        let padded = (w * 4).div_ceil(256) * 256;
        let buffer = self.dev.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("kolpona-ui-readback"),
            size: (padded * h) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = self
            .dev
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("readback"),
            });
        encoder.copy_texture_to_buffer(
            wgpu::ImageCopyTexture {
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::ImageCopyBuffer {
                buffer: &buffer,
                layout: wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(padded),
                    rows_per_image: Some(h),
                },
            },
            wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
        );
        self.dev.queue.submit(Some(encoder.finish()));
        let slice = buffer.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |r| {
            let _ = tx.send(r);
        });
        self.dev.device.poll(wgpu::Maintain::Wait);
        rx.recv()
            .map_err(|e| e.to_string())?
            .map_err(|e| format!("map failed: {e}"))?;
        let data = slice.get_mapped_range();
        let bgra = matches!(
            self.dev.format,
            wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb
        );
        let mut out = Vec::with_capacity((w * h) as usize);
        for y in 0..h {
            let row = &data[(y * padded) as usize..(y * padded + w * 4) as usize];
            for px in row.as_chunks::<4>().0 {
                let (r, g, b) = if bgra {
                    (px[2], px[1], px[0])
                } else {
                    (px[0], px[1], px[2])
                };
                out.push(0xFF00_0000 | (r as u32) << 16 | (g as u32) << 8 | b as u32);
            }
        }
        drop(data);
        buffer.unmap();
        Ok(out)
    }
}

impl RenderBackend for GpuBackend {
    fn kind(&self) -> BackendKind {
        BackendKind::HardwareGpu
    }

    fn set_scale_factor(&mut self, scale: f32) {
        if scale.is_finite() && scale > 0.0 && (scale - self.scale).abs() > f32::EPSILON {
            self.scale = scale;
            // Glyph rasters are size-specific; old entries stay valid but new sizes are added.
        }
    }

    fn begin_frame(&mut self, width: u32, height: u32, clear_color: Color) {
        if width != self.width || height != self.height {
            self.width = width;
            self.height = height;
            if !self.is_device_lost() {
                self.resize_target(width, height);
            }
        }
        self.clear_color = clear_color;
        self.stats.draw_calls = 0;
        self.stats.primitives_count = 0;
        self.stats.vertices = 0;
    }

    fn submit(&mut self, buffer: &CommandBuffer) -> RenderStats {
        let start = Instant::now();
        self.tessellate(buffer);
        if self.atlas_full {
            // Atlas exhausted (e.g. many sizes after DPI changes): rebuild once from scratch.
            self.reset_atlas();
            self.tessellate(buffer);
        }
        self.stats.primitives_count = buffer.len();
        self.stats.vertices = self.vertices.len();
        self.stats.draw_calls = self.batches.iter().filter(|b| b.end > b.start).count();
        self.stats.frame_time_us = start.elapsed().as_micros() as u64;
        self.stats.backend = Some(BackendKind::HardwareGpu);
        self.stats
    }

    fn end_frame(&mut self) -> Result<RenderStats, String> {
        if self.is_device_lost() {
            return Err("GPU device lost; call recover() or fall back to CPU safe mode".into());
        }
        if self.width == 0 || self.height == 0 {
            return Ok(self.stats); // minimized: nothing to present
        }
        let start = Instant::now();
        self.upload_atlas();
        let globals = Globals {
            viewport: [
                self.width as f32 / self.scale,
                self.height as f32 / self.scale,
            ],
            aa: 1.0 / self.scale,
            srgb_target: if self.dev.format.is_srgb() { 1.0 } else { 0.0 },
        };
        self.dev
            .queue
            .write_buffer(&self.dev.uniform_buf, 0, bytemuck::bytes_of(&globals));
        self.ensure_capacity();
        if !self.vertices.is_empty() {
            self.dev
                .queue
                .write_buffer(&self.dev.vbuf, 0, bytemuck::cast_slice(&self.vertices));
            self.dev
                .queue
                .write_buffer(&self.dev.ibuf, 0, bytemuck::cast_slice(&self.indices));
        }

        let mut encoder = self
            .dev
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("kolpona-ui-frame"),
            });

        match &self.target {
            Some(Target::Surface { surface, config }) => {
                let frame = match surface.get_current_texture() {
                    Ok(f) => f,
                    Err(wgpu::SurfaceError::Outdated | wgpu::SurfaceError::Lost) => {
                        surface.configure(&self.dev.device, config);
                        surface
                            .get_current_texture()
                            .map_err(|e| format!("surface acquire failed after reconfigure: {e}"))?
                    }
                    Err(wgpu::SurfaceError::Timeout) => {
                        return Err("surface acquire timed out (frame skipped, unpresented)".into())
                    }
                    Err(e) => return Err(format!("surface acquire failed: {e}")),
                };
                let view = frame
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());
                self.encode_pass(&mut encoder, &view);
                self.dev.queue.submit(Some(encoder.finish()));
                frame.present();
            }
            Some(Target::Offscreen { view, .. }) => {
                self.encode_pass(&mut encoder, view);
                let idx = self.dev.queue.submit(Some(encoder.finish()));
                // Wait for the GPU so headless timings include real GPU execution.
                self.dev.device.poll(wgpu::Maintain::wait_for(idx));
            }
            None => return Err("no render target".into()),
        }
        self.stats.frame_time_us += start.elapsed().as_micros() as u64;

        if self.is_device_lost() {
            return Err("GPU device lost during frame".into());
        }
        Ok(self.stats)
    }

    fn recover(&mut self) -> Result<(), String> {
        // Drop the old surface before creating a new swapchain for the same window.
        self.target = None;
        let lost = Arc::new(AtomicBool::new(false));
        let (dev, target) = Self::create_device_and_target(
            &self.instance,
            self.window.as_ref(),
            &lost,
            self.width,
            self.height,
            self.vsync,
        )?;
        self.dev = dev;
        self.target = Some(target);
        self.lost = lost;
        self.reset_atlas();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn headless() -> Option<GpuBackend> {
        match GpuBackend::new_headless(320, 200) {
            Ok(g) => Some(g),
            Err(e) => {
                eprintln!("skipping GPU test: {e}");
                None
            }
        }
    }

    #[test]
    fn gpu_backend_renders_real_pixels_and_recovers_from_real_device_loss() {
        let Some(mut gpu) = headless() else { return };
        let mut buf = CommandBuffer::new();
        buf.push(DrawCommand::Rect {
            rect: Rect::from_min_size(Vec2::new(20.0, 20.0), Vec2::new(100.0, 60.0)),
            fill: Color::rgb(255, 0, 0),
            radius: CornerRadius::same(8.0),
            stroke: Stroke::NONE,
        });
        buf.push(DrawCommand::Text {
            text: "GPU কল্পনা".into(),
            pos: Vec2::new(20.0, 120.0),
            size: 18.0,
            color: Color::WHITE,
            strong: false,
        });

        gpu.begin_frame(320, 200, Color::BLACK);
        let stats = gpu.submit(&buf);
        assert!(stats.vertices > 8);
        assert!(gpu.end_frame().is_ok());
        let pixels = gpu.read_pixels().unwrap();
        assert_eq!(
            pixels[50 * 320 + 70],
            0xFFFF_0000,
            "rect centre must be red"
        );
        assert_eq!(
            pixels[5 * 320 + 5],
            0xFF00_0000,
            "background must be clear colour"
        );
        let text_lit = pixels[110 * 320..150 * 320]
            .iter()
            .filter(|&&p| p & 0xFF > 0)
            .count();
        assert!(text_lit > 50, "text must be drawn on the GPU");

        gpu.simulate_device_loss();
        assert!(
            gpu.end_frame().is_err(),
            "destroyed device must fail the frame"
        );
        gpu.recover().expect("device must be recreated");
        gpu.begin_frame(320, 200, Color::BLACK);
        gpu.submit(&buf);
        assert!(gpu.end_frame().is_ok());
        assert_eq!(gpu.read_pixels().unwrap()[50 * 320 + 70], 0xFFFF_0000);
    }
}
