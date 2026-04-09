use bevy::{
    camera::visibility::{Visibility, VisibilityClass, add_visibility_class},
    prelude::*,
    render::sync_world::SyncToRenderWorld,
};

#[derive(Reflect, Clone, Debug)]
pub enum OccluderShape2d {
    Rectangle {
        half_size: Vec2,
    },
    Circle {
        radius: f32,
        segments: u16,
    },
    Polygon {
        points: Vec<Vec2>,
    },
    Mask {
        mask: Handle<Image>,
        alpha_threshold: f32,
    },
}

impl Default for OccluderShape2d {
    fn default() -> Self {
        Self::Rectangle {
            half_size: Vec2::new(16.0, 16.0),
        }
    }
}

#[derive(Reflect, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum LightShadowMode2d {
    Solid,
    Occluded,
    #[default]
    Illuminated,
}

/// Radial light authoring component.
#[derive(Component, Reflect, Clone, Copy, Debug)]
#[reflect(Component, Default)]
#[require(SyncToRenderWorld, Transform, Visibility, VisibilityClass)]
#[component(on_add = add_visibility_class::<PointLight2d>)]
pub struct PointLight2d {
    pub color: Color,
    pub intensity: f32,
    pub radius: f32,
    pub falloff: f32,
    pub height: f32,
    pub source_radius: f32,
    pub shadow_mode: LightShadowMode2d,
    pub occluder_mask: u32,
}

impl Default for PointLight2d {
    fn default() -> Self {
        Self {
            color: Color::WHITE,
            intensity: 1.0,
            radius: 96.0,
            falloff: 1.0,
            height: 48.0,
            source_radius: 12.0,
            shadow_mode: LightShadowMode2d::Illuminated,
            occluder_mask: u32::MAX,
        }
    }
}

impl PointLight2d {
    #[must_use]
    pub fn half_extents(self) -> Vec2 {
        Vec2::splat(self.radius.max(1.0))
    }
}

/// Cone light authoring component.
#[derive(Component, Reflect, Clone, Copy, Debug)]
#[reflect(Component, Default)]
#[require(SyncToRenderWorld, Transform, Visibility, VisibilityClass)]
#[component(on_add = add_visibility_class::<SpotLight2d>)]
pub struct SpotLight2d {
    pub color: Color,
    pub intensity: f32,
    pub radius: f32,
    pub falloff: f32,
    pub height: f32,
    pub source_width: f32,
    pub direction_radians: f32,
    pub inner_angle_radians: f32,
    pub outer_angle_radians: f32,
    pub shadow_mode: LightShadowMode2d,
    pub occluder_mask: u32,
}

impl Default for SpotLight2d {
    fn default() -> Self {
        Self {
            color: Color::WHITE,
            intensity: 1.0,
            radius: 128.0,
            falloff: 1.0,
            height: 56.0,
            source_width: 20.0,
            direction_radians: 0.0,
            inner_angle_radians: std::f32::consts::FRAC_PI_4,
            outer_angle_radians: std::f32::consts::FRAC_PI_2,
            shadow_mode: LightShadowMode2d::Illuminated,
            occluder_mask: u32::MAX,
        }
    }
}

impl SpotLight2d {
    #[must_use]
    pub fn half_extents(self) -> Vec2 {
        Vec2::splat(self.radius.max(1.0))
    }
}

/// Textured light / cookie light authoring component.
#[derive(Component, Reflect, Clone, Debug)]
#[reflect(Component, Default)]
#[require(SyncToRenderWorld, Transform, Visibility, VisibilityClass)]
#[component(on_add = add_visibility_class::<TextureLight2d>)]
pub struct TextureLight2d {
    pub texture: Handle<Image>,
    pub color: Color,
    pub intensity: f32,
    pub size: Vec2,
    pub height: f32,
    pub source_radius: f32,
    pub rotation_radians: f32,
    pub shadow_mode: LightShadowMode2d,
    pub occluder_mask: u32,
}

impl Default for TextureLight2d {
    fn default() -> Self {
        Self {
            texture: Handle::default(),
            color: Color::WHITE,
            intensity: 1.0,
            size: Vec2::splat(96.0),
            height: 40.0,
            source_radius: 10.0,
            rotation_radians: 0.0,
            shadow_mode: LightShadowMode2d::Illuminated,
            occluder_mask: u32::MAX,
        }
    }
}

impl TextureLight2d {
    #[must_use]
    pub fn half_extents(&self) -> Vec2 {
        (0.5 * self.size).max(Vec2::splat(1.0))
    }
}

/// Scene blocker authoring component.
#[derive(Component, Reflect, Clone, Debug)]
#[reflect(Component, Default)]
#[require(SyncToRenderWorld, Transform, Visibility, VisibilityClass)]
#[component(on_add = add_visibility_class::<LightOccluder2d>)]
pub struct LightOccluder2d {
    pub shape: OccluderShape2d,
    pub casts_shadows: bool,
    pub absorption: f32,
    pub shadow_tint: Color,
    pub groups: u32,
}

impl Default for LightOccluder2d {
    fn default() -> Self {
        Self {
            shape: OccluderShape2d::default(),
            casts_shadows: true,
            absorption: 1.0,
            shadow_tint: Color::BLACK,
            groups: 1,
        }
    }
}

impl LightOccluder2d {
    #[must_use]
    pub fn rectangle(half_size: Vec2) -> Self {
        Self {
            shape: OccluderShape2d::Rectangle { half_size },
            ..Self::default()
        }
    }

    #[must_use]
    pub fn circle(radius: f32, segments: u16) -> Self {
        Self {
            shape: OccluderShape2d::Circle { radius, segments },
            ..Self::default()
        }
    }

    #[must_use]
    pub fn polygon(points: Vec<Vec2>) -> Self {
        Self {
            shape: OccluderShape2d::Polygon { points },
            ..Self::default()
        }
    }

    #[must_use]
    pub fn mask(mask: Handle<Image>) -> Self {
        Self {
            shape: OccluderShape2d::Mask {
                mask,
                alpha_threshold: 0.1,
            },
            ..Self::default()
        }
    }
}

/// Receiver metadata for normal-mapped sprites.
#[derive(Component, Reflect, Clone, Debug)]
#[reflect(Component)]
#[require(SyncToRenderWorld)]
pub struct NormalMappedSprite2d {
    pub normal_map: Handle<Image>,
    pub strength: f32,
    pub height: f32,
}

impl NormalMappedSprite2d {
    #[must_use]
    pub fn new(normal_map: Handle<Image>) -> Self {
        Self {
            normal_map,
            strength: 1.0,
            height: 0.0,
        }
    }
}

/// Receiver metadata for emissive sprites.
#[derive(Component, Reflect, Clone, Debug)]
#[reflect(Component, Default)]
#[require(SyncToRenderWorld)]
pub struct EmissiveSprite2d {
    pub color: Color,
    pub intensity: f32,
    pub mask: Option<Handle<Image>>,
}

impl Default for EmissiveSprite2d {
    fn default() -> Self {
        Self {
            color: Color::srgb(1.0, 0.88, 0.64),
            intensity: 1.0,
            mask: None,
        }
    }
}
