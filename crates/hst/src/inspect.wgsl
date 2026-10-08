// A preview's image (inspect.rs `PreviewMaterial`): GS-encoded colour, decoded for the sRGB HUD; alpha −1 = nothing drawn.
#import bevy_ui::ui_vertex_output::UiVertexOutput

@group(1) @binding(0) var image: texture_2d<f32>;
@group(1) @binding(1) var image_sampler: sampler;

fn dec(c: vec3<f32>) -> vec3<f32> {
    return select(pow((c + 0.055) / 1.055, vec3(2.4)), c / 12.92, c <= vec3(0.04045));
}

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    let c = textureSample(image, image_sampler, in.uv);
    // an opaque draw writes its alpha (0..1) over the −1; a blended one leaves 2a − 1
    return vec4(dec(clamp(c.rgb, vec3(0.0), vec3(1.0))), clamp(c.a + 1.0, 0.0, 1.0));
}
