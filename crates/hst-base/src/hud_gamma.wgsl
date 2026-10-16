// The window's last two passes (`hud_gamma/out.rs`), full-screen triangles: with PERSIST the frame persistence,
// else the composite.
#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

#ifdef PERSIST
// the match's frame persistence: this frame's scene with `weight.x` of the last frame shown mixed in, on the
// encoded values as the GS blends (Cd + (Cs − Cd)·FIX, Cs the last frame, FIX 0x20 = 25 % at 60 Hz).
// Facing the sun (`weight.y` > 0) the sun glare instead: the frame drawn over itself twice at FIX `weight.y`,
// bilinear at (−½, −½) then (+½, +½) texels of the game's 640×224 field (`weight.zw` = one such texel in uv), so
// each pass mixes in the 2×2 average next to every pixel.
@group(0) @binding(0) var scene: texture_2d<f32>;
@group(0) @binding(1) var last: texture_2d<f32>;
@group(0) @binding(2) var<uniform> weight: vec4<f32>;
@group(0) @binding(3) var linear: sampler;

fn f(uv: vec2<f32>) -> vec3<f32> {
    return textureSampleLevel(scene, linear, uv, 0.0).rgb;
}

// the first pass's result at uv: mixed with the average of its up-left 2×2
fn first(uv: vec2<f32>) -> vec3<f32> {
    let d = weight.zw;
    let avg = (f(uv) + f(uv - vec2(d.x, 0.0)) + f(uv - vec2(0.0, d.y)) + f(uv - d)) * 0.25;
    return mix(f(uv), avg, weight.y);
}

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    if weight.y > 0.0 {
        let d = weight.zw;
        let p = first(in.uv);
        let avg = (p + first(in.uv + vec2(d.x, 0.0)) + first(in.uv + vec2(0.0, d.y)) + first(in.uv + d)) * 0.25;
        return vec4(mix(p, avg, weight.y), 1.0);
    }
    let p = vec2<i32>(in.position.xy);
    return vec4(mix(textureLoad(scene, p, 0).rgb, textureLoad(last, p, 0).rgb, weight.x), 1.0);
}
#else

// the composite: the HUD (premultiplied, linear) over the scene (gamma-encoded), blended on the encoded values as the
// GS does.
@group(0) @binding(0) var shown: texture_2d<f32>;
@group(0) @binding(1) var hud: texture_2d<f32>;
@group(0) @binding(2) var shown_sampler: sampler;

fn enc(c: vec3<f32>) -> vec3<f32> {
    return select(1.055 * pow(c, vec3(1.0 / 2.4)) - 0.055, c * 12.92, c <= vec3(0.0031308));
}

fn dec(c: vec3<f32>) -> vec3<f32> {
    return select(pow((c + 0.055) / 1.055, vec3(2.4)), c / 12.92, c <= vec3(0.04045));
}

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    // the scene is drawn at the render scale setting: filtered up to the window (at 100 % this hits texel centres);
    // the HUD at the window's size, or at 448 lines in the original mode (nearest)
    let b = textureSampleLevel(shown, shown_sampler, in.uv, 0.0).rgb;
    let hd = vec2<f32>(textureDimensions(hud));
    let h = textureLoad(hud, vec2<i32>(min(in.uv * hd, hd - 1.0)), 0);
    let c = select(vec3(0.0), h.rgb / h.a, h.a > 0.0);
    return vec4(dec(enc(c) * h.a + b * (1.0 - h.a)), 1.0);
}
#endif
