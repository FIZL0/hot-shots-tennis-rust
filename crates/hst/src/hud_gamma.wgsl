// The HUD (premultiplied, linear) over the scene (gamma-encoded), blended on the encoded values as the GS does.
#import bevy_ui::ui_vertex_output::UiVertexOutput

@group(1) @binding(0) var scene: texture_2d<f32>;
@group(1) @binding(1) var hud: texture_2d<f32>;
@group(1) @binding(2) var scene_sampler: sampler;

fn enc(c: vec3<f32>) -> vec3<f32> {
    return select(1.055 * pow(c, vec3(1.0 / 2.4)) - 0.055, c * 12.92, c <= vec3(0.0031308));
}

fn dec(c: vec3<f32>) -> vec3<f32> {
    return select(pow((c + 0.055) / 1.055, vec3(2.4)), c / 12.92, c <= vec3(0.04045));
}

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    // the scene is drawn at the render scale setting: filtered up to the window (at 100 % this hits texel centres);
    // the HUD at the window's size, or at 448 lines in the original mode (nearest)
    let b = textureSampleLevel(scene, scene_sampler, in.uv, 0.0).rgb;
    // B118: a HUD image the other way round from the scene's is laid out turned (`widescreen::turned`): its top
    // along the view's right edge
    let hd = vec2<f32>(textureDimensions(hud));
    let sd = vec2<f32>(textureDimensions(scene));
    let uv = select(in.uv, vec2(1.0 - in.uv.y, in.uv.x), (hd.x > hd.y) != (sd.x > sd.y));
    let h = textureLoad(hud, vec2<i32>(min(uv * hd, hd - 1.0)), 0);
    let c = select(vec3(0.0), h.rgb / h.a, h.a > 0.0);
    return vec4(dec(enc(c) * h.a + b * (1.0 - h.a)), 1.0);
}
