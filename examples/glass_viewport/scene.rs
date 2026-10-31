//! Real 3D renderer for the glass viewport demo: its own wgpu pipeline, depth buffer
//! and 4x MSAA, rendering into a texture the Kathamo UI shows without copying.

use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

pub const OBJECTS: [&str; 6] = ["Floor", "Torus", "Sphere", "Cube", "Orb A", "Orb B"];
const MSAA: u32 = 4;
const COLOR: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
const DEPTH: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

type Mat4 = [[f32; 4]; 4];

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Vertex {
    pos: [f32; 3],
    normal: [f32; 3],
    obj: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Obj {
    model: Mat4,
    color: [f32; 4],
    params: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Globals {
    view_proj: Mat4,
    cam_pos: [f32; 4],
    light_dir: [f32; 4],
    sky_top: [f32; 4],
    sky_horizon: [f32; 4],
    objs: [Obj; 8],
}

/// Orbit camera.
#[derive(Clone, Copy)]
pub struct Camera {
    pub yaw: f32,
    pub pitch: f32,
    pub distance: f32,
}

/// Time of day: sun direction and sky colours.
#[derive(Clone, Copy, PartialEq)]
pub enum Sky {
    Dusk,
    Night,
    Day,
}

pub struct Scene {
    sky: wgpu::RenderPipeline,
    mesh: wgpu::RenderPipeline,
    vbuf: wgpu::Buffer,
    ibuf: wgpu::Buffer,
    index_count: u32,
    ubuf: wgpu::Buffer,
    bind: wgpu::BindGroup,
    targets: Option<Targets>,
    pub triangles: [u32; 6],
}

struct Targets {
    size: (u32, u32),
    msaa: wgpu::TextureView,
    depth: wgpu::TextureView,
    resolve: wgpu::Texture,
}

impl Scene {
    pub fn new(device: &wgpu::Device) -> Self {
        let (vertices, indices, triangles) = build_meshes();
        let vbuf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("glass-demo-vertices"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let ibuf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("glass-demo-indices"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        let ubuf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("glass-demo-globals"),
            size: std::mem::size_of::<Globals>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("glass-demo-layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("glass-demo-bind"),
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: ubuf.as_entire_binding(),
            }],
        });
        let (sky, mesh) = pipelines(device, &layout);
        Self {
            sky,
            mesh,
            vbuf,
            ibuf,
            index_count: indices.len() as u32,
            ubuf,
            bind,
            targets: None,
            triangles,
        }
    }

    /// (Re)creates render targets for `size`; returns a fresh view to register when it did.
    pub fn ensure_size(&mut self, device: &wgpu::Device, size: (u32, u32)) -> Option<wgpu::TextureView> {
        let size = (size.0.max(1), size.1.max(1));
        if self.targets.as_ref().is_some_and(|t| t.size == size) {
            return None;
        }
        let tex = |label, format, samples, usage| {
            device.create_texture(&wgpu::TextureDescriptor {
                label: Some(label),
                size: wgpu::Extent3d {
                    width: size.0,
                    height: size.1,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: samples,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage,
                view_formats: &[],
            })
        };
        let attach = wgpu::TextureUsages::RENDER_ATTACHMENT;
        let msaa = tex("glass-demo-msaa", COLOR, MSAA, attach).create_view(&Default::default());
        let depth = tex("glass-demo-depth", DEPTH, MSAA, attach).create_view(&Default::default());
        let resolve = tex(
            "glass-demo-resolve",
            COLOR,
            1,
            attach | wgpu::TextureUsages::TEXTURE_BINDING,
        );
        let view = resolve.create_view(&Default::default());
        self.targets = Some(Targets {
            size,
            msaa,
            depth,
            resolve,
        });
        Some(view)
    }

    /// Bytes of GPU memory held by the render targets (colour MSAA + depth MSAA + resolve).
    pub fn target_bytes(&self) -> u64 {
        self.targets.as_ref().map_or(0, |t| {
            let px = t.size.0 as u64 * t.size.1 as u64;
            px * 4 * (MSAA as u64 * 2 + 1)
        })
    }

    pub fn render(&self, device: &wgpu::Device, queue: &wgpu::Queue, frame: &FrameInput) {
        let Some(t) = &self.targets else { return };
        let aspect = t.size.0 as f32 / t.size.1 as f32;
        queue.write_buffer(&self.ubuf, 0, bytemuck::bytes_of(&globals(frame, aspect)));
        let mut encoder = device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("glass-demo-scene"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &t.msaa,
                    resolve_target: Some(&t.resolve.create_view(&Default::default())),
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Discard,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &t.depth,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Discard,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_bind_group(0, &self.bind, &[]);
            pass.set_pipeline(&self.sky);
            pass.draw(0..3, 0..1);
            pass.set_pipeline(&self.mesh);
            pass.set_vertex_buffer(0, self.vbuf.slice(..));
            pass.set_index_buffer(self.ibuf.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..self.index_count, 0, 0..1);
        }
        queue.submit(Some(encoder.finish()));
    }
}

/// Per-frame scene state from the app.
pub struct FrameInput {
    pub time: f32,
    pub camera: Camera,
    pub sky: Sky,
    pub visible: [bool; 6],
}

fn globals(f: &FrameInput, aspect: f32) -> Globals {
    let c = f.camera;
    let eye = [
        c.distance * c.pitch.cos() * c.yaw.sin(),
        c.distance * c.pitch.sin() + 0.6,
        c.distance * c.pitch.cos() * c.yaw.cos(),
    ];
    let view_proj = mul(perspective(0.85, aspect, 0.1, 120.0), look_at(eye, [0.0, 0.8, 0.0]));
    let (light, top, horizon) = match f.sky {
        Sky::Dusk => ([0.6, 0.35, -0.4], [0.06, 0.05, 0.16], [0.95, 0.42, 0.32]),
        Sky::Night => ([-0.3, 0.6, 0.3], [0.01, 0.015, 0.05], [0.12, 0.2, 0.45]),
        Sky::Day => ([0.3, 0.9, 0.4], [0.18, 0.38, 0.85], [0.75, 0.85, 0.98]),
    };
    let t = f.time;
    let spin = t * 0.6;
    let orb = |phase: f32, r: f32, y: f32| [r * (t * 0.7 + phase).cos(), y + 0.3 * (t * 1.3 + phase).sin(), r * (t * 0.7 + phase).sin()];
    let specs: [(Mat4, [f32; 4], f32, f32); 6] = [
        (trs([0.0; 3], 0.0, 0.0, 1.0), [1.0; 4], 0.0, 1.0),
        (trs([-2.4, 1.2, 0.0], spin, 0.9, 1.0), [0.95, 0.25, 0.6, 1.0], 0.0, 0.0),
        (trs([0.0, 1.1, -1.2], 0.0, 0.0, 1.0), [0.2, 0.75, 0.95, 1.0], 0.0, 0.0),
        (trs([2.4, 0.9, 0.4], -spin * 0.8, spin * 0.5, 0.9), [0.98, 0.68, 0.25, 1.0], 0.0, 0.0),
        (trs(orb(0.0, 3.4, 1.6), 0.0, 0.0, 0.22), [1.0, 0.3, 0.75, 1.0], 1.0, 0.0),
        (trs(orb(3.14, 3.0, 2.1), 0.0, 0.0, 0.18), [0.3, 0.65, 1.0, 1.0], 1.0, 0.0),
    ];
    let mut objs = [Obj::zeroed(); 8];
    for (i, (model, color, emissive, ground)) in specs.into_iter().enumerate() {
        let visible = if f.visible[i] { 1.0 } else { 0.0 };
        objs[i] = Obj {
            model,
            color,
            params: [emissive, ground, visible, 0.0],
        };
    }
    Globals {
        view_proj,
        cam_pos: [eye[0], eye[1], eye[2], 1.0],
        light_dir: [light[0], light[1], light[2], 0.0],
        sky_top: [top[0], top[1], top[2], 1.0],
        sky_horizon: [horizon[0], horizon[1], horizon[2], 1.0],
        objs,
    }
}

fn pipelines(device: &wgpu::Device, layout: &wgpu::BindGroupLayout) -> (wgpu::RenderPipeline, wgpu::RenderPipeline) {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("glass-demo-shader"),
        source: wgpu::ShaderSource::Wgsl(include_str!("scene.wgsl").into()),
    });
    let pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("glass-demo-pipeline-layout"),
        bind_group_layouts: &[layout],
        push_constant_ranges: &[],
    });
    let attrs = wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Uint32];
    let vertex_layout = [wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<Vertex>() as u64,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &attrs,
    }];
    let make = |vs, fs, buffers: &[wgpu::VertexBufferLayout], depth_write, compare| {
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("glass-demo-pipeline"),
            layout: Some(&pl),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: vs,
                buffers,
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: fs,
                targets: &[Some(COLOR.into())],
                compilation_options: Default::default(),
            }),
            // No culling: generated meshes are tiny and never hidden by a winding slip.
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH,
                depth_write_enabled: depth_write,
                depth_compare: compare,
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: MSAA,
                ..Default::default()
            },
            multiview: None,
            cache: None,
        })
    };
    let sky = make("vs_sky", "fs_sky", &[], false, wgpu::CompareFunction::Always);
    let mesh = make("vs_mesh", "fs_mesh", &vertex_layout, true, wgpu::CompareFunction::Less);
    (sky, mesh)
}

// ---------- meshes ----------

fn build_meshes() -> (Vec<Vertex>, Vec<u32>, [u32; 6]) {
    let mut v = Vec::new();
    let mut idx = Vec::new();
    let mut tris = [0u32; 6];
    let mut add = |obj: u32, (pv, pi): (Vec<([f32; 3], [f32; 3])>, Vec<u32>)| {
        let base = v.len() as u32;
        v.extend(pv.into_iter().map(|(pos, normal)| Vertex { pos, normal, obj }));
        tris[obj as usize] = pi.len() as u32 / 3;
        idx.extend(pi.into_iter().map(|i| i + base));
    };
    add(0, plane(40.0));
    add(1, torus(0.9, 0.32, 96, 40));
    add(2, sphere(1.0, 64, 40));
    add(3, cube(0.8));
    add(4, sphere(1.0, 24, 16));
    add(5, sphere(1.0, 24, 16));
    (v, idx, tris)
}

type Mesh = (Vec<([f32; 3], [f32; 3])>, Vec<u32>);

fn plane(half: f32) -> Mesh {
    let n = [0.0, 1.0, 0.0];
    let p = |x, z| ([x, 0.0, z], n);
    (vec![p(-half, -half), p(-half, half), p(half, half), p(half, -half)], vec![0, 1, 2, 0, 2, 3])
}

fn sphere(r: f32, seg: u32, rings: u32) -> Mesh {
    let mut v = Vec::new();
    for j in 0..=rings {
        let phi = std::f32::consts::PI * j as f32 / rings as f32;
        for i in 0..=seg {
            let th = std::f32::consts::TAU * i as f32 / seg as f32;
            let n = [phi.sin() * th.cos(), phi.cos(), phi.sin() * th.sin()];
            v.push(([n[0] * r, n[1] * r, n[2] * r], n));
        }
    }
    (v, grid_indices(seg, rings))
}

fn torus(big: f32, small: f32, seg: u32, rings: u32) -> Mesh {
    let mut v = Vec::new();
    for j in 0..=rings {
        let phi = std::f32::consts::TAU * j as f32 / rings as f32;
        for i in 0..=seg {
            let th = std::f32::consts::TAU * i as f32 / seg as f32;
            let n = [phi.cos() * th.cos(), phi.sin(), phi.cos() * th.sin()];
            let c = [big * th.cos(), 0.0, big * th.sin()];
            v.push(([c[0] + small * n[0], small * n[1], c[2] + small * n[2]], n));
        }
    }
    (v, grid_indices(seg, rings))
}

/// Triangles for a (seg+1) x (rings+1) vertex grid.
fn grid_indices(seg: u32, rings: u32) -> Vec<u32> {
    let mut idx = Vec::new();
    for j in 0..rings {
        for i in 0..seg {
            let a = j * (seg + 1) + i;
            let b = a + seg + 1;
            idx.extend_from_slice(&[a, a + 1, b, a + 1, b + 1, b]);
        }
    }
    idx
}

fn cube(h: f32) -> Mesh {
    let mut v = Vec::new();
    let mut idx = Vec::new();
    let faces: [([f32; 3], [f32; 3], [f32; 3]); 6] = [
        ([1.0, 0.0, 0.0], [0.0, 0.0, -1.0], [0.0, 1.0, 0.0]),
        ([-1.0, 0.0, 0.0], [0.0, 0.0, 1.0], [0.0, 1.0, 0.0]),
        ([0.0, 1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, -1.0]),
        ([0.0, -1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]),
        ([0.0, 0.0, 1.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]),
        ([0.0, 0.0, -1.0], [-1.0, 0.0, 0.0], [0.0, 1.0, 0.0]),
    ];
    for (n, u, w) in faces {
        let base = v.len() as u32;
        for (su, sw) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
            let p = [0, 1, 2].map(|k| (n[k] + u[k] * su + w[k] * sw) * h);
            v.push((p, n));
        }
        idx.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }
    (v, idx)
}

// ---------- 4x4 matrices (column-major, matching WGSL) ----------

fn mul(a: Mat4, b: Mat4) -> Mat4 {
    let mut m = [[0.0; 4]; 4];
    for (c, col) in m.iter_mut().enumerate() {
        for (r, cell) in col.iter_mut().enumerate() {
            *cell = (0..4).map(|k| a[k][r] * b[c][k]).sum();
        }
    }
    m
}

/// Right-handed perspective with wgpu's 0..1 depth range.
fn perspective(fovy: f32, aspect: f32, near: f32, far: f32) -> Mat4 {
    let f = 1.0 / (fovy * 0.5).tan();
    let nf = 1.0 / (near - far);
    [[f / aspect, 0.0, 0.0, 0.0], [0.0, f, 0.0, 0.0], [0.0, 0.0, far * nf, -1.0], [0.0, 0.0, near * far * nf, 0.0]]
}

fn look_at(eye: [f32; 3], target: [f32; 3]) -> Mat4 {
    let sub = |a: [f32; 3], b: [f32; 3]| [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
    let dot = |a: [f32; 3], b: [f32; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
    let cross = |a: [f32; 3], b: [f32; 3]| [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]];
    let norm = |a: [f32; 3]| {
        let l = dot(a, a).sqrt().max(1e-6);
        [a[0] / l, a[1] / l, a[2] / l]
    };
    let f = norm(sub(target, eye));
    let s = norm(cross(f, [0.0, 1.0, 0.0]));
    let u = cross(s, f);
    [[s[0], u[0], -f[0], 0.0], [s[1], u[1], -f[1], 0.0], [s[2], u[2], -f[2], 0.0], [-dot(s, eye), -dot(u, eye), dot(f, eye), 1.0]]
}

/// Translation * RotY(yaw) * RotX(pitch) * uniform scale.
fn trs(t: [f32; 3], yaw: f32, pitch: f32, s: f32) -> Mat4 {
    let (sy, cy) = yaw.sin_cos();
    let (sx, cx) = pitch.sin_cos();
    [
        [cy * s, 0.0, -sy * s, 0.0],
        [sy * sx * s, cx * s, cy * sx * s, 0.0],
        [sy * cx * s, -sx * s, cy * cx * s, 0.0],
        [t[0], t[1], t[2], 1.0],
    ]
}
