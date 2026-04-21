// 2D FFT of the rendered field.
//
// Pipeline: init → 2·log2(N) butterfly passes (Stockham DIT, radix-2) →
// display fragment pass (fftshift + log-magnitude + turbo colormap).

const PI: f32 = 3.14159265359;

struct Params {
    n: u32,
    stage: u32,
    axis: u32,   // 0 = row pass, 1 = col pass
    _pad: u32,
};

@group(0) @binding(0) var<uniform> p: Params;
@group(0) @binding(1) var src_tex: texture_2d<f32>;
@group(0) @binding(2) var dst_tex: texture_storage_2d<rgba32float, write>;

// ─── init ────────────────────────────────────────────────────────────────
// Sample the rendered sim texture (RGBA8, arbitrary resolution), fold down to
// luminance, center around 0 to suppress DC, write complex value (re, im=0).

@compute @workgroup_size(8, 8, 1)
fn cs_init(@builtin(global_invocation_id) gid: vec3<u32>) {
    let n = p.n;
    if (gid.x >= n || gid.y >= n) { return; }

    let sim_dim = textureDimensions(src_tex);
    // Nearest-pixel mapping from FFT grid to sim grid.
    let sx = (gid.x * sim_dim.x) / n;
    let sy = (gid.y * sim_dim.y) / n;
    let rgba = textureLoad(src_tex, vec2<i32>(i32(sx), i32(sy)), 0);
    let lum = 0.2126 * rgba.r + 0.7152 * rgba.g + 0.0722 * rgba.b;
    let v = lum - 0.5;

    // Hann window to reduce edge discontinuity artifacts (DC cross).
    let u = f32(gid.x) / f32(n - 1u);
    let w = f32(gid.y) / f32(n - 1u);
    let wu = 0.5 - 0.5 * cos(2.0 * PI * u);
    let wv = 0.5 - 0.5 * cos(2.0 * PI * w);
    let windowed = v * wu * wv;

    textureStore(dst_tex, vec2<i32>(i32(gid.x), i32(gid.y)),
                 vec4<f32>(windowed, 0.0, 0.0, 0.0));
}

// ─── butterfly ──────────────────────────────────────────────────────────
// Stockham auto-sort decimation-in-time. Dispatch with gid.x in [0, N/2),
// gid.y in [0, N). Axis picks whether the transform runs along rows or cols.

@compute @workgroup_size(8, 8, 1)
fn cs_fft(@builtin(global_invocation_id) gid: vec3<u32>) {
    let n = p.n;
    let half_n = n >> 1u;
    if (gid.x >= half_n || gid.y >= n) { return; }

    let s = p.stage;
    let m = 1u << s;
    let k = gid.x & (m - 1u);
    let j = gid.x >> s;

    let in_lo = j * m + k;
    let in_hi = in_lo + half_n;
    let out_a = (j << (s + 1u)) + k;
    let out_b = out_a + m;

    var c_lo: vec2<i32>;
    var c_hi: vec2<i32>;
    var c_a: vec2<i32>;
    var c_b: vec2<i32>;
    if (p.axis == 0u) {
        let row = i32(gid.y);
        c_lo = vec2<i32>(i32(in_lo), row);
        c_hi = vec2<i32>(i32(in_hi), row);
        c_a  = vec2<i32>(i32(out_a), row);
        c_b  = vec2<i32>(i32(out_b), row);
    } else {
        let col = i32(gid.y);
        c_lo = vec2<i32>(col, i32(in_lo));
        c_hi = vec2<i32>(col, i32(in_hi));
        c_a  = vec2<i32>(col, i32(out_a));
        c_b  = vec2<i32>(col, i32(out_b));
    }

    let a = textureLoad(src_tex, c_lo, 0).xy;
    let b = textureLoad(src_tex, c_hi, 0).xy;

    let angle = -2.0 * PI * f32(k) / f32(m << 1u);
    let w = vec2<f32>(cos(angle), sin(angle));
    // complex multiply w * b
    let wb = vec2<f32>(w.x * b.x - w.y * b.y, w.x * b.y + w.y * b.x);
    let A = a + wb;
    let B = a - wb;

    textureStore(dst_tex, c_a, vec4<f32>(A, 0.0, 0.0));
    textureStore(dst_tex, c_b, vec4<f32>(B, 0.0, 0.0));
}

// ─── display ────────────────────────────────────────────────────────────
// Fragment pass: full-screen triangle writes an RGBA8 display texture.
// fftshift (DC → center) + log-mag + turbo colormap.

struct DisplayU {
    n: u32,
    _pad: vec3<u32>,
};

@group(0) @binding(0) var<uniform> du: DisplayU;
@group(0) @binding(1) var fft_tex: texture_2d<f32>;

struct VOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_display(@builtin(vertex_index) vid: u32) -> VOut {
    let x = f32((vid << 1u) & 2u);
    let y = f32(vid & 2u);
    var o: VOut;
    o.pos = vec4<f32>(x * 2.0 - 1.0, 1.0 - y * 2.0, 0.0, 1.0);
    o.uv = vec2<f32>(x, y);
    return o;
}

fn turbo(t: f32) -> vec3<f32> {
    let x = clamp(t, 0.0, 1.0);
    let x2 = x * x;
    let x3 = x2 * x;
    let x4 = x2 * x2;
    let x5 = x4 * x;
    let r = 0.13572138 + 4.61539260 * x - 42.66032258 * x2
          + 132.13108234 * x3 - 152.94239396 * x4 + 59.28637943 * x5;
    let g = 0.09140261 + 2.19418839 * x + 4.84296658 * x2
          - 14.18503333 * x3 + 4.27729857 * x4 + 2.82956604 * x5;
    let b = 0.10667330 + 12.64194608 * x - 60.58204836 * x2
          + 110.36276771 * x3 - 89.90310912 * x4 + 27.34824973 * x5;
    return clamp(vec3<f32>(r, g, b), vec3<f32>(0.0), vec3<f32>(1.0));
}

@fragment
fn fs_display(in: VOut) -> @location(0) vec4<f32> {
    let n = du.n;
    let half_n = n >> 1u;

    // uv in [0,1]² → pixel in output grid
    let px = u32(clamp(in.uv.x * f32(n), 0.0, f32(n - 1u)));
    let py = u32(clamp(in.uv.y * f32(n), 0.0, f32(n - 1u)));

    // fftshift: add N/2 mod N
    let sx = (px + half_n) & (n - 1u);
    let sy = (py + half_n) & (n - 1u);

    let c = textureLoad(fft_tex, vec2<i32>(i32(sx), i32(sy)), 0).xy;
    let mag = length(c);

    // Log compression. N·N normalizer keeps range roughly in [0,1] for
    // typical images; tanh clamps the tail.
    let denom = max(log(1.0 + f32(n) * f32(n) * 0.25), 1.0);
    let t = tanh(log(1.0 + mag) / denom * 3.0);

    return vec4<f32>(turbo(t), 1.0);
}
