// The HUD (premultiplied, linear) over the scene, blended on the sRGB-encoded values as the GS does.
#import bevy_ui::ui_vertex_output::UiVertexOutput

@group(1) @binding(0) var scene: texture_2d<f32>;
@group(1) @binding(1) var hud: texture_2d<f32>;

fn enc(c: vec3<f32>) -> vec3<f32> {
    return select(1.055 * pow(c, vec3(1.0 / 2.4)) - 0.055, c * 12.92, c <= vec3(0.0031308));
}

fn dec(c: vec3<f32>) -> vec3<f32> {
    return select(pow((c + 0.055) / 1.055, vec3(2.4)), c / 12.92, c <= vec3(0.04045));
}

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    let p = vec2<i32>(in.position.xy);
    let b = textureLoad(scene, p, 0).rgb;
    let h = textureLoad(hud, p, 0);
    let c = select(vec3(0.0), h.rgb / h.a, h.a > 0.0);
    return vec4(dec(enc(c) * h.a + enc(b) * (1.0 - h.a)), 1.0);
}
