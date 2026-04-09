use bevy::{prelude::*, render::sync_world::SyncToRenderWorld};

#[derive(Reflect, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum LightingBackend2d {
    #[default]
    ScreenSpace,
    ExperimentalGi,
}

#[derive(Reflect, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ShadowFiltering2d {
    #[default]
    Hard,
    Soft,
}

#[derive(Reflect, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum LightingCompositeMode2d {
    #[default]
    Multiply,
    Additive,
}

/// Camera-scoped settings for 2D lighting.
#[derive(Component, Reflect, Clone)]
#[reflect(Component, Default)]
#[require(SyncToRenderWorld)]
pub struct Lighting2dSettings {
    pub lighting_enabled: bool,
    pub ambient_color: Color,
    pub ambient_intensity: f32,
    pub lightmap_scale: f32,
    pub normal_map_scale: f32,
    pub blur_radius: u32,
    pub enable_occlusion: bool,
    pub enable_normal_maps: bool,
    pub enable_emissive: bool,
    pub backend: LightingBackend2d,
    pub shadow_filter: ShadowFiltering2d,
    pub composite_mode: LightingCompositeMode2d,
}

impl Default for Lighting2dSettings {
    fn default() -> Self {
        Self {
            lighting_enabled: true,
            ambient_color: Color::srgb(0.78, 0.82, 0.92),
            ambient_intensity: 0.15,
            lightmap_scale: 0.5,
            normal_map_scale: 1.0,
            blur_radius: 0,
            enable_occlusion: true,
            enable_normal_maps: true,
            enable_emissive: true,
            backend: LightingBackend2d::ScreenSpace,
            shadow_filter: ShadowFiltering2d::Hard,
            composite_mode: LightingCompositeMode2d::Multiply,
        }
    }
}

impl Lighting2dSettings {
    #[must_use]
    pub fn fast_unshadowed() -> Self {
        Self {
            lightmap_scale: 0.35,
            enable_occlusion: false,
            shadow_filter: ShadowFiltering2d::Hard,
            ..Self::default()
        }
    }

    #[must_use]
    pub fn showcase_soft() -> Self {
        Self {
            ambient_intensity: 0.12,
            shadow_filter: ShadowFiltering2d::Soft,
            ..Self::default()
        }
    }
}
