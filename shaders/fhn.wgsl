// FitzHugh-Nagumo excitable medium coupled to the wave sim texture.
// State textures store (u, v) in R, G channels (Rgba32Float).
//
//   ∂u/∂t = D_u ∇²u + u − u³/3 − v + I
//   ∂v/∂t = D_v ∇²v + ε (u + a − b v)
//
// u is the fast "voltage" variable, v the slow recovery. External input I
// combines wave coupling + emitter drive to seed traveling excitation waves.

struct Params {
    n: u32,
    num_emitters: u32,
    emit_radius: f32,
    emit_rate: f32,
    diff_u: f32,
    diff_v: f32,
    epsilon: f32,
    a: f32,
    b: f32,
    coupling: f32,
    dt: f32,
    time: f32,
};

@group(0) @binding(0) var<uniform> u_p: Params;
@group(0) @binding(1) var src: texture_2d<f32>;
@group(0) @binding(2) var dst: texture_storage_2d<rgba32float, write>;
@group(0) @binding(3) var sim_tex: texture_2d<f32>;
@group(0) @binding(4) var<storage, read> emitters: array<vec4<f32>>;

fn emitter_density(uv: vec2<f32>) -> f32 {
    let r = max(u_p.emit_radius, 1e-4);
    let inv_r2 = 1.0 / (r * r);
    var acc: f32 = 0.0;
    let count = u_p.num_emitters;
    for (var i: u32 = 0u; i < count; i = i + 1u) {
        let e = emitters[i].xy;
        let d = uv - e;
        acc = acc + exp(-dot(d, d) * inv_r2);
    }
    return clamp(acc, 0.0, 1.0);
}

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
    let N = i32(u_p.n);
    if (i32(gid.x) >= N || i32(gid.y) >= N) { return; }
    let p = vec2<i32>(i32(gid.x), i32(gid.y));

    let uv = textureLoad(src, p, 0).rg;
    let U = uv.x;
    let V = uv.y;
    let L = lap5(p, N);

    // Wave input: white=+, black=−  →  signed [−1, 1].
    let sim_dim = vec2<i32>(textureDimensions(sim_tex));
    let sp = (vec2<f32>(gid.xy) + vec2<f32>(0.5)) / f32(u_p.n);
    let sim_pixel = clamp(
        vec2<i32>(sp * vec2<f32>(sim_dim)),
        vec2<i32>(0),
        sim_dim - vec2<i32>(1),
    );
    let wave_raw = textureLoad(sim_tex, sim_pixel, 0).r;
    let w = 1.0 - 2.0 * wave_raw;

    let uv_norm = (vec2<f32>(gid.xy) + vec2<f32>(0.5)) / f32(u_p.n);
    let emit = emitter_density(uv_norm);

    let I = u_p.coupling * w + u_p.emit_rate * emit;

    let du = u_p.diff_u * L.x + U - U * U * U / 3.0 - V + I;
    let dv = u_p.diff_v * L.y + u_p.epsilon * (U + u_p.a - u_p.b * V);

    var newU = U + u_p.dt * du;
    var newV = V + u_p.dt * dv;

    // Soft bound to keep state finite under strong forcing.
    newU = clamp(newU, -3.0, 3.0);
    newV = clamp(newV, -3.0, 3.0);

    textureStore(dst, p, vec4<f32>(newU, newV, 0.0, 1.0));
}

@compute @workgroup_size(8, 8, 1)
fn cs_init(@builtin(global_invocation_id) gid: vec3<u32>) {
    let N = i32(u_p.n);
    if (i32(gid.x) >= N || i32(gid.y) >= N) { return; }
    let p = vec2<i32>(i32(gid.x), i32(gid.y));
    let uv_norm = (vec2<f32>(gid.xy) + vec2<f32>(0.5)) / f32(u_p.n);
    // Seed with a wider emitter blob kick so excitation waves launch fast.
    let r = max(u_p.emit_radius * 1.5, 1e-4);
    let inv_r2 = 1.0 / (r * r);
    var blob: f32 = 0.0;
    let count = u_p.num_emitters;
    for (var i: u32 = 0u; i < count; i = i + 1u) {
        let e = emitters[i].xy;
        let d = uv_norm - e;
        blob = blob + exp(-dot(d, d) * inv_r2);
    }
    let noise = hash21(vec2<u32>(gid.x + u32(u_p.time * 1000.0), gid.y + 73u));
    let U = clamp(1.2 * blob + 0.05 * (noise - 0.5), -2.0, 2.0);
    let V = -0.3;
    textureStore(dst, p, vec4<f32>(U, V, 0.0, 1.0));
}

// ─── display ─────────────────────────────────────────────────────────────
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
    let U = s.r;
    let V = s.g;
    // u typically ranges ≈ [−2, 2]. Map to [0, 1] for palette.
    let t = clamp((U + 2.0) / 4.0, 0.0, 1.0);
    // Cool → excited: indigo → teal → citrine → cream.
    let indigo = vec3<f32>(0.10, 0.08, 0.22);
    let teal   = vec3<f32>(0.10, 0.45, 0.55);
    let citron = vec3<f32>(0.90, 0.78, 0.28);
    let cream  = vec3<f32>(0.98, 0.96, 0.88);
    var col: vec3<f32>;
    if (t < 0.33) {
        col = mix(indigo, teal, t / 0.33);
    } else if (t < 0.66) {
        col = mix(teal, citron, (t - 0.33) / 0.33);
    } else {
        col = mix(citron, cream, (t - 0.66) / 0.34);
    }
    // Recovery variable darkens the field (refractory shading).
    let shade = clamp((V + 1.0) * 0.5, 0.0, 1.0);
    col = col * (1.0 - 0.35 * shade);
    return vec4<f32>(col, 1.0);
}
