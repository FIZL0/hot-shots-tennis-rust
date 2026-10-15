// PS2 GS colour path for court models (see gs.rs). Values are in PS2 units: 1.0 = 0x80 for vertex/material colour
// and alpha, texel colour 1.0 = 0xff, texel alpha 1.0 = 0x80 (mtl.rs expands it to 0xff).
#import bevy_pbr::{mesh_view_bindings as view_bindings, mesh_view_types, shadows, shadow_sampling}
#import bevy_pbr::{mesh_bindings::mesh, mesh_functions, skinning, morph, forward_io::Vertex}
#import bevy_pbr::view_transformations::position_world_to_clip

struct Gs {
    color: vec4<f32>,
    shininess: f32,
    highlight: f32,
    shadow: f32,
    uv_offset: vec2<f32>,
    fog: vec4<f32>,
    fog_color: vec4<f32>,
    lod_k: f32,
    light_dir: vec4<f32>,
    light_color: vec4<f32>,
    ambient: vec4<f32>,
    light2_dir: vec4<f32>,
    light2_color: vec4<f32>,
    glare: f32,
    unlit: f32,
    screen_lod: f32,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> gs: Gs;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var gs_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var gs_sampler: sampler;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
#ifdef VERTEX_UVS_A
    @location(2) uv: vec2<f32>,
#endif
    // VU1's vertex colour, as the GS gets it: vertex × material colour × light, 8-bit (0x80 = 1.0, at most 0xff)
    @location(5) color: vec4<f32>,
    // its specular term (the vertex alpha HIGHLIGHT2 adds)
    @location(8) spec: f32,
    // the fog coefficient F VU1 writes to the vertex (0..255); the GS interpolates it linearly on screen
#ifdef GS_NO_LINEAR
    // WebGL2 has no linear (noperspective) interpolation: F·w and w, both perspective-correct, divide to F linear on
    // screen
    @location(9) fog: vec2<f32>,
#else
    @location(9) @interpolate(linear) fog: f32,
#endif
#ifdef GS_AO
    // G1: the share of the vertex's light that isn't the sun's (ambient + the second light), what SSAO occludes
    @location(10) ambient: f32,
#endif
}

#ifdef MORPH_TARGETS
// as Bevy's mesh.wgsl
fn morph_vertex(vertex_in: Vertex, instance_index: u32) -> Vertex {
    var vertex = vertex_in;
    let vertex_index = vertex.index - mesh[instance_index].first_vertex_index;
#ifndef SKINS_USE_UNIFORM_BUFFERS
    // B277: Bevy 0.19 builds a mesh instance that just became visible before its morph descriptor, so it has none
    // (u32::MAX) until it is extracted again. Browsers' WebGPU clamps that read to the last descriptor, another mesh's
    // weights and targets, which flung a face's vertices across the screen: draw it unmorphed instead.
    if mesh[instance_index].morph_descriptor_index == 0xffffffffu {
        return vertex;
    }
#endif
    for (var i = 0u; i < morph::layer_count(instance_index); i++) {
        let weight = morph::weight_at(i, instance_index);
        if weight != 0.0 {
            vertex.position += weight * morph::morph_position(vertex_index, i, instance_index);
        }
    }
    return vertex;
}
#endif

@vertex
fn vertex(vertex_no_morph: Vertex) -> VertexOutput {
    var out: VertexOutput;
    let instance = vertex_no_morph.instance_index;
#ifdef MORPH_TARGETS
    let vertex = morph_vertex(vertex_no_morph, instance);
#else
    let vertex = vertex_no_morph;
#endif
#ifdef SKINNED
    let world_from_local = skinning::skin_model(vertex.joint_indices, vertex.joint_weights, instance);
#else
    let world_from_local = mesh_functions::get_world_from_local(instance);
#endif
    out.world_position = mesh_functions::mesh_position_local_to_world(world_from_local, vec4(vertex.position, 1.0));
    out.position = position_world_to_clip(out.world_position.xyz);
#ifdef GS_GUARD
    // VU1's reject pass drops every court triangle with a vertex outside the GS guard band,
    // 2048 ± 1280 in GS coordinates: ±1280/278.5 half-widths across (at the original's 4:3, one half-width — tan of
    // the 10° half field of view — is 278.5 GS pixels, not 320; B168: court 7's under-floor box, GS dump) and
    // ±1280/112 half-heights of the 224-line field. That is what hides w-house2 (Grand Palais) with the eye inside it. A NaN position drops every
    // triangle using the vertex.
    // ponytail: band size from a stage-7 GS dump (largest drawn |x − 2048| 1261), not the microcode's own constant
    // CLIP tests against |w|, so a vertex behind the eye is only flagged when it lies outside the band too.
    // Ground (facing up, under 1 m) is exempt: the GPU clips it, so a low replay camera keeps the floor in front of it
    // (B169); the original's floor would lose its big near triangles there too.
    let p = out.position;
    let across = view_bindings::view.clip_from_view[1][1] * 0.75 / view_bindings::view.clip_from_view[0][0];
#ifdef VERTEX_NORMALS
    let ground = out.world_position.y < 1.0 && normalize(mesh_functions::mesh_normal_local_to_world(vertex.normal, instance)).y > 0.7;
#else
    let ground = false;
#endif
    if !ground && (abs(p.x) * across > 1280.0 / 278.5 * abs(p.w) || abs(p.y) > 1280.0 / 112.0 * abs(p.w)) {
        let z = p.w * 0.0;
        out.position = vec4(z / z);
    }
#endif
#ifdef VERTEX_UVS_A
    out.uv = vertex.uv;
#endif
    // vertex colour × material colour (0x80 = 1.0)
#ifdef VERTEX_COLORS
    var c = vertex.color * gs.color;
#else
    var c = gs.color;
#endif
    // VU1 lighting, per vertex in game space (y down; GameSpace turns it 180° about X): ambient + each light's colour
    // × its diffuse term (the first dimmed with the ambient when the camera looks near it, `glare`), and a specular term on the half vector between the light and the camera's forward axis,
    // Schlick's t/(k − (k − 1)t) for t^k. A skinned vertex's normal is the sum of its bones' pre-weighted normals
    // (|n| = the weight), each turned by its bone, not normalised; the second bone's rides in the tangent slot
    // (character.rs). A mod's vertex has one normal N for up to four bones: w0·N in the normal slot, (1 − w0)·N in
    // the tangent slot turned by bones 1..3 blended by their weights (bone 1 alone on the disc), so Σ wᵢ·Rᵢ·N.
    // Meshes without normals (the weather's particles) go unlit.
    out.spec = 0.0;
#ifdef VERTEX_NORMALS
#ifdef SKINNED
    let j0 = skinning::skin_model(vertex.joint_indices, vec4(1.0, 0.0, 0.0, 0.0), instance);
    var n = mat3x3(j0[0].xyz, j0[1].xyz, j0[2].xyz) * vertex.normal;
#ifdef VERTEX_TANGENTS
    let rest = vertex.joint_weights.y + vertex.joint_weights.z + vertex.joint_weights.w;
    let j1 = skinning::skin_model(vertex.joint_indices, select(vec4(0.0, 1.0, 0.0, 0.0), vec4(0.0, vertex.joint_weights.yzw / rest), rest > 0.0), instance);
    n += mat3x3(j1[0].xyz, j1[1].xyz, j1[2].xyz) * vertex.tangent.xyz;
#endif
#else
    var n = mesh_functions::mesh_normal_local_to_world(vertex.normal, instance);
#endif
    let flip = vec3(1.0, -1.0, -1.0);
    n *= flip;
    // unlit (object flag bit 1): VU1 skips the normal transform, N' = (1, 0, 0): full light, no second light or highlight
    let lit = 1.0 - gs.unlit;
    let diffuse = select(max(-dot(n, gs.light_dir.xyz), 0.0), 1.0, gs.unlit != 0.0);
    let forward = -view_bindings::view.world_from_view[2].xyz * flip;
    let h = lit * max(-dot(n, normalize(gs.light_dir.xyz + forward)), 0.0);
    out.spec = gs.highlight * h / (gs.shininess - (gs.shininess - 1.0) * h);
    let x = gs.glare * max((-dot(forward, gs.light_dir.xyz) - 0.8) / 0.2, 0.0);
    let diffuse2 = lit * max(-dot(n, gs.light2_dir.xyz), 0.0);
    // B343: a court creature sharing its draws with others of its model carries its own light scale in its tag (the
    // scale's bits, top bit set; `shade::shade_tag`), the others have it in `light_color` already
    let tag = mesh_functions::get_tag(instance);
    let scale = select(1.0, bitcast<f32>(tag & 0x7fffffffu), (tag & 0x80000000u) != 0u);
    c = vec4(c.rgb * (gs.ambient.rgb * (1.0 - x * 0.5) + gs.light_color.rgb * scale * (1.0 - 0.6 * x) * diffuse + gs.light2_color.rgb * diffuse2), c.a);
#ifdef GS_AO
    let fill = dot(gs.ambient.rgb * (1.0 - x * 0.5) + gs.light2_color.rgb * diffuse2, vec3(1.0));
    let sun = dot(gs.light_color.rgb * scale * (1.0 - 0.6 * x) * diffuse, vec3(1.0));
    out.ambient = select(1.0, fill / (fill + sun), fill + sun > 0.0);
#endif
#else
#ifdef GS_AO
    out.ambient = 1.0;
#endif
#endif
    // VU1: F from the vertex's view depth (clip w), at most F near, then at least F far (MINI, MAX), FTOI to an
    // integer; both ends go towards 255 as the eye rises past 30 m (game y = −Bevy y)
    let vz = view_bindings::view.view_from_world;
    let w = -dot(vec4(vz[0].z, vz[1].z, vz[2].z, vz[3].z), out.world_position);
    let lift = clamp((view_bindings::view.world_position.y - 30.0) * 0.02, 0.0, 0.7);
    let ends = gs.fog.xy + lift * (255.0 - gs.fog.xy);
    let f = floor(max(min(ends.x + (w - gs.fog.z) * (ends.y - ends.x) / (gs.fog.w - gs.fog.z), ends.x), ends.y));
#ifdef GS_NO_LINEAR
    out.fog = vec2(f, 1.0) * out.position.w;
#else
    out.fog = f;
#endif
    // FTOI0 to the GS's 8 bits
    out.color = floor(min(c * 128.0, vec4(255.0)) + 1e-3) / 128.0;
    return out;
}

#ifdef GS_SOFT
// G4 (Soft Shadows, not the original's): contact-hardening edges. A blocker search finds how high above the ground
// the caster is along the sun; the penumbra's radius grows SOFT_SPREAD metres a metre of it, up to SOFT_MAX, so a
// player's feet stay sharp and a tall tree's canopy blurs. `graphics::soft_shadows` turns it on by giving the view
// Bevy's Temporal filter (its key's only spare state); this replaces Bevy's filter, so there is no temporal noise.
// (the real sun's is ~0.005; 3× that reads as soft without washing out a tall pole's shadow)
const SOFT_SPREAD: f32 = 0.015;
const SOFT_MAX: f32 = 0.25;

fn soft_cascade(i: u32, c: u32, pos: vec4<f32>) -> f32 {
    let light = &view_bindings::lights.directional_lights[i];
    let cascade = &(*light).cascades[c];
    let texel = (*cascade).texel_size;
    // Bevy's biases (`sample_directional_cascade`), the ground's normal up
    let p = vec4(pos.xyz + (*light).shadow_normal_bias * texel * vec3(0.0, 1.0, 0.0) + (*light).shadow_depth_bias * (*light).direction_to_light.xyz, pos.w);
    let ll = shadows::world_to_directional_light_local(i, c, p);
    if ll.w == 0.0 {
        return 1.0;
    }
    let layer = i32((*light).depth_texture_base_index + c);
    let size = vec2<f32>(textureDimensions(view_bindings::directional_shadow_textures));
    // the cascade is orthographic: shadow-map depth per metre along the light is its projection's z row's length
    let m = (*cascade).clip_from_world;
    let per_metre = length(vec3(m[0].z, m[1].z, m[2].z));
    // blocker search, 6×6 texels over the widest penumbra (at most ~2 texels apart, so the estimate doesn't jump): the casters' mean depth (reversed Z: nearer the sun is
    // larger; the ground itself doesn't cast)
    let reach = SOFT_MAX / texel;
    var sum = 0.0;
    var n = 0.0;
    for (var y = 0; y < 6; y++) {
        for (var x = 0; x < 6; x++) {
            let o = (vec2(f32(x), f32(y)) - 2.5) * 0.4 * reach;
            let t = clamp(vec2<i32>(floor(ll.xy * size + o)), vec2(0), vec2<i32>(size) - 1);
            let d = textureLoad(view_bindings::directional_shadow_textures, t, layer, 0);
            if d > ll.z {
                sum += d;
                n += 1.0;
            }
        }
    }
    let height = select(0.0, (sum / max(n, 1.0) - ll.z) / per_metre, n > 0.0);
    // PCF over the penumbra: k×k bilinear comparisons at most ~2 texels apart (k grows with it, fewer near contact)
    let r = clamp(height * SOFT_SPREAD / texel, 1.0, reach);
    let k = clamp(i32(ceil(r)) + 1, 3, 9);
    let step = 2.0 * r / f32(k - 1);
    var lit = 0.0;
    for (var y = 0; y < k; y++) {
        for (var x = 0; x < k; x++) {
            let o = (vec2(f32(x), f32(y)) * step - r) / size;
            lit += shadow_sampling::sample_shadow_map_hardware(ll.xy + o, ll.z, layer);
        }
    }
    return lit / f32(k * k);
}

// as Bevy's `fetch_directional_shadow`, each cascade through `soft_cascade`
fn soft_shadow(i: u32, pos: vec4<f32>, view_z: f32) -> f32 {
    let light = &view_bindings::lights.directional_lights[i];
    let c = shadows::get_cascade_index(i, view_z);
    if c >= (*light).num_cascades {
        return 1.0;
    }
    var shadow = soft_cascade(i, c, pos);
    if c + 1u < (*light).num_cascades {
        let far = (*light).cascades[c].far_bound;
        let near = (1.0 - (*light).cascades_overlap_proportion) * far;
        if -view_z >= near {
            shadow = mix(shadow, soft_cascade(i, c + 1u, pos), (-view_z - near) / (far - near));
        }
    }
    return shadow;
}
#endif

// 1 lit, 0 in shadow: the first directional light that casts shadows (the match's sun)
fn sun_visibility(in: VertexOutput) -> f32 {
    for (var i = 0u; i < view_bindings::lights.n_directional_lights; i++) {
        if (view_bindings::lights.directional_lights[i].flags & mesh_view_types::DIRECTIONAL_LIGHT_FLAGS_SHADOWS_ENABLED_BIT) != 0u {
            let v = view_bindings::view.view_from_world;
            let view_z = dot(vec4(v[0].z, v[1].z, v[2].z, v[3].z), in.world_position);
#ifdef GS_SOFT
            return soft_shadow(i, in.world_position, view_z);
#else
            // receivers are the flat ground: up is its normal
            return shadows::fetch_directional_shadow(i, in.world_position, vec3(0.0, 1.0, 0.0), view_z, in.position.xy);
#endif
        }
    }
    return 1.0;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    // the vertex's lit colour, Gouraud-shaded
    var rgb = in.color.rgb;
    var a = in.color.a;
    let spec = in.spec;
#ifdef GS_TEXTURED
    // TEX1 as the game sets it: LCM 0, L 0, K per model material, MMIN linear-mipmap-nearest, MXL = the levels
    // uploaded. Q = 1/w (VU1), w the view depth (m), so LOD = log2(w) + K, rounded off to a level; the sampler
    // clamps it to the last level (MXL; textureNumLevels doesn't exist on WebGL2)
    let vz = view_bindings::view.view_from_world;
    let w = -dot(vec4(vz[0].z, vz[1].z, vz[2].z, vz[3].z), in.world_position);
    var level = max(floor(log2(w) + gs.lod_k + 0.5), 0.0);
    // a cut-out the GS samples at level 0 only, given levels of our own (B161): trilinear by the texel's size on
    // screen, one level up, so a 2-texel wire grid is averaged before it beats against the pixel grid (moiré); the
    // original at 448 lines sees it ~1.5–2 levels coarser still
    let texels = (in.uv + gs.uv_offset) * vec2<f32>(textureDimensions(gs_texture, 0));
    let dx = dpdx(texels);
    let dy = dpdy(texels);
    if gs.screen_lod != 0.0 {
        level = max(0.5 * log2(max(dot(dx, dx), dot(dy, dy))) + 1.0, 0.0);
    }
    let t = textureSampleLevel(gs_texture, gs_sampler, in.uv + gs.uv_offset, level);
    rgb = t.rgb * rgb;
#ifdef GS_MODULATE
    // Av = (At·Af) >> 7 on the GS's integers: At 0..0x80 (stored ·255/0x80), Af the interpolated vertex alpha,
    // truncated (+1/64 takes up float error, so a constant 0x80 stays 0x80 and passes A ≥ 0x80)
    a = round(t.a * 128.0) * floor(a * 128.0 + 1.0 / 64.0) / 16384.0;
#else
    // HIGHLIGHT2: + vertex alpha (0..0xff), which carries the highlight
    a = t.a;
    rgb += min(spec, 1.0);
#endif
#else
    // untextured: the GS draws VU1's 8-bit colour as it is (0x80 is 128 of 255, not 1.0 as MODULATE makes it)
#ifdef GS_HDR
    rgb = (rgb + spec * 255.0 / 128.0) * 128.0 / 255.0;
#else
    rgb = min(rgb + spec * 255.0 / 128.0, vec3(255.0 / 128.0)) * 128.0 / 255.0;
#endif
#endif
    // alpha test on the 8-bit value
    let a8 = floor(a * 128.0);
#ifdef GS_GE40
    if a8 < 64.0 { discard; }
#endif
#ifdef GS_GE60
    if a8 < 96.0 { discard; }
#endif
#ifdef GS_LT60
    if a8 >= 96.0 { discard; }
#endif
#ifdef GS_GE70
    if a8 < 112.0 { discard; }
#endif
#ifdef GS_LT70
    if a8 >= 112.0 { discard; }
#endif
#ifdef GS_GE80
    if a8 < 128.0 { discard; }
#endif
#ifdef GS_LT80
    if a8 >= 128.0 { discard; }
#endif
#ifdef GS_HDR
    // G5: the GS's 8-bit clamp left to the tonemapper, which rolls off what is brighter than 1 (hud_gamma.rs)
    rgb = max(rgb, vec3(0.0));
#else
    rgb = clamp(rgb, vec3(0.0), vec3(1.0));
#endif
#ifdef GS_AO
    // G1 (not the original's): the SSAO's visibility takes off the ambient share of the light, before the fog
    // (Bevy's GTAO reads ≈ 0.89 on the open, flat court seen from the match camera: that is 1, unoccluded)
    let ao = min(textureLoad(view_bindings::screen_space_ambient_occlusion_texture, vec2<i32>(in.position.xy), 0).r / 0.89, 1.0);
    rgb *= 1.0 - in.ambient * (1.0 - ao);
#endif
#ifdef GS_FOG
    // GS: the interpolated F truncated to 8 bits, (F·C + (255 − F)·FOGCOL) >> 8 on 8-bit values
#ifdef GS_NO_LINEAR
    let fog = floor(in.fog.x / in.fog.y);
#else
    let fog = floor(in.fog);
#endif
    rgb = (floor(fog * floor(rgb * 255.0) / 256.0) + floor((255.0 - fog) * floor(gs.fog_color.rgb * 255.0 + 0.5) / 256.0)) / 255.0;
#endif
    // shadow: (0 − Cd)·A + Cd on the frame buffer, the same as darkening every layer drawn there
    if gs.shadow > 0.0 {
        rgb *= 1.0 - gs.shadow * (1.0 - sun_visibility(in));
    }
#ifdef GS_PREMUL
    // `@add`/`@sub`: the GS adds Cs·As >> 7 on its integers, Cs (texel × vertex >> 7, clamped) and As truncated, so
    // the blend is One/One on this already-floored 8-bit term (the scene is Rgba8Unorm: k/255 lands on k exactly)
#ifdef GS_SUB
    // (0 − Cs)·As >> 7 shifts a negative: the GS takes off the term rounded up
    rgb = ceil(floor(rgb * 255.0 + 1e-4) * a8 / 128.0) / 255.0;
#else
    rgb = floor(floor(rgb * 255.0 + 1e-4) * a8 / 128.0) / 255.0;
#endif
#endif
    // the scene holds gamma-encoded values, as the GS's frame buffer (hud_gamma.rs; G5's HDR: past 1 too)
    return vec4(rgb, clamp(a, 0.0, 1.0));
}
