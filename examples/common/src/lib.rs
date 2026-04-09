use bevy::{
    app::AppExit,
    asset::RenderAssetUsages,
    camera::{ClearColorConfig, Viewport},
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
    text::TextColor,
    ui::IsDefaultUiCamera,
    window::PrimaryWindow,
    winit::WinitSettings,
};
use saddle_pane::prelude::*;
use saddle_rendering_2d_lighting::{
    EmissiveSprite2d, LightOccluder2d, LightShadowMode2d, Lighting2dDiagnostics,
    Lighting2dPlugin, Lighting2dSettings, NormalMappedSprite2d, OccluderShape2d, PointLight2d,
    SpotLight2d, TextureLight2d,
};

#[derive(Resource)]
struct AutoExitAfter(Timer);

#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExampleSceneMode(pub ExampleMode);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExampleMode {
    Basic,
    Occluders,
    NormalMaps,
    MixedLights,
    Dungeon,
    TextureCursor,
    MaskOccluder,
    RoadConvoy,
    GameDemo,
    Stress,
}

#[derive(Resource, Clone)]
pub struct ExampleSceneText {
    pub title: String,
    pub subtitle: String,
}

#[derive(Resource, Clone, Copy, Debug, Default)]
pub struct ExampleEntities {
    pub camera: Option<Entity>,
    pub comparison_camera: Option<Entity>,
    pub primary_point_light: Option<Entity>,
    pub primary_spot_light: Option<Entity>,
    pub texture_light: Option<Entity>,
    pub emissive_sprite: Option<Entity>,
    pub mask_occluder: Option<Entity>,
    pub overlay: Option<Entity>,
}

#[derive(Component)]
pub struct ExampleOverlay;

#[derive(Component)]
struct OscillateTransform {
    origin: Vec3,
    amplitude: Vec3,
    frequency: Vec3,
    phase: f32,
}

#[derive(Component)]
struct PulsingPointLight {
    base: f32,
    amplitude: f32,
    speed: f32,
    phase: f32,
}

#[derive(Component)]
struct CursorDrivenTextureLight {
    orbit_center: Vec2,
    orbit_radius: Vec2,
    orbit_speed: f32,
}

#[derive(Component)]
struct ConvoyMover {
    speed: f32,
    wrap_min_y: f32,
    wrap_max_y: f32,
}

#[derive(Component)]
struct FollowTargetY {
    target: Entity,
    offset_y: f32,
    smoothing: f32,
}

#[derive(Resource, Clone)]
pub struct DemoAssets {
    pub cookie: Handle<Image>,
    pub receiver_diffuse: Handle<Image>,
    pub normal_map: Handle<Image>,
    pub emissive_diffuse: Handle<Image>,
    pub emissive_mask: Handle<Image>,
    pub occluder_mask: Handle<Image>,
}

#[derive(Resource, Debug, Clone, PartialEq, Pane)]
#[pane(title = "2D Lighting", position = "top-right")]
pub struct ExampleLightingPane {
    #[pane(checkbox)]
    pub motion_enabled: bool,
    #[pane(slider, min = 0.0, max = 3.0, step = 0.05)]
    pub motion_speed: f32,
    #[pane(slider, min = 0.0, max = 1.5, step = 0.05)]
    pub ambient_intensity: f32,
    #[pane(slider, min = 0.2, max = 1.0, step = 0.05)]
    pub lightmap_scale: f32,
    #[pane(slider, min = 0.0, max = 8.0, step = 1.0)]
    pub blur_radius: f32,
    #[pane(slider, min = 24.0, max = 320.0, step = 4.0)]
    pub point_radius: f32,
    #[pane(slider, min = 0.0, max = 40.0, step = 1.0)]
    pub point_source_radius: f32,
    #[pane(slider, min = 24.0, max = 320.0, step = 4.0)]
    pub spot_radius: f32,
    #[pane(slider, min = 0.0, max = 80.0, step = 2.0)]
    pub spot_source_width: f32,
    #[pane(slider, min = 0.0, max = 4.0, step = 0.05)]
    pub texture_intensity: f32,
    #[pane(slider, min = 0.0, max = 6.0, step = 0.05)]
    pub emissive_intensity: f32,
    #[pane(monitor)]
    pub total_lights: f32,
    #[pane(monitor)]
    pub occluders: f32,
    #[pane(monitor)]
    pub normal_sprites: f32,
}

impl Default for ExampleLightingPane {
    fn default() -> Self {
        Self {
            motion_enabled: true,
            motion_speed: 1.0,
            ambient_intensity: 0.12,
            lightmap_scale: 0.5,
            blur_radius: 3.0,
            point_radius: 140.0,
            point_source_radius: 14.0,
            spot_radius: 180.0,
            spot_source_width: 24.0,
            texture_intensity: 1.0,
            emissive_intensity: 3.5,
            total_lights: 0.0,
            occluders: 0.0,
            normal_sprites: 0.0,
        }
    }
}

pub fn install_common_plugins(app: &mut App, window_title: &str, auto_exit_env: &str) {
    app.insert_resource(ClearColor(Color::srgb(0.04, 0.05, 0.08)));
    app.insert_resource(WinitSettings::continuous());
    app.add_plugins(
        DefaultPlugins
            .set(ImagePlugin::default_nearest())
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: window_title.into(),
                    resolution: (1440, 900).into(),
                    ..default()
                }),
                ..default()
            }),
    );
    app.add_plugins(Lighting2dPlugin::default());
    install_pane(app);
    install_auto_exit(app, auto_exit_env);
}

pub fn add_example_systems(app: &mut App) {
    app.add_systems(
        Update,
        (
            sync_example_pane,
            update_example_pane_monitors,
            animate_demo,
            animate_pulsing_lights,
            move_cursor_texture_lights,
            move_convoy_entities,
            follow_targets_y,
            draw_debug_gizmos,
            update_overlay,
        ),
    );
}

pub fn install_auto_exit(app: &mut App, env_var: &str) {
    let timer = std::env::var(env_var)
        .ok()
        .and_then(|value| value.parse::<f32>().ok())
        .map(|seconds| AutoExitAfter(Timer::from_seconds(seconds.max(0.1), TimerMode::Once)));

    if let Some(timer) = timer {
        app.insert_resource(timer);
        app.add_systems(Update, auto_exit_after);
    }
}

pub fn install_pane(app: &mut App) {
    if !app.is_plugin_added::<PanePlugin>() {
        app.add_plugins((
            bevy_flair::FlairPlugin,
            bevy_input_focus::InputDispatchPlugin,
            bevy_ui_widgets::UiWidgetsPlugins,
            bevy_input_focus::tab_navigation::TabNavigationPlugin,
            PanePlugin,
        ));
    }

    app.register_pane::<ExampleLightingPane>();
}

pub fn setup_scene(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mode: Res<ExampleSceneMode>,
    text: Res<ExampleSceneText>,
) {
    let assets = DemoAssets::generate(&mut images);
    let mut entities = ExampleEntities::default();
    let lighting_settings = match mode.0 {
        ExampleMode::Stress => Lighting2dSettings::fast_unshadowed(),
        _ => Lighting2dSettings::showcase_soft(),
    };

    commands.insert_resource(assets.clone());

    let camera = commands
        .spawn((
            Name::new("Main Camera"),
            Camera2d,
            IsDefaultUiCamera,
            lighting_settings,
        ))
        .id();
    entities.camera = Some(camera);

    spawn_scene_shell(&mut commands);

    if mode.0 == ExampleMode::GameDemo {
        let comparison_camera = commands
            .spawn((
                Name::new("Comparison Camera"),
                Camera2d,
                Camera {
                    order: 1,
                    clear_color: ClearColorConfig::Custom(Color::srgb(0.03, 0.04, 0.06)),
                    viewport: Some(Viewport {
                        physical_position: UVec2::new(1070, 54),
                        physical_size: UVec2::new(320, 220),
                        ..default()
                    }),
                    ..default()
                },
                Projection::Orthographic(OrthographicProjection {
                    scale: 0.78,
                    ..OrthographicProjection::default_2d()
                }),
            ))
            .id();
        entities.comparison_camera = Some(comparison_camera);

        commands.spawn((
            Name::new("Comparison Label"),
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(18.0),
                right: Val::Px(24.0),
                padding: UiRect::axes(Val::Px(12.0), Val::Px(8.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.02, 0.03, 0.06, 0.86)),
            Text::new("Inset: raw scene view\nlighting disabled"),
            TextFont {
                font_size: 14.0,
                ..default()
            },
            TextColor(Color::srgb(0.86, 0.90, 0.98)),
        ));
    }

    let overlay = commands
        .spawn((
            Name::new("Example Overlay"),
            ExampleOverlay,
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(18.0),
                left: Val::Px(18.0),
                width: Val::Px(460.0),
                padding: UiRect::all(Val::Px(14.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.02, 0.03, 0.06, 0.80)),
            Text::new(format!(
                "{}\n{}\n\nLive backend: downsampled lightmaps, optional blur, soft shadows, shadow groups, textured lights, receiver cookies, and the raw-scene inset are all driven by the crate runtime.",
                text.title, text.subtitle
            )),
            TextFont {
                font_size: 15.0,
                ..default()
            },
            TextColor(Color::WHITE),
        ))
        .id();
    entities.overlay = Some(overlay);

    spawn_mode_scene(&mut commands, &mode.0, &assets, &mut entities, Vec2::ZERO);

    commands.insert_resource(entities);
}

fn spawn_mode_scene(
    commands: &mut Commands,
    mode: &ExampleMode,
    assets: &DemoAssets,
    entities: &mut ExampleEntities,
    offset: Vec2,
) {
    if matches!(
        *mode,
        ExampleMode::Basic
            | ExampleMode::Occluders
            | ExampleMode::NormalMaps
            | ExampleMode::MixedLights
            | ExampleMode::GameDemo
            | ExampleMode::Stress
    ) {
        let point = commands
            .spawn((
                Name::new("Primary Point Light"),
                PointLight2d {
                    color: Color::srgb(1.0, 0.84, 0.58),
                    intensity: 2.0,
                    radius: 140.0,
                    source_radius: 14.0,
                    shadow_mode: LightShadowMode2d::Occluded,
                    occluder_mask: 0b0001,
                    ..default()
                },
                OscillateTransform {
                    origin: Vec3::new(offset.x - 220.0, offset.y + 92.0, 2.0),
                    amplitude: Vec3::new(54.0, 30.0, 0.0),
                    frequency: Vec3::new(1.0, 1.3, 0.0),
                    phase: 0.0,
                },
                Sprite::from_color(Color::srgb(1.0, 0.75, 0.25), Vec2::splat(18.0)),
                Transform::from_xyz(offset.x - 220.0, offset.y + 92.0, 2.0),
            ))
            .id();
        entities.primary_point_light = Some(point);

        let spot = commands
            .spawn((
                Name::new("Primary Spot Light"),
                SpotLight2d {
                    color: Color::srgb(0.55, 0.78, 1.0),
                    intensity: 1.8,
                    radius: 180.0,
                    source_width: 26.0,
                    direction_radians: 0.6,
                    inner_angle_radians: 0.3,
                    outer_angle_radians: 0.9,
                    shadow_mode: LightShadowMode2d::Illuminated,
                    occluder_mask: u32::MAX,
                    ..default()
                },
                Sprite::from_color(Color::srgb(0.35, 0.7, 1.0), Vec2::new(22.0, 14.0)),
                Transform::from_xyz(offset.x + 210.0, offset.y + 120.0, 2.0),
            ))
            .id();
        entities.primary_spot_light = Some(spot);

        commands.spawn((
            Name::new("Receiver Sprite"),
            Sprite::from_image(assets.receiver_diffuse.clone()),
            Transform::from_xyz(offset.x - 18.0, offset.y - 30.0, 1.0)
                .with_scale(Vec3::splat(1.35)),
        ));
    }

    match mode {
        ExampleMode::Basic => {}
        ExampleMode::Occluders => {
            spawn_occluders(commands, assets, entities, offset);
        }
        ExampleMode::NormalMaps => {
            spawn_occluders(commands, assets, entities, offset);
            spawn_normal_and_emissive(commands, assets, entities, offset);
        }
        ExampleMode::MixedLights => {
            spawn_occluders(commands, assets, entities, offset);
            spawn_texture_light(commands, assets, entities, offset);
        }
        ExampleMode::Dungeon => {
            spawn_dungeon_scene(commands, assets, entities, offset);
        }
        ExampleMode::TextureCursor => {
            spawn_texture_cursor_scene(commands, assets, entities, offset);
        }
        ExampleMode::MaskOccluder => {
            spawn_mask_occluder_scene(commands, assets, entities, offset);
        }
        ExampleMode::RoadConvoy => {
            spawn_road_convoy_scene(commands, assets, entities, offset);
        }
        ExampleMode::GameDemo => {
            spawn_occluders(commands, assets, entities, offset);
            spawn_texture_light(commands, assets, entities, offset);
            spawn_normal_and_emissive(commands, assets, entities, offset);
            commands.spawn((
                Name::new("North Wall"),
                Sprite::from_color(Color::srgb(0.18, 0.19, 0.23), Vec2::new(720.0, 30.0)),
                LightOccluder2d::rectangle(Vec2::new(360.0, 15.0)),
                Transform::from_xyz(offset.x, offset.y + 210.0, 0.5),
            ));
            commands.spawn((
                Name::new("South Stair"),
                Sprite::from_color(Color::srgb(0.12, 0.13, 0.18), Vec2::new(500.0, 44.0)),
                LightOccluder2d {
                    absorption: 1.0,
                    groups: 0b0001,
                    ..LightOccluder2d::rectangle(Vec2::new(250.0, 22.0))
                },
                Transform::from_xyz(offset.x - 20.0, offset.y - 210.0, 0.5),
            ));
        }
        ExampleMode::Stress => {
            for y in -2..=2 {
                for x in -3..=3 {
                    let position = offset + Vec2::new(x as f32 * 110.0, y as f32 * 80.0);
                    commands.spawn((
                        Name::new(format!("Stress Light {x} {y}")),
                        PointLight2d {
                            color: Color::hsl(((x + 3 + (y + 2) * 3) * 20) as f32, 0.75, 0.68),
                            intensity: 1.0,
                            radius: 72.0,
                            ..default()
                        },
                        Sprite::from_color(Color::WHITE, Vec2::splat(10.0)),
                        Transform::from_xyz(position.x, position.y, 2.0),
                    ));
                }
            }

            for index in -4..=4 {
                let x = offset.x + index as f32 * 120.0;
                commands.spawn((
                    Name::new(format!("Stress Occluder {index}")),
                    Sprite::from_color(
                        Color::srgba(0.12, 0.13, 0.16, 0.72),
                        Vec2::new(36.0, 180.0),
                    ),
                    LightOccluder2d::rectangle(Vec2::new(18.0, 90.0)),
                    Transform::from_xyz(x, offset.y, 0.4),
                ));
            }
        }
    }
}

fn spawn_dungeon_scene(
    commands: &mut Commands,
    assets: &DemoAssets,
    entities: &mut ExampleEntities,
    offset: Vec2,
) {
    for y in -2i32..=2 {
        for x in -3i32..=3 {
            let position = offset + Vec2::new(x as f32 * 72.0, y as f32 * 72.0);
            let is_border = x.abs() == 3 || y.abs() == 2;
            let color = if is_border {
                Color::srgb(0.18, 0.18, 0.22)
            } else {
                Color::srgb(0.14, 0.13, 0.16)
            };
            let mut entity = commands.spawn((
                Name::new(format!("Dungeon Tile {x} {y}")),
                Sprite::from_color(color, Vec2::splat(68.0)),
                Transform::from_xyz(position.x, position.y, 0.1),
            ));
            if is_border {
                entity.insert(LightOccluder2d::rectangle(Vec2::splat(34.0)));
            }
        }
    }

    for (index, (position, color, phase)) in [
        (Vec2::new(-152.0, 36.0), Color::srgb(1.0, 0.82, 0.46), 0.0),
        (Vec2::new(0.0, -8.0), Color::srgb(1.0, 0.74, 0.34), 1.3),
        (Vec2::new(146.0, 42.0), Color::srgb(1.0, 0.88, 0.56), 2.4),
    ]
    .into_iter()
    .enumerate()
    {
        let light = commands
            .spawn((
                Name::new(format!("Dungeon Candle Light {index}")),
                PointLight2d {
                    color,
                    intensity: 1.8,
                    radius: 92.0,
                    falloff: 1.4,
                    source_radius: 10.0,
                    ..default()
                },
                PulsingPointLight {
                    base: 1.8,
                    amplitude: 0.45,
                    speed: 2.4,
                    phase,
                },
                OscillateTransform {
                    origin: position.extend(2.0),
                    amplitude: Vec3::new(8.0, 6.0, 0.0),
                    frequency: Vec3::splat(0.8),
                    phase,
                },
                Transform::from_xyz(position.x, position.y, 2.0),
            ))
            .id();

        if index == 0 {
            entities.primary_point_light = Some(light);
        }

        commands.spawn((
            Name::new(format!("Dungeon Candle Base {index}")),
            Sprite::from_color(Color::srgb(0.36, 0.22, 0.12), Vec2::new(16.0, 42.0)),
            Transform::from_xyz(position.x, position.y - 22.0, 1.1),
        ));
    }

    commands.spawn((
        Name::new("Dungeon Shrine"),
        Sprite::from_image(assets.receiver_diffuse.clone()),
        NormalMappedSprite2d {
            normal_map: assets.normal_map.clone(),
            strength: 1.5,
            height: 10.0,
        },
        Transform::from_xyz(offset.x + 8.0, offset.y + 10.0, 1.0).with_scale(Vec3::splat(1.7)),
    ));
}

fn spawn_texture_cursor_scene(
    commands: &mut Commands,
    assets: &DemoAssets,
    entities: &mut ExampleEntities,
    offset: Vec2,
) {
    spawn_occluders(commands, assets, entities, offset);
    spawn_normal_and_emissive(commands, assets, entities, offset);

    if let Some(point) = entities.primary_point_light.take() {
        commands.entity(point).despawn();
    }
    if let Some(spot) = entities.primary_spot_light.take() {
        commands.entity(spot).despawn();
    }

    let texture_light = commands
        .spawn((
            Name::new("Cursor Cookie Light"),
            TextureLight2d {
                texture: assets.cookie.clone(),
                color: Color::srgb(1.0, 0.96, 0.78),
                intensity: 2.2,
                size: Vec2::new(160.0, 160.0),
                height: 28.0,
                source_radius: 12.0,
                occluder_mask: 0b0011,
                ..default()
            },
            CursorDrivenTextureLight {
                orbit_center: offset + Vec2::new(158.0, -18.0),
                orbit_radius: Vec2::new(88.0, 52.0),
                orbit_speed: 0.6,
            },
            Sprite::from_color(Color::srgb(1.0, 0.95, 0.80), Vec2::splat(18.0)),
            Transform::from_xyz(offset.x + 158.0, offset.y - 18.0, 2.0),
        ))
        .id();
    entities.texture_light = Some(texture_light);
}

fn spawn_mask_occluder_scene(
    commands: &mut Commands,
    assets: &DemoAssets,
    entities: &mut ExampleEntities,
    offset: Vec2,
) {
    spawn_occluders(commands, assets, entities, offset);
    spawn_texture_light(commands, assets, entities, offset);

    if let Some(point) = entities.primary_point_light {
        commands.entity(point).insert((
            OscillateTransform {
                origin: Vec3::new(offset.x - 210.0, offset.y + 28.0, 2.0),
                amplitude: Vec3::new(120.0, 0.0, 0.0),
                frequency: Vec3::new(0.55, 0.0, 0.0),
                phase: 0.0,
            },
            PulsingPointLight {
                base: 1.9,
                amplitude: 0.35,
                speed: 1.8,
                phase: 0.0,
            },
        ));
    }
    if let Some(spot) = entities.primary_spot_light {
        commands.entity(spot).insert(OscillateTransform {
            origin: Vec3::new(offset.x + 242.0, offset.y + 132.0, 2.0),
            amplitude: Vec3::new(0.0, 84.0, 0.0),
            frequency: Vec3::new(0.0, 0.6, 0.0),
            phase: 1.7,
        });
    }

    commands.spawn((
        Name::new("Mask Occluder Plinth"),
        Sprite::from_color(Color::srgb(0.18, 0.14, 0.11), Vec2::new(240.0, 22.0)),
        LightOccluder2d::rectangle(Vec2::new(120.0, 11.0)),
        Transform::from_xyz(offset.x - 220.0, offset.y - 184.0, 0.35),
    ));
}

fn spawn_road_convoy_scene(
    commands: &mut Commands,
    assets: &DemoAssets,
    entities: &mut ExampleEntities,
    offset: Vec2,
) {
    for index in -7..=7 {
        let y = offset.y + index as f32 * 88.0;
        commands.spawn((
            Name::new(format!("Road Segment {index}")),
            Sprite::from_color(Color::srgb(0.16, 0.16, 0.18), Vec2::new(180.0, 88.0)),
            Transform::from_xyz(offset.x, y, 0.0),
        ));
        commands.spawn((
            Name::new(format!("Road Edge Left {index}")),
            Sprite::from_color(Color::srgb(0.08, 0.16, 0.10), Vec2::new(120.0, 88.0)),
            Transform::from_xyz(offset.x - 150.0, y, -0.2),
        ));
        commands.spawn((
            Name::new(format!("Road Edge Right {index}")),
            Sprite::from_color(Color::srgb(0.08, 0.16, 0.10), Vec2::new(120.0, 88.0)),
            Transform::from_xyz(offset.x + 150.0, y, -0.2),
        ));
    }

    for index in -4..=4 {
        let y = offset.y + index as f32 * 144.0;
        commands.spawn((
            Name::new(format!("Streetlight Pole Left {index}")),
            Sprite::from_color(Color::srgb(0.20, 0.20, 0.24), Vec2::new(12.0, 96.0)),
            LightOccluder2d::rectangle(Vec2::new(6.0, 48.0)),
            Transform::from_xyz(offset.x - 86.0, y, 0.3),
        ));
        commands.spawn((
            Name::new(format!("Streetlight Pole Right {index}")),
            Sprite::from_color(Color::srgb(0.20, 0.20, 0.24), Vec2::new(12.0, 96.0)),
            LightOccluder2d::rectangle(Vec2::new(6.0, 48.0)),
            Transform::from_xyz(offset.x + 86.0, y, 0.3),
        ));
        commands.spawn((
            Name::new(format!("Streetlight Glow Left {index}")),
            PointLight2d {
                color: Color::srgb(1.0, 0.90, 0.56),
                intensity: 1.1,
                radius: 86.0,
                falloff: 1.8,
                ..default()
            },
            Transform::from_xyz(offset.x - 86.0, y + 34.0, 1.8),
        ));
        commands.spawn((
            Name::new(format!("Streetlight Glow Right {index}")),
            PointLight2d {
                color: Color::srgb(1.0, 0.90, 0.56),
                intensity: 1.1,
                radius: 86.0,
                falloff: 1.8,
                ..default()
            },
            Transform::from_xyz(offset.x + 86.0, y + 34.0, 1.8),
        ));
    }

    let convoy_root = commands
        .spawn((
            Name::new("Truck Convoy"),
            ConvoyMover {
                speed: 108.0,
                wrap_min_y: -420.0,
                wrap_max_y: 420.0,
            },
            Visibility::default(),
            Transform::from_xyz(offset.x, offset.y + 260.0, 0.0),
        ))
        .id();

    commands.entity(convoy_root).with_children(|parent| {
        parent.spawn((
            Name::new("Truck Body"),
            Sprite::from_color(Color::srgb(0.22, 0.40, 0.72), Vec2::new(54.0, 112.0)),
            Transform::from_xyz(0.0, 0.0, 1.0),
        ));
        parent.spawn((
            Name::new("Truck Cab"),
            Sprite::from_color(Color::srgb(0.72, 0.78, 0.84), Vec2::new(44.0, 38.0)),
            Transform::from_xyz(0.0, -36.0, 1.1),
        ));
        parent.spawn((
            Name::new("Truck Occluder"),
            LightOccluder2d::rectangle(Vec2::new(27.0, 56.0)),
            Transform::from_xyz(0.0, 0.0, 0.8),
        ));
        parent.spawn((
            Name::new("Truck Headlights"),
            TextureLight2d {
                texture: assets.cookie.clone(),
                color: Color::srgb(1.0, 0.96, 0.74),
                intensity: 2.8,
                size: Vec2::new(180.0, 240.0),
                height: 26.0,
                source_radius: 8.0,
                rotation_radians: std::f32::consts::PI,
                ..default()
            },
            Sprite::from_color(Color::srgb(1.0, 0.95, 0.74), Vec2::new(20.0, 12.0)),
            Transform::from_xyz(0.0, -68.0, 2.0),
        ));
        parent.spawn((
            Name::new("Truck Tail Light"),
            EmissiveSprite2d {
                color: Color::srgb(1.0, 0.22, 0.10),
                intensity: 3.2,
                ..default()
            },
            Sprite::from_color(Color::srgb(0.88, 0.16, 0.08), Vec2::new(18.0, 10.0)),
            Transform::from_xyz(0.0, 60.0, 1.2),
        ));
    });

    if let Some(camera) = entities.camera {
        commands.entity(camera).insert(FollowTargetY {
            target: convoy_root,
            offset_y: 0.0,
            smoothing: 0.12,
        });
    }
}

fn spawn_occluders(
    commands: &mut Commands,
    assets: &DemoAssets,
    entities: &mut ExampleEntities,
    offset: Vec2,
) {
    commands.spawn((
        Name::new("Stone Pillar"),
        Sprite::from_color(Color::srgba(0.11, 0.12, 0.16, 0.86), Vec2::new(58.0, 260.0)),
        LightOccluder2d {
            groups: 0b0001,
            ..LightOccluder2d::rectangle(Vec2::new(29.0, 130.0))
        },
        Transform::from_xyz(offset.x - 58.0, offset.y + 4.0, 0.4),
    ));
    commands.spawn((
        Name::new("Bronze Brazier"),
        Sprite::from_color(Color::srgba(0.24, 0.14, 0.11, 0.82), Vec2::splat(90.0)),
        LightOccluder2d {
            groups: 0b0001,
            ..LightOccluder2d::circle(45.0, 24)
        },
        Transform::from_xyz(offset.x + 88.0, offset.y - 98.0, 0.4),
    ));
    commands.spawn((
        Name::new("Broken Shrine"),
        Sprite::from_color(Color::srgba(0.09, 0.18, 0.16, 0.82), Vec2::new(118.0, 112.0)),
        LightOccluder2d {
            groups: 0b0001,
            ..LightOccluder2d::polygon(vec![
                Vec2::new(-40.0, -30.0),
                Vec2::new(22.0, -46.0),
                Vec2::new(46.0, 8.0),
                Vec2::new(6.0, 44.0),
                Vec2::new(-36.0, 24.0),
            ])
        },
        Transform::from_xyz(offset.x + 230.0, offset.y + 14.0, 0.4),
    ));
    commands.spawn((
        Name::new("Stained Glass Slit"),
        Sprite::from_color(Color::srgba(0.18, 0.82, 0.74, 0.42), Vec2::new(44.0, 180.0)),
        LightOccluder2d {
            absorption: 1.0,
            shadow_tint: Color::srgb(0.18, 0.80, 0.72),
            groups: 0b0001,
            ..LightOccluder2d::rectangle(Vec2::new(22.0, 90.0))
        },
        Transform::from_xyz(offset.x + 34.0, offset.y + 126.0, 0.45),
    ));

    let mask_occluder = commands
        .spawn((
            Name::new("Mask Banner"),
            Sprite::from_image(assets.occluder_mask.clone()),
            LightOccluder2d {
                shape: OccluderShape2d::Mask {
                    mask: assets.occluder_mask.clone(),
                    alpha_threshold: 0.1,
                },
                absorption: 1.0,
                shadow_tint: Color::srgb(0.84, 0.32, 0.18),
                groups: 0b0010,
                ..default()
            },
            Transform::from_xyz(offset.x - 244.0, offset.y - 132.0, 0.42)
                .with_scale(Vec3::splat(1.25)),
        ))
        .id();
    entities.mask_occluder = Some(mask_occluder);
}

fn spawn_normal_and_emissive(
    commands: &mut Commands,
    assets: &DemoAssets,
    entities: &mut ExampleEntities,
    offset: Vec2,
) {
    commands.spawn((
        Name::new("Normal Receiver"),
        Sprite::from_image(assets.receiver_diffuse.clone()),
        NormalMappedSprite2d {
            normal_map: assets.normal_map.clone(),
            strength: 1.35,
            height: 6.0,
        },
        Transform::from_xyz(offset.x + 160.0, offset.y - 12.0, 1.0).with_scale(Vec3::splat(1.45)),
    ));
    let emissive = commands
        .spawn((
            Name::new("Emissive Sign"),
            Sprite::from_image(assets.emissive_diffuse.clone()),
            EmissiveSprite2d {
                color: Color::srgb(1.0, 0.84, 0.45),
                intensity: 4.0,
                mask: Some(assets.emissive_mask.clone()),
                ..default()
            },
            Transform::from_xyz(offset.x + 238.0, offset.y - 122.0, 1.0)
                .with_scale(Vec3::new(1.1, 1.1, 1.0)),
        ))
        .id();
    entities.emissive_sprite = Some(emissive);
}

fn spawn_texture_light(
    commands: &mut Commands,
    assets: &DemoAssets,
    entities: &mut ExampleEntities,
    offset: Vec2,
) {
    let texture_light = commands
        .spawn((
            Name::new("Texture Light"),
            TextureLight2d {
                texture: assets.cookie.clone(),
                color: Color::srgb(1.0, 0.92, 0.70),
                intensity: 1.3,
                size: Vec2::new(220.0, 240.0),
                height: 40.0,
                source_radius: 18.0,
                rotation_radians: -0.18,
                shadow_mode: LightShadowMode2d::Solid,
                occluder_mask: 0b0011,
                ..default()
            },
            Sprite::from_color(Color::srgb(1.0, 0.95, 0.76), Vec2::new(20.0, 20.0)),
            Transform::from_xyz(offset.x + 22.0, offset.y + 208.0, 2.0),
        ))
        .id();
    entities.texture_light = Some(texture_light);
}

fn auto_exit_after(
    time: Res<Time>,
    mut timer: ResMut<AutoExitAfter>,
    mut exit: MessageWriter<AppExit>,
) {
    if timer.0.tick(time.delta()).just_finished() {
        exit.write(AppExit::Success);
    }
}

fn sync_example_pane(
    pane: Res<ExampleLightingPane>,
    entities: Res<ExampleEntities>,
    mut cameras: Query<&mut Lighting2dSettings>,
    mut point_lights: Query<&mut PointLight2d>,
    mut spot_lights: Query<&mut SpotLight2d>,
    mut texture_lights: Query<&mut TextureLight2d>,
    mut emissive_sprites: Query<&mut EmissiveSprite2d>,
) {
    if let Some(camera) = entities.camera {
        if let Ok(mut settings) = cameras.get_mut(camera) {
            settings.ambient_intensity = pane.ambient_intensity;
            settings.lightmap_scale = pane.lightmap_scale;
            settings.blur_radius = pane.blur_radius.max(0.0).round() as u32;
        }
    }

    if let Some(entity) = entities.primary_point_light {
        if let Ok(mut light) = point_lights.get_mut(entity) {
            light.radius = pane.point_radius;
            light.source_radius = pane.point_source_radius;
        }
    }

    if let Some(entity) = entities.primary_spot_light {
        if let Ok(mut light) = spot_lights.get_mut(entity) {
            light.radius = pane.spot_radius;
            light.source_width = pane.spot_source_width;
        }
    }

    if let Some(entity) = entities.texture_light {
        if let Ok(mut light) = texture_lights.get_mut(entity) {
            light.intensity = pane.texture_intensity;
        }
    }

    if let Some(entity) = entities.emissive_sprite {
        if let Ok(mut emissive) = emissive_sprites.get_mut(entity) {
            emissive.intensity = pane.emissive_intensity;
        }
    }
}

fn update_example_pane_monitors(
    diagnostics: Res<Lighting2dDiagnostics>,
    mut pane: ResMut<ExampleLightingPane>,
) {
    pane.total_lights = diagnostics.total_lights() as f32;
    pane.occluders = diagnostics.active_occluders as f32;
    pane.normal_sprites = diagnostics.active_normal_mapped_sprites as f32;
}

fn animate_demo(
    time: Res<Time>,
    pane: Res<ExampleLightingPane>,
    mode: Res<ExampleSceneMode>,
    entities: Res<ExampleEntities>,
    mut transforms: Query<&mut Transform>,
    mut spot_lights: Query<&mut SpotLight2d>,
    mut texture_lights: Query<&mut TextureLight2d>,
    oscillating: Query<(Entity, &OscillateTransform)>,
) {
    let speed = pane.motion_speed.max(0.0);
    let seconds = time.elapsed_secs() * speed;

    if pane.motion_enabled {
        for (entity, oscillate) in &oscillating {
            if let Ok(mut transform) = transforms.get_mut(entity) {
                let phase = seconds + oscillate.phase;
                transform.translation = oscillate.origin
                    + Vec3::new(
                        oscillate.amplitude.x * (phase * oscillate.frequency.x).sin(),
                        oscillate.amplitude.y * (phase * oscillate.frequency.y).sin(),
                        oscillate.amplitude.z * (phase * oscillate.frequency.z).sin(),
                    );
            }
        }
    } else {
        for (entity, oscillate) in &oscillating {
            if let Ok(mut transform) = transforms.get_mut(entity) {
                transform.translation = oscillate.origin;
            }
        }
    }

    if let Some(entity) = entities.primary_spot_light {
        if let Ok(mut light) = spot_lights.get_mut(entity) {
            light.direction_radians = if pane.motion_enabled {
                0.35 + (seconds * 0.9).sin() * 0.5
            } else {
                0.35
            };
        }
    }

    if let Some(entity) = entities.texture_light {
        if let Ok(mut light) = texture_lights.get_mut(entity) {
            light.rotation_radians = match mode.0 {
                ExampleMode::TextureCursor => light.rotation_radians,
                _ if pane.motion_enabled => (seconds * 0.35).sin() * 0.35,
                _ => 0.0,
            };
        }
    }

    if matches!(mode.0, ExampleMode::GameDemo | ExampleMode::MixedLights) {
        if let Some(camera) = entities.camera {
            if let Ok(mut transform) = transforms.get_mut(camera) {
                transform.translation.x = if pane.motion_enabled {
                    (seconds * 0.3).sin() * 40.0
                } else {
                    0.0
                };
            }
        }
        if let Some(camera) = entities.comparison_camera {
            if let Ok(mut transform) = transforms.get_mut(camera) {
                transform.translation.x = if pane.motion_enabled {
                    (seconds * 0.3).sin() * 40.0
                } else {
                    0.0
                };
            }
        }
    }
}

fn animate_pulsing_lights(
    time: Res<Time>,
    pane: Res<ExampleLightingPane>,
    mut lights: Query<(&PulsingPointLight, &mut PointLight2d)>,
) {
    for (pulse, mut light) in &mut lights {
        if pane.motion_enabled {
            let phase = time.elapsed_secs() * pane.motion_speed.max(0.0) * pulse.speed + pulse.phase;
            light.intensity = pulse.base + pulse.amplitude * (phase.sin() * 0.5 + 0.5);
        } else {
            light.intensity = pulse.base;
        }
    }
}

fn move_cursor_texture_lights(
    window: Single<&Window, With<PrimaryWindow>>,
    camera_query: Single<(&Camera, &GlobalTransform), With<Lighting2dSettings>>,
    time: Res<Time>,
    pane: Res<ExampleLightingPane>,
    mut query: Query<(&CursorDrivenTextureLight, &mut Transform), With<TextureLight2d>>,
) {
    let (camera, camera_transform) = camera_query.into_inner();
    let width = window.resolution.physical_width() as f32;
    let height = window.resolution.physical_height() as f32;
    let cursor_position = window.cursor_position().filter(|cursor| {
        cursor.x > 4.0
            && cursor.y > 4.0
            && cursor.x < width - 4.0
            && cursor.y < height - 4.0
    });
    let world_cursor = cursor_position
        .and_then(|cursor| camera.viewport_to_world(camera_transform, cursor).ok())
        .map(|ray| ray.origin.truncate());

    for (driver, mut transform) in &mut query {
        if let Some(cursor) = world_cursor {
            transform.translation.x = cursor.x;
            transform.translation.y = cursor.y;
        } else if pane.motion_enabled {
            let phase = time.elapsed_secs() * pane.motion_speed.max(0.0) * driver.orbit_speed;
            transform.translation.x = driver.orbit_center.x + driver.orbit_radius.x * phase.cos();
            transform.translation.y =
                driver.orbit_center.y + driver.orbit_radius.y * (phase * 1.3).sin();
        } else {
            transform.translation.x = driver.orbit_center.x;
            transform.translation.y = driver.orbit_center.y;
        }
    }
}

fn move_convoy_entities(
    time: Res<Time>,
    pane: Res<ExampleLightingPane>,
    mut query: Query<(&ConvoyMover, &mut Transform)>,
) {
    for (mover, mut transform) in &mut query {
        if pane.motion_enabled {
            transform.translation.y -= mover.speed * pane.motion_speed.max(0.0) * time.delta_secs();
            if transform.translation.y < mover.wrap_min_y {
                transform.translation.y = mover.wrap_max_y;
            }
        }
    }
}

fn follow_targets_y(
    pane: Res<ExampleLightingPane>,
    targets: Query<&Transform, Without<Camera>>,
    mut followers: Query<(&FollowTargetY, &mut Transform), With<Camera>>,
) {
    for (follow, mut transform) in &mut followers {
        let Ok(target) = targets.get(follow.target) else {
            continue;
        };
        let desired = target.translation.y + follow.offset_y;
        if pane.motion_enabled {
            transform.translation.y += (desired - transform.translation.y) * follow.smoothing;
        } else {
            transform.translation.y = desired;
        }
    }
}

fn draw_debug_gizmos(
    mut gizmos: Gizmos,
    point_lights: Query<(&Transform, &PointLight2d)>,
    spot_lights: Query<(&Transform, &SpotLight2d)>,
    texture_lights: Query<(&Transform, &TextureLight2d)>,
    occluders: Query<(&Transform, &LightOccluder2d)>,
) {
    for (transform, light) in &point_lights {
        gizmos.circle_2d(
            transform.translation.truncate(),
            light.radius,
            Color::srgba(1.0, 0.82, 0.36, 0.95),
        );
    }

    for (transform, light) in &spot_lights {
        let origin = transform.translation.truncate();
        let forward = Vec2::from_angle(light.direction_radians);
        let left = Vec2::from_angle(light.direction_radians - light.outer_angle_radians);
        let right = Vec2::from_angle(light.direction_radians + light.outer_angle_radians);
        gizmos.line_2d(
            origin,
            origin + forward * light.radius,
            Color::srgb(0.56, 0.80, 1.0),
        );
        gizmos.line_2d(
            origin,
            origin + left * light.radius,
            Color::srgb(0.56, 0.80, 1.0),
        );
        gizmos.line_2d(
            origin,
            origin + right * light.radius,
            Color::srgb(0.56, 0.80, 1.0),
        );
        if light.source_width > 0.0 {
            let side = Vec2::new(-forward.y, forward.x) * light.source_width * 0.5;
            gizmos.line_2d(
                origin - side,
                origin + side,
                Color::srgb(0.74, 0.92, 1.0),
            );
        }
    }

    for (transform, light) in &texture_lights {
        gizmos.rect_2d(
            Isometry2d::new(
                transform.translation.truncate(),
                Rot2::radians(light.rotation_radians),
            ),
            light.size,
            Color::srgb(1.0, 0.93, 0.72),
        );
    }

    for (transform, occluder) in &occluders {
        let center = transform.translation.truncate();
        match &occluder.shape {
            OccluderShape2d::Rectangle { half_size } => {
                gizmos.rect_2d(
                    Isometry2d::new(center, Rot2::default()),
                    *half_size * 2.0,
                    tinted_gizmo_color(occluder.shadow_tint),
                );
            }
            OccluderShape2d::Circle { radius, .. } => {
                gizmos.circle_2d(center, *radius, tinted_gizmo_color(occluder.shadow_tint));
            }
            OccluderShape2d::Polygon { points } => {
                if points.len() < 2 {
                    continue;
                }
                for (index, point) in points.iter().enumerate() {
                    let next = points[(index + 1) % points.len()];
                    gizmos.line_2d(
                        center + *point,
                        center + next,
                        tinted_gizmo_color(occluder.shadow_tint),
                    );
                }
            }
            OccluderShape2d::Mask { .. } => {
                gizmos.rect_2d(
                    Isometry2d::new(center, Rot2::default()),
                    Vec2::splat(28.0),
                    tinted_gizmo_color(occluder.shadow_tint),
                );
            }
        }
    }
}

fn update_overlay(
    mode: Res<ExampleSceneMode>,
    title: Res<ExampleSceneText>,
    diagnostics: Res<Lighting2dDiagnostics>,
    entities: Res<ExampleEntities>,
    mut overlays: Query<&mut Text, With<ExampleOverlay>>,
) {
    let Some(overlay) = entities.overlay else {
        return;
    };
    let Ok(mut text) = overlays.get_mut(overlay) else {
        return;
    };

    text.0 = format!(
        "{}\n{}\n\nMode: {:?}\nLights: {} (point {}, spot {}, texture {})\nOccluders: {}\nNormal receivers: {}\nEmissive receivers: {}\n\nThese examples are intentionally dynamic: lights, blockers, cursor-follow cookies, and convoy/camera motion are live, and the pane can pause or retime them while you tune the renderer. Gizmo outlines remain enabled as an authoring aid.",
        title.title,
        title.subtitle,
        mode.0,
        diagnostics.total_lights(),
        diagnostics.active_point_lights,
        diagnostics.active_spot_lights,
        diagnostics.active_texture_lights,
        diagnostics.active_occluders,
        diagnostics.active_normal_mapped_sprites,
        diagnostics.active_emissive_sprites,
    );
}

fn spawn_scene_shell(commands: &mut Commands) {
    commands.spawn((
        Name::new("Backdrop"),
        Sprite::from_color(Color::srgb(0.09, 0.10, 0.13), Vec2::new(2400.0, 1400.0)),
        Transform::from_xyz(0.0, 0.0, -20.0),
    ));
    commands.spawn((
        Name::new("Rear Glow"),
        Sprite::from_color(Color::srgba(0.10, 0.18, 0.22, 0.28), Vec2::new(900.0, 520.0)),
        Transform::from_xyz(80.0, 80.0, -18.0).with_scale(Vec3::new(1.1, 1.0, 1.0)),
    ));
    commands.spawn((
        Name::new("Main Wall"),
        Sprite::from_color(Color::srgb(0.16, 0.17, 0.21), Vec2::new(1020.0, 420.0)),
        Transform::from_xyz(110.0, -8.0, -12.0),
    ));
    commands.spawn((
        Name::new("Wall Lower Band"),
        Sprite::from_color(Color::srgb(0.12, 0.13, 0.17), Vec2::new(1100.0, 80.0)),
        Transform::from_xyz(40.0, -120.0, -11.0),
    ));
    commands.spawn((
        Name::new("Floor Strip"),
        Sprite::from_color(Color::srgb(0.14, 0.13, 0.13), Vec2::new(2200.0, 250.0)),
        Transform::from_xyz(0.0, -238.0, -10.0),
    ));
    commands.spawn((
        Name::new("Floor Highlight"),
        Sprite::from_color(Color::srgba(0.28, 0.24, 0.14, 0.24), Vec2::new(1600.0, 84.0)),
        Transform::from_xyz(0.0, -178.0, -9.0),
    ));
    commands.spawn((
        Name::new("Ceiling Beam"),
        Sprite::from_color(Color::srgb(0.14, 0.15, 0.19), Vec2::new(2200.0, 54.0)),
        Transform::from_xyz(0.0, 252.0, -9.0),
    ));
    commands.spawn((
        Name::new("Rear Arch"),
        Sprite::from_color(Color::srgb(0.10, 0.11, 0.14), Vec2::new(380.0, 320.0)),
        Transform::from_xyz(140.0, 10.0, -8.0),
    ));
}

fn tinted_gizmo_color(shadow_tint: Color) -> Color {
    let tint = shadow_tint.to_linear();
    Color::linear_rgba(
        (tint.red * 0.7 + 0.3).clamp(0.0, 1.0),
        (tint.green * 0.7 + 0.3).clamp(0.0, 1.0),
        (tint.blue * 0.7 + 0.3).clamp(0.0, 1.0),
        1.0,
    )
}

impl DemoAssets {
    fn generate(images: &mut Assets<Image>) -> Self {
        Self {
            cookie: images.add(gobo_cookie_image()),
            receiver_diffuse: images.add(receiver_diffuse_image()),
            normal_map: images.add(receiver_normal_image()),
            emissive_diffuse: images.add(emissive_sign_diffuse_image()),
            emissive_mask: images.add(emissive_sign_mask_image()),
            occluder_mask: images.add(occluder_mask_image()),
        }
    }
}

fn image_from_rgba(width: u32, height: u32, data: Vec<u8>) -> Image {
    Image::new_fill(
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    )
}

fn image_from_rgba_unorm(width: u32, height: u32, data: Vec<u8>) -> Image {
    Image::new_fill(
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &data,
        TextureFormat::Rgba8Unorm,
        RenderAssetUsages::default(),
    )
}

fn receiver_diffuse_image() -> Image {
    let size = 96u32;
    let mut data = Vec::with_capacity((size * size * 4) as usize);
    let center = Vec2::splat(size as f32 * 0.5);
    for y in 0..size {
        for x in 0..size {
            let pos = Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
            let uv = (pos - center) / center.x;
            let border = uv.abs().max_element();
            let ring = ((uv.length() - 0.45).abs() < 0.06) as i32 as f32;
            let rivet_1 = (uv - Vec2::new(-0.46, -0.46)).length() < 0.10;
            let rivet_2 = (uv - Vec2::new(0.46, -0.46)).length() < 0.10;
            let rivet_3 = (uv - Vec2::new(-0.46, 0.46)).length() < 0.10;
            let rivet_4 = (uv - Vec2::new(0.46, 0.46)).length() < 0.10;
            let rivet = rivet_1 || rivet_2 || rivet_3 || rivet_4;
            let stripe = uv.y.abs() < 0.08;

            let color = if border > 0.92 {
                [34, 37, 45, 255]
            } else if rivet {
                [194, 204, 214, 255]
            } else if ring > 0.5 || stripe {
                [142, 162, 184, 255]
            } else {
                [92, 106, 126, 255]
            };
            data.extend_from_slice(&color);
        }
    }
    image_from_rgba_unorm(size, size, data)
}

fn receiver_normal_image() -> Image {
    let size = 96u32;
    let center = Vec2::splat(size as f32 * 0.5);
    let mut heights = vec![0.0; (size * size) as usize];
    for y in 0..size {
        for x in 0..size {
            let pos = Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
            let uv = (pos - center) / center.x;
            let mut height = 0.08;
            if uv.abs().max_element() > 0.92 {
                height -= 0.08;
            }
            let ring_distance = (uv.length() - 0.45).abs();
            if ring_distance < 0.06 {
                height += 0.16;
            }
            if uv.y.abs() < 0.08 {
                height += 0.10;
            }
            for rivet in [
                Vec2::new(-0.46, -0.46),
                Vec2::new(0.46, -0.46),
                Vec2::new(-0.46, 0.46),
                Vec2::new(0.46, 0.46),
            ] {
                let d = (uv - rivet).length();
                if d < 0.11 {
                    height += (1.0 - d / 0.11) * 0.22;
                }
            }
            heights[(y * size + x) as usize] = height;
        }
    }

    let mut data = Vec::with_capacity((size * size * 4) as usize);
    for y in 0..size {
        for x in 0..size {
            let sample = |sx: u32, sy: u32| heights[(sy * size + sx) as usize];
            let x0 = x.saturating_sub(1);
            let x1 = (x + 1).min(size - 1);
            let y0 = y.saturating_sub(1);
            let y1 = (y + 1).min(size - 1);
            let dx = sample(x0, y) - sample(x1, y);
            let dy = sample(x, y0) - sample(x, y1);
            let normal = Vec3::new(dx * 6.0, dy * 6.0, 1.0).normalize();
            data.push(((normal.x * 0.5 + 0.5) * 255.0) as u8);
            data.push(((normal.y * 0.5 + 0.5) * 255.0) as u8);
            data.push(((normal.z * 0.5 + 0.5) * 255.0) as u8);
            data.push(255);
        }
    }

    image_from_rgba(size, size, data)
}

fn emissive_sign_diffuse_image() -> Image {
    let width = 160u32;
    let height = 64u32;
    let mut data = Vec::with_capacity((width * height * 4) as usize);
    for y in 0..height {
        for x in 0..width {
            let border = x < 6 || x >= width - 6 || y < 6 || y >= height - 6;
            let center_band = (x > 34 && x < 126) && (y > 18 && y < 46);
            let rune_band = (x > 44 && x < 118) && (y > 22 && y < 42);
            let color = if border {
                [34, 28, 20, 255]
            } else if rune_band {
                [178, 110, 26, 255]
            } else if center_band {
                [74, 44, 20, 255]
            } else {
                [48, 32, 18, 255]
            };
            data.extend_from_slice(&color);
        }
    }
    image_from_rgba(width, height, data)
}

fn emissive_sign_mask_image() -> Image {
    let width = 160u32;
    let height = 64u32;
    let mut data = Vec::with_capacity((width * height * 4) as usize);
    for y in 0..height {
        for x in 0..width {
            let left_bar = x > 52 && x < 62 && y > 22 && y < 42;
            let center_bar = x > 74 && x < 86 && y > 16 && y < 48;
            let right_bar = x > 98 && x < 108 && y > 22 && y < 42;
            let top_band = x > 52 && x < 108 && y > 18 && y < 24;
            let bottom_band = x > 52 && x < 108 && y > 40 && y < 46;
            let on = left_bar || center_bar || right_bar || top_band || bottom_band;
            let value = if on { 255 } else { 0 };
            data.extend_from_slice(&[value, value, value, value]);
        }
    }
    image_from_rgba(width, height, data)
}

fn gobo_cookie_image() -> Image {
    let size = 128u32;
    let mut data = Vec::with_capacity((size * size * 4) as usize);
    let center = size as f32 * 0.5;
    for y in 0..size {
        for x in 0..size {
            let uv = Vec2::new(
                (x as f32 + 0.5 - center) / center,
                (y as f32 + 0.5 - center) / center,
            );
            let arch = if uv.y < 0.12 {
                1.0 - ((uv.x / 0.45).abs().powf(2.0) + ((uv.y + 0.12) / 0.55).abs().powf(2.0))
            } else {
                1.0 - (uv.x.abs() / 0.38).max((uv.y - 0.1).abs() / 0.78)
            };
            let slat = if (uv.x.abs() < 0.05) || (uv.y > -0.02 && uv.y < 0.03) {
                0.45
            } else {
                1.0
            };
            let falloff = (1.0 - uv.length().powf(1.5)).clamp(0.0, 1.0);
            let alpha = arch.clamp(0.0, 1.0) * falloff * slat;
            data.extend_from_slice(&[
                255,
                (230.0 * alpha + 20.0) as u8,
                (170.0 * alpha + 30.0) as u8,
                (alpha * 255.0) as u8,
            ]);
        }
    }
    image_from_rgba(size, size, data)
}

fn occluder_mask_image() -> Image {
    let width = 72u32;
    let height = 120u32;
    let mut data = Vec::with_capacity((width * height * 4) as usize);
    let center_x = width as f32 * 0.5;
    for y in 0..height {
        for x in 0..width {
            let xf = x as f32 + 0.5;
            let yf = y as f32 + 0.5;
            let arch = if yf < 34.0 {
                let dx = (xf - center_x) / 20.0;
                let dy = (34.0 - yf) / 24.0;
                dx * dx + dy * dy <= 1.0
            } else {
                (xf - center_x).abs() < 16.0
            };
            let notch = (yf > 74.0 && yf < 98.0) && (xf - center_x).abs() < 6.0;
            let alpha = if arch && !notch { 255 } else { 0 };
            data.extend_from_slice(&[220, 140, 96, alpha]);
        }
    }
    image_from_rgba(width, height, data)
}
