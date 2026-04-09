#[cfg(feature = "e2e")]
mod e2e;
#[cfg(feature = "e2e")]
mod scenarios;

use bevy::prelude::*;
#[cfg(feature = "dev")]
use bevy_brp_extras::BrpExtrasPlugin;
use common::{
    ExampleMode, ExampleSceneMode, ExampleSceneText, add_example_systems, install_common_plugins,
    setup_scene,
};
use saddle_rendering_2d_lighting_example_common as common;

const DEFAULT_BRP_PORT: u16 = 15_744;

fn main() {
    let mut app = App::new();
    install_common_plugins(
        &mut app,
        "saddle-rendering-2d-lighting crate-local lab",
        "LIGHTING_2D_LAB_EXIT_AFTER_SECONDS",
    );
    #[cfg(feature = "dev")]
    app.add_plugins(BrpExtrasPlugin::with_port(lab_brp_port()));
    #[cfg(feature = "e2e")]
    app.add_plugins(e2e::Lighting2dLabE2EPlugin);
    app.insert_resource(ExampleSceneMode(lab_mode_from_args()));
    app.insert_resource(ExampleSceneText {
        title: "2D Lighting Lab".into(),
        subtitle: "Crate-local verification scene for authoring coverage, diagnostics, and future render backend work.".into(),
    });
    add_example_systems(&mut app);
    app.add_systems(Startup, setup_scene);
    app.run();
}

fn lab_mode_from_args() -> ExampleMode {
    let scenario_name = std::env::args().skip(1).find(|arg| !arg.starts_with('-'));
    match scenario_name.as_deref() {
        Some(name) if name.contains("dungeon") => ExampleMode::Dungeon,
        Some(name) if name.contains("texture_cursor") => ExampleMode::TextureCursor,
        Some(name) if name.contains("mask_occluder") => ExampleMode::MaskOccluder,
        Some(name) if name.contains("road_convoy") => ExampleMode::RoadConvoy,
        _ => ExampleMode::GameDemo,
    }
}

#[cfg(feature = "dev")]
fn lab_brp_port() -> u16 {
    std::env::var("BRP_EXTRAS_PORT")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(DEFAULT_BRP_PORT)
}
