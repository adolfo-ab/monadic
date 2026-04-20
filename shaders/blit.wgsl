struct BlitUniforms {
    dst_origin: vec2<f32>,
    dst_size: vec2<f32>,
    screen_size: vec2<f32>,
    _pad: vec2<f32>,
};

@group(0) @binding(0) var<uniform> u: BlitUniforms;
@group(0) @binding(1) var tex: texture_2d<f32>;
@group(0) @binding(2) var smp: sampler;

struct VOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vid: u32) -> VOut {
    let x = f32((vid << 1u) & 2u);
    let y = f32(vid & 2u);
    let uv = vec2<f32>(x, y);
    let px = u.dst_origin + uv * u.dst_size;
    let ndc = vec2<f32>(
        px.x / u.screen_size.x * 2.0 - 1.0,
        1.0 - px.y / u.screen_size.y * 2.0,
    );
    var o: VOut;
    o.pos = vec4<f32>(ndc, 0.0, 1.0);
    o.uv = uv;
    return o;
}

@fragment
fn fs_main(in: VOut) -> @location(0) vec4<f32> {
    if (in.uv.x > 1.0 || in.uv.y > 1.0) {
        discard;
    }
    return textureSample(tex, smp, in.uv);
}
