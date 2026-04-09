use bevy::prelude::*;
use common::{
    ExampleMode, ExampleSceneMode, ExampleSceneText, add_example_systems, install_common_plugins,
    setup_scene,
};
use saddle_rendering_2d_lighting_example_common as common;

fn main() {
    let mut app = App::new();
    install_common_plugins(
        &mut app,
        "saddle-rendering-2d-lighting — mask occluder",
        "LIGHTING_2D_EXIT_AFTER_SECONDS",
    );
    app.insert_resource(ExampleSceneMode(ExampleMode::MaskOccluder));
    app.insert_resource(ExampleSceneText {
        title: "2D lighting — mask occluder".into(),
        subtitle: "Moving-light showcase for alpha-mask blockers and per-light occluder filtering.".into(),
    });
    add_example_systems(&mut app);
    app.add_systems(Startup, setup_scene);
    app.run();
}
