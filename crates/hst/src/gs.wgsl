// PS2 GS colour path for court models (see gs.rs). Values are in PS2 units: 1.0 = 0x80 for vertex/material colour
// and alpha, texel colour 1.0 = 0xff, texel alpha 1.0 = 0x80 (mtl.rs expands it to 0xff).
#import bevy_pbr::forward_io::VertexOutput

struct Gs {
    color: vec4<f32>,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> gs: Gs;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var gs_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var gs_sampler: sampler;

fn srgb_to_linear(c: vec3<f32>) -> vec3<f32> {
    return select(pow((c + 0.055) / 1.055, vec3(2.4)), c / 12.92, c <= vec3(0.04045));
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    // vertex colour × material colour, as 8-bit RGBAQ (0x80 = 1.0, at most 0xff)
#ifdef VERTEX_COLORS
    var f = in.color * gs.color;
#else
    var f = gs.color;
#endif
    f = min(f, vec4(255.0 / 128.0));
    var rgb = f.rgb;
    var a = f.a;
#ifdef GS_TEXTURED
    let t = textureSample(gs_texture, gs_sampler, in.uv);
    rgb = t.rgb * f.rgb;
#ifdef GS_MODULATE
    a = t.a * f.a;
#else
    a = t.a;
#endif
#endif
    // alpha test on the 8-bit value
    let a8 = floor(a * 128.0);
#ifdef GS_GE40
    if a8 < 64.0 { discard; }
#endif
#ifdef GS_GE70
    if a8 < 112.0 { discard; }
#endif
#ifdef GS_LT70
    if a8 >= 112.0 { discard; }
#endif
    return vec4(srgb_to_linear(clamp(rgb, vec3(0.0), vec3(1.0))), clamp(a, 0.0, 1.0));
}
