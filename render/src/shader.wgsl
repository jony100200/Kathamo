// Kolpona UI shader: one pipeline, three primitive modes, all anti-aliased.
//   mode 0: rounded rectangle with inset border (signed distance field)
//   mode 1: coverage mask from the glyph/icon atlas, tinted
//   mode 2: thick line segment with round caps (segment distance)
// Coordinates are logical points; `aa` is the size of one physical pixel in points.

struct Globals {
    viewport: vec2<f32>,
    aa: f32,
    srgb_target: f32,
};

@group(0) @binding(0) var<uniform> globals: Globals;
@group(0) @binding(1) var atlas_tex: texture_2d<f32>;
@group(0) @binding(2) var atlas_smp: sampler;
@group(0) @binding(3) var image_tex: texture_2d<f32>;
@group(0) @binding(4) var image_smp: sampler;

struct VsIn {
    @location(0) pos: vec2<f32>,
    @location(1) shape: vec4<f32>,   // rect: center.xy half.xy | line: a.xy b.xy | mask: uv.xy
    @location(2) radii: vec4<f32>,   // rect: tl tr br bl       | line: half_width
    @location(3) color: vec4<f32>,
    @location(4) color2: vec4<f32>,  // rect border colour
    @location(5) extra: vec2<f32>,   // x: border width, y: mode
};

struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) world: vec2<f32>,
    @location(1) shape: vec4<f32>,
    @location(2) radii: vec4<f32>,
    @location(3) color: vec4<f32>,
    @location(4) color2: vec4<f32>,
    @location(5) extra: vec2<f32>,
};

@vertex
fn vs_main(v: VsIn) -> VsOut {
    var o: VsOut;
    let ndc = v.pos / globals.viewport * 2.0 - vec2<f32>(1.0, 1.0);
    o.clip = vec4<f32>(ndc.x, -ndc.y, 0.0, 1.0);
    o.world = v.pos;
    o.shape = v.shape;
    o.radii = v.radii;
    o.color = v.color;
    o.color2 = v.color2;
    o.extra = v.extra;
    return o;
}

fn sd_round_box(p: vec2<f32>, half: vec2<f32>, radii: vec4<f32>) -> f32 {
    // radii = (top_left, top_right, bottom_right, bottom_left); y points down.
    var r: f32;
    if (p.x > 0.0) {
        r = select(radii.y, radii.z, p.y > 0.0);
    } else {
        r = select(radii.x, radii.w, p.y > 0.0);
    }
    r = clamp(r, 0.0, min(half.x, half.y));
    let q = abs(p) - half + vec2<f32>(r, r);
    return min(max(q.x, q.y), 0.0) + length(max(q, vec2<f32>(0.0, 0.0))) - r;
}

fn to_linear(c: vec3<f32>) -> vec3<f32> {
    let lo = c / 12.92;
    let hi = pow((c + vec3<f32>(0.055)) / 1.055, vec3<f32>(2.4));
    return select(hi, lo, c <= vec3<f32>(0.04045));
}

fn out_color(rgb: vec3<f32>, a: f32) -> vec4<f32> {
    var c = rgb;
    if (globals.srgb_target > 0.5) {
        c = to_linear(c);
    }
    // Premultiplied alpha output.
    return vec4<f32>(c * a, a);
}

@fragment
fn fs_main(i: VsOut) -> @location(0) vec4<f32> {
    let mode = i32(i.extra.y + 0.5);
    let aa = globals.aa;
    if (mode == 1) {
        let cov = textureSample(atlas_tex, atlas_smp, i.shape.xy).r;
        return out_color(i.color.rgb, i.color.a * cov);
    }
    if (mode == 3) {
        let sample_color = textureSample(image_tex, image_smp, i.shape.xy);
        return out_color(sample_color.rgb * i.color.rgb, sample_color.a * i.color.a);
    }
    if (mode == 2) {
        let a = i.shape.xy;
        let b = i.shape.zw;
        let pa = i.world - a;
        let ba = b - a;
        let h = clamp(dot(pa, ba) / max(dot(ba, ba), 1e-6), 0.0, 1.0);
        let d = length(pa - ba * h);
        let cov = clamp((i.radii.x - d) / aa + 0.5, 0.0, 1.0);
        return out_color(i.color.rgb, i.color.a * cov);
    }
    // mode 0: rounded rect
    let d = sd_round_box(i.world - i.shape.xy, i.shape.zw, i.radii);
    let outer = clamp(0.5 - d / aa, 0.0, 1.0);
    let bw = i.extra.x;
    if (bw > 0.0) {
        let inner = clamp(0.5 - (d + bw) / aa, 0.0, 1.0);
        let fill_a = i.color.a * inner;
        let ring_a = i.color2.a * (outer - inner);
        var fill_rgb = i.color.rgb;
        var ring_rgb = i.color2.rgb;
        if (globals.srgb_target > 0.5) {
            fill_rgb = to_linear(fill_rgb);
            ring_rgb = to_linear(ring_rgb);
        }
        return vec4<f32>(fill_rgb * fill_a + ring_rgb * ring_a, fill_a + ring_a);
    }
    return out_color(i.color.rgb, i.color.a * outer);
}
