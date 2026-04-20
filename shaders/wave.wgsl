struct Emitter {
    pos: vec2<f32>,
    base_k: f32,
    phase_seed: f32,
};

struct Spec {
    k_mult: f32,
    amp: f32,
    phase_off: f32,
    _pad: f32,
};

struct Uniforms {
    resolution: vec2<f32>,
    canvas_origin: vec2<f32>,
    canvas_size: vec2<f32>,
    time: f32,
    num_emitters: u32,
    wave_speed: f32,
    amp_scale: f32,
    color_mode: u32,
    decay_mode: u32,
    num_spec: u32,
    phase_mode: u32,
    phase_param_a: f32,
    phase_param_b: f32,
};

@group(0) @binding(0) var<uniform> u: Uniforms;
@group(0) @binding(1) var<storage, read> emitters: array<Emitter>;
@group(0) @binding(2) var<storage, read> spectrum: array<Spec>;

const PI: f32 = 3.14159265359;
const MAX_SPEC: u32 = 16u;

@vertex
fn vs_main(@builtin(vertex_index) vid: u32) -> @builtin(position) vec4<f32> {
    let x = f32((vid << 1u) & 2u);
    let y = f32(vid & 2u);
    return vec4<f32>(x * 2.0 - 1.0, 1.0 - y * 2.0, 0.0, 1.0);
}

fn node_phase(pos: vec2<f32>, base_k: f32, phase_seed: f32, idx: u32) -> f32 {
    let cx = u.canvas_size.x * 0.5;
    let cy = u.canvas_size.y * 0.5;
    let dx = pos.x - cx;
    let dy = pos.y - cy;
    let r = sqrt(dx * dx + dy * dy);
    let theta = atan2(dy, dx);
    let m = u.phase_mode;

    if (m == 0u) { return 0.0; }
    if (m == 1u) { return phase_seed; }
    if (m == 2u) { return -base_k * r; }
    if (m == 3u) { return u.phase_param_a * theta; }
    if (m == 4u) {
        let nx = cos(u.phase_param_a);
        let ny = sin(u.phase_param_a);
        return base_k * (dx * nx + dy * ny);
    }
    if (m == 5u) { return u.phase_param_a * u.time; }
    if (m == 6u) {
        if ((idx & 1u) == 1u) { return PI; }
        return 0.0;
    }
    if (m == 7u) {
        // Spiral: vortex + focus
        return u.phase_param_a * theta - base_k * r;
    }
    if (m == 8u) {
        // Hyperbolic saddle, normalized by (canvas/2)^2
        let half = u.canvas_size.x * 0.5;
        let denom = max(half * half, 1.0);
        return u.phase_param_a * (dx * dx - dy * dy) / denom;
    }
    if (m == 9u) {
        // Radial bands: π·floor(β·r/half)
        let half = max(u.canvas_size.x * 0.5, 1.0);
        return PI * floor(u.phase_param_a * r / half);
    }
    if (m == 10u) {
        // Checker: π·((floor(β·x/half)+floor(β·y/half)) mod 2)
        let half = max(u.canvas_size.x * 0.5, 1.0);
        let ix = floor(u.phase_param_a * dx / half);
        let iy = floor(u.phase_param_a * dy / half);
        let parity = (i32(ix) + i32(iy)) & 1;
        if (parity != 0) { return PI; }
        return 0.0;
    }
    return 0.0;
}

fn hsv2rgb(c: vec3<f32>) -> vec3<f32> {
    let k = vec4<f32>(1.0, 2.0 / 3.0, 1.0 / 3.0, 3.0);
    let p = abs(fract(vec3<f32>(c.x) + k.xyz) * 6.0 - vec3<f32>(k.w));
    return c.z * mix(vec3<f32>(k.x), clamp(p - vec3<f32>(k.x), vec3<f32>(0.0), vec3<f32>(1.0)), c.y);
}

@fragment
fn fs_main(@builtin(position) frag: vec4<f32>) -> @location(0) vec4<f32> {
    let p = frag.xy - u.canvas_origin;

    if (p.x < 0.0 || p.y < 0.0 || p.x >= u.canvas_size.x || p.y >= u.canvas_size.y) {
        return vec4<f32>(1.0, 1.0, 1.0, 1.0);
    }

    var re: f32 = 0.0;
    var im: f32 = 0.0;
    var re_arr: array<f32, 16>;
    var im_arr: array<f32, 16>;

    let n = u.num_emitters;
    let ms = min(u.num_spec, MAX_SPEC);
    let speed = u.wave_speed;
    let t = u.time;
    let spectral = u.color_mode == 3u;

    for (var i: u32 = 0u; i < n; i = i + 1u) {
        let e = emitters[i];
        let dx = p.x - e.pos.x;
        let dy = p.y - e.pos.y;
        let d = sqrt(dx * dx + dy * dy);
        let safe_d = max(d, 1.0);

        var decay: f32 = 1.0;
        if (u.decay_mode == 1u) {
            decay = 1.0 / sqrt(safe_d);
        } else if (u.decay_mode == 2u) {
            decay = 1.0 / safe_d;
        }

        let phi_node = node_phase(e.pos, e.base_k, e.phase_seed, i);

        for (var j: u32 = 0u; j < ms; j = j + 1u) {
            let s = spectrum[j];
            let k_eff = e.base_k * s.k_mult;
            let omega = k_eff * speed;
            let theta = k_eff * d - omega * t + phi_node + s.phase_off;
            let a = decay * s.amp;
            let cre = a * cos(theta);
            let cim = a * sin(theta);
            re = re + cre;
            im = im + cim;
            if (spectral) {
                re_arr[j] = re_arr[j] + cre;
                im_arr[j] = im_arr[j] + cim;
            }
        }
    }

    if (u.color_mode == 0u) {
        // ψ real, signed grayscale (white = +, black = −)
        let v = 0.5 - 0.5 * tanh(re * u.amp_scale);
        return vec4<f32>(v, v, v, 1.0);
    }
    if (u.color_mode == 1u) {
        // |ψ|² intensity, dark = constructive
        let mag = sqrt(re * re + im * im) * u.amp_scale;
        let v = 1.0 - clamp(tanh(mag), 0.0, 1.0);
        return vec4<f32>(v, v, v, 1.0);
    }
    if (u.color_mode == 2u) {
        // Domain coloring: arg → hue, |ψ| → value
        let arg = atan2(im, re);
        let mag = sqrt(re * re + im * im) * u.amp_scale;
        let hue = arg / (2.0 * PI) + 0.5;
        let val = clamp(tanh(mag), 0.0, 1.0);
        let rgb = hsv2rgb(vec3<f32>(hue, 0.85, val));
        return vec4<f32>(rgb, 1.0);
    }
    // Spectral coloring: each component j contributes its magnitude tinted by hue (j+0.5)/M
    var rgb = vec3<f32>(0.0);
    let denom = max(f32(ms), 1.0);
    for (var j: u32 = 0u; j < ms; j = j + 1u) {
        let mag_j = sqrt(re_arr[j] * re_arr[j] + im_arr[j] * im_arr[j]);
        let hue = (f32(j) + 0.5) / denom;
        let intensity = clamp(tanh(mag_j * u.amp_scale), 0.0, 1.0);
        rgb = rgb + hsv2rgb(vec3<f32>(hue, 0.9, 1.0)) * intensity;
    }
    rgb = clamp(rgb, vec3<f32>(0.0), vec3<f32>(1.0));
    return vec4<f32>(rgb, 1.0);
}
