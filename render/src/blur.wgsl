// Dual-Kawase blur (Bjorge, SIGGRAPH 2015): cheap wide blur by down- then up-sampling.
// `half_texel` is 0.5 / size of the texture being read in this pass.

struct Params {
    half_texel: vec2<f32>,
    _pad: vec2<f32>,
};

@group(0) @binding(0) var src: texture_2d<f32>;
@group(0) @binding(1) var smp: sampler;
@group(0) @binding(2) var<uniform> p: Params;

struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

// One triangle covering the target; no vertex buffer.
@vertex
fn vs_main(@builtin(vertex_index) i: u32) -> VsOut {
    let uv = vec2<f32>(f32((i << 1u) & 2u), f32(i & 2u));
    var o: VsOut;
    o.clip = vec4<f32>(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0, 0.0, 1.0);
    o.uv = uv;
    return o;
}

@fragment
fn fs_down(i: VsOut) -> @location(0) vec4<f32> {
    let h = p.half_texel;
    var sum = textureSample(src, smp, i.uv) * 4.0;
    sum += textureSample(src, smp, i.uv - h);
    sum += textureSample(src, smp, i.uv + h);
    sum += textureSample(src, smp, i.uv + vec2<f32>(h.x, -h.y));
    sum += textureSample(src, smp, i.uv - vec2<f32>(h.x, -h.y));
    return sum / 8.0;
}

@fragment
fn fs_up(i: VsOut) -> @location(0) vec4<f32> {
    let h = p.half_texel;
    var sum = textureSample(src, smp, i.uv + vec2<f32>(-h.x * 2.0, 0.0));
    sum += textureSample(src, smp, i.uv + vec2<f32>(h.x * 2.0, 0.0));
    sum += textureSample(src, smp, i.uv + vec2<f32>(0.0, -h.y * 2.0));
    sum += textureSample(src, smp, i.uv + vec2<f32>(0.0, h.y * 2.0));
    sum += textureSample(src, smp, i.uv + vec2<f32>(-h.x, h.y)) * 2.0;
    sum += textureSample(src, smp, i.uv + vec2<f32>(h.x, h.y)) * 2.0;
    sum += textureSample(src, smp, i.uv + vec2<f32>(h.x, -h.y)) * 2.0;
    sum += textureSample(src, smp, i.uv + vec2<f32>(-h.x, -h.y)) * 2.0;
    return sum / 12.0;
}
