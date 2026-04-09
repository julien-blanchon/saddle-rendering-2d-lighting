use std::collections::HashMap;

use bevy::{
    asset::{AssetId, Assets},
    image::TextureAtlasLayout,
    prelude::*,
    sprite::Anchor,
};

use crate::{
    components::{
        LightOccluder2d, LightShadowMode2d, OccluderShape2d, PointLight2d, SpotLight2d,
        TextureLight2d,
    },
    materials::MAX_OCCLUDER_SEGMENTS,
};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct PackedSegment {
    pub start: Vec2,
    pub end: Vec2,
    pub transmission: Vec3,
    pub occluder_groups: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PackedReceiverLight {
    pub kind: f32,
    pub color: Vec4,
    pub position: Vec2,
    pub height: f32,
    pub radius: f32,
    pub falloff: f32,
    pub direction: Vec2,
    pub inner_cos: f32,
    pub outer_cos: f32,
    pub source_extent: f32,
    pub cookie_strength: f32,
    pub shadow_mode: LightShadowMode2d,
    pub occluder_mask: u32,
    pub cookie_size: Vec2,
    pub cookie_rotation: Vec2,
    pub cookie_slot: u32,
}

impl Default for PackedReceiverLight {
    fn default() -> Self {
        Self {
            kind: 0.0,
            color: Vec4::ZERO,
            position: Vec2::ZERO,
            height: 0.0,
            radius: 0.0,
            falloff: 0.0,
            direction: Vec2::ZERO,
            inner_cos: 0.0,
            outer_cos: 0.0,
            source_extent: 0.0,
            cookie_strength: 0.0,
            shadow_mode: LightShadowMode2d::Illuminated,
            occluder_mask: u32::MAX,
            cookie_size: Vec2::ZERO,
            cookie_rotation: Vec2::ZERO,
            cookie_slot: 0,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct CachedMaskGeometry {
    pub segments: Vec<(Vec2, Vec2)>,
    pub bounding_radius: f32,
}

pub(crate) type OccluderMaskCache = HashMap<(AssetId<Image>, u8), CachedMaskGeometry>;

fn quantized_alpha_threshold(threshold: f32) -> u8 {
    (threshold.clamp(0.0, 1.0) * 255.0).round() as u8
}

#[must_use]
pub(crate) fn color_to_vec4(color: Color) -> Vec4 {
    let linear = color.to_linear();
    Vec4::new(linear.red, linear.green, linear.blue, linear.alpha)
}

#[must_use]
pub(crate) fn point_light_world_radius(light: &PointLight2d, global: &GlobalTransform) -> f32 {
    light.radius * global.scale().truncate().abs().max_element().max(0.001)
}

#[must_use]
pub(crate) fn spot_light_world_radius(light: &SpotLight2d, global: &GlobalTransform) -> f32 {
    light.radius * global.scale().truncate().abs().max_element().max(0.001)
}

#[must_use]
pub(crate) fn texture_light_world_size(light: &TextureLight2d, global: &GlobalTransform) -> Vec2 {
    light.size * global.scale().truncate().abs().max(Vec2::splat(0.001))
}

#[must_use]
pub(crate) fn global_rotation_z(global: &GlobalTransform) -> f32 {
    global
        .compute_transform()
        .rotation
        .to_euler(EulerRot::XYZ)
        .2
}

#[must_use]
pub(crate) fn light_direction(direction_radians: f32, global: &GlobalTransform) -> Vec2 {
    let angle = direction_radians + global_rotation_z(global);
    Vec2::from_angle(angle)
}

#[must_use]
pub(crate) fn sprite_draw_size(
    sprite: &Sprite,
    images: &Assets<Image>,
    atlases: &Assets<TextureAtlasLayout>,
) -> Vec2 {
    let image_size = images
        .get(&sprite.image)
        .map(|image| image.size_f32())
        .unwrap_or(Vec2::ONE);

    let atlas_rect = sprite
        .texture_atlas
        .as_ref()
        .and_then(|atlas| atlas.texture_rect(atlases))
        .map(|rect| rect.as_rect());

    let texture_rect = match (atlas_rect, sprite.rect) {
        (None, None) => Rect::from_corners(Vec2::ZERO, image_size),
        (None, Some(rect)) => rect,
        (Some(rect), None) => rect,
        (Some(atlas_rect), Some(mut rect)) => {
            rect.min += atlas_rect.min;
            rect.max += atlas_rect.min;
            rect
        }
    };

    sprite.custom_size.unwrap_or_else(|| texture_rect.size())
}

#[must_use]
pub(crate) fn sprite_uv_rect(
    sprite: &Sprite,
    images: &Assets<Image>,
    atlases: &Assets<TextureAtlasLayout>,
) -> Rect {
    let image_size = images
        .get(&sprite.image)
        .map(|image| image.size_f32())
        .unwrap_or(Vec2::ONE);

    let atlas_rect = sprite
        .texture_atlas
        .as_ref()
        .and_then(|atlas| atlas.texture_rect(atlases))
        .map(|rect| rect.as_rect());

    let texture_rect = match (atlas_rect, sprite.rect) {
        (None, None) => Rect::from_corners(Vec2::ZERO, image_size),
        (None, Some(rect)) => rect,
        (Some(rect), None) => rect,
        (Some(atlas_rect), Some(mut rect)) => {
            rect.min += atlas_rect.min;
            rect.max += atlas_rect.min;
            rect
        }
    };

    Rect::from_corners(texture_rect.min / image_size, texture_rect.max / image_size)
}

#[must_use]
pub(crate) fn sprite_anchor_translation(anchor: &Anchor, size: Vec2, z: f32) -> Vec3 {
    Vec3::new(-anchor.as_vec().x * size.x, -anchor.as_vec().y * size.y, z)
}

#[must_use]
pub(crate) fn occluder_bounding_radius(
    occluder: &LightOccluder2d,
    global: &GlobalTransform,
    images: &Assets<Image>,
    mask_cache: &mut OccluderMaskCache,
) -> f32 {
    let scale = global.scale().truncate().abs().max(Vec2::splat(0.001));

    match &occluder.shape {
        OccluderShape2d::Rectangle { half_size } => (*half_size * scale).length(),
        OccluderShape2d::Circle { radius, .. } => *radius * scale.max_element(),
        OccluderShape2d::Polygon { points } => points
            .iter()
            .map(|point| (*point * scale).length())
            .fold(0.0, f32::max)
            .max(1.0),
        OccluderShape2d::Mask {
            mask,
            alpha_threshold,
        } => resolve_mask_geometry(mask, *alpha_threshold, images, mask_cache).map_or(
            1.0,
            |geometry| {
                let scale_radius = scale.max_element();
                geometry.bounding_radius.max(1.0) * scale_radius
            },
        ),
    }
}

#[must_use]
pub(crate) fn flatten_occluder_segments(
    occluder: &LightOccluder2d,
    global: &GlobalTransform,
    images: &Assets<Image>,
    mask_cache: &mut OccluderMaskCache,
) -> Vec<PackedSegment> {
    if !occluder.casts_shadows {
        return Vec::new();
    }

    let local_points = match &occluder.shape {
        OccluderShape2d::Rectangle { half_size } => vec![
            Vec2::new(-half_size.x, -half_size.y),
            Vec2::new(half_size.x, -half_size.y),
            Vec2::new(half_size.x, half_size.y),
            Vec2::new(-half_size.x, half_size.y),
        ],
        OccluderShape2d::Circle { radius, segments } => {
            let segments = (*segments).max(6);
            (0..segments)
                .map(|index| {
                    let angle = index as f32 / segments as f32 * std::f32::consts::TAU;
                    Vec2::from_angle(angle) * *radius
                })
                .collect()
        }
        OccluderShape2d::Polygon { points } => points.clone(),
        OccluderShape2d::Mask {
            mask,
            alpha_threshold,
        } => {
            let Some(geometry) = resolve_mask_geometry(mask, *alpha_threshold, images, mask_cache)
            else {
                return Vec::new();
            };

            let transform_segment = |(start, end): &(Vec2, Vec2)| PackedSegment {
                start: global.transform_point(start.extend(0.0)).truncate(),
                end: global.transform_point(end.extend(0.0)).truncate(),
                transmission: Vec3::ZERO,
                occluder_groups: occluder.groups,
            };

            let opacity = occluder.absorption.clamp(0.0, 1.0);
            let tint = color_to_vec4(occluder.shadow_tint)
                .truncate()
                .clamp(Vec3::ZERO, Vec3::ONE);
            let transmission = Vec3::splat(1.0 - opacity) + tint * opacity;

            return geometry
                .segments
                .iter()
                .map(|segment| {
                    let mut packed = transform_segment(segment);
                    packed.transmission = transmission;
                    packed
                })
                .collect();
        }
    };

    if local_points.len() < 2 {
        return Vec::new();
    }

    let world_points = local_points
        .iter()
        .map(|point| global.transform_point(point.extend(0.0)).truncate())
        .collect::<Vec<_>>();
    let opacity = occluder.absorption.clamp(0.0, 1.0);
    let tint = color_to_vec4(occluder.shadow_tint)
        .truncate()
        .clamp(Vec3::ZERO, Vec3::ONE);
    let transmission = Vec3::splat(1.0 - opacity) + tint * opacity;

    let mut segments = Vec::with_capacity(world_points.len());
    for index in 0..world_points.len() {
        let start = world_points[index];
        let end = world_points[(index + 1) % world_points.len()];
        segments.push(PackedSegment {
            start,
            end,
            transmission,
            occluder_groups: occluder.groups,
        });
    }

    segments
}

#[must_use]
pub(crate) fn truncate_segments(mut segments: Vec<PackedSegment>) -> Vec<PackedSegment> {
    segments.truncate(MAX_OCCLUDER_SEGMENTS);
    segments
}

#[must_use]
pub(crate) fn sample_texture_light_cookie(
    light: &TextureLight2d,
    global: &GlobalTransform,
    images: &Assets<Image>,
    world_point: Vec2,
) -> f32 {
    if light.texture == Handle::default() {
        return 1.0;
    }

    let Some(image) = images.get(&light.texture) else {
        return 1.0;
    };

    let world_size = texture_light_world_size(light, global).max(Vec2::splat(0.001));
    let rotation = global_rotation_z(global) + light.rotation_radians;
    let local = world_point - global.translation().truncate();
    let rotated = Vec2::new(
        rotation.cos() * local.x + rotation.sin() * local.y,
        -rotation.sin() * local.x + rotation.cos() * local.y,
    );
    let uv = rotated / world_size + Vec2::splat(0.5);
    if uv.cmplt(Vec2::ZERO).any() || uv.cmpgt(Vec2::ONE).any() {
        return 0.0;
    }

    let size = image.texture_descriptor.size;
    let px = ((uv.x.clamp(0.0, 0.999_9)) * size.width as f32) as u32;
    let py = ((uv.y.clamp(0.0, 0.999_9)) * size.height as f32) as u32;
    let Some(pixel) = image.pixel_bytes(UVec3::new(px, py, 0)) else {
        return 1.0;
    };

    let rgb = match pixel {
        [r, g, b, ..] => (*r as f32 + *g as f32 + *b as f32) / (3.0 * 255.0),
        [l, ..] => *l as f32 / 255.0,
        [] => 1.0,
    };
    let alpha = if pixel.len() >= 4 {
        pixel[3] as f32 / 255.0
    } else {
        1.0
    };

    (rgb * alpha).clamp(0.0, 1.0)
}

fn resolve_mask_geometry<'a>(
    mask: &Handle<Image>,
    alpha_threshold: f32,
    images: &'a Assets<Image>,
    mask_cache: &'a mut OccluderMaskCache,
) -> Option<&'a CachedMaskGeometry> {
    let key = (mask.id(), quantized_alpha_threshold(alpha_threshold));
    if let std::collections::hash_map::Entry::Vacant(entry) = mask_cache.entry(key) {
        let image = images.get(mask)?;
        let geometry = build_mask_geometry(image, key.1);
        entry.insert(geometry);
    }

    mask_cache.get(&key)
}

fn build_mask_geometry(image: &Image, alpha_threshold: u8) -> CachedMaskGeometry {
    let size = image.texture_descriptor.size;
    let width = size.width as i32;
    let height = size.height as i32;

    if width <= 0 || height <= 0 {
        return CachedMaskGeometry::default();
    }

    let mut opaque = vec![false; (width * height) as usize];
    for y in 0..height {
        for x in 0..width {
            opaque[(y * width + x) as usize] =
                pixel_alpha(image, x as u32, y as u32) >= alpha_threshold;
        }
    }

    let mut horizontal_edges: HashMap<i32, Vec<(i32, i32)>> = HashMap::new();
    let mut vertical_edges: HashMap<i32, Vec<(i32, i32)>> = HashMap::new();

    let is_opaque = |x: i32, y: i32| -> bool {
        if x < 0 || y < 0 || x >= width || y >= height {
            return false;
        }
        opaque[(y * width + x) as usize]
    };

    for y in 0..height {
        for x in 0..width {
            if !is_opaque(x, y) {
                continue;
            }

            if !is_opaque(x, y - 1) {
                horizontal_edges.entry(y).or_default().push((x, x + 1));
            }
            if !is_opaque(x, y + 1) {
                horizontal_edges.entry(y + 1).or_default().push((x, x + 1));
            }
            if !is_opaque(x - 1, y) {
                vertical_edges.entry(x).or_default().push((y, y + 1));
            }
            if !is_opaque(x + 1, y) {
                vertical_edges.entry(x + 1).or_default().push((y, y + 1));
            }
        }
    }

    let mut segments = Vec::new();
    let width_half = width as f32 * 0.5;
    let height_half = height as f32 * 0.5;

    for (grid_y, ranges) in &mut horizontal_edges {
        merge_ranges(ranges);
        let world_y = height_half - *grid_y as f32;
        for (start, end) in ranges.iter().copied() {
            let x0 = start as f32 - width_half;
            let x1 = end as f32 - width_half;
            segments.push((Vec2::new(x0, world_y), Vec2::new(x1, world_y)));
        }
    }

    for (grid_x, ranges) in &mut vertical_edges {
        merge_ranges(ranges);
        let world_x = *grid_x as f32 - width_half;
        for (start, end) in ranges.iter().copied() {
            let y0 = height_half - end as f32;
            let y1 = height_half - start as f32;
            segments.push((Vec2::new(world_x, y0), Vec2::new(world_x, y1)));
        }
    }

    let bounding_radius = segments
        .iter()
        .flat_map(|(start, end)| [start.length(), end.length()])
        .fold(0.0, f32::max)
        .max(1.0);

    CachedMaskGeometry {
        segments,
        bounding_radius,
    }
}

fn merge_ranges(ranges: &mut Vec<(i32, i32)>) {
    ranges.sort_unstable_by_key(|(start, _)| *start);
    let mut merged: Vec<(i32, i32)> = Vec::with_capacity(ranges.len());
    for &(start, end) in ranges.iter() {
        if let Some(last) = merged.last_mut()
            && start <= last.1
        {
            last.1 = last.1.max(end);
        } else {
            merged.push((start, end));
        }
    }
    *ranges = merged;
}

fn pixel_alpha(image: &Image, x: u32, y: u32) -> u8 {
    let Some(pixel) = image.pixel_bytes(UVec3::new(x, y, 0)) else {
        return 0;
    };

    match pixel {
        [_, _, _, a, ..] => *a,
        [l, ..] => *l,
        [] => 0,
    }
}
