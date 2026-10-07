// Shadow pass for court models (see gs.rs): the same alpha test as gs.wgsl, so cut-out leaves cast leaf shadows.
#import bevy_pbr::prepass_io::VertexOutput
#ifdef PREPASS_FRAGMENT
#import bevy_pbr::prepass_io::FragmentOutput
#endif

struct Gs {
    color: vec4<f32>,
    shininess: f32,
    highlight: f32,
    shadow: f32,
    uv_offset: vec2<f32>,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> gs: Gs;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var gs_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var gs_sampler: sampler;

fn alpha_test(in: VertexOutput) {
#ifdef GS_NO_Z
    discard;
#endif
#ifdef VERTEX_COLORS
    var a = min(in.color.a * gs.color.a, 255.0 / 128.0);
#else
    var a = min(gs.color.a, 255.0 / 128.0);
#endif
#ifdef GS_TEXTURED
#ifdef VERTEX_UVS_A
    let t = textureSample(gs_texture, gs_sampler, in.uv + gs.uv_offset).a;
#ifdef GS_MODULATE
    a = t * a;
#else
    a = t;
#endif
#endif
#endif
    let a8 = floor(a * 128.0);
#ifdef GS_GE40
    if a8 < 64.0 { discard; }
#endif
#ifdef GS_GE70
    if a8 < 112.0 { discard; }
#endif
}

#ifdef PREPASS_FRAGMENT
@fragment
fn fragment(in: VertexOutput) -> FragmentOutput {
    alpha_test(in);
    var out: FragmentOutput;
#ifdef UNCLIPPED_DEPTH_ORTHO_EMULATION
    out.frag_depth = in.unclipped_depth;
#endif
    return out;
}
#else
@fragment
fn fragment(in: VertexOutput) {
    alpha_test(in);
}
#endif
