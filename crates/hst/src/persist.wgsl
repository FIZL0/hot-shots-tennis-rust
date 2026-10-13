// The match's frame persistence: this frame's scene with `weight.x` of the last frame shown mixed in, on the
// encoded values as the GS blends (Cd + (Cs − Cd)·FIX, Cs the last frame, FIX 0x20 = 25 % at 60 Hz).
// Facing the sun (`weight.y` > 0) the sun glare instead: the frame drawn over itself twice at FIX `weight.y`,
// bilinear at (−½, −½) then (+½, +½) texels of the game's 640×224 field (`weight.zw` = one such texel in uv), so
// each pass mixes in the 2×2 average next to every pixel.
#import bevy_ui::ui_vertex_output::UiVertexOutput

@group(1) @binding(0) var scene: texture_2d<f32>;
@group(1) @binding(1) var last: texture_2d<f32>;
@group(1) @binding(2) var<uniform> weight: vec4<f32>;
@group(1) @binding(3) var linear: sampler;

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
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    if weight.y > 0.0 {
        let d = weight.zw;
        let p = first(in.uv);
        let avg = (p + first(in.uv + vec2(d.x, 0.0)) + first(in.uv + vec2(0.0, d.y)) + first(in.uv + d)) * 0.25;
        return vec4(mix(p, avg, weight.y), 1.0);
    }
    let p = vec2<i32>(in.uv * vec2<f32>(textureDimensions(scene)));
    return vec4(mix(textureLoad(scene, p, 0).rgb, textureLoad(last, p, 0).rgb, weight.x), 1.0);
}
