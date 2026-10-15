//! The window's last camera (P45): instead of a 2D camera drawing a full-screen UI node (and two more for the
//! persistence), a camera of our own render schedule [`Out`] with one system, `draw`. It records this frame's
//! persistence mix into `last[i]` and the composite of that (or of the bare scene) with the HUD onto the window,
//! two passes in one command buffer. A Core2d camera costs a schedule run and up to four passes (clear, sprites,
//! UI, upscaling blit), each its own command buffer; the three it replaces were most of a frame's queue submit.

use bevy::core_pipeline::FullscreenShader;
use bevy::ecs::schedule::ScheduleLabel;
use bevy::prelude::*;
use bevy::render::camera::ExtractedCamera;
use bevy::render::extract_resource::{ExtractResource, ExtractResourcePlugin};
use bevy::render::render_asset::RenderAssets;
use bevy::render::render_resource::binding_types::{sampler, texture_2d, uniform_buffer};
use bevy::render::render_resource::*;
use bevy::render::renderer::{RenderContext, RenderDevice, RenderQueue, ViewQuery};
use bevy::render::texture::GpuImage;
use bevy::render::view::{ExtractedWindows, ViewTargetAttachments};
use bevy::render::{GpuResourceAppExt, Render, RenderApp, RenderSystems};

/// The window camera's render schedule.
#[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
pub struct Out;

/// What `draw` puts on the window this frame, set by `hud_gamma::persist`.
#[derive(Resource, Clone, ExtractResource)]
pub struct Frame {
    pub scene: Handle<Image>,
    pub hud: Handle<Image>,
    pub last: [Handle<Image>; 2],
    /// In a match: this frame's parity `i` (mix the scene and `last[1 - i]` into `last[i]`, show that) and the mix's
    /// weights (x: how much of the last frame; y: the sun glare's FIX, 0 = none; zw: its texel in uv).
    pub mix: Option<(usize, Vec4)>,
}

pub fn plugin(app: &mut App) {
    app.add_plugins(ExtractResourcePlugin::<Frame>::default());
    let Some(r) = app.get_sub_app_mut(RenderApp) else { return };
    let mut schedule = Schedule::new(Out);
    schedule.set_build_settings(bevy::ecs::schedule::ScheduleBuildSettings { auto_insert_apply_deferred: false, ..default() });
    r.add_schedule(schedule)
        .add_systems(Out, draw)
        .init_gpu_resource::<Pipeline>()
        .init_gpu_resource::<SpecializedRenderPipelines<Pipeline>>()
        .init_resource::<Ids>()
        .add_systems(Render, prepare.in_set(RenderSystems::Prepare));
}

/// The window camera: drawn by [`Out`], last (after the HUD's 50).
pub fn camera() -> impl Bundle {
    // `Projection` only so Bevy computes its target size (`camera_system`); a layer nothing is on, as nothing is drawn
    (Camera { order: 51, ..default() }, bevy::render::camera::CameraRenderGraph::new(Out), Projection::default(), bevy::camera::visibility::RenderLayers::layer(super::LAYER))
}

#[derive(Resource)]
pub struct Pipeline {
    mix: BindGroupLayoutDescriptor,
    composite: BindGroupLayoutDescriptor,
    weight: Buffer,
    fullscreen: FullscreenShader,
    shader: Handle<Shader>,
}

impl FromWorld for Pipeline {
    fn from_world(world: &mut World) -> Self {
        let tex = || texture_2d(TextureSampleType::Float { filterable: true });
        let mix = BindGroupLayoutDescriptor::new(
            "hst_persist",
            &BindGroupLayoutEntries::sequential(ShaderStages::FRAGMENT, (tex(), tex(), uniform_buffer::<Vec4>(false), sampler(SamplerBindingType::Filtering))),
        );
        let composite = BindGroupLayoutDescriptor::new("hst_composite", &BindGroupLayoutEntries::sequential(ShaderStages::FRAGMENT, (tex(), tex(), sampler(SamplerBindingType::Filtering))));
        let weight = world.resource::<RenderDevice>().create_buffer(&BufferDescriptor {
            label: Some("hst_persist_weight"),
            size: 16,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let shader = world.resource::<AssetServer>().load("embedded://hst/hud_gamma.wgsl");
        Pipeline { mix, composite, weight, fullscreen: world.resource::<FullscreenShader>().clone(), shader }
    }
}

impl SpecializedRenderPipeline for Pipeline {
    /// The target's format; `true` for the persistence mix, `false` for the composite.
    type Key = (TextureFormat, bool);

    fn specialize(&self, (format, mix): Self::Key) -> RenderPipelineDescriptor {
        RenderPipelineDescriptor {
            label: Some(if mix { "hst_persist" } else { "hst_composite" }.into()),
            layout: vec![if mix { self.mix.clone() } else { self.composite.clone() }],
            vertex: self.fullscreen.to_vertex_state(),
            fragment: Some(FragmentState {
                shader: self.shader.clone(),
                shader_defs: if mix { vec!["PERSIST".into()] } else { vec![] },
                entry_point: Some("fragment".into()),
                targets: vec![Some(ColorTargetState { format, blend: None, write_mask: ColorWrites::ALL })],
                ..default()
            }),
            ..default()
        }
    }
}

/// The mix's and the composite's pipelines for this frame's window format.
#[derive(Resource, Default)]
struct Ids {
    mix: Option<CachedRenderPipelineId>,
    composite: Option<(TextureFormat, CachedRenderPipelineId)>,
}

fn prepare(
    frame: Option<Res<Frame>>,
    windows: Res<ExtractedWindows>,
    pipeline: Res<Pipeline>,
    cache: Res<PipelineCache>,
    queue: Res<RenderQueue>,
    mut pipelines: ResMut<SpecializedRenderPipelines<Pipeline>>,
    mut ids: ResMut<Ids>,
) {
    if ids.mix.is_none() {
        ids.mix = Some(pipelines.specialize(&cache, &pipeline, (TextureFormat::Rgba8Unorm, true)));
    }
    let format = windows.primary.and_then(|w| windows.windows.get(&w)).and_then(|w| w.swap_chain_texture_view_format);
    if let Some(format) = format
        && ids.composite.is_none_or(|(f, _)| f != format)
    {
        ids.composite = Some((format, pipelines.specialize(&cache, &pipeline, (format, false))));
    }
    if let Some((_, w)) = frame.and_then(|f| f.mix) {
        let bytes: Vec<u8> = w.to_array().iter().flat_map(|f| f.to_le_bytes()).collect();
        queue.write_buffer(&pipeline.weight, 0, &bytes);
    }
}

/// Bind groups kept while the texture views they hold stay the same (they change when an image is resized).
#[derive(Default)]
struct Groups {
    mix: [Option<([TextureViewId; 2], SamplerId, BindGroup)>; 2],
    composite: Option<([TextureViewId; 2], SamplerId, BindGroup)>,
}

#[allow(clippy::too_many_arguments)]
fn draw(
    view: ViewQuery<&ExtractedCamera>,
    frame: Option<Res<Frame>>,
    attachments: Res<ViewTargetAttachments>,
    images: Res<RenderAssets<GpuImage>>,
    pipeline: Res<Pipeline>,
    ids: Res<Ids>,
    cache: Res<PipelineCache>,
    mut groups: Local<Groups>,
    mut ctx: RenderContext,
) {
    let camera = view.into_inner();
    // the window's output, or the capture texture standing in for it while a screenshot is taken
    let Some(out) = camera.target.as_ref().and_then(|t| attachments.get(t)) else { return };
    let Some(frame) = frame else { return };
    let device = ctx.render_device().clone();
    let group = |slot: &mut Option<([TextureViewId; 2], SamplerId, BindGroup)>, layout: &BindGroupLayoutDescriptor, a: &GpuImage, b: &GpuImage, weight: Option<&Buffer>| {
        let key = [a.texture_view.id(), b.texture_view.id()];
        match slot {
            Some((k, s, _)) if *k == key && *s == a.sampler.id() => {}
            _ => {
                let layout = cache.get_bind_group_layout(layout);
                let g = match weight {
                    Some(wb) => device.create_bind_group(None, &layout, &BindGroupEntries::sequential((&a.texture_view, &b.texture_view, wb.as_entire_binding(), &a.sampler))),
                    None => device.create_bind_group(None, &layout, &BindGroupEntries::sequential((&a.texture_view, &b.texture_view, &a.sampler))),
                };
                *slot = Some((key, a.sampler.id(), g));
            }
        }
        slot.as_ref().unwrap().2.clone()
    };
    let (Some(scene), Some(hud)) = (images.get(&frame.scene), images.get(&frame.hud)) else { return };
    // this frame's persistence: the scene and the last frame shown into `last[i]`
    let mut shown = scene;
    if let Some((i, _)) = frame.mix
        && let (Some(dst), Some(prev)) = (images.get(&frame.last[i]), images.get(&frame.last[1 - i]))
        && let Some(p) = ids.mix.and_then(|id| cache.get_render_pipeline(id))
    {
        let g = group(&mut groups.mix[i], &pipeline.mix, scene, prev, Some(&pipeline.weight));
        let mut pass = ctx.command_encoder().begin_render_pass(&RenderPassDescriptor {
            label: Some("hst_persist"),
            color_attachments: &[Some(RenderPassColorAttachment { view: &dst.texture_view, depth_slice: None, resolve_target: None, ops: Operations { load: LoadOp::Clear(default()), store: StoreOp::Store } })],
            ..default()
        });
        pass.set_pipeline(p);
        pass.set_bind_group(0, &g, &[]);
        pass.draw(0..3, 0..1);
        shown = dst;
    }
    // the composite, only in the view (B117's fixed aspect ratios), black around it; the window is cleared even
    // while the pipeline compiles
    let p = ids.composite.and_then(|(_, id)| cache.get_render_pipeline(id));
    let g = group(&mut groups.composite, &pipeline.composite, shown, hud, None);
    let mut pass = ctx.command_encoder().begin_render_pass(&RenderPassDescriptor {
        label: Some("hst_composite"),
        color_attachments: &[Some(out.get_attachment(Some(LinearRgba::BLACK)))],
        ..default()
    });
    let Some(p) = p else { return };
    if let Some(v) = &camera.viewport {
        let (pos, size) = (v.physical_position.as_vec2(), v.physical_size.as_vec2());
        pass.set_viewport(pos.x, pos.y, size.x, size.y, 0.0, 1.0);
    }
    pass.set_pipeline(p);
    pass.set_bind_group(0, &g, &[]);
    pass.draw(0..3, 0..1);
}
