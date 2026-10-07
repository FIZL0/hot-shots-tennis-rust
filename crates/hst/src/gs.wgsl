// PS2 GS colour path for court models (see gs.rs). Values are in PS2 units: 1.0 = 0x80 for vertex/material colour
// and alpha, texel colour 1.0 = 0xff, texel alpha 1.0 = 0x80 (mtl.rs expands it to 0xff).
#import bevy_pbr::forward_io::VertexOutput
#import bevy_pbr::{mesh_view_bindings as view_bindings, mesh_view_types, shadows}

struct Gs {
    color: vec4<f32>,
    shininess: f32,
    highlight: f32,
    shadow: f32,
    uv_offset: vec2<f32>,
    fog: vec4<f32>,
    fog_color: vec4<f32>,
    lod_k: f32,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> gs: Gs;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var gs_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var gs_sampler: sampler;

fn srgb_to_linear(c: vec3<f32>) -> vec3<f32> {
    return select(pow((c + 0.055) / 1.055, vec3(2.4)), c / 12.92, c <= vec3(0.04045));
}

// 1 lit, 0 in shadow: the first directional light that casts shadows (the match's sun)
fn sun_visibility(in: VertexOutput) -> f32 {
    for (var i = 0u; i < view_bindings::lights.n_directional_lights; i++) {
        if (view_bindings::lights.directional_lights[i].flags & mesh_view_types::DIRECTIONAL_LIGHT_FLAGS_SHADOWS_ENABLED_BIT) != 0u {
            let v = view_bindings::view.view_from_world;
            let view_z = dot(vec4(v[0].z, v[1].z, v[2].z, v[3].z), in.world_position);
            // receivers are the flat ground: up is its normal
            return shadows::fetch_directional_shadow(i, in.world_position, vec3(0.0, 1.0, 0.0), view_z, in.position.xy);
        }
    }
    return 1.0;
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
    // TEX1 as the game sets it: LCM 0, L 0, K per model material, MMIN linear-mipmap-nearest, MXL = the levels
    // uploaded. Q = 1/w (VU1), w the view depth (m), so LOD = log2(w) + K, rounded off to a level
    let vz = view_bindings::view.view_from_world;
    let w = -dot(vec4(vz[0].z, vz[1].z, vz[2].z, vz[3].z), in.world_position);
    let level = clamp(floor(log2(w) + gs.lod_k + 0.5), 0.0, f32(textureNumLevels(gs_texture) - 1u));
    let t = textureSampleLevel(gs_texture, gs_sampler, in.uv + gs.uv_offset, level);
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
    rgb = clamp(rgb, vec3(0.0), vec3(1.0));
#ifdef GS_FOG
    // VU1: F from the view depth (clip w), at most F near, then at least F far (VU1's MINI, MAX); both go towards 255 as the eye rises past 30 m
    // (game y = −Bevy y). GS: (F·C + (255 − F)·FOGCOL) >> 8 on 8-bit values.
    let v = view_bindings::view.view_from_world;
    let depth = -dot(vec4(v[0].z, v[1].z, v[2].z, v[3].z), in.world_position);
    let lift = clamp((view_bindings::view.world_position.y - 30.0) * 0.02, 0.0, 0.7);
    let ends = gs.fog.xy + lift * (255.0 - gs.fog.xy);
    let fog = floor(max(min(ends.x + (depth - gs.fog.z) * (ends.y - ends.x) / (gs.fog.w - gs.fog.z), ends.x), ends.y));
    rgb = (floor(fog * floor(rgb * 255.0) / 256.0) + floor((255.0 - fog) * floor(gs.fog_color.rgb * 255.0 + 0.5) / 256.0)) / 255.0;
#endif
    // shadow: (0 − Cd)·A + Cd on the frame buffer, the same as darkening every layer drawn there
    if gs.shadow > 0.0 {
        rgb *= 1.0 - gs.shadow * (1.0 - sun_visibility(in));
    }
    return vec4(srgb_to_linear(rgb), clamp(a, 0.0, 1.0));
}
