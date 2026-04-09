use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};

use crate::{
    LightOccluder2d, OccluderShape2d,
    TextureLight2d,
    geometry::{
        flatten_occluder_segments, sample_texture_light_cookie, sprite_draw_size, sprite_uv_rect,
        truncate_segments,
    },
};

fn empty_mask_context() -> (Assets<Image>, crate::geometry::OccluderMaskCache) {
    (Assets::default(), crate::geometry::OccluderMaskCache::default())
}

#[test]
fn rectangle_occluder_flattens_to_four_segments() {
    let occluder = LightOccluder2d::rectangle(Vec2::new(10.0, 6.0));
    let transform = GlobalTransform::default();
    let (images, mut cache) = empty_mask_context();

    let segments = flatten_occluder_segments(&occluder, &transform, &images, &mut cache);

    assert_eq!(segments.len(), 4);
    assert_eq!(segments[0].start, Vec2::new(-10.0, -6.0));
    assert_eq!(segments[0].end, Vec2::new(10.0, -6.0));
}

#[test]
fn polygon_occluder_preserves_vertex_count() {
    let occluder = LightOccluder2d::polygon(vec![
        Vec2::new(-6.0, -4.0),
        Vec2::new(5.0, -3.0),
        Vec2::new(4.0, 7.0),
    ]);
    let transform = GlobalTransform::from_translation(Vec3::new(4.0, 2.0, 0.0));
    let (images, mut cache) = empty_mask_context();

    let segments = flatten_occluder_segments(&occluder, &transform, &images, &mut cache);

    assert_eq!(segments.len(), 3);
    assert_eq!(segments[0].start, Vec2::new(-2.0, -2.0));
}

#[test]
fn circle_occluder_respects_requested_segments() {
    let occluder = LightOccluder2d {
        shape: OccluderShape2d::Circle {
            radius: 8.0,
            segments: 12,
        },
        ..default()
    };
    let (images, mut cache) = empty_mask_context();

    let segments = flatten_occluder_segments(
        &occluder,
        &GlobalTransform::default(),
        &images,
        &mut cache,
    );

    assert_eq!(segments.len(), 12);
}

#[test]
fn segment_truncation_caps_at_shader_limit() {
    let segments = (0..96)
        .map(|index| crate::geometry::PackedSegment {
            start: Vec2::splat(index as f32),
            end: Vec2::splat(index as f32 + 1.0),
            transmission: Vec3::ZERO,
            occluder_groups: 1,
        })
        .collect::<Vec<_>>();

    let truncated = truncate_segments(segments);

    assert_eq!(truncated.len(), crate::materials::MAX_OCCLUDER_SEGMENTS);
}

#[test]
fn sprite_rect_produces_expected_size_and_uvs() {
    let mut images = Assets::<Image>::default();
    let image = Image::new_fill(
        Extent3d {
            width: 100,
            height: 50,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[255, 255, 255, 255],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    let handle = images.add(image);
    let sprite = Sprite {
        image: handle,
        rect: Some(Rect::new(10.0, 5.0, 60.0, 25.0)),
        ..default()
    };

    let size = sprite_draw_size(&sprite, &images, &Assets::default());
    let uv = sprite_uv_rect(&sprite, &images, &Assets::default());

    assert_eq!(size, Vec2::new(50.0, 20.0));
    assert_eq!(uv.min, Vec2::new(0.1, 0.1));
    assert_eq!(uv.max, Vec2::new(0.6, 0.5));
}

#[test]
fn occluder_shadow_tint_is_encoded_as_transmission() {
    let occluder = LightOccluder2d {
        absorption: 1.0,
        shadow_tint: Color::srgb(0.8, 0.45, 0.15),
        ..LightOccluder2d::rectangle(Vec2::new(8.0, 8.0))
    };
    let (images, mut cache) = empty_mask_context();

    let segments = flatten_occluder_segments(
        &occluder,
        &GlobalTransform::default(),
        &images,
        &mut cache,
    );

    assert_eq!(segments.len(), 4);
    assert!(segments[0].transmission.x > segments[0].transmission.z);
    assert!(segments[0].transmission.y > segments[0].transmission.z);
}

#[test]
fn mask_occluder_extracts_boundary_segments() {
    let mut images = Assets::<Image>::default();
    let image = Image::new_fill(
        Extent3d {
            width: 3,
            height: 3,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[
            0, 0, 0, 0, 255, 255, 255, 255, 0, 0, 0, 0,
            255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
            0, 0, 0, 0, 255, 255, 255, 255, 0, 0, 0, 0,
        ],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    let handle = images.add(image);
    let occluder = LightOccluder2d {
        shape: OccluderShape2d::Mask {
            mask: handle,
            alpha_threshold: 0.1,
        },
        groups: 0b1010,
        ..default()
    };
    let mut cache = crate::geometry::OccluderMaskCache::default();

    let segments = flatten_occluder_segments(
        &occluder,
        &GlobalTransform::default(),
        &images,
        &mut cache,
    );

    assert!(segments.len() >= 4);
    assert!(segments.iter().all(|segment| segment.occluder_groups == 0b1010));
}

#[test]
fn texture_light_cookie_sampling_reads_image_intensity() {
    let mut images = Assets::<Image>::default();
    let image = Image::new_fill(
        Extent3d {
            width: 2,
            height: 2,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[
            255, 255, 255, 255, 0, 0, 0, 255, 0, 0, 0, 255, 0, 0, 0, 255,
        ],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    let handle = images.add(image);
    let light = TextureLight2d {
        texture: handle,
        size: Vec2::splat(100.0),
        ..default()
    };

    let strength = sample_texture_light_cookie(
        &light,
        &GlobalTransform::default(),
        &images,
        Vec2::new(-24.0, -24.0),
    );

    assert!(strength > 0.9);
}
