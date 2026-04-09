use bevy::prelude::*;
use saddle_bevy_e2e::{
    action::Action,
    actions::{assertions, inspect},
    scenario::Scenario,
};
use saddle_rendering_2d_lighting::{
    LightOccluder2d, Lighting2dDiagnostics, Lighting2dSettings, NormalMappedSprite2d,
    OccluderShape2d, PointLight2d, ShadowFiltering2d, SpotLight2d, TextureLight2d,
};
use saddle_rendering_2d_lighting_example_common::{ExampleEntities, ExampleLightingPane};

pub fn list_scenarios() -> Vec<&'static str> {
    vec![
        "smoke_launch",
        "lighting_overview",
        "lighting_occluders",
        "lighting_normal_maps",
        "lighting_mixed_lights",
        "lighting_blurred_lightmap",
        "lighting_receiver_cookies",
        "lighting_mask_occluders",
        "lighting_soft_shadows",
        "lighting_multi_camera",
        "reference_dungeon",
        "reference_texture_cursor",
        "reference_mask_occluder",
        "reference_road_convoy",
        "lighting_stress",
    ]
}

pub fn scenario_by_name(name: &str) -> Option<Scenario> {
    match name {
        "smoke_launch" => Some(smoke_launch()),
        "lighting_overview" => Some(lighting_overview()),
        "lighting_occluders" => Some(lighting_occluders()),
        "lighting_normal_maps" => Some(lighting_normal_maps()),
        "lighting_mixed_lights" => Some(lighting_mixed_lights()),
        "lighting_blurred_lightmap" => Some(lighting_blurred_lightmap()),
        "lighting_receiver_cookies" => Some(lighting_receiver_cookies()),
        "lighting_mask_occluders" => Some(lighting_mask_occluders()),
        "lighting_soft_shadows" => Some(lighting_soft_shadows()),
        "lighting_multi_camera" => Some(lighting_multi_camera()),
        "reference_dungeon" => Some(reference_dungeon()),
        "reference_texture_cursor" => Some(reference_texture_cursor()),
        "reference_mask_occluder" => Some(reference_mask_occluder()),
        "reference_road_convoy" => Some(reference_road_convoy()),
        "lighting_stress" => Some(lighting_stress()),
        _ => None,
    }
}

fn smoke_launch() -> Scenario {
    Scenario::builder("smoke_launch")
        .description("Boot the lab, verify the diagnostics surface exists, and capture the opening composition.")
        .then(Action::WaitFrames(20))
        .then(assertions::resource_exists::<Lighting2dDiagnostics>(
            "lighting diagnostics resource exists",
        ))
        .then(assertions::custom("overlay contains the lab title", |world| {
            world
                .get_resource::<ExampleEntities>()
                .and_then(|entities| entities.overlay)
                .and_then(|overlay| world.get::<Text>(overlay))
                .is_some_and(|text| text.0.contains("2D Lighting Lab"))
        }))
        .then(inspect::log_resource::<Lighting2dDiagnostics>(
            "lighting smoke diagnostics",
        ))
        .then(Action::Screenshot("smoke_launch".into()))
        .then(Action::WaitFrames(1))
        .then(assertions::log_summary("smoke_launch"))
        .build()
}

fn lighting_overview() -> Scenario {
    Scenario::builder("lighting_overview")
        .description("Capture the integrated lab composition and verify at least one light family is active.")
        .then(Action::WaitFrames(20))
        .then(assertions::resource_satisfies::<Lighting2dDiagnostics>(
            "the lab has authored lights",
            |diagnostics| diagnostics.total_lights() >= 3,
        ))
        .then(Action::Screenshot("lighting_overview".into()))
        .then(Action::WaitFrames(1))
        .then(assertions::log_summary("lighting_overview"))
        .build()
}

fn lighting_occluders() -> Scenario {
    Scenario::builder("lighting_occluders")
        .description("Verify blocker authoring is present in the lab and capture the occluder-heavy composition.")
        .then(Action::WaitFrames(20))
        .then(assertions::resource_satisfies::<Lighting2dDiagnostics>(
            "the lab has multiple occluders",
            |diagnostics| diagnostics.active_occluders >= 3,
        ))
        .then(Action::Screenshot("lighting_occluders".into()))
        .then(Action::WaitFrames(1))
        .then(assertions::log_summary("lighting_occluders"))
        .build()
}

fn lighting_normal_maps() -> Scenario {
    Scenario::builder("lighting_normal_maps")
        .description("Verify the integrated lab scene contains normal-mapped and emissive receiver authoring.")
        .then(Action::WaitFrames(20))
        .then(assertions::resource_satisfies::<Lighting2dDiagnostics>(
            "normal and emissive receivers are present",
            |diagnostics| {
                diagnostics.active_normal_mapped_sprites >= 1
                    && diagnostics.active_emissive_sprites >= 1
            },
        ))
        .then(Action::Screenshot("lighting_normal_maps".into()))
        .then(Action::WaitFrames(1))
        .then(assertions::log_summary("lighting_normal_maps"))
        .build()
}

fn lighting_mixed_lights() -> Scenario {
    Scenario::builder("lighting_mixed_lights")
        .description("Verify point, spot, and texture light authoring all exist in the lab.")
        .then(Action::WaitFrames(20))
        .then(assertions::resource_satisfies::<Lighting2dDiagnostics>(
            "mixed light families are present",
            |diagnostics| {
                diagnostics.active_point_lights >= 1
                    && diagnostics.active_spot_lights >= 1
                    && diagnostics.active_texture_lights >= 1
            },
        ))
        .then(Action::Screenshot("lighting_mixed_lights".into()))
        .then(Action::WaitFrames(1))
        .then(assertions::log_summary("lighting_mixed_lights"))
        .build()
}

fn lighting_stress() -> Scenario {
    Scenario::builder("lighting_stress")
        .description("Spawn an extra ring of point lights into the lab and verify diagnostics grow accordingly.")
        .then(Action::WaitFrames(10))
        .then(Action::Custom(Box::new(|world: &mut World| {
            for index in 0..16 {
                let angle = index as f32 / 16.0 * std::f32::consts::TAU;
                let radius = 260.0;
                let position = Vec2::from_angle(angle) * radius + Vec2::new(0.0, -20.0);
                world.spawn((
                    Name::new(format!("Scenario Stress Light {index}")),
                    PointLight2d {
                        color: Color::hsl(index as f32 * 20.0, 0.75, 0.65),
                        intensity: 1.0,
                        radius: 48.0,
                        ..default()
                    },
                    Transform::from_xyz(position.x, position.y, 2.0),
                ));
            }
        })))
        .then(Action::WaitFrames(2))
        .then(assertions::resource_satisfies::<Lighting2dDiagnostics>(
            "stress injection increased point light count",
            |diagnostics| diagnostics.active_point_lights >= 17,
        ))
        .then(inspect::log_resource::<Lighting2dDiagnostics>(
            "lighting stress diagnostics",
        ))
        .then(Action::Screenshot("lighting_stress".into()))
        .then(Action::WaitFrames(1))
        .then(assertions::log_summary("lighting_stress"))
        .build()
}

fn lighting_soft_shadows() -> Scenario {
    Scenario::builder("lighting_soft_shadows")
        .description("Capture a hard-vs-soft shadow comparison using the same authored scene and main lighting camera.")
        .then(Action::WaitFrames(20))
        .then(Action::Custom(Box::new(|world: &mut World| {
            let Some(camera) = world
                .get_resource::<ExampleEntities>()
                .and_then(|entities| entities.camera)
            else {
                return;
            };
            if let Some(mut settings) = world.get_mut::<Lighting2dSettings>(camera) {
                settings.shadow_filter = ShadowFiltering2d::Hard;
            }
            if let Some(mut projection) = world.get_mut::<Projection>(camera) {
                if let Projection::Orthographic(orthographic) = &mut *projection {
                    orthographic.scale = 0.64;
                }
            }
            if let Some(mut transform) = world.get_mut::<Transform>(camera) {
                transform.translation = Vec3::new(-26.0, 24.0, transform.translation.z);
            }
            if let Some(mut pane) = world.get_resource_mut::<ExampleLightingPane>() {
                pane.point_source_radius = 180.0;
                pane.point_radius = 260.0;
                pane.blur_radius = 0.0;
                pane.texture_intensity = 0.0;
            }
            if let Some(point) = world
                .get_resource::<ExampleEntities>()
                .and_then(|entities| entities.primary_point_light)
            {
                if let Some(mut light) = world.get_mut::<PointLight2d>(point) {
                    light.intensity = 2.8;
                }
                if let Some(mut transform) = world.get_mut::<Transform>(point) {
                    transform.translation = Vec3::new(-148.0, 134.0, 2.0);
                }
            }
            if let Some(spot) = world
                .get_resource::<ExampleEntities>()
                .and_then(|entities| entities.primary_spot_light)
            {
                if let Some(mut light) = world.get_mut::<SpotLight2d>(spot) {
                    light.intensity = 0.0;
                }
            }
        })))
        .then(Action::WaitFrames(4))
        .then(Action::Screenshot("lighting_soft_shadows_hard".into()))
        .then(Action::Custom(Box::new(|world: &mut World| {
            let Some(camera) = world
                .get_resource::<ExampleEntities>()
                .and_then(|entities| entities.camera)
            else {
                return;
            };
            if let Some(mut settings) = world.get_mut::<Lighting2dSettings>(camera) {
                settings.shadow_filter = ShadowFiltering2d::Soft;
            }
        })))
        .then(Action::WaitFrames(4))
        .then(assertions::custom("soft shadows are enabled on the main camera", |world| {
            world
                .get_resource::<ExampleEntities>()
                .and_then(|entities| entities.camera)
                .and_then(|camera| world.get::<Lighting2dSettings>(camera))
                .is_some_and(|settings| settings.shadow_filter == ShadowFiltering2d::Soft)
        }))
        .then(Action::Screenshot("lighting_soft_shadows_soft".into()))
        .then(Action::WaitFrames(1))
        .then(assertions::log_summary("lighting_soft_shadows"))
        .build()
}

fn lighting_blurred_lightmap() -> Scenario {
    Scenario::builder("lighting_blurred_lightmap")
        .description("Compare the same scene with blur disabled and enabled on the real offscreen lightmap path.")
        .then(Action::WaitFrames(20))
        .then(Action::Custom(Box::new(|world: &mut World| {
            if let Some(mut pane) = world.get_resource_mut::<ExampleLightingPane>() {
                pane.lightmap_scale = 0.2;
                pane.blur_radius = 0.0;
            }
        })))
        .then(Action::WaitFrames(4))
        .then(Action::Screenshot("lighting_blurred_lightmap_raw".into()))
        .then(Action::Custom(Box::new(|world: &mut World| {
            if let Some(mut pane) = world.get_resource_mut::<ExampleLightingPane>() {
                pane.blur_radius = 8.0;
            }
        })))
        .then(Action::WaitFrames(4))
        .then(assertions::custom("blur is enabled on the main camera", |world| {
            world
                .get_resource::<ExampleEntities>()
                .and_then(|entities| entities.camera)
                .and_then(|camera| world.get::<Lighting2dSettings>(camera))
                .is_some_and(|settings| settings.blur_radius == 8)
        }))
        .then(Action::Screenshot("lighting_blurred_lightmap_blurred".into()))
        .then(Action::WaitFrames(1))
        .then(assertions::log_summary("lighting_blurred_lightmap"))
        .build()
}

fn lighting_mask_occluders() -> Scenario {
    Scenario::builder("lighting_mask_occluders")
        .description("Verify the lab contains a mask-derived occluder shape and capture the integrated result.")
        .then(Action::WaitFrames(20))
        .then(assertions::custom("the lab contains at least one mask occluder", |world| {
            world
                .get_resource::<ExampleEntities>()
                .and_then(|entities| entities.mask_occluder)
                .and_then(|entity| world.get::<LightOccluder2d>(entity))
                .is_some_and(|occluder| matches!(occluder.shape, OccluderShape2d::Mask { .. }))
        }))
        .then(Action::Custom(Box::new(|world: &mut World| {
            let Some((point, camera)) = world
                .get_resource::<ExampleEntities>()
                .map(|entities| (entities.primary_point_light, entities.camera))
            else {
                return;
            };
            if let Some(point) = point {
                if let Some(mut pane) = world.get_resource_mut::<ExampleLightingPane>() {
                    pane.point_source_radius = 36.0;
                    pane.point_radius = 180.0;
                }
                if let Some(mut light) = world.get_mut::<PointLight2d>(point) {
                    light.occluder_mask = u32::MAX;
                }
                if let Some(mut transform) = world.get_mut::<Transform>(point) {
                    transform.translation = Vec3::new(-190.0, -26.0, 2.0);
                }
            }
            if let Some(camera) = camera {
                if let Some(mut projection) = world.get_mut::<Projection>(camera) {
                    if let Projection::Orthographic(orthographic) = &mut *projection {
                        orthographic.scale = 0.74;
                    }
                }
                if let Some(mut transform) = world.get_mut::<Transform>(camera) {
                    transform.translation = Vec3::new(-120.0, -40.0, transform.translation.z);
                }
            }
        })))
        .then(Action::WaitFrames(4))
        .then(Action::Screenshot("lighting_mask_occluders".into()))
        .then(Action::WaitFrames(1))
        .then(assertions::log_summary("lighting_mask_occluders"))
        .build()
}

fn lighting_receiver_cookies() -> Scenario {
    Scenario::builder("lighting_receiver_cookies")
        .description("Focus the normal-mapped receiver under the textured light to verify receiver-side cookie shading.")
        .then(Action::WaitFrames(20))
        .then(assertions::resource_satisfies::<Lighting2dDiagnostics>(
            "textured lights and normal-mapped receivers are both present",
            |diagnostics| {
                diagnostics.active_texture_lights >= 1
                    && diagnostics.active_normal_mapped_sprites >= 1
            },
        ))
        .then(Action::Custom(Box::new(|world: &mut World| {
            let Some((camera, point, spot, texture_light)) = world
                .get_resource::<ExampleEntities>()
                .map(|entities| {
                    (
                        entities.camera,
                        entities.primary_point_light,
                        entities.primary_spot_light,
                        entities.texture_light,
                    )
                })
            else {
                return;
            };
            let receiver_translation = {
                let mut query = world.query_filtered::<&Transform, With<NormalMappedSprite2d>>();
                query
                    .iter(world)
                    .next()
                    .map_or(Vec3::ZERO, |transform| transform.translation)
            };
            if let Some(mut pane) = world.get_resource_mut::<ExampleLightingPane>() {
                pane.ambient_intensity = 0.04;
                pane.texture_intensity = 4.0;
                pane.blur_radius = 2.0;
            }
            if let Some(point) = point {
                if let Some(mut light) = world.get_mut::<PointLight2d>(point) {
                    light.intensity = 0.0;
                }
            }
            if let Some(spot) = spot {
                if let Some(mut light) = world.get_mut::<SpotLight2d>(spot) {
                    light.intensity = 0.0;
                }
            }
            if let Some(texture_light) = texture_light {
                if let Some(mut light) = world.get_mut::<TextureLight2d>(texture_light) {
                    light.size = Vec2::new(110.0, 110.0);
                    light.height = 20.0;
                    light.rotation_radians = 0.0;
                }
                if let Some(mut transform) = world.get_mut::<Transform>(texture_light) {
                    transform.translation =
                        receiver_translation + Vec3::new(0.0, 12.0, 1.0);
                }
            }
            if let Some(camera) = camera {
                if let Some(mut projection) = world.get_mut::<Projection>(camera) {
                    if let Projection::Orthographic(orthographic) = &mut *projection {
                        orthographic.scale = 0.42;
                    }
                }
                if let Some(mut transform) = world.get_mut::<Transform>(camera) {
                    transform.translation = Vec3::new(
                        receiver_translation.x + 12.0,
                        receiver_translation.y + 4.0,
                        transform.translation.z,
                    );
                }
            }
        })))
        .then(Action::WaitFrames(4))
        .then(Action::Screenshot("lighting_receiver_cookies".into()))
        .then(Action::WaitFrames(1))
        .then(assertions::log_summary("lighting_receiver_cookies"))
        .build()
}

fn lighting_multi_camera() -> Scenario {
    Scenario::builder("lighting_multi_camera")
        .description("Enable lighting on the inset comparison camera with different settings and verify two independent lit views coexist.")
        .then(Action::WaitFrames(20))
        .then(assertions::custom("the lab spawned two cameras", |world| {
            world
                .get_resource::<ExampleEntities>()
                .is_some_and(|entities| entities.camera.is_some() && entities.comparison_camera.is_some())
        }))
        .then(Action::Custom(Box::new(|world: &mut World| {
            let Some(comparison_camera) = world
                .get_resource::<ExampleEntities>()
                .and_then(|entities| entities.comparison_camera)
            else {
                return;
            };
            world.entity_mut(comparison_camera).insert(Lighting2dSettings {
                ambient_intensity: 0.22,
                lightmap_scale: 0.7,
                blur_radius: 1,
                shadow_filter: ShadowFiltering2d::Hard,
                ..Lighting2dSettings::default()
            });
        })))
        .then(Action::WaitFrames(6))
        .then(assertions::custom("both cameras are lit", |world| {
            world
                .get_resource::<ExampleEntities>()
                .is_some_and(|entities| {
                    entities
                        .camera
                        .and_then(|camera| world.get::<Lighting2dSettings>(camera))
                        .is_some()
                        && entities
                            .comparison_camera
                            .and_then(|camera| world.get::<Lighting2dSettings>(camera))
                            .is_some()
                })
        }))
        .then(Action::Screenshot("lighting_multi_camera".into()))
        .then(Action::WaitFrames(1))
        .then(assertions::log_summary("lighting_multi_camera"))
        .build()
}

fn reference_dungeon() -> Scenario {
    Scenario::builder("reference_dungeon")
        .description("Reference-inspired candlelit dungeon scene with multiple captures over flicker and motion.")
        .then(Action::WaitFrames(20))
        .then(assertions::resource_satisfies::<Lighting2dDiagnostics>(
            "dungeon scene has multiple candle lights and blockers",
            |diagnostics| diagnostics.active_point_lights >= 3 && diagnostics.active_occluders >= 16,
        ))
        .then(Action::Screenshot("reference_dungeon_start".into()))
        .then(Action::WaitFrames(40))
        .then(Action::Screenshot("reference_dungeon_mid".into()))
        .then(Action::WaitFrames(40))
        .then(Action::Screenshot("reference_dungeon_end".into()))
        .then(Action::WaitFrames(1))
        .then(assertions::log_summary("reference_dungeon"))
        .build()
}

fn reference_texture_cursor() -> Scenario {
    Scenario::builder("reference_texture_cursor")
        .description("Reference-inspired textured cursor light scene with motion over multiple captures.")
        .then(Action::WaitFrames(20))
        .then(Action::Custom(Box::new(|world: &mut World| {
            if let Some(mut pane) = world.get_resource_mut::<ExampleLightingPane>() {
                pane.motion_speed = 1.5;
                pane.texture_intensity = 3.0;
                pane.blur_radius = 1.0;
            }
        })))
        .then(assertions::resource_satisfies::<Lighting2dDiagnostics>(
            "texture cursor scene has a texture light and normal receiver",
            |diagnostics| diagnostics.active_texture_lights >= 1 && diagnostics.active_normal_mapped_sprites >= 1,
        ))
        .then(Action::Screenshot("reference_texture_cursor_start".into()))
        .then(Action::WaitFrames(35))
        .then(Action::Screenshot("reference_texture_cursor_mid".into()))
        .then(Action::WaitFrames(35))
        .then(Action::Screenshot("reference_texture_cursor_end".into()))
        .then(Action::WaitFrames(1))
        .then(assertions::log_summary("reference_texture_cursor"))
        .build()
}

fn reference_mask_occluder() -> Scenario {
    Scenario::builder("reference_mask_occluder")
        .description("Reference-inspired alpha-mask occluder demo with moving lights and repeated captures.")
        .then(Action::WaitFrames(20))
        .then(Action::Custom(Box::new(|world: &mut World| {
            if let Some(mut pane) = world.get_resource_mut::<ExampleLightingPane>() {
                pane.motion_speed = 1.25;
                pane.point_source_radius = 18.0;
                pane.point_radius = 210.0;
            }
        })))
        .then(assertions::custom("mask occluder exists in reference mask scene", |world| {
            world
                .get_resource::<ExampleEntities>()
                .and_then(|entities| entities.mask_occluder)
                .and_then(|entity| world.get::<LightOccluder2d>(entity))
                .is_some_and(|occluder| matches!(occluder.shape, OccluderShape2d::Mask { .. }))
        }))
        .then(Action::Screenshot("reference_mask_occluder_start".into()))
        .then(Action::WaitFrames(45))
        .then(Action::Screenshot("reference_mask_occluder_mid".into()))
        .then(Action::WaitFrames(45))
        .then(Action::Screenshot("reference_mask_occluder_end".into()))
        .then(Action::WaitFrames(1))
        .then(assertions::log_summary("reference_mask_occluder"))
        .build()
}

fn reference_road_convoy() -> Scenario {
    Scenario::builder("reference_road_convoy")
        .description("Reference-inspired scrolling road convoy scene with headlight motion and multiple captures.")
        .then(Action::WaitFrames(20))
        .then(Action::Custom(Box::new(|world: &mut World| {
            if let Some(mut pane) = world.get_resource_mut::<ExampleLightingPane>() {
                pane.motion_speed = 1.2;
                pane.texture_intensity = 3.2;
                pane.blur_radius = 2.0;
            }
        })))
        .then(assertions::resource_satisfies::<Lighting2dDiagnostics>(
            "road convoy scene has the expected moving-light budget",
            |diagnostics| diagnostics.active_texture_lights >= 1
                && diagnostics.active_emissive_sprites >= 1
                && diagnostics.active_occluders >= 8,
        ))
        .then(Action::Screenshot("reference_road_convoy_start".into()))
        .then(Action::WaitFrames(50))
        .then(Action::Screenshot("reference_road_convoy_mid".into()))
        .then(Action::WaitFrames(50))
        .then(Action::Screenshot("reference_road_convoy_end".into()))
        .then(Action::WaitFrames(1))
        .then(assertions::log_summary("reference_road_convoy"))
        .build()
}
