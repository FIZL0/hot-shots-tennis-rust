// PS2 GS colour path for court models (see gs.rs). Values are in PS2 units: 1.0 = 0x80 for vertex/material colour
// and alpha, texel colour 1.0 = 0xff, texel alpha 1.0 = 0x80 (mtl.rs expands it to 0xff).
#import bevy_pbr::forward_io::VertexOutput

struct Gs {
    color: vec4<f32>,
    shininess: f32,
    highlight: f32,
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
    // VU1 lighting, in game space (y down; GameSpace turns it 180° about X): ambient 0.5 + 0.49 × the light from
    // (1, 2, 1)/√6, and a specular term on the fixed half vector, Schlick's t/(k − (k − 1)t) for t^k
    let n = normalize(in.world_normal) * vec3(1.0, -1.0, -1.0);
    let diffuse = max(-dot(n, vec3(0.408248, 0.816497, 0.408248)), 0.0);
    let h = max(-dot(n, vec3(0.243259, 0.486519, 0.839121)), 0.0);
    let spec = gs.highlight * h / (gs.shininess - (gs.shininess - 1.0) * h);
    f = vec4(f.rgb * (0.5 + 0.49 * diffuse), f.a);
    var rgb = min(f.rgb, vec3(255.0 / 128.0));
    var a = min(f.a, 255.0 / 128.0);
#ifdef GS_TEXTURED
    let t = textureSample(gs_texture, gs_sampler, in.uv);
    rgb = t.rgb * rgb;
#ifdef GS_MODULATE
    a = t.a * a;
#else
    // HIGHLIGHT2: + vertex alpha (0..0xff), which carries the highlight
    a = t.a;
    rgb += min(spec, 1.0);
#endif
#else
    rgb = min(rgb + spec * 255.0 / 128.0, vec3(255.0 / 128.0));
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
