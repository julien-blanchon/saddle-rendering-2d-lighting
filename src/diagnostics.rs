use bevy::prelude::*;

/// Runtime counts for the public authoring surface.
#[derive(Resource, Reflect, Clone, Debug, Default)]
#[reflect(Resource, Default)]
pub struct Lighting2dDiagnostics {
    pub active_cameras: usize,
    pub active_point_lights: usize,
    pub active_spot_lights: usize,
    pub active_texture_lights: usize,
    pub active_occluders: usize,
    pub active_normal_mapped_sprites: usize,
    pub active_emissive_sprites: usize,
}

impl Lighting2dDiagnostics {
    #[must_use]
    pub fn total_lights(&self) -> usize {
        self.active_point_lights + self.active_spot_lights + self.active_texture_lights
    }
}
