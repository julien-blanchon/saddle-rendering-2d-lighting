use std::collections::{HashMap, HashSet};

use bevy::{
    asset::RenderAssetUsages,
    camera::{
        Camera, Camera2d, ClearColorConfig, Projection, RenderTarget,
        primitives::Aabb,
        visibility::{NoFrustumCulling, RenderLayers},
    },
    ecs::query::{Changed, Or, Without},
    image::TextureAtlasLayout,
    math::{Vec3A, primitives::Rectangle},
    mesh::Mesh2d,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages},
    sprite::Anchor,
    sprite_render::MeshMaterial2d,
    window::{PrimaryWindow, Window},
};

use crate::{
    Lighting2dRuntimeState,
    components::{
        EmissiveSprite2d, LightOccluder2d, NormalMappedSprite2d, PointLight2d, SpotLight2d,
        TextureLight2d,
    },
    config::{Lighting2dSettings, ShadowFiltering2d},
    diagnostics::Lighting2dDiagnostics,
    geometry::{
        OccluderMaskCache, PackedReceiverLight, PackedSegment, color_to_vec4,
        flatten_occluder_segments, global_rotation_z, light_direction, occluder_bounding_radius,
        point_light_world_radius, sample_texture_light_cookie, spot_light_world_radius,
        sprite_anchor_translation, sprite_draw_size, sprite_uv_rect, texture_light_world_size,
        truncate_segments,
    },
    materials::{
        AmbientOverlayMaterial, AmbientOverlayUniform, LightBlurMaterial, LightBlurUniform,
        LightCompositeMaterial, LightMaterialUniform, LightRenderMaterial,
        Lighting2dInternalAssets, MAX_OCCLUDER_SEGMENTS, MAX_RECEIVER_COOKIE_TEXTURES,
        MAX_RECEIVER_LIGHTS, ReceiverLightUniform, ReceiverLightingMaterial,
        ReceiverLightingUniform, SegmentUniform,
    },
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct LightingLayerGroup {
    pub light: usize,
    pub blur_x: usize,
    pub blur_y: usize,
    pub composite: usize,
}

#[derive(Resource)]
pub(crate) struct LightingLayerAllocator {
    groups: Vec<LightingLayerGroup>,
}

impl Default for LightingLayerAllocator {
    fn default() -> Self {
        let mut groups = Vec::new();
        let mut top = 63usize;
        while top >= 7 {
            groups.push(LightingLayerGroup {
                light: top - 3,
                blur_x: top - 2,
                blur_y: top - 1,
                composite: top,
            });
            top -= 4;
        }

        Self { groups }
    }
}

impl LightingLayerAllocator {
    fn allocate(&self, used: &HashSet<LightingLayerGroup>) -> Option<LightingLayerGroup> {
        self.groups
            .iter()
            .copied()
            .find(|group| !used.contains(group))
    }
}

#[derive(Resource, Default)]
pub(crate) struct LightingOccluderMaskCache {
    pub masks: OccluderMaskCache,
}

#[derive(Component, Clone)]
pub(crate) struct LightingViewRuntime {
    pub layers: LightingLayerGroup,
    pub light_camera: Entity,
    pub blur_x_camera: Entity,
    pub blur_y_camera: Entity,
    pub ambient_overlay: Entity,
    pub blur_x_quad: Entity,
    pub blur_y_quad: Entity,
    pub composite_quad: Entity,
    pub lightmap_image: Handle<Image>,
    pub blur_ping_image: Handle<Image>,
    pub blur_output_image: Handle<Image>,
    pub ambient_material: Handle<AmbientOverlayMaterial>,
    pub blur_x_material: Handle<LightBlurMaterial>,
    pub blur_y_material: Handle<LightBlurMaterial>,
    pub composite_material: Handle<LightCompositeMaterial>,
}

#[derive(Component, Clone, Copy)]
pub(crate) struct LightingViewChild;

#[derive(Component)]
pub(crate) struct LightingLightCameraChild;

#[derive(Component)]
pub(crate) struct LightingBlurXCameraChild;

#[derive(Component)]
pub(crate) struct LightingBlurYCameraChild;

#[derive(Component)]
pub(crate) struct AmbientOverlayChild;

#[derive(Component)]
pub(crate) struct BlurXQuadChild;

#[derive(Component)]
pub(crate) struct BlurYQuadChild;

#[derive(Component)]
pub(crate) struct CompositeQuadChild;

#[derive(Component, Clone)]
pub(crate) struct LightViewProxy {
    pub owner_camera: Entity,
    pub owner_light: Entity,
    pub material: Handle<LightRenderMaterial>,
}

#[derive(Component)]
pub(crate) struct LightViewProxyChild;

#[derive(Component, Clone)]
pub(crate) struct ReceiverViewProxy {
    pub owner_camera: Entity,
    pub owner_receiver: Entity,
    pub material: Handle<ReceiverLightingMaterial>,
}

#[derive(Component)]
pub(crate) struct ReceiverViewProxyChild;

#[derive(Clone)]
struct CollectedReceiverLight {
    packed: PackedReceiverLight,
    cookie_texture: Option<Handle<Image>>,
}

pub fn activate_runtime(mut runtime: ResMut<Lighting2dRuntimeState>) {
    runtime.active = true;
}

pub fn deactivate_runtime(mut runtime: ResMut<Lighting2dRuntimeState>) {
    runtime.active = false;
}

pub fn runtime_is_active(runtime: Res<Lighting2dRuntimeState>) -> bool {
    runtime.active
}

pub fn ensure_internal_assets(
    mut internal: ResMut<Lighting2dInternalAssets>,
    mut meshes: Option<ResMut<Assets<Mesh>>>,
    mut images: Option<ResMut<Assets<Image>>>,
) {
    let (Some(meshes), Some(images)) = (meshes.as_deref_mut(), images.as_deref_mut()) else {
        return;
    };

    if internal.quad_mesh == Handle::default() {
        internal.quad_mesh = meshes.add(Rectangle::new(1.0, 1.0));
    }
    if internal.white_image == Handle::default() {
        internal.white_image = images.add(Lighting2dInternalAssets::make_white_image());
    }
    if internal.flat_normal_image == Handle::default() {
        internal.flat_normal_image = images.add(Lighting2dInternalAssets::make_flat_normal_image());
    }
    if internal.white_mask_image == Handle::default() {
        internal.white_mask_image = images.add(Lighting2dInternalAssets::make_white_image());
    }
}

fn build_render_target_image(size: UVec2) -> Image {
    let mut image = Image::new_fill(
        Extent3d {
            width: size.x.max(1),
            height: size.y.max(1),
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0; 8],
        TextureFormat::Rgba16Float,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    image.texture_descriptor.usage =
        TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST | TextureUsages::RENDER_ATTACHMENT;
    image
}

fn resize_render_target(images: &mut Assets<Image>, handle: &Handle<Image>, size: UVec2) {
    let Some(image) = images.get_mut(handle) else {
        return;
    };
    let size = size.max(UVec2::ONE);
    image.resize(Extent3d {
        width: size.x,
        height: size.y,
        depth_or_array_layers: 1,
    });
    image.texture_descriptor.usage =
        TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST | TextureUsages::RENDER_ATTACHMENT;
}

fn make_aabb(half_extents: Vec2) -> Aabb {
    Aabb {
        center: Vec3A::ZERO,
        half_extents: Vec3A::new(half_extents.x.max(1.0), half_extents.y.max(1.0), 0.0),
    }
}

pub fn update_point_light_bounds(
    lights: Query<
        (Entity, &PointLight2d),
        (
            Or<(Changed<PointLight2d>, Without<Aabb>)>,
            Without<NoFrustumCulling>,
        ),
    >,
    mut commands: Commands,
) {
    for (entity, light) in &lights {
        commands
            .entity(entity)
            .insert(make_aabb(light.half_extents()));
    }
}

pub fn update_spot_light_bounds(
    lights: Query<
        (Entity, &SpotLight2d),
        (
            Or<(Changed<SpotLight2d>, Without<Aabb>)>,
            Without<NoFrustumCulling>,
        ),
    >,
    mut commands: Commands,
) {
    for (entity, light) in &lights {
        commands
            .entity(entity)
            .insert(make_aabb(light.half_extents()));
    }
}

pub fn update_texture_light_bounds(
    lights: Query<
        (Entity, &TextureLight2d),
        (
            Or<(Changed<TextureLight2d>, Without<Aabb>)>,
            Without<NoFrustumCulling>,
        ),
    >,
    mut commands: Commands,
) {
    for (entity, light) in &lights {
        commands
            .entity(entity)
            .insert(make_aabb(light.half_extents()));
    }
}

#[allow(clippy::too_many_arguments)]
pub fn sync_view_runtimes(
    mut commands: Commands,
    allocator: Res<LightingLayerAllocator>,
    internal: Res<Lighting2dInternalAssets>,
    primary_window: Query<&Window, With<PrimaryWindow>>,
    mut images: ResMut<Assets<Image>>,
    mut ambient_materials: ResMut<Assets<AmbientOverlayMaterial>>,
    mut blur_materials: ResMut<Assets<LightBlurMaterial>>,
    mut composite_materials: ResMut<Assets<LightCompositeMaterial>>,
    cameras: Query<
        (
            Entity,
            &Lighting2dSettings,
            &Camera,
            &Projection,
            Option<&RenderLayers>,
            Option<&LightingViewRuntime>,
        ),
        (With<Camera>, Without<LightingViewChild>),
    >,
    mut light_cameras: Query<
        (
            &mut Camera,
            &mut Projection,
            &mut RenderTarget,
            &mut RenderLayers,
            &mut Transform,
        ),
        (
            With<LightingViewChild>,
            With<LightingLightCameraChild>,
            Without<LightingBlurXCameraChild>,
            Without<LightingBlurYCameraChild>,
            Without<AmbientOverlayChild>,
            Without<BlurXQuadChild>,
            Without<BlurYQuadChild>,
            Without<CompositeQuadChild>,
        ),
    >,
    mut blur_x_cameras: Query<
        (
            &mut Camera,
            &mut Projection,
            &mut RenderTarget,
            &mut RenderLayers,
            &mut Transform,
        ),
        (
            With<LightingViewChild>,
            With<LightingBlurXCameraChild>,
            Without<LightingLightCameraChild>,
            Without<LightingBlurYCameraChild>,
            Without<AmbientOverlayChild>,
            Without<BlurXQuadChild>,
            Without<BlurYQuadChild>,
            Without<CompositeQuadChild>,
        ),
    >,
    mut blur_y_cameras: Query<
        (
            &mut Camera,
            &mut Projection,
            &mut RenderTarget,
            &mut RenderLayers,
            &mut Transform,
        ),
        (
            With<LightingViewChild>,
            With<LightingBlurYCameraChild>,
            Without<LightingLightCameraChild>,
            Without<LightingBlurXCameraChild>,
            Without<AmbientOverlayChild>,
            Without<BlurXQuadChild>,
            Without<BlurYQuadChild>,
            Without<CompositeQuadChild>,
        ),
    >,
    mut ambient_overlays: Query<
        (&mut Transform, &mut Visibility),
        (
            With<LightingViewChild>,
            With<AmbientOverlayChild>,
            Without<BlurXQuadChild>,
            Without<BlurYQuadChild>,
            Without<CompositeQuadChild>,
        ),
    >,
    mut blur_x_quads: Query<
        (&mut Transform, &mut Visibility),
        (
            With<LightingViewChild>,
            With<BlurXQuadChild>,
            Without<AmbientOverlayChild>,
            Without<BlurYQuadChild>,
            Without<CompositeQuadChild>,
        ),
    >,
    mut blur_y_quads: Query<
        (&mut Transform, &mut Visibility),
        (
            With<LightingViewChild>,
            With<BlurYQuadChild>,
            Without<AmbientOverlayChild>,
            Without<BlurXQuadChild>,
            Without<CompositeQuadChild>,
        ),
    >,
    mut composite_quads: Query<
        (&mut Transform, &mut Visibility),
        (
            With<LightingViewChild>,
            With<CompositeQuadChild>,
            Without<AmbientOverlayChild>,
            Without<BlurXQuadChild>,
            Without<BlurYQuadChild>,
        ),
    >,
) {
    if internal.quad_mesh == Handle::default() {
        return;
    }

    let primary_window = primary_window.iter().next();
    let mut used_groups = cameras
        .iter()
        .filter_map(|(_, _, _, _, _, runtime)| runtime.map(|runtime| runtime.layers))
        .collect::<HashSet<_>>();

    for (entity, settings, camera, projection, layers, runtime) in &cameras {
        let runtime = if settings.lighting_enabled {
            ensure_view_runtime(
                &mut commands,
                &allocator,
                &mut used_groups,
                &mut images,
                &mut ambient_materials,
                &mut blur_materials,
                &mut composite_materials,
                &internal,
                entity,
                camera.order,
                projection,
                runtime,
            )
        } else {
            runtime.cloned()
        };

        let Some(runtime) = runtime else {
            continue;
        };

        let (logical_size, area_center) = camera_logical_area(camera, projection, primary_window);
        let physical_size = camera_physical_size(camera, primary_window)
            .or_else(|| Some(logical_size.max(Vec2::ONE).as_uvec2()))
            .unwrap_or(UVec2::ONE);
        let scaled_size = scaled_lightmap_size(physical_size, settings.lightmap_scale);

        resize_render_target(&mut images, &runtime.lightmap_image, scaled_size);
        resize_render_target(&mut images, &runtime.blur_ping_image, scaled_size);
        resize_render_target(&mut images, &runtime.blur_output_image, scaled_size);

        let desired_layers = layers
            .cloned()
            .unwrap_or_default()
            .with(runtime.layers.composite);
        if layers != Some(&desired_layers) {
            commands.entity(entity).insert(desired_layers);
        }

        let light_visible = settings.lighting_enabled;
        let blur_visible = light_visible && settings.blur_radius > 0;
        let composite_source = if blur_visible {
            runtime.blur_output_image.clone()
        } else {
            runtime.lightmap_image.clone()
        };

        if let Ok((
            mut child_camera,
            mut child_projection,
            mut child_target,
            mut child_layers,
            mut transform,
        )) = light_cameras.get_mut(runtime.light_camera)
        {
            child_camera.order = camera.order - 3;
            child_camera.clear_color = ClearColorConfig::Custom(Color::NONE);
            *child_target = RenderTarget::Image(runtime.lightmap_image.clone().into());
            child_camera.is_active = light_visible;
            *child_projection = projection.clone();
            *child_layers = RenderLayers::layer(runtime.layers.light);
            *transform = Transform::default();
        }

        if let Ok((
            mut child_camera,
            mut child_projection,
            mut child_target,
            mut child_layers,
            mut transform,
        )) = blur_x_cameras.get_mut(runtime.blur_x_camera)
        {
            child_camera.order = camera.order - 2;
            child_camera.clear_color = ClearColorConfig::Custom(Color::NONE);
            *child_target = RenderTarget::Image(runtime.blur_ping_image.clone().into());
            child_camera.is_active = blur_visible;
            *child_projection = projection.clone();
            *child_layers = RenderLayers::layer(runtime.layers.blur_x);
            *transform = Transform::default();
        }

        if let Ok((
            mut child_camera,
            mut child_projection,
            mut child_target,
            mut child_layers,
            mut transform,
        )) = blur_y_cameras.get_mut(runtime.blur_y_camera)
        {
            child_camera.order = camera.order - 1;
            child_camera.clear_color = ClearColorConfig::Custom(Color::NONE);
            *child_target = RenderTarget::Image(runtime.blur_output_image.clone().into());
            child_camera.is_active = blur_visible;
            *child_projection = projection.clone();
            *child_layers = RenderLayers::layer(runtime.layers.blur_y);
            *transform = Transform::default();
        }

        let quad_transform = Transform::from_xyz(area_center.x, area_center.y, 900.0)
            .with_scale(logical_size.extend(1.0).max(Vec3::splat(1.0)));

        if let Ok((mut transform, mut visibility)) =
            ambient_overlays.get_mut(runtime.ambient_overlay)
        {
            *transform = quad_transform;
            *visibility = if light_visible {
                Visibility::Visible
            } else {
                Visibility::Hidden
            };
        }

        if let Ok((mut transform, mut visibility)) = blur_x_quads.get_mut(runtime.blur_x_quad) {
            *transform = quad_transform;
            *visibility = if blur_visible {
                Visibility::Visible
            } else {
                Visibility::Hidden
            };
        }

        if let Ok((mut transform, mut visibility)) = blur_y_quads.get_mut(runtime.blur_y_quad) {
            *transform = quad_transform;
            *visibility = if blur_visible {
                Visibility::Visible
            } else {
                Visibility::Hidden
            };
        }

        if let Ok((mut transform, mut visibility)) = composite_quads.get_mut(runtime.composite_quad)
        {
            *transform = Transform::from_xyz(area_center.x, area_center.y, 910.0)
                .with_scale(logical_size.extend(1.0).max(Vec3::splat(1.0)));
            *visibility = if light_visible {
                Visibility::Visible
            } else {
                Visibility::Hidden
            };
        }

        if let Some(material) = ambient_materials.get_mut(&runtime.ambient_material) {
            let tint = if light_visible {
                let linear = color_to_vec4(settings.ambient_color) * settings.ambient_intensity;
                match settings.composite_mode {
                    crate::LightingCompositeMode2d::Multiply => {
                        Vec4::new(linear.x, linear.y, linear.z, 1.0)
                    }
                    crate::LightingCompositeMode2d::Additive => Vec4::ONE,
                }
            } else {
                Vec4::ONE
            };
            material.uniform = AmbientOverlayUniform { tint };
        }

        if let Some(material) = blur_materials.get_mut(&runtime.blur_x_material) {
            material.uniform = LightBlurUniform {
                direction_and_radius: Vec4::new(
                    if scaled_size.x > 0 {
                        1.0 / scaled_size.x as f32
                    } else {
                        0.0
                    },
                    0.0,
                    settings.blur_radius as f32,
                    0.0,
                ),
            };
            material.source_texture = Some(runtime.lightmap_image.clone());
        }

        if let Some(material) = blur_materials.get_mut(&runtime.blur_y_material) {
            material.uniform = LightBlurUniform {
                direction_and_radius: Vec4::new(
                    0.0,
                    if scaled_size.y > 0 {
                        1.0 / scaled_size.y as f32
                    } else {
                        0.0
                    },
                    settings.blur_radius as f32,
                    0.0,
                ),
            };
            material.source_texture = Some(runtime.blur_ping_image.clone());
        }

        if let Some(material) = composite_materials.get_mut(&runtime.composite_material) {
            material.source_texture = Some(composite_source);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn ensure_view_runtime(
    commands: &mut Commands,
    allocator: &LightingLayerAllocator,
    used_groups: &mut HashSet<LightingLayerGroup>,
    images: &mut Assets<Image>,
    ambient_materials: &mut Assets<AmbientOverlayMaterial>,
    blur_materials: &mut Assets<LightBlurMaterial>,
    composite_materials: &mut Assets<LightCompositeMaterial>,
    internal: &Lighting2dInternalAssets,
    owner: Entity,
    order: isize,
    projection: &Projection,
    existing: Option<&LightingViewRuntime>,
) -> Option<LightingViewRuntime> {
    if let Some(existing) = existing.cloned() {
        return Some(existing);
    }

    let layers = allocator.allocate(used_groups)?;
    used_groups.insert(layers);

    let lightmap_image = images.add(build_render_target_image(UVec2::new(1, 1)));
    let blur_ping_image = images.add(build_render_target_image(UVec2::new(1, 1)));
    let blur_output_image = images.add(build_render_target_image(UVec2::new(1, 1)));

    let ambient_material = ambient_materials.add(AmbientOverlayMaterial::default());
    let blur_x_material = blur_materials.add(LightBlurMaterial::default());
    let blur_y_material = blur_materials.add(LightBlurMaterial::default());
    let composite_material = composite_materials.add(LightCompositeMaterial::default());

    let light_camera = commands
        .spawn((
            Name::new("Lighting2d Lightmap Camera"),
            LightingViewChild,
            LightingLightCameraChild,
            Camera2d,
            Camera {
                order: order - 3,
                clear_color: ClearColorConfig::Custom(Color::NONE),
                ..default()
            },
            projection.clone(),
            RenderTarget::Image(lightmap_image.clone().into()),
            Msaa::Off,
            RenderLayers::layer(layers.light),
            Transform::default(),
        ))
        .id();

    let blur_x_camera = commands
        .spawn((
            Name::new("Lighting2d Blur X Camera"),
            LightingViewChild,
            LightingBlurXCameraChild,
            Camera2d,
            Camera {
                order: order - 2,
                clear_color: ClearColorConfig::Custom(Color::NONE),
                ..default()
            },
            projection.clone(),
            RenderTarget::Image(blur_ping_image.clone().into()),
            Msaa::Off,
            RenderLayers::layer(layers.blur_x),
            Transform::default(),
        ))
        .id();

    let blur_y_camera = commands
        .spawn((
            Name::new("Lighting2d Blur Y Camera"),
            LightingViewChild,
            LightingBlurYCameraChild,
            Camera2d,
            Camera {
                order: order - 1,
                clear_color: ClearColorConfig::Custom(Color::NONE),
                ..default()
            },
            projection.clone(),
            RenderTarget::Image(blur_output_image.clone().into()),
            Msaa::Off,
            RenderLayers::layer(layers.blur_y),
            Transform::default(),
        ))
        .id();

    let ambient_overlay = commands
        .spawn((
            Name::new("Lighting2d Ambient Overlay"),
            LightingViewChild,
            AmbientOverlayChild,
            Mesh2d(internal.quad_mesh.clone()),
            MeshMaterial2d(ambient_material.clone()),
            NoFrustumCulling,
            RenderLayers::layer(layers.composite),
            Transform::from_xyz(0.0, 0.0, 900.0),
            Visibility::Visible,
        ))
        .id();

    let blur_x_quad = commands
        .spawn((
            Name::new("Lighting2d Blur X Quad"),
            LightingViewChild,
            BlurXQuadChild,
            Mesh2d(internal.quad_mesh.clone()),
            MeshMaterial2d(blur_x_material.clone()),
            NoFrustumCulling,
            RenderLayers::layer(layers.blur_x),
            Transform::default(),
            Visibility::Hidden,
        ))
        .id();

    let blur_y_quad = commands
        .spawn((
            Name::new("Lighting2d Blur Y Quad"),
            LightingViewChild,
            BlurYQuadChild,
            Mesh2d(internal.quad_mesh.clone()),
            MeshMaterial2d(blur_y_material.clone()),
            NoFrustumCulling,
            RenderLayers::layer(layers.blur_y),
            Transform::default(),
            Visibility::Hidden,
        ))
        .id();

    let composite_quad = commands
        .spawn((
            Name::new("Lighting2d Composite Quad"),
            LightingViewChild,
            CompositeQuadChild,
            Mesh2d(internal.quad_mesh.clone()),
            MeshMaterial2d(composite_material.clone()),
            NoFrustumCulling,
            RenderLayers::layer(layers.composite),
            Transform::from_xyz(0.0, 0.0, 910.0),
            Visibility::Visible,
        ))
        .id();

    commands.entity(owner).add_children(&[
        light_camera,
        blur_x_camera,
        blur_y_camera,
        ambient_overlay,
        blur_x_quad,
        blur_y_quad,
        composite_quad,
    ]);

    let runtime = LightingViewRuntime {
        layers,
        light_camera,
        blur_x_camera,
        blur_y_camera,
        ambient_overlay,
        blur_x_quad,
        blur_y_quad,
        composite_quad,
        lightmap_image,
        blur_ping_image,
        blur_output_image,
        ambient_material,
        blur_x_material,
        blur_y_material,
        composite_material,
    };

    commands.entity(owner).insert(runtime.clone());
    Some(runtime)
}

pub fn cleanup_orphaned_views(
    mut commands: Commands,
    cameras: Query<
        (
            Entity,
            &LightingViewRuntime,
            Option<&Lighting2dSettings>,
            Option<&RenderLayers>,
        ),
        (With<Camera>, Without<LightingViewChild>),
    >,
) {
    for (entity, runtime, settings, layers) in &cameras {
        if settings.is_some_and(|settings| settings.lighting_enabled) {
            continue;
        }

        cleanup_view_runtime(&mut commands, entity, runtime, layers);
    }
}

fn cleanup_view_runtime(
    commands: &mut Commands,
    owner: Entity,
    runtime: &LightingViewRuntime,
    layers: Option<&RenderLayers>,
) {
    for child in [
        runtime.light_camera,
        runtime.blur_x_camera,
        runtime.blur_y_camera,
        runtime.ambient_overlay,
        runtime.blur_x_quad,
        runtime.blur_y_quad,
        runtime.composite_quad,
    ] {
        if let Ok(mut entity_commands) = commands.get_entity(child) {
            entity_commands.despawn();
        }
    }

    if let Some(layers) = layers {
        let desired = layers.clone().without(runtime.layers.composite);
        commands.entity(owner).insert(desired);
    }
    commands.entity(owner).remove::<LightingViewRuntime>();
}

#[allow(clippy::too_many_arguments)]
pub fn sync_light_proxies(
    mut commands: Commands,
    runtime: Res<Lighting2dRuntimeState>,
    internal: Res<Lighting2dInternalAssets>,
    images: Res<Assets<Image>>,
    mut mask_cache: ResMut<LightingOccluderMaskCache>,
    mut materials: ResMut<Assets<LightRenderMaterial>>,
    cameras: Query<(Entity, &Lighting2dSettings, &LightingViewRuntime), Without<LightingViewChild>>,
    occluders: Query<(&LightOccluder2d, &GlobalTransform), Without<LightingViewChild>>,
    point_lights: Query<
        (Entity, &PointLight2d, &GlobalTransform),
        (Without<LightViewProxyChild>, Without<LightingViewChild>),
    >,
    spot_lights: Query<
        (Entity, &SpotLight2d, &GlobalTransform),
        (Without<LightViewProxyChild>, Without<LightingViewChild>),
    >,
    texture_lights: Query<
        (Entity, &TextureLight2d, &GlobalTransform),
        (Without<LightViewProxyChild>, Without<LightingViewChild>),
    >,
    existing_proxies: Query<(Entity, &LightViewProxy), With<LightViewProxyChild>>,
    mut light_transforms: Query<(&mut Transform, &mut Visibility), With<LightViewProxyChild>>,
) {
    if internal.quad_mesh == Handle::default() {
        return;
    }

    let existing = existing_proxies
        .iter()
        .map(|(entity, proxy)| {
            (
                (proxy.owner_camera, proxy.owner_light),
                (entity, proxy.material.clone()),
            )
        })
        .collect::<HashMap<_, _>>();
    let mut seen = HashSet::new();

    for (camera_entity, settings, view) in &cameras {
        if !settings.lighting_enabled || !runtime.active {
            continue;
        }

        let occlusion_mode = if settings.enable_occlusion {
            match settings.shadow_filter {
                ShadowFiltering2d::Hard => 1.0,
                ShadowFiltering2d::Soft => 2.0,
            }
        } else {
            0.0
        };

        for (light_entity, light, global) in &point_lights {
            let world_radius = point_light_world_radius(light, global);
            let segments = collect_light_segments(
                global.translation().truncate(),
                world_radius,
                settings.enable_occlusion,
                &images,
                &mut mask_cache.masks,
                &occluders,
            );
            let uniform = LightMaterialUniform {
                color: color_to_vec4(light.color) * light.intensity,
                position_and_kind: Vec4::new(
                    global.translation().x,
                    global.translation().y,
                    light.height,
                    0.0,
                ),
                radius_and_angles: Vec4::new(world_radius, light.falloff.max(0.001), 0.0, 0.0),
                direction_and_flags: Vec4::new(1.0, 0.0, segments.len() as f32, occlusion_mode),
                texture_size_and_rotation: Vec4::new(
                    world_radius * 2.0,
                    world_radius * 2.0,
                    1.0,
                    0.0,
                ),
                source_params: Vec4::new(light.source_radius.max(0.0), 0.0, 0.0, 0.0),
                metadata: UVec4::new(light.shadow_mode as u32, light.occluder_mask, 0, 0),
                segments: segment_uniforms(&segments),
            };

            let scale = Vec3::new(light.radius * 2.0, light.radius * 2.0, 1.0);
            sync_single_light_proxy(
                &mut commands,
                &mut materials,
                &internal,
                &existing,
                &mut seen,
                &mut light_transforms,
                view,
                camera_entity,
                light_entity,
                None,
                uniform,
                scale,
                true,
            );
        }

        for (light_entity, light, global) in &spot_lights {
            let world_radius = spot_light_world_radius(light, global);
            let direction = light_direction(light.direction_radians, global);
            let segments = collect_light_segments(
                global.translation().truncate(),
                world_radius,
                settings.enable_occlusion,
                &images,
                &mut mask_cache.masks,
                &occluders,
            );
            let uniform = LightMaterialUniform {
                color: color_to_vec4(light.color) * light.intensity,
                position_and_kind: Vec4::new(
                    global.translation().x,
                    global.translation().y,
                    light.height,
                    1.0,
                ),
                radius_and_angles: Vec4::new(
                    world_radius,
                    light.falloff.max(0.001),
                    light.inner_angle_radians.cos(),
                    light.outer_angle_radians.cos(),
                ),
                direction_and_flags: Vec4::new(
                    direction.x,
                    direction.y,
                    segments.len() as f32,
                    occlusion_mode,
                ),
                texture_size_and_rotation: Vec4::new(
                    world_radius * 2.0,
                    world_radius * 2.0,
                    1.0,
                    0.0,
                ),
                source_params: Vec4::new((light.source_width * 0.5).max(0.0), 0.0, 0.0, 0.0),
                metadata: UVec4::new(light.shadow_mode as u32, light.occluder_mask, 0, 0),
                segments: segment_uniforms(&segments),
            };

            let scale = Vec3::new(light.radius * 2.0, light.radius * 2.0, 1.0);
            sync_single_light_proxy(
                &mut commands,
                &mut materials,
                &internal,
                &existing,
                &mut seen,
                &mut light_transforms,
                view,
                camera_entity,
                light_entity,
                None,
                uniform,
                scale,
                true,
            );
        }

        for (light_entity, light, global) in &texture_lights {
            let world_size = texture_light_world_size(light, global);
            let search_radius = world_size.length() * 0.5;
            let segments = collect_light_segments(
                global.translation().truncate(),
                search_radius,
                settings.enable_occlusion,
                &images,
                &mut mask_cache.masks,
                &occluders,
            );
            let rotation = global_rotation_z(global) + light.rotation_radians;
            let uniform = LightMaterialUniform {
                color: color_to_vec4(light.color) * light.intensity,
                position_and_kind: Vec4::new(
                    global.translation().x,
                    global.translation().y,
                    light.height,
                    2.0,
                ),
                radius_and_angles: Vec4::new(search_radius, 1.0, 0.0, 0.0),
                direction_and_flags: Vec4::new(1.0, 0.0, segments.len() as f32, occlusion_mode),
                texture_size_and_rotation: Vec4::new(
                    world_size.x,
                    world_size.y,
                    rotation.cos(),
                    rotation.sin(),
                ),
                source_params: Vec4::new(light.source_radius.max(0.0), 0.0, 0.0, 0.0),
                metadata: UVec4::new(light.shadow_mode as u32, light.occluder_mask, 0, 0),
                segments: segment_uniforms(&segments),
            };

            sync_single_light_proxy(
                &mut commands,
                &mut materials,
                &internal,
                &existing,
                &mut seen,
                &mut light_transforms,
                view,
                camera_entity,
                light_entity,
                Some(light.texture.clone()),
                uniform,
                light.size.extend(1.0),
                true,
            );
        }
    }

    for (entity, proxy) in &existing_proxies {
        if !seen.contains(&(proxy.owner_camera, proxy.owner_light)) {
            commands.entity(entity).despawn();
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn sync_single_light_proxy(
    commands: &mut Commands,
    materials: &mut Assets<LightRenderMaterial>,
    internal: &Lighting2dInternalAssets,
    existing: &HashMap<(Entity, Entity), (Entity, Handle<LightRenderMaterial>)>,
    seen: &mut HashSet<(Entity, Entity)>,
    light_transforms: &mut Query<(&mut Transform, &mut Visibility), With<LightViewProxyChild>>,
    view: &LightingViewRuntime,
    owner_camera: Entity,
    owner_light: Entity,
    cookie_texture: Option<Handle<Image>>,
    uniform: LightMaterialUniform,
    scale: Vec3,
    visible: bool,
) {
    let key = (owner_camera, owner_light);
    seen.insert(key);

    let (child, material_handle) = if let Some((entity, material)) = existing.get(&key) {
        (*entity, material.clone())
    } else {
        let material_handle = materials.add(LightRenderMaterial::default());
        let child = commands
            .spawn((
                Name::new("Lighting2d Light Proxy"),
                LightViewProxyChild,
                LightViewProxy {
                    owner_camera,
                    owner_light,
                    material: material_handle.clone(),
                },
                Mesh2d(internal.quad_mesh.clone()),
                MeshMaterial2d(material_handle.clone()),
                RenderLayers::layer(view.layers.light),
                NoFrustumCulling,
                Transform::from_xyz(0.0, 0.0, 910.0).with_scale(scale),
                Visibility::Visible,
            ))
            .id();
        commands.entity(owner_light).add_child(child);
        (child, material_handle)
    };

    if let Ok((mut transform, mut proxy_visibility)) = light_transforms.get_mut(child) {
        transform.translation = Vec3::new(0.0, 0.0, 910.0);
        transform.scale = scale;
        *proxy_visibility = if visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }

    if let Some(material) = materials.get_mut(&material_handle) {
        material.uniform = uniform;
        material.cookie_texture = cookie_texture;
    }
}

#[allow(clippy::too_many_arguments)]
pub fn sync_receiver_proxies(
    mut commands: Commands,
    runtime: Res<Lighting2dRuntimeState>,
    internal: Res<Lighting2dInternalAssets>,
    images: Res<Assets<Image>>,
    atlases: Res<Assets<TextureAtlasLayout>>,
    mut mask_cache: ResMut<LightingOccluderMaskCache>,
    mut materials: ResMut<Assets<ReceiverLightingMaterial>>,
    cameras: Query<(Entity, &Lighting2dSettings, &LightingViewRuntime), Without<LightingViewChild>>,
    occluders: Query<(&LightOccluder2d, &GlobalTransform), Without<LightingViewChild>>,
    point_lights: Query<(Entity, &PointLight2d, &GlobalTransform), Without<LightingViewChild>>,
    spot_lights: Query<(Entity, &SpotLight2d, &GlobalTransform), Without<LightingViewChild>>,
    texture_lights: Query<(Entity, &TextureLight2d, &GlobalTransform), Without<LightingViewChild>>,
    owners: Query<
        (
            Entity,
            &Sprite,
            &Anchor,
            &GlobalTransform,
            Option<&NormalMappedSprite2d>,
            Option<&EmissiveSprite2d>,
        ),
        (
            Or<(With<NormalMappedSprite2d>, With<EmissiveSprite2d>)>,
            Without<ReceiverViewProxyChild>,
            Without<LightingViewChild>,
        ),
    >,
    existing_proxies: Query<(Entity, &ReceiverViewProxy), With<ReceiverViewProxyChild>>,
    mut receiver_transforms: Query<(&mut Transform, &mut Visibility), With<ReceiverViewProxyChild>>,
) {
    if internal.quad_mesh == Handle::default() {
        return;
    }

    let existing = existing_proxies
        .iter()
        .map(|(entity, proxy)| {
            (
                (proxy.owner_camera, proxy.owner_receiver),
                (entity, proxy.material.clone()),
            )
        })
        .collect::<HashMap<_, _>>();
    let mut seen = HashSet::new();

    for (camera_entity, settings, view) in &cameras {
        if !settings.lighting_enabled || !runtime.active {
            continue;
        }

        for (owner, sprite, anchor, global, normal, emissive) in &owners {
            let has_normal = normal.is_some() && settings.enable_normal_maps;
            let has_emissive = emissive.is_some() && settings.enable_emissive;

            if !has_normal && !has_emissive {
                continue;
            }

            let size = sprite_draw_size(sprite, &images, &atlases);
            let sprite_scale = global.scale().truncate().abs().max_element().max(0.001);
            let sprite_radius = size.length() * 0.5 * sprite_scale;
            let sprite_position = global.translation().truncate();

            let mut lights = collect_receiver_lights(
                sprite_position,
                sprite_radius,
                &images,
                &point_lights,
                &spot_lights,
                &texture_lights,
            );
            let cookie_textures = assign_receiver_cookie_slots(&mut lights);
            lights.retain(|light| light.packed.kind != 2.0 || light.packed.cookie_slot > 0);
            lights.truncate(MAX_RECEIVER_LIGHTS);

            let search_radius = lights
                .iter()
                .map(|light| light.packed.radius + light.packed.position.distance(sprite_position))
                .fold(sprite_radius, f32::max);
            let segments = if settings.enable_occlusion {
                collect_receiver_segments(
                    sprite_position,
                    search_radius,
                    &images,
                    &mut mask_cache.masks,
                    &occluders,
                )
            } else {
                Vec::new()
            };

            let key = (camera_entity, owner);
            seen.insert(key);

            let (child, material_handle) = if let Some((entity, material)) = existing.get(&key) {
                (*entity, material.clone())
            } else {
                let material_handle = materials.add(ReceiverLightingMaterial::default());
                let child = commands
                    .spawn((
                        Name::new("Lighting2d Receiver Proxy"),
                        ReceiverViewProxyChild,
                        ReceiverViewProxy {
                            owner_camera: camera_entity,
                            owner_receiver: owner,
                            material: material_handle.clone(),
                        },
                        Mesh2d(internal.quad_mesh.clone()),
                        MeshMaterial2d(material_handle.clone()),
                        RenderLayers::layer(view.layers.light),
                        NoFrustumCulling,
                        Transform::default(),
                        Visibility::Visible,
                    ))
                    .id();
                commands.entity(owner).add_child(child);
                (child, material_handle)
            };

            if let Ok((mut transform, mut visibility)) = receiver_transforms.get_mut(child) {
                transform.translation = sprite_anchor_translation(anchor, size, 920.0);
                transform.scale = size.extend(1.0).max(Vec3::splat(1.0));
                *visibility = if runtime.active && (!lights.is_empty() || has_emissive) {
                    Visibility::Visible
                } else {
                    Visibility::Hidden
                };
            }

            let diffuse_texture = if sprite.image != Handle::default() {
                sprite.image.clone()
            } else {
                internal.white_image.clone()
            };
            let normal_texture =
                if let Some(normal) = normal.filter(|_| settings.enable_normal_maps) {
                    normal.normal_map.clone()
                } else {
                    internal.flat_normal_image.clone()
                };
            let emissive_mask = emissive
                .filter(|_| settings.enable_emissive)
                .and_then(|emissive| emissive.mask.clone())
                .unwrap_or_else(|| internal.white_mask_image.clone());
            let uv_rect = sprite_uv_rect(sprite, &images, &atlases);

            if let Some(material) = materials.get_mut(&material_handle) {
                material.uniform = ReceiverLightingUniform {
                    base_color: color_to_vec4(sprite.color),
                    emissive_color: emissive
                        .filter(|_| settings.enable_emissive)
                        .map_or(Vec4::ZERO, |emissive| color_to_vec4(emissive.color)),
                    uv_rect: Vec4::new(uv_rect.min.x, uv_rect.min.y, uv_rect.max.x, uv_rect.max.y),
                    params: Vec4::new(
                        normal.map_or(0.0, |normal| normal.strength * settings.normal_map_scale),
                        normal.map_or(0.0, |normal| normal.height),
                        emissive.map_or(0.0, |emissive| emissive.intensity),
                        if settings.shadow_filter == ShadowFiltering2d::Soft {
                            1.0
                        } else {
                            0.0
                        },
                    ),
                    flags: Vec4::new(
                        if sprite.flip_x { 1.0 } else { 0.0 },
                        if sprite.flip_y { 1.0 } else { 0.0 },
                        lights.len() as f32,
                        segments.len() as f32,
                    ),
                    lights: receiver_light_uniforms(&lights),
                    segments: segment_uniforms(&segments),
                };
                material.diffuse_texture = Some(diffuse_texture);
                material.normal_texture = Some(normal_texture);
                material.emissive_mask = Some(emissive_mask);
                material.cookie_texture_0 = cookie_textures[0].clone();
                material.cookie_texture_1 = cookie_textures[1].clone();
                material.cookie_texture_2 = cookie_textures[2].clone();
                material.cookie_texture_3 = cookie_textures[3].clone();
            }
        }
    }

    for (entity, proxy) in &existing_proxies {
        if !seen.contains(&(proxy.owner_camera, proxy.owner_receiver)) {
            commands.entity(entity).despawn();
        }
    }
}

pub fn cleanup_orphaned_light_proxies(
    mut commands: Commands,
    cameras: Query<(), With<LightingViewRuntime>>,
    lights: Query<(), Or<(With<PointLight2d>, With<SpotLight2d>, With<TextureLight2d>)>>,
    proxies: Query<(Entity, &LightViewProxy), With<LightViewProxyChild>>,
) {
    for (entity, proxy) in &proxies {
        if cameras.get(proxy.owner_camera).is_err() || lights.get(proxy.owner_light).is_err() {
            commands.entity(entity).despawn();
        }
    }
}

pub fn cleanup_orphaned_receiver_proxies(
    mut commands: Commands,
    cameras: Query<(), With<LightingViewRuntime>>,
    receivers: Query<(), Or<(With<NormalMappedSprite2d>, With<EmissiveSprite2d>)>>,
    proxies: Query<(Entity, &ReceiverViewProxy), With<ReceiverViewProxyChild>>,
) {
    for (entity, proxy) in &proxies {
        if cameras.get(proxy.owner_camera).is_err() || receivers.get(proxy.owner_receiver).is_err()
        {
            commands.entity(entity).despawn();
        }
    }
}

pub fn cleanup_all(
    cameras: Query<
        (Entity, &LightingViewRuntime, Option<&RenderLayers>),
        (With<Camera>, Without<LightingViewChild>),
    >,
    lights: Query<Entity, With<LightViewProxyChild>>,
    receivers: Query<Entity, With<ReceiverViewProxyChild>>,
    mut commands: Commands,
) {
    for (entity, runtime, layers) in &cameras {
        cleanup_view_runtime(&mut commands, entity, runtime, layers);
    }
    for entity in &lights {
        commands.entity(entity).despawn();
    }
    for entity in &receivers {
        commands.entity(entity).despawn();
    }
}

fn camera_logical_area(
    camera: &Camera,
    projection: &Projection,
    primary_window: Option<&Window>,
) -> (Vec2, Vec2) {
    if let Projection::Orthographic(orthographic) = projection {
        return (
            orthographic.area.size().max(Vec2::ONE),
            orthographic.area.center(),
        );
    }

    let logical_size = camera
        .logical_viewport_size()
        .or_else(|| {
            primary_window.map(|window| {
                Vec2::new(
                    window.physical_width() as f32 / window.scale_factor(),
                    window.physical_height() as f32 / window.scale_factor(),
                )
            })
        })
        .unwrap_or(Vec2::splat(2048.0));
    (logical_size.max(Vec2::ONE), Vec2::ZERO)
}

fn camera_physical_size(camera: &Camera, primary_window: Option<&Window>) -> Option<UVec2> {
    camera.physical_viewport_size().or_else(|| {
        primary_window.map(|window| UVec2::new(window.physical_width(), window.physical_height()))
    })
}

fn scaled_lightmap_size(physical_size: UVec2, scale: f32) -> UVec2 {
    let scale = scale.max(0.05);
    UVec2::new(
        ((physical_size.x.max(1) as f32) * scale).round().max(1.0) as u32,
        ((physical_size.y.max(1) as f32) * scale).round().max(1.0) as u32,
    )
}

fn collect_light_segments(
    center: Vec2,
    radius: f32,
    enable_occlusion: bool,
    images: &Assets<Image>,
    mask_cache: &mut OccluderMaskCache,
    occluders: &Query<(&LightOccluder2d, &GlobalTransform), Without<LightingViewChild>>,
) -> Vec<PackedSegment> {
    if !enable_occlusion {
        return Vec::new();
    }

    let mut packed = Vec::new();
    for (occluder, global) in occluders.iter() {
        let distance = global.translation().truncate().distance(center);
        let bound_radius = occluder_bounding_radius(occluder, global, images, mask_cache);
        if distance > radius + bound_radius {
            continue;
        }

        packed.extend(flatten_occluder_segments(
            occluder, global, images, mask_cache,
        ));
        if packed.len() >= MAX_OCCLUDER_SEGMENTS {
            break;
        }
    }

    truncate_segments(packed)
}

fn collect_receiver_segments(
    center: Vec2,
    search_radius: f32,
    images: &Assets<Image>,
    mask_cache: &mut OccluderMaskCache,
    occluders: &Query<(&LightOccluder2d, &GlobalTransform), Without<LightingViewChild>>,
) -> Vec<PackedSegment> {
    let mut packed = Vec::new();
    for (occluder, global) in occluders.iter() {
        let distance = global.translation().truncate().distance(center);
        let bound_radius = occluder_bounding_radius(occluder, global, images, mask_cache);
        if distance > search_radius + bound_radius {
            continue;
        }

        packed.extend(flatten_occluder_segments(
            occluder, global, images, mask_cache,
        ));
        if packed.len() >= MAX_OCCLUDER_SEGMENTS {
            break;
        }
    }

    truncate_segments(packed)
}

fn collect_receiver_lights(
    center: Vec2,
    radius: f32,
    images: &Assets<Image>,
    point_lights: &Query<(Entity, &PointLight2d, &GlobalTransform), Without<LightingViewChild>>,
    spot_lights: &Query<(Entity, &SpotLight2d, &GlobalTransform), Without<LightingViewChild>>,
    texture_lights: &Query<(Entity, &TextureLight2d, &GlobalTransform), Without<LightingViewChild>>,
) -> Vec<CollectedReceiverLight> {
    let mut lights = Vec::new();

    for (_, light, global) in point_lights.iter() {
        let world_radius = point_light_world_radius(light, global);
        let distance = global.translation().truncate().distance(center);
        if distance > world_radius + radius {
            continue;
        }

        lights.push(CollectedReceiverLight {
            packed: PackedReceiverLight {
                kind: 0.0,
                color: color_to_vec4(light.color) * light.intensity,
                position: global.translation().truncate(),
                height: light.height,
                radius: world_radius,
                falloff: light.falloff.max(0.001),
                direction: Vec2::X,
                inner_cos: 1.0,
                outer_cos: 0.0,
                source_extent: light.source_radius.max(0.0),
                cookie_strength: 1.0,
                shadow_mode: light.shadow_mode,
                occluder_mask: light.occluder_mask,
                cookie_size: Vec2::ZERO,
                cookie_rotation: Vec2::ZERO,
                cookie_slot: 0,
            },
            cookie_texture: None,
        });
    }

    for (_, light, global) in spot_lights.iter() {
        let world_radius = spot_light_world_radius(light, global);
        let distance = global.translation().truncate().distance(center);
        if distance > world_radius + radius {
            continue;
        }

        lights.push(CollectedReceiverLight {
            packed: PackedReceiverLight {
                kind: 1.0,
                color: color_to_vec4(light.color) * light.intensity,
                position: global.translation().truncate(),
                height: light.height,
                radius: world_radius,
                falloff: light.falloff.max(0.001),
                direction: light_direction(light.direction_radians, global),
                inner_cos: light.inner_angle_radians.cos(),
                outer_cos: light.outer_angle_radians.cos(),
                source_extent: (light.source_width * 0.5).max(0.0),
                cookie_strength: 1.0,
                shadow_mode: light.shadow_mode,
                occluder_mask: light.occluder_mask,
                cookie_size: Vec2::ZERO,
                cookie_rotation: Vec2::ZERO,
                cookie_slot: 0,
            },
            cookie_texture: None,
        });
    }

    for (_, light, global) in texture_lights.iter() {
        let world_size = texture_light_world_size(light, global);
        let search_radius = world_size.length() * 0.5;
        let distance = global.translation().truncate().distance(center);
        if distance > search_radius + radius {
            continue;
        }

        let cookie_strength = sample_texture_light_cookie(light, global, images, center);
        if cookie_strength <= 0.0001 {
            continue;
        }

        let rotation = global_rotation_z(global) + light.rotation_radians;
        lights.push(CollectedReceiverLight {
            packed: PackedReceiverLight {
                kind: 2.0,
                color: color_to_vec4(light.color) * light.intensity,
                position: global.translation().truncate(),
                height: light.height,
                radius: search_radius,
                falloff: 1.0,
                direction: Vec2::new(rotation.cos(), rotation.sin()),
                inner_cos: 0.0,
                outer_cos: 0.0,
                source_extent: light.source_radius.max(0.0),
                cookie_strength,
                shadow_mode: light.shadow_mode,
                occluder_mask: light.occluder_mask,
                cookie_size: world_size,
                cookie_rotation: Vec2::new(rotation.cos(), rotation.sin()),
                cookie_slot: 0,
            },
            cookie_texture: Some(light.texture.clone()),
        });
    }

    lights.sort_by(|left, right| {
        let left_score = score_receiver_light(&left.packed, center);
        let right_score = score_receiver_light(&right.packed, center);
        right_score.total_cmp(&left_score)
    });

    lights
}

fn score_receiver_light(light: &PackedReceiverLight, center: Vec2) -> f32 {
    light.color.max_element() * light.cookie_strength / light.position.distance(center).max(1.0)
}

fn assign_receiver_cookie_slots(
    lights: &mut [CollectedReceiverLight],
) -> [Option<Handle<Image>>; MAX_RECEIVER_COOKIE_TEXTURES] {
    let mut textures = std::array::from_fn(|_| None);
    let mut slots = HashMap::new();
    let mut next_slot = 1u32;

    for light in lights.iter_mut() {
        let Some(texture) = light.cookie_texture.clone() else {
            continue;
        };

        let slot = if let Some(slot) = slots.get(&texture.id()) {
            *slot
        } else if next_slot <= MAX_RECEIVER_COOKIE_TEXTURES as u32 {
            let slot = next_slot;
            textures[(slot - 1) as usize] = Some(texture.clone());
            slots.insert(texture.id(), slot);
            next_slot += 1;
            slot
        } else {
            0
        };

        light.packed.cookie_slot = slot;
    }

    textures
}

fn segment_uniforms(segments: &[PackedSegment]) -> [SegmentUniform; MAX_OCCLUDER_SEGMENTS] {
    let mut uniforms = [SegmentUniform::default(); MAX_OCCLUDER_SEGMENTS];
    for (uniform, segment) in uniforms.iter_mut().zip(segments.iter()) {
        *uniform = SegmentUniform {
            segment: Vec4::new(
                segment.start.x,
                segment.start.y,
                segment.end.x,
                segment.end.y,
            ),
            transmission: Vec4::new(
                segment.transmission.x,
                segment.transmission.y,
                segment.transmission.z,
                segment.transmission.max_element(),
            ),
            metadata: UVec4::new(segment.occluder_groups, 0, 0, 0),
        };
    }
    uniforms
}

fn receiver_light_uniforms(
    lights: &[CollectedReceiverLight],
) -> [ReceiverLightUniform; MAX_RECEIVER_LIGHTS] {
    let mut uniforms = [ReceiverLightUniform::default(); MAX_RECEIVER_LIGHTS];
    for (uniform, light) in uniforms.iter_mut().zip(lights.iter()) {
        *uniform = ReceiverLightUniform {
            color: light.packed.color,
            position_and_kind: Vec4::new(
                light.packed.position.x,
                light.packed.position.y,
                light.packed.height,
                light.packed.kind,
            ),
            radius_and_angles: Vec4::new(
                light.packed.radius,
                light.packed.falloff,
                light.packed.inner_cos,
                light.packed.outer_cos,
            ),
            direction_and_source: Vec4::new(
                light.packed.direction.x,
                light.packed.direction.y,
                light.packed.source_extent,
                light.packed.cookie_strength,
            ),
            cookie_size_and_rotation: Vec4::new(
                light.packed.cookie_size.x,
                light.packed.cookie_size.y,
                light.packed.cookie_rotation.x,
                light.packed.cookie_rotation.y,
            ),
            metadata: UVec4::new(
                light.packed.shadow_mode as u32,
                light.packed.occluder_mask,
                light.packed.cookie_slot,
                0,
            ),
        };
    }
    uniforms
}

pub fn publish_diagnostics(
    cameras: Query<(), (With<Lighting2dSettings>, Without<LightingViewChild>)>,
    point_lights: Query<(), With<PointLight2d>>,
    spot_lights: Query<(), With<SpotLight2d>>,
    texture_lights: Query<(), With<TextureLight2d>>,
    occluders: Query<(), With<LightOccluder2d>>,
    normal_sprites: Query<(), With<NormalMappedSprite2d>>,
    emissive_sprites: Query<(), With<EmissiveSprite2d>>,
    mut diagnostics: ResMut<Lighting2dDiagnostics>,
) {
    diagnostics.active_cameras = cameras.iter().count();
    diagnostics.active_point_lights = point_lights.iter().count();
    diagnostics.active_spot_lights = spot_lights.iter().count();
    diagnostics.active_texture_lights = texture_lights.iter().count();
    diagnostics.active_occluders = occluders.iter().count();
    diagnostics.active_normal_mapped_sprites = normal_sprites.iter().count();
    diagnostics.active_emissive_sprites = emissive_sprites.iter().count();
}
