//! Backdrop blur for frosted-glass panels (`DrawCommand::Backdrop`).
//!
//! The frame drawn so far is copied, blurred with dual Kawase down to 1/8 and
//! back up to 1/2 resolution, and the half-resolution result is sampled by the
//! UI shader under a rounded mask. Textures exist only while backdrops are used:
//! about 1.33x one frame of VRAM (5 MB copy + 1.7 MB of levels at 1440x900).

/// Down-sample steps: 3 gives a wide, soft frosted-glass blur.
const BLUR_LEVELS: usize = 3;

pub(crate) struct BackdropBlur {
    size: (u32, u32),
    copy: wgpu::Texture,
    /// Level 0 is the full-size copy; level n is 1/2^n size.
    views: Vec<wgpu::TextureView>,
    _levels: Vec<wgpu::Texture>,
    passes: Vec<(bool, wgpu::BindGroup, usize)>, // (is_down, source bindings, target level)
    down: wgpu::RenderPipeline,
    up: wgpu::RenderPipeline,
    _params: Vec<wgpu::Buffer>,
}

impl BackdropBlur {
    pub(crate) fn new(device: &wgpu::Device, format: wgpu::TextureFormat, w: u32, h: u32) -> Self {
        let format = format.remove_srgb_suffix();
        let make = |label, w: u32, h: u32, usage| {
            device.create_texture(&wgpu::TextureDescriptor {
                label: Some(label),
                size: wgpu::Extent3d {
                    width: w.max(1),
                    height: h.max(1),
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage,
                view_formats: &[],
            })
        };
        let copy = make(
            "kathamo-backdrop-copy",
            w,
            h,
            wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
        );
        let level_usage =
            wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING;
        let mut sizes = vec![(w.max(1), h.max(1))];
        let mut views = vec![copy.create_view(&Default::default())];
        let mut keep = Vec::new();
        for n in 1..=BLUR_LEVELS {
            let (lw, lh) = ((w >> n).max(1), (h >> n).max(1));
            let t = make("kathamo-backdrop-level", lw, lh, level_usage);
            views.push(t.create_view(&Default::default()));
            sizes.push((lw, lh));
            keep.push(t);
        }

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("kathamo-backdrop-blur"),
            source: wgpu::ShaderSource::Wgsl(include_str!("blur.wgsl").into()),
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("kathamo-backdrop-layout"),
            entries: &[
                entry(0, wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                }),
                entry(1, wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering)),
                entry(2, wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                }),
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("kathamo-backdrop-pipeline"),
            bind_group_layouts: &[&layout],
            push_constant_ranges: &[],
        });
        let pipeline = |entry_point| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("kathamo-backdrop-pass"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: "vs_main",
                    buffers: &[],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point,
                    targets: &[Some(format.into())],
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
                cache: None,
            })
        };
        let (down, up) = (pipeline("fs_down"), pipeline("fs_up"));
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("kathamo-backdrop-sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        // Down 0->1->..->N, then up N->..->1.
        let mut plan: Vec<(bool, usize, usize)> = (0..BLUR_LEVELS).map(|n| (true, n, n + 1)).collect();
        plan.extend((2..=BLUR_LEVELS).rev().map(|n| (false, n, n - 1)));
        let mut params = Vec::new();
        let passes = plan
            .into_iter()
            .map(|(is_down, src, dst)| {
                let (sw, sh) = sizes[src];
                let data = [0.5 / sw as f32, 0.5 / sh as f32, 0.0, 0.0];
                let buf = wgpu::util::DeviceExt::create_buffer_init(
                    device,
                    &wgpu::util::BufferInitDescriptor {
                        label: Some("kathamo-backdrop-params"),
                        contents: bytemuck::cast_slice(&data),
                        usage: wgpu::BufferUsages::UNIFORM,
                    },
                );
                let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("kathamo-backdrop-bind"),
                    layout: &layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(&views[src]),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::Sampler(&sampler),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: buf.as_entire_binding(),
                        },
                    ],
                });
                params.push(buf);
                (is_down, bind, dst)
            })
            .collect();
        Self {
            size: (w, h),
            copy,
            views,
            _levels: keep,
            passes,
            down,
            up,
            _params: params,
        }
    }

    pub(crate) fn size(&self) -> (u32, u32) {
        self.size
    }

    /// The blurred result (half resolution), sampled by the UI shader.
    pub(crate) fn result(&self) -> &wgpu::TextureView {
        &self.views[1]
    }

    /// Copies `frame` and blurs it. Call between render passes.
    pub(crate) fn run(&self, encoder: &mut wgpu::CommandEncoder, frame: &wgpu::Texture) {
        encoder.copy_texture_to_texture(
            frame.as_image_copy(),
            self.copy.as_image_copy(),
            wgpu::Extent3d {
                width: self.size.0,
                height: self.size.1,
                depth_or_array_layers: 1,
            },
        );
        for (is_down, bind, dst) in &self.passes {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("kathamo-backdrop-blur"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.views[*dst],
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_pipeline(if *is_down { &self.down } else { &self.up });
            pass.set_bind_group(0, bind, &[]);
            pass.draw(0..3, 0..1);
        }
    }
}

fn entry(binding: u32, ty: wgpu::BindingType) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty,
        count: None,
    }
}

#[cfg(test)]
mod tests {
    use crate::{CommandBuffer, DrawCommand, GpuBackend, RenderBackend};
    use ui_core::{Color, CornerRadius, Rect, Vec2};

    #[test]
    fn backdrop_blurs_only_inside_its_rect() {
        let Ok(mut gpu) = GpuBackend::new_headless(320, 200) else {
            eprintln!("skipping GPU test: no adapter");
            return;
        };
        let mut cmds = CommandBuffer::new();
        let half = |x: f32, c: Color| DrawCommand::Rect {
            rect: Rect::from_min_size(Vec2::new(x, 0.0), Vec2::new(160.0, 200.0)),
            fill: c,
            radius: CornerRadius::ZERO,
            stroke: ui_core::Stroke::NONE,
        };
        cmds.push(half(0.0, Color::rgb(255, 0, 0)));
        cmds.push(half(160.0, Color::rgb(0, 255, 0)));
        cmds.push(DrawCommand::Backdrop {
            rect: Rect::from_min_size(Vec2::new(100.0, 50.0), Vec2::new(120.0, 100.0)),
            radius: CornerRadius::ZERO,
            tint: Color::TRANSPARENT,
        });
        gpu.begin_frame(320, 200, Color::BLACK);
        gpu.submit(&cmds);
        gpu.end_frame().unwrap();
        let px = gpu.read_pixels().unwrap();
        let at = |x: usize, y: usize| px[y * 320 + x];
        // Outside the panel the hard edge is untouched.
        assert_eq!(at(158, 20), 0xFFFF_0000);
        assert_eq!(at(161, 20), 0xFF00_FF00);
        // Inside, the edge is blurred: both colours mix on each side.
        let (lr, lg) = r_g(at(158, 100));
        assert!(lr < 250 && lg > 5, "left of edge not blurred: r={lr} g={lg}");
        let (rr, rg) = r_g(at(161, 100));
        assert!(rr > 5 && rg < 250, "right of edge not blurred: r={rr} g={rg}");
    }

    fn r_g(p: u32) -> (u32, u32) {
        ((p >> 16) & 0xFF, (p >> 8) & 0xFF)
    }
}
