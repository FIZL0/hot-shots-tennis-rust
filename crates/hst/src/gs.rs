//! Court models drawn the way the PS2's GS draws them: each material's registers as the game sets them up from its
//! MTL header and name, each batch's PRIM bits from the MDL.
//!
//! - ALPHA: default `(Cs − Cd)·As + Cd`, a name with `@add` `Cs·As + Cd`, with `@sub` `Cd − Cs·As` (tested last, wins).
//! - TEST (header +0x1e): 10..19 keep only A ≥ 0x40; 20..29 write Z only where A ≥ 0x70, colour everywhere; any
//!   other value passes everything. `@add`/`@sub` never pass the test and write colour without Z.
//! - TEX0 TFX: HIGHLIGHT2 (alpha = texture alpha) when the material colour's alpha is 0x80 and the name has no
//!   `@vert`, else MODULATE (alpha = texture × vertex alpha).
//! - PRIM (batch header +0x31): TME textured, ABE blended, FGE fogged (every court batch, sky included).
//! - Fog: `envir_cNN.dat` (CMN.XB) row k (time of day) at 0x290 + k·0x60: byte 0 set turns it off, FOGCOL RGB at
//!   +0x18, F at the near/far end +0x28/+0x2c, view depths +0x38/+0x3c. VU1 gives each vertex F = the depth (clip
//!   w) mapped linearly from near to far and clamped between the two values; the GS takes ⌊F⌋ and draws
//!   (F·C + (255 − F)·FOGCOL) >> 8. With the eye higher than 30 m both ends move 2 %/m (up to 70 %) towards 255.
//! - TEX1: MMAG linear, MMIN linear-mipmap-nearest, LCM 0, L 0, MXL from the texture header (mtl.rs), K from the
//!   model's material header (mdl.rs). Q = 1/w (VU1), so a pixel's level is ⌊log2(view depth m) + K + ½⌋ clamped
//!   to 0..MXL.
//! - Colour = texture × vertex colour × material colour × light in 8-bit PS2 units (0x80 = 1.0), clamped, in gamma
//!   space. VU1 lights each vertex with one fixed directional light and an ambient (gs.wgsl); its specular term,
//!   scaled by header +0x14, goes out as the vertex alpha, which HIGHLIGHT2 adds to the colour (untextured: added to
//!   the colour directly). The specular exponent is max(1, 128·(header +0x10)^1.65).
//!
//! - Shadows (see shadow.rs): caster draws go into the sun's shadow map through `gs_prepass.wgsl`, which keeps the
//!   alpha test; receiver draws (`shadow` > 0) multiply their colour by 1 − `shadow` where the sun is blocked.
//!
//! ponytail: lit per pixel, not per vertex as VU1 does; the same where the light is flat across a triangle.
//! ponytail: textured MODULATE draws keep vertex × material alpha; on VU1 their vertex alpha is the highlight too.

use bevy::mesh::MeshVertexBufferLayoutRef;
use bevy::pbr::{MaterialPipeline, MaterialPipelineKey};
use bevy::prelude::*;
use bevy::render::render_resource::{
    AsBindGroup, BlendComponent, BlendFactor, BlendOperation, BlendState, RenderPipelineDescriptor, ShaderType,
    SpecializedMeshPipelineError,
};
use bevy::shader::ShaderRef;
use hst_data::mtl;

pub fn plugin(app: &mut App) {
    bevy::asset::embedded_asset!(app, "gs.wgsl");
    bevy::asset::embedded_asset!(app, "gs_prepass.wgsl");
    app.add_plugins(MaterialPlugin::<GsMaterial>::default());
}

/// Which pixels pass the alpha test, and what a failing one writes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Test {
    /// Every pixel, colour and Z.
    Always,
    /// A ≥ 0x40, colour and Z; the rest nothing.
    Ge40,
    /// A ≥ 0x70, colour and Z (the Z-writing half of TEST mode 20..29).
    Ge70,
    /// A < 0x70, colour only (its other half).
    Lt70,
    /// Nothing passes: every pixel writes colour only.
    Never,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct GsKey {
    pub textured: bool,
    /// TFX MODULATE (else HIGHLIGHT2).
    pub modulate: bool,
    pub test: Test,
    /// ABE: `None` replaces the frame buffer.
    pub blend: Option<mtl::Blend>,
    /// PRIM FGE.
    pub fog: bool,
}

#[derive(ShaderType, Clone, Copy, Debug)]
pub struct GsUniform {
    /// Material colour, 1.0 = 0x80.
    pub color: Vec4,
    /// Specular exponent, max(1, 128·(header +0x10)^1.65).
    pub shininess: f32,
    /// Highlight strength, header +0x14.
    pub highlight: f32,
    /// Shadow receiver: how much a shadow takes off the colour (0 = not a receiver).
    pub shadow: f32,
    /// Texture coordinate offset (a court `.UVA`).
    pub uv_offset: Vec2,
    /// F at the near and far depth, the near and far view depth (m); `NO_FOG` draws as without FGE.
    pub fog: Vec4,
    /// FOGCOL, 1.0 = 0xff.
    pub fog_color: Vec4,
    /// TEX1 K, the mipmap LOD bias (the model's material header, `mdl::Model::lod_k`).
    pub lod_k: f32,
}

/// Fog parameters that leave every pixel as it is.
pub const NO_FOG: Vec4 = Vec4::new(255.0, 255.0, 0.0, 1.0);

/// A court's fog for season `k` (1 in singles, 0 in doubles) from its `envir_cNN.dat`: (`GsUniform::fog`, `fog_color`).
pub fn court_fog(envir: &[u8], k: usize) -> Option<(Vec4, Vec4)> {
    let row = envir.get(0x290 + k * 0x60..0x2f0 + k * 0x60)?;
    let f = |o: usize| f32::from_le_bytes(row[o..o + 4].try_into().unwrap());
    let colour = Vec4::new(row[0x18] as f32, row[0x19] as f32, row[0x1a] as f32, 255.0) / 255.0;
    // off: the game sets (255, 255, 0, 0.001)
    let (near, far, z0, mut z1) = if row[0] != 0 { (255.0, 255.0, 0.0, 0.001) } else { (f(0x28), f(0x2c), f(0x38), f(0x3c)) };
    if z1 <= z0 {
        z1 = z0 + 1.0;
    }
    Some((Vec4::new(near, far, z0, z1), colour))
}

/// The colour the game clears the screen to before a court frame (what shows above the sky dome's open top).
/// `sky` is the drawn sky's top-ring vertex colour × material colour / 128 (0..255). Per season the file has a
/// time-of-day threshold byte at 0x10 (sub-row 1 when it is below the hole, 1) and two light rows at 0xd0
/// (RGB, intensity, ambient); the fog row's F at +0x34 mixes in FOGCOL.
/// ponytail: clear weather (0/1); weather 2..5 grey the light and white the fog (P17i). The eye-height term that
/// lifts F only matters off court (0 for a match camera).
pub fn court_clear(envir: &[u8], season: usize, sky: [f32; 3]) -> Option<[u8; 3]> {
    let f = |o: usize| Some(f32::from_le_bytes(envir.get(o..o + 4)?.try_into().ok()?));
    let t = ((*envir.get(0x10 + season * 0x30)? as i8) < 1) as usize;
    let light = 0xd0 + season * 0x70 + t * 0x38;
    let (fog, k) = (0x290 + season * 0x60, f(light + 0xc)? + f(light + 0x10)?);
    let big_f = f(fog + 0x34)?;
    let mut out = [0; 3];
    for i in 0..3 {
        let v = sky[i] * f(light + 4 * i)? * k * big_f + *envir.get(fog + 0x18 + i)? as f32 * (255.0 - big_f);
        out[i] = (v / 255.0).min(255.0) as u8;
    }
    Some(out)
}

#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
#[bind_group_data(GsKey)]
pub struct GsMaterial {
    #[uniform(0)]
    pub uniform: GsUniform,
    /// Raw (non-sRGB) texels: the GS works on the stored values.
    #[texture(1)]
    #[sampler(2)]
    pub texture: Option<Handle<Image>>,
    pub key: GsKey,
}

impl From<&GsMaterial> for GsKey {
    fn from(m: &GsMaterial) -> Self {
        m.key
    }
}

impl GsMaterial {
    /// The draws (one, or two for TEST mode 20..29) of material `m` in a batch with PRIM bits `prim`.
    pub fn for_batch(m: &mtl::Material, prim: u8, texture: Option<Handle<Image>>) -> Vec<GsMaterial> {
        let textured = prim & 0x10 != 0 && texture.is_some();
        let opaque_colour = m.color[3] * 128.0;
        let modulate = !(opaque_colour as i32 == 0x80 && !m.name.contains("@vert"));
        // `@add`/`@sub` batches without ABE (the court lines) still blend on the PS2 (compared on court 10)
        let blend = (prim & 0x40 != 0 || m.blend() != mtl::Blend::Normal).then(|| m.blend());
        let mode = i16::from_le_bytes([m.header[0x1e], m.header[0x1f]]);
        let tests = if m.blend() != mtl::Blend::Normal {
            vec![Test::Never]
        } else {
            match mode {
                10..=19 => vec![Test::Ge40],
                20..=29 => vec![Test::Ge70, Test::Lt70],
                _ => vec![Test::Always],
            }
        };
        let f = |o: usize| f32::from_le_bytes(m.header[o..o + 4].try_into().unwrap());
        let uniform = GsUniform { color: Vec4::from(m.color), shininess: (128.0 * f(0x10).powf(1.65)).max(1.0), highlight: f(0x14), shadow: 0.0, uv_offset: Vec2::ZERO, fog: NO_FOG, fog_color: Vec4::ONE, lod_k: 0.0 };
        tests
            .into_iter()
            .map(|test| GsMaterial {
                uniform,
                texture: texture.clone().filter(|_| textured),
                key: GsKey { textured, modulate, test, blend, fog: prim & 0x20 != 0 },
            })
            .collect()
    }
}

impl Material for GsMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://hst/gs.wgsl".into()
    }

    fn prepass_fragment_shader() -> ShaderRef {
        "embedded://hst/gs_prepass.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        // blended draws and those that write no Z (e.g. the unblended `line@add` court lines) go after the opaque
        // ones, back to front, as the PS2 draws them after the ground they lie on; specialize sets the real blend.
        // Alpha-tested ones are Mask so the shadow pass runs the test too.
        if self.key.blend.is_some() || matches!(self.key.test, Test::Lt70 | Test::Never) {
            AlphaMode::Blend
        } else if self.key.test == Test::Always {
            AlphaMode::Opaque
        } else {
            AlphaMode::Mask(0.5)
        }
    }

    fn enable_prepass() -> bool {
        false
    }

    fn specialize(
        _pipeline: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        let k = key.bind_group_data;
        descriptor.primitive.cull_mode = None;
        if let Some(ds) = descriptor.depth_stencil.as_mut() {
            ds.depth_write_enabled = Some(!matches!(k.test, Test::Lt70 | Test::Never));
        }
        // the shadow pass has no fragment stage for draws that cannot discard
        let Some(fragment) = descriptor.fragment.as_mut() else { return Ok(()) };
        let defs = &mut fragment.shader_defs;
        if k.textured {
            defs.push("GS_TEXTURED".into());
        }
        if k.modulate {
            defs.push("GS_MODULATE".into());
        }
        match k.test {
            Test::Ge40 => defs.push("GS_GE40".into()),
            Test::Ge70 => defs.push("GS_GE70".into()),
            Test::Lt70 => defs.push("GS_LT70".into()),
            Test::Always | Test::Never => {}
        }
        if k.fog {
            defs.push("GS_FOG".into());
        }
        if matches!(k.test, Test::Lt70 | Test::Never) {
            defs.push("GS_NO_Z".into());
        }
        let keep_alpha = BlendComponent { src_factor: BlendFactor::Zero, dst_factor: BlendFactor::One, operation: BlendOperation::Add };
        let color = |dst_factor, operation| BlendComponent { src_factor: BlendFactor::SrcAlpha, dst_factor, operation };
        let blend = k.blend.map(|b| BlendState {
            color: match b {
                mtl::Blend::Normal => color(BlendFactor::OneMinusSrcAlpha, BlendOperation::Add),
                mtl::Blend::Add => color(BlendFactor::One, BlendOperation::Add),
                // dst·1 − src·As
                mtl::Blend::Sub => color(BlendFactor::One, BlendOperation::ReverseSubtract),
            },
            alpha: keep_alpha,
        });
        for t in fragment.targets.iter_mut().flatten() {
            t.blend = blend;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn material(name: &str, mode: i16, alpha: f32) -> mtl::Material {
        let mut header = [0; 0x30];
        header[0x1e..0x20].copy_from_slice(&mode.to_le_bytes());
        mtl::Material { texture: Some(0), color: [1.0, 1.0, 1.0, alpha], attributes: None, two_sided: false, name: name.into(), header }
    }

    #[test]
    fn for_batch_keys() {
        let tex = Some(Handle::default());
        let keys = |m: &mtl::Material, prim| GsMaterial::for_batch(m, prim, tex.clone()).iter().map(|g| g.key).collect::<Vec<_>>();
        // opaque material, textured unblended batch: HIGHLIGHT2, no test
        let k = keys(&material("grass", 5, 1.0), 0x30);
        assert_eq!(k, [GsKey { textured: true, modulate: false, test: Test::Always, blend: None, fog: true }]);
        // mode 15 cut-out, translucent colour → MODULATE
        let k = keys(&material("leaf", 15, 0.5), 0x70);
        assert_eq!(k, [GsKey { textured: true, modulate: true, test: Test::Ge40, blend: Some(mtl::Blend::Normal), fog: true }]);
        // mode 25: two draws
        assert_eq!(keys(&material("fence@vert", 25, 1.0), 0x70).iter().map(|k| k.test).collect::<Vec<_>>(), [Test::Ge70, Test::Lt70]);
        // court lines: @add without ABE still adds, no Z; @sub wins over @add; untextured batch
        let k = keys(&material("line@add@sub", 15, 0.4), 0x20);
        assert_eq!(k, [GsKey { textured: false, modulate: true, test: Test::Never, blend: Some(mtl::Blend::Sub), fog: true }]);
        // specular exponent 128·(+0x10)^1.65, at least 1; highlight +0x14
        let mut m = material("crayline", 5, 1.0);
        m.header[0x10..0x14].copy_from_slice(&0.49f32.to_le_bytes());
        m.header[0x14..0x18].copy_from_slice(&0.9f32.to_le_bytes());
        let u = GsMaterial::for_batch(&m, 0x30, tex.clone())[0].uniform;
        assert!((u.shininess - 39.46).abs() < 0.05 && u.highlight == 0.9, "{u:?}");
        assert_eq!(GsMaterial::for_batch(&material("x", 5, 1.0), 0x30, tex.clone())[0].uniform.shininess, 1.0);
        assert!(!keys(&material("x", 5, 1.0), 0x10)[0].fog);
    }

    #[test]
    fn court_fog_row() {
        // court 10's row 0 as the game holds it in RAM during a match (slot 5): FOGCOL 0xdcd1b5, (255, 224.4, 40, 190)
        let mut envir = vec![0; 0x550];
        envir[0x2a8..0x2ab].copy_from_slice(&[0xb5, 0xd1, 0xdc]);
        for (o, v) in [(0x2b8, 255.0f32), (0x2bc, 224.4), (0x2c8, 40.0), (0x2cc, 190.0)] {
            envir[o..o + 4].copy_from_slice(&v.to_le_bytes());
        }
        let (fog, colour) = court_fog(&envir, 0).unwrap();
        assert_eq!(fog, Vec4::new(255.0, 224.4, 40.0, 190.0));
        assert_eq!((colour * 255.0).round(), Vec4::new(181.0, 209.0, 220.0, 255.0));
        envir[0x290] = 1;
        assert_eq!(court_fog(&envir, 0).unwrap().0, Vec4::new(255.0, 255.0, 0.0, 0.001));
    }

    #[test]
    fn court_clear_colour() {
        // court 10 season 0 (its envir values) with greece00's sky (7, 17, 43) × 255/128: slot 5 RAM clears to 0x672811
        let mut envir = vec![0; 0x550];
        envir[0x10] = 18;
        for (o, v) in [(0xd0, 0.8627451f32), (0xd4, 0.8392157), (0xd8, 0.8392157), (0xdc, 0.74), (0xe0, 0.7), (0x2c4, 255.0)] {
            envir[o..o + 4].copy_from_slice(&v.to_le_bytes());
        }
        envir[0x2a8..0x2ab].copy_from_slice(&[0xb5, 0xd1, 0xdc]);
        let sky = [7.0, 17.0, 43.0].map(|c: f32| c * 255.0 / 128.0);
        assert_eq!(court_clear(&envir, 0, sky), Some([0x11, 0x28, 0x67]));
        // threshold 0 (court 9 season 0) picks the second light row, all zero here
        envir[0x10] = 0;
        assert_eq!(court_clear(&envir, 0, sky), Some([0, 0, 0]));
    }
}
