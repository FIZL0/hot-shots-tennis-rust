//! Court models drawn the way the PS2's GS draws them: each material's registers as the game sets them up from its
//! MTL header and name, each batch's PRIM bits from the MDL.
//!
//! - ALPHA: default `(Cs − Cd)·As + Cd`, a name with `@add` `Cs·As + Cd`, with `@sub` `Cd − Cs·As` (tested last, wins).
//! - TEST (header +0x1e): 10..19 keep only A ≥ 0x40; 20..29 write Z only where A ≥ 0x70, colour everywhere; any
//!   other value passes everything. `@add`/`@sub` never pass the test and write colour without Z.
//! - TEX0 TFX: HIGHLIGHT2 (alpha = texture alpha) when the material colour's alpha is 0x80 and the name has no
//!   `@vert`, else MODULATE (alpha = texture × vertex alpha).
//! - PRIM (batch header +0x31): TME textured, ABE blended; FGE (fog) is not drawn yet.
//! - Colour = texture × vertex colour × material colour in 8-bit PS2 units (0x80 = 1.0), clamped, in gamma space.
//!
//! - Shadows (see shadow.rs): caster draws go into the sun's shadow map through `gs_prepass.wgsl`, which keeps the
//!   alpha test; receiver draws (`shadow` > 0) multiply their colour by 1 − `shadow` where the sun is blocked.
//!
//! ponytail: HIGHLIGHT2 adds the vertex alpha (the VU1 lighting's highlight term) to the colour; it is taken as 0,
//! which it is for every material whose header +0x14 is 0 (nearly all court ones). Port the VU1 lighting to add it.

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
}

#[derive(ShaderType, Clone, Copy, Debug)]
pub struct GsUniform {
    /// Material colour, 1.0 = 0x80.
    pub color: Vec4,
    /// Shadow receiver: how much a shadow takes off the colour (0 = not a receiver).
    pub shadow: f32,
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
        tests
            .into_iter()
            .map(|test| GsMaterial {
                uniform: GsUniform { color: Vec4::from(m.color), shadow: 0.0 },
                texture: texture.clone().filter(|_| textured),
                key: GsKey { textured, modulate, test, blend },
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
        assert_eq!(k, [GsKey { textured: true, modulate: false, test: Test::Always, blend: None }]);
        // mode 15 cut-out, translucent colour → MODULATE
        let k = keys(&material("leaf", 15, 0.5), 0x70);
        assert_eq!(k, [GsKey { textured: true, modulate: true, test: Test::Ge40, blend: Some(mtl::Blend::Normal) }]);
        // mode 25: two draws
        assert_eq!(keys(&material("fence@vert", 25, 1.0), 0x70).iter().map(|k| k.test).collect::<Vec<_>>(), [Test::Ge70, Test::Lt70]);
        // court lines: @add without ABE still adds, no Z; @sub wins over @add; untextured batch
        let k = keys(&material("line@add@sub", 15, 0.4), 0x20);
        assert_eq!(k, [GsKey { textured: false, modulate: true, test: Test::Never, blend: Some(mtl::Blend::Sub) }]);
    }
}
