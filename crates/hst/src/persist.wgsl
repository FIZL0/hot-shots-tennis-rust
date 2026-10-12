// The match's frame persistence: this frame's scene with `weight.x` of the last frame shown mixed in, on the
// encoded values as the GS blends (Cd + (Cs − Cd)·FIX, Cs the last frame, FIX 0x20 = 25 % at 60 Hz).
#import bevy_ui::ui_vertex_output::UiVertexOutput

@group(1) @binding(0) var scene: texture_2d<f32>;
@group(1) @binding(1) var last: texture_2d<f32>;
@group(1) @binding(2) var<uniform> weight: vec4<f32>;

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    let p = vec2<i32>(in.uv * vec2<f32>(textureDimensions(scene)));
    return vec4(mix(textureLoad(scene, p, 0).rgb, textureLoad(last, p, 0).rgb, weight.x), 1.0);
}
