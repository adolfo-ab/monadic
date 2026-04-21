// Gray-Scott reaction-diffusion coupled to the wave sim texture.
// State textures store (U, V) in R, G channels (Rgba32Float).

struct Params {
    n: u32,
    reset: u32,
    _pad0: u32,
    _pad1: u32,
    feed: f32,
    kill: f32,
    coupling: f32,
    dt: f32,
    diff_u: f32,
    diff_v: f32,
    time: f32,
    _pad2: f32,
};

@group(0) @binding(0) var<uniform> u: Params;
@group(0) @binding(1) var src: texture_2d<f32>;
@group(0) @binding(2) var dst: texture_storage_2d<rgba32float, write>;
@group(0) @binding(3) var sim_tex: texture_2d<f32>;

fn lap5(p: vec2<i32>, N: i32) -> vec2<f32> {
    let xm = (p.x + N - 1) % N;
    let xp = (p.x + 1) % N;
    let ym = (p.y + N - 1) % N;
    let yp = (p.y + 1) % N;
    let c = textureLoad(src, p, 0).rg;
    let s = textureLoad(src, vec2<i32>(xm, p.y), 0).rg
          + textureLoad(src, vec2<i32>(xp, p.y), 0).rg
          + textureLoad(src, vec2<i32>(p.x, ym), 0).rg
          + textureLoad(src, vec2<i32>(p.x, yp), 0).rg;
    return s - 4.0 * c;
}

fn hash21(p: vec2<u32>) -> f32 {
    var h: u32 = p.x * 374761393u + p.y * 668265263u;
    h = (h ^ (h >> 13u)) * 1274126177u;
    h = h ^ (h >> 16u);
    return f32(h) / 4294967296.0;
}

@compute @workgroup_size(8, 8, 1)
fn cs_step(@builtin(global_invocation_id) gid: vec3<u32>) {
    let N = i32(u.n);
    if (i32(gid.x) >= N || i32(gid.y) >= N) { return; }
    let p = vec2<i32>(i32(gid.x), i32(gid.y));

    let uv = textureLoad(src, p, 0).rg;
    let U = uv.x;
    let V = uv.y;
    let L = lap5(p, N);

    // Wave sample. Map real-channel brightness (white=+, black=−) → signed [-1, 1].
    let sim_dim = vec2<i32>(textureDimensions(sim_tex));
    let sp = (vec2<f32>(gid.xy) + vec2<f32>(0.5)) / f32(u.n);
    let sim_pixel = clamp(
        vec2<i32>(sp * vec2<f32>(sim_dim)),
        vec2<i32>(0),
        sim_dim - vec2<i32>(1),
    );
    let wave_raw = textureLoad(sim_tex, sim_pixel, 0).r;
    let w = 1.0 - 2.0 * wave_raw;

    let react = U * V * V;
    var newU = U + u.dt * (u.diff_u * L.x - react + u.feed * (1.0 - U));
    var newV = V + u.dt * (u.diff_v * L.y + react - (u.feed + u.kill) * V);

    // Coupling: positive wave crests seed V (autocatalyst), consume U.
    let wpos = max(w, 0.0);
    let inject = u.dt * u.coupling * wpos;
    newV = newV + inject;
    newU = newU - 0.5 * inject;

    newU = clamp(newU, 0.0, 1.0);
    newV = clamp(newV, 0.0, 1.0);

    textureStore(dst, p, vec4<f32>(newU, newV, 0.0, 1.0));
}

@compute @workgroup_size(8, 8, 1)
fn cs_init(@builtin(global_invocation_id) gid: vec3<u32>) {
    let N = i32(u.n);
    if (i32(gid.x) >= N || i32(gid.y) >= N) { return; }
    let p = vec2<i32>(i32(gid.x), i32(gid.y));
    let center = vec2<f32>(f32(u.n) * 0.5);
    let d = length(vec2<f32>(gid.xy) - center);
    let sigma = f32(u.n) * 0.12;
    let blob = exp(-d * d / (sigma * sigma));
    let noise = hash21(vec2<u32>(gid.x + u32(u.time * 1000.0), gid.y + 73u));
    let V = 0.4 * blob + 0.05 * noise;
    let U = 1.0 - V;
    textureStore(dst, p, vec4<f32>(U, V, 0.0, 1.0));
}

// Display: colorize V into a Rgba8Unorm display texture.
struct DispU { n: u32, _p0: u32, _p1: u32, _p2: u32, };
@group(0) @binding(0) var<uniform> du: DispU;
@group(0) @binding(1) var state_tex: texture_2d<f32>;

@vertex
fn vs_display(@builtin(vertex_index) vid: u32) -> @builtin(position) vec4<f32> {
    let x = f32((vid << 1u) & 2u);
    let y = f32(vid & 2u);
    return vec4<f32>(x * 2.0 - 1.0, 1.0 - y * 2.0, 0.0, 1.0);
}

@fragment
fn fs_display(@builtin(position) frag: vec4<f32>) -> @location(0) vec4<f32> {
    let p = vec2<i32>(i32(frag.x), i32(frag.y));
    let s = textureLoad(state_tex, p, 0);
    let V = s.g;
    let U = s.r;
    // Organic palette: bone → ochre → umber → oxblood.
    let t = clamp(V * 2.5, 0.0, 1.0);
    let bone  = vec3<f32>(0.97, 0.95, 0.90);
    let ochre = vec3<f32>(0.82, 0.62, 0.32);
    let umber = vec3<f32>(0.45, 0.25, 0.15);
    let blood = vec3<f32>(0.15, 0.04, 0.03);
    var col: vec3<f32>;
    if (t < 0.33) {
        col = mix(bone, ochre, t / 0.33);
    } else if (t < 0.66) {
        col = mix(ochre, umber, (t - 0.33) / 0.33);
    } else {
        col = mix(umber, blood, (t - 0.66) / 0.34);
    }
    // Faint gradient shading from U to add "depth".
    col = col * (0.85 + 0.15 * U);
    return vec4<f32>(col, 1.0);
}
