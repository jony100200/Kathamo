// Real 3D scene for the glass viewport demo: sky, lit meshes, glowing orbs, grid floor.
// Output is sRGB-encoded into an Rgba8Unorm texture, which the Kathamo UI samples as an image.

struct Obj {
    model: mat4x4<f32>,
    color: vec4<f32>,
    params: vec4<f32>, // x: emissive, y: 1 = ground, z: 1 = visible
};

struct Globals {
    view_proj: mat4x4<f32>,
    cam_pos: vec4<f32>,
    light_dir: vec4<f32>,  // xyz: direction towards the sun
    sky_top: vec4<f32>,
    sky_horizon: vec4<f32>,
    objs: array<Obj, 8>,
};

@group(0) @binding(0) var<uniform> g: Globals;

const ORB_A: u32 = 4u;
const ORB_B: u32 = 5u;

// ---------- sky (full-screen triangle, no depth) ----------

struct SkyOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_sky(@builtin(vertex_index) i: u32) -> SkyOut {
    let uv = vec2<f32>(f32((i << 1u) & 2u), f32(i & 2u));
    var o: SkyOut;
    o.clip = vec4<f32>(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0, 0.0, 1.0);
    o.uv = uv;
    return o;
}

@fragment
fn fs_sky(i: SkyOut) -> @location(0) vec4<f32> {
    let t = smoothstep(0.0, 0.62, i.uv.y);
    var c = mix(g.sky_top.rgb, g.sky_horizon.rgb, t);
    // Soft sun glow near the horizon on the light's side.
    let sun_x = 0.5 + g.light_dir.x * 0.35;
    let d = distance(vec2<f32>(i.uv.x, i.uv.y * 1.6), vec2<f32>(sun_x, 0.55 * 1.6));
    c += g.sky_horizon.rgb * 0.9 * exp(-d * d * 18.0);
    return vec4<f32>(encode(c), 1.0);
}

// ---------- meshes ----------

struct VsIn {
    @location(0) pos: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) obj: u32,
};

struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) world: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) @interpolate(flat) obj: u32,
};

@vertex
fn vs_mesh(v: VsIn) -> VsOut {
    let o = g.objs[v.obj];
    var out: VsOut;
    let world = o.model * vec4<f32>(v.pos, 1.0);
    out.world = world.xyz;
    out.normal = normalize((o.model * vec4<f32>(v.normal, 0.0)).xyz);
    out.obj = v.obj;
    // Hidden objects collapse outside the clip volume.
    out.clip = select(vec4<f32>(2.0, 2.0, 2.0, 1.0), g.view_proj * world, o.params.z > 0.5);
    return out;
}

fn orb_pos(i: u32) -> vec3<f32> {
    return g.objs[i].model[3].xyz;
}

fn orb_light(i: u32, p: vec3<f32>, n: vec3<f32>) -> vec3<f32> {
    if (g.objs[i].params.z < 0.5) {
        return vec3<f32>(0.0);
    }
    let to = orb_pos(i) - p;
    let d2 = dot(to, to);
    let lambert = max(dot(n, normalize(to)), 0.0);
    return g.objs[i].color.rgb * (0.35 + lambert) * 3.0 / (1.0 + d2 * 0.9);
}

@fragment
fn fs_mesh(i: VsOut) -> @location(0) vec4<f32> {
    let o = g.objs[i.obj];
    let n = normalize(i.normal);
    let v = normalize(g.cam_pos.xyz - i.world);
    let l = normalize(g.light_dir.xyz);

    if (o.params.x > 0.5) {
        // Emissive orb: hot core, coloured falloff at grazing angles.
        let core = pow(max(dot(n, v), 0.0), 1.5);
        return vec4<f32>(encode(o.color.rgb * (1.2 + core * 2.5)), 1.0);
    }

    var base = o.color.rgb;
    var spec_strength = 0.55;
    if (o.params.y > 0.5) {
        // Grid floor: anti-aliased lines from world position.
        let p = i.world.xz;
        let w = fwidth(p) * 1.2;
        let cell = abs(fract(p - 0.5) - 0.5) / max(w, vec2<f32>(1e-4));
        let line = 1.0 - min(min(cell.x, cell.y), 1.0);
        let major = abs(fract(p / 5.0 - 0.5) - 0.5) / max(w / 5.0, vec2<f32>(1e-4));
        let major_line = 1.0 - min(min(major.x, major.y), 1.0);
        base = mix(vec3<f32>(0.018, 0.02, 0.035), vec3<f32>(0.16, 0.28, 0.5), line * 0.45 + major_line * 0.5);
        spec_strength = 0.15;
    }

    let h = normalize(l + v);
    let diffuse = max(dot(n, l), 0.0);
    let ambient = mix(vec3<f32>(0.02, 0.02, 0.035), g.sky_top.rgb * 0.9 + 0.04, n.y * 0.5 + 0.5);
    let spec = pow(max(dot(n, h), 0.0), 80.0) * spec_strength;
    let rim = pow(1.0 - max(dot(n, v), 0.0), 3.0) * 0.45;
    var c = base * (ambient + diffuse * 1.1) + vec3<f32>(spec) + g.sky_horizon.rgb * rim;
    c += base * (orb_light(ORB_A, i.world, n) + orb_light(ORB_B, i.world, n));

    // Distance fog into the horizon colour.
    let dist = distance(g.cam_pos.xyz, i.world);
    c = mix(c, g.sky_horizon.rgb * 0.55, smoothstep(14.0, 38.0, dist));
    return vec4<f32>(encode(c), 1.0);
}

// Filmic tone map, then sRGB encode for the Rgba8Unorm target.
fn encode(c: vec3<f32>) -> vec3<f32> {
    let x = max(c, vec3<f32>(0.0));
    let mapped = (x * (2.51 * x + 0.03)) / (x * (2.43 * x + 0.59) + 0.14);
    return pow(clamp(mapped, vec3<f32>(0.0), vec3<f32>(1.0)), vec3<f32>(1.0 / 2.2));
}
