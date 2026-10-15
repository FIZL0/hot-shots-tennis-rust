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

#ifdef HDR
// G5's tonemap (hud_gamma.rs) on the encoded values: the identity up to the knee, above it a shoulder on the
// brightest channel, k + (1 − k)·x/(1 + x) for x = (m − k)/(1 − k) (slope 1 at the knee, 1 only at infinity), the
// other channels scaled alike, then towards white by the square of the share it took off
const KNEE: f32 = 0.8;

fn tonemap(c: vec3<f32>) -> vec3<f32> {
    let m = max(max(c.r, c.g), c.b);
    if m <= KNEE {
        return c;
    }
    let x = (m - KNEE) / (1.0 - KNEE);
    let f = KNEE + (1.0 - KNEE) * x / (1.0 + x);
    let off = 1.0 - f / m;
    return mix(c * (f / m), vec3(f), off * off);
}
#endif

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    // the scene is drawn at the render scale setting: filtered up to the window (at 100 % this hits texel centres);
    // the HUD at the window's size, or at 448 lines in the original mode (nearest)
    var b = textureSampleLevel(scene, scene_sampler, in.uv, 0.0).rgb;
#ifdef HDR
    b = tonemap(max(b, vec3(0.0)));
#endif
    let hd = vec2<f32>(textureDimensions(hud));
    let h = textureLoad(hud, vec2<i32>(min(in.uv * hd, hd - 1.0)), 0);
    let c = select(vec3(0.0), h.rgb / h.a, h.a > 0.0);
    return vec4(dec(enc(c) * h.a + b * (1.0 - h.a)), 1.0);
}
