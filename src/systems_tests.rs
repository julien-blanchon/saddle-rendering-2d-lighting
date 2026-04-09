use bevy::asset::AssetPlugin;
use bevy::prelude::*;

use crate::{
    EmissiveSprite2d, LightOccluder2d, Lighting2dDiagnostics, Lighting2dPlugin, Lighting2dSettings,
    NormalMappedSprite2d, PointLight2d, SpotLight2d, TextureLight2d,
};

#[test]
fn diagnostics_count_public_authoring_components() {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()));
    app.add_plugins(Lighting2dPlugin::default());

    let cookie = Handle::<Image>::default();

    app.world_mut()
        .spawn((Camera2d, Lighting2dSettings::default(), Name::new("Camera")));
    app.world_mut().spawn((
        PointLight2d::default(),
        Transform::default(),
        Name::new("Point"),
    ));
    app.world_mut().spawn((
        SpotLight2d::default(),
        Transform::default(),
        Name::new("Spot"),
    ));
    app.world_mut().spawn((
        TextureLight2d {
            texture: cookie.clone(),
            ..TextureLight2d::default()
        },
        Transform::default(),
        Name::new("Cookie"),
    ));
    app.world_mut().spawn((
        LightOccluder2d::rectangle(Vec2::new(8.0, 12.0)),
        Transform::default(),
        Name::new("Occluder"),
    ));
    app.world_mut().spawn((
        NormalMappedSprite2d::new(cookie.clone()),
        Sprite::from_color(Color::WHITE, Vec2::splat(8.0)),
        Name::new("Normal"),
    ));
    app.world_mut().spawn((
        EmissiveSprite2d::default(),
        Sprite::from_color(Color::WHITE, Vec2::splat(8.0)),
        Name::new("Emissive"),
    ));

    app.update();

    let diagnostics = app.world().resource::<Lighting2dDiagnostics>();
    assert_eq!(diagnostics.active_cameras, 1);
    assert_eq!(diagnostics.active_point_lights, 1);
    assert_eq!(diagnostics.active_spot_lights, 1);
    assert_eq!(diagnostics.active_texture_lights, 1);
    assert_eq!(diagnostics.active_occluders, 1);
    assert_eq!(diagnostics.active_normal_mapped_sprites, 1);
    assert_eq!(diagnostics.active_emissive_sprites, 1);
}
