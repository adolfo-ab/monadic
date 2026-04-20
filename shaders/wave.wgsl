struct Emitter {
    pos: vec2<f32>,
    k: f32,
    phase: f32,
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
};

@group(0) @binding(0) var<uniform> u: Uniforms;
@group(0) @binding(1) var<storage, read> emitters: array<Emitter>;

@vertex
fn vs_main(@builtin(vertex_index) vid: u32) -> @builtin(position) vec4<f32> {
    let x = f32((vid << 1u) & 2u);
    let y = f32(vid & 2u);
    return vec4<f32>(x * 2.0 - 1.0, 1.0 - y * 2.0, 0.0, 1.0);
}

@fragment
fn fs_main(@builtin(position) frag: vec4<f32>) -> @location(0) vec4<f32> {
    let p = frag.xy - u.canvas_origin;

    if (p.x < 0.0 || p.y < 0.0 || p.x >= u.canvas_size.x || p.y >= u.canvas_size.y) {
        return vec4<f32>(1.0, 1.0, 1.0, 1.0);
    }

    var re: f32 = 0.0;
    var im: f32 = 0.0;
    let n = u.num_emitters;
    let speed = u.wave_speed;
    let t = u.time;

    for (var i: u32 = 0u; i < n; i = i + 1u) {
        let e = emitters[i];
        let dx = p.x - e.pos.x;
        let dy = p.y - e.pos.y;
        let d2 = dx * dx + dy * dy;
        let d = sqrt(d2);
        let safe_d = max(d, 1.0);
        let omega = e.k * speed;
        let theta = e.k * d - omega * t + e.phase;

        var amp: f32 = 1.0;
        if (u.decay_mode == 1u) {
            amp = 1.0 / sqrt(safe_d);
        } else if (u.decay_mode == 2u) {
            amp = 1.0 / safe_d;
        }

        re = re + amp * cos(theta);
        im = im + amp * sin(theta);
    }

    var v: f32;
    if (u.color_mode == 0u) {
        // Real part, signed grayscale: white = +, black = -, mid = 0
        v = 0.5 - 0.5 * tanh(re * u.amp_scale);
    } else {
        // Intensity |sum|^2 -> dark = constructive, white = destructive
        let mag = sqrt(re * re + im * im) * u.amp_scale;
        v = 1.0 - clamp(tanh(mag), 0.0, 1.0);
    }

    return vec4<f32>(v, v, v, 1.0);
}
