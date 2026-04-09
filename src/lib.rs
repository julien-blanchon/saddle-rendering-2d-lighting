use bevy::{
    app::PostStartup,
    asset::{load_internal_asset, uuid_handle},
    ecs::{intern::Interned, schedule::ScheduleLabel},
    image::TextureAtlasLayout,
    mesh::Mesh,
    prelude::*,
    shader::Shader,
    sprite_render::Material2dPlugin,
};

mod components;
mod config;
mod diagnostics;
mod geometry;
mod materials;
mod systems;

pub use components::{
    EmissiveSprite2d, LightOccluder2d, LightShadowMode2d, NormalMappedSprite2d, OccluderShape2d,
    PointLight2d, SpotLight2d, TextureLight2d,
};
pub use config::{
    Lighting2dSettings, LightingBackend2d, LightingCompositeMode2d, ShadowFiltering2d,
};
pub use diagnostics::Lighting2dDiagnostics;

pub(crate) const AMBIENT_OVERLAY_SHADER_HANDLE: Handle<Shader> =
    uuid_handle!("290f763b-548a-4e53-8bf2-7fe2ba8aa001");
pub(crate) const LIGHT_RENDER_SHADER_HANDLE: Handle<Shader> =
    uuid_handle!("290f763b-548a-4e53-8bf2-7fe2ba8aa002");
pub(crate) const RECEIVER_LIGHTING_SHADER_HANDLE: Handle<Shader> =
    uuid_handle!("290f763b-548a-4e53-8bf2-7fe2ba8aa003");
pub(crate) const LIGHT_BLUR_SHADER_HANDLE: Handle<Shader> =
    uuid_handle!("290f763b-548a-4e53-8bf2-7fe2ba8aa004");
pub(crate) const LIGHT_COMPOSITE_SHADER_HANDLE: Handle<Shader> =
    uuid_handle!("290f763b-548a-4e53-8bf2-7fe2ba8aa005");

#[derive(SystemSet, Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum Lighting2dSystems {
    Prepare,
    SyncAuthoring,
    UpdateBounds,
    UpdateProxies,
    Diagnostics,
    Debug,
}

#[derive(Resource, Default)]
pub(crate) struct Lighting2dRuntimeState {
    pub active: bool,
}

#[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
struct NeverDeactivateSchedule;

pub struct Lighting2dPlugin {
    pub activate_schedule: Interned<dyn ScheduleLabel>,
    pub deactivate_schedule: Interned<dyn ScheduleLabel>,
    pub update_schedule: Interned<dyn ScheduleLabel>,
}

impl Lighting2dPlugin {
    #[must_use]
    pub fn new(
        activate_schedule: impl ScheduleLabel,
        deactivate_schedule: impl ScheduleLabel,
        update_schedule: impl ScheduleLabel,
    ) -> Self {
        Self {
            activate_schedule: activate_schedule.intern(),
            deactivate_schedule: deactivate_schedule.intern(),
            update_schedule: update_schedule.intern(),
        }
    }

    #[must_use]
    pub fn always_on(update_schedule: impl ScheduleLabel) -> Self {
        Self::new(PostStartup, NeverDeactivateSchedule, update_schedule)
    }
}

impl Default for Lighting2dPlugin {
    fn default() -> Self {
        Self::always_on(Update)
    }
}

impl Plugin for Lighting2dPlugin {
    fn build(&self, app: &mut App) {
        if self.deactivate_schedule == NeverDeactivateSchedule.intern() {
            app.init_schedule(NeverDeactivateSchedule);
        }

        if !app.world().contains_resource::<Assets<Image>>() {
            app.init_asset::<Image>();
        }
        if !app.world().contains_resource::<Assets<Mesh>>() {
            app.init_asset::<Mesh>();
        }
        if !app
            .world()
            .contains_resource::<Assets<TextureAtlasLayout>>()
        {
            app.init_asset::<TextureAtlasLayout>();
        }

        if app.world().contains_resource::<Assets<Shader>>() {
            load_internal_asset!(
                app,
                AMBIENT_OVERLAY_SHADER_HANDLE,
                "shaders/ambient_overlay.wgsl",
                Shader::from_wgsl
            );
            load_internal_asset!(
                app,
                LIGHT_RENDER_SHADER_HANDLE,
                "shaders/light_render.wgsl",
                Shader::from_wgsl
            );
            load_internal_asset!(
                app,
                RECEIVER_LIGHTING_SHADER_HANDLE,
                "shaders/receiver_lighting.wgsl",
                Shader::from_wgsl
            );
            load_internal_asset!(
                app,
                LIGHT_BLUR_SHADER_HANDLE,
                "shaders/light_blur.wgsl",
                Shader::from_wgsl
            );
            load_internal_asset!(
                app,
                LIGHT_COMPOSITE_SHADER_HANDLE,
                "shaders/light_composite.wgsl",
                Shader::from_wgsl
            );
        }

        app.add_plugins((
            Material2dPlugin::<materials::AmbientOverlayMaterial>::default(),
            Material2dPlugin::<materials::LightRenderMaterial>::default(),
            Material2dPlugin::<materials::ReceiverLightingMaterial>::default(),
            Material2dPlugin::<materials::LightBlurMaterial>::default(),
            Material2dPlugin::<materials::LightCompositeMaterial>::default(),
        ))
        .init_resource::<Lighting2dRuntimeState>()
        .init_resource::<Lighting2dDiagnostics>()
        .init_resource::<materials::Lighting2dInternalAssets>()
        .init_resource::<systems::LightingLayerAllocator>()
        .init_resource::<systems::LightingOccluderMaskCache>()
        .register_type::<EmissiveSprite2d>()
        .register_type::<LightOccluder2d>()
        .register_type::<LightShadowMode2d>()
        .register_type::<Lighting2dDiagnostics>()
        .register_type::<Lighting2dSettings>()
        .register_type::<LightingBackend2d>()
        .register_type::<LightingCompositeMode2d>()
        .register_type::<NormalMappedSprite2d>()
        .register_type::<OccluderShape2d>()
        .register_type::<PointLight2d>()
        .register_type::<ShadowFiltering2d>()
        .register_type::<SpotLight2d>()
        .register_type::<TextureLight2d>()
        .add_systems(self.activate_schedule, systems::activate_runtime)
        .add_systems(
            self.deactivate_schedule,
            (systems::deactivate_runtime, systems::cleanup_all)
                .chain()
                .in_set(Lighting2dSystems::Debug),
        )
        .configure_sets(
            self.update_schedule,
            (
                Lighting2dSystems::Prepare,
                Lighting2dSystems::SyncAuthoring,
                Lighting2dSystems::UpdateBounds,
                Lighting2dSystems::UpdateProxies,
                Lighting2dSystems::Diagnostics,
                Lighting2dSystems::Debug,
            )
                .chain(),
        )
        .add_systems(
            self.update_schedule,
            (
                systems::ensure_internal_assets.in_set(Lighting2dSystems::Prepare),
                (
                    systems::update_point_light_bounds,
                    systems::update_spot_light_bounds,
                    systems::update_texture_light_bounds,
                )
                    .in_set(Lighting2dSystems::UpdateBounds),
                (
                    systems::sync_view_runtimes,
                    systems::cleanup_orphaned_views,
                    systems::sync_light_proxies,
                    systems::cleanup_orphaned_light_proxies,
                    systems::sync_receiver_proxies,
                    systems::cleanup_orphaned_receiver_proxies,
                )
                    .chain()
                    .in_set(Lighting2dSystems::UpdateProxies)
                    .run_if(systems::runtime_is_active),
                systems::publish_diagnostics.in_set(Lighting2dSystems::Diagnostics),
            ),
        );
    }
}

#[cfg(test)]
#[path = "config_tests.rs"]
mod config_tests;

#[cfg(test)]
#[path = "geometry_tests.rs"]
mod geometry_tests;

#[cfg(test)]
#[path = "systems_tests.rs"]
mod systems_tests;
