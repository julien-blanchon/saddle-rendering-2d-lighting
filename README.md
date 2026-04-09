# Saddle Rendering 2D Lighting

Flexible 2D lighting toolkit for Bevy focused on reusable authoring components, strong documentation, crate-local examples, and lab-first verification.

## Status

This crate now ships a **production-ready reusable backend**.

What exists today:

- multiplicative ambient composition per lit camera
- per-view offscreen lightmap accumulation and additive composite
- optional blur filtering on the real lightmap output
- additive point, spot, and texture / cookie light rendering
- occluder-driven hard and soft shadowing in the light shader
- colored / translucent shadow transmission through occluders
- occluder groups, per-light occluder masks, and explicit shadow behavior modes
- image-mask occluders with cached edge extraction
- receiver proxies for normal-mapped and emissive sprites
- point, spot, and texture lights feeding receiver shading
- per-pixel texture / cookie sampling in receiver shading
- multiple lit cameras with isolated per-view runtime state and internal layers
- `Lighting2dSettings::showcase_soft()` and `Lighting2dSettings::fast_unshadowed()` presets
- standalone example workspace structure
- a crate-local lab with discoverable scenarios and runtime artifacts
- a detailed public docs surface covering configuration, architecture, and examples

What is still intentionally deferred:

- deeper performance tuning
- experimental GI exploration

The crate is usable now, and the remaining work is focused on hardening and deeper feature coverage rather than inventing the first backend.

## Quick Start

```toml
saddle-rendering-2d-lighting = { git = "https://github.com/julien-blanchon/saddle-rendering-2d-lighting" }
```

```rust,no_run
use bevy::prelude::*;
use saddle_rendering_2d_lighting::{
    Lighting2dPlugin, Lighting2dSettings, PointLight2d, SpotLight2d,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(Lighting2dPlugin::default())
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        Lighting2dSettings::default(),
        Name::new("Main Camera"),
    ));

    commands.spawn((
        Name::new("Torch"),
        PointLight2d::default(),
        Transform::from_xyz(0.0, 32.0, 0.0),
    ));

    commands.spawn((
        Name::new("Cone Light"),
        SpotLight2d::default(),
        Transform::from_xyz(120.0, 48.0, 0.0),
    ));
}
```

## Public API

| Type | Purpose |
|------|---------|
| `Lighting2dPlugin` | Registers the runtime with injectable activate / deactivate / update schedules |
| `Lighting2dSystems` | Public ordering hooks for internal prep, authoring sync, bounds, proxy updates, and diagnostics |
| `Lighting2dSettings` | Camera-scoped lighting configuration and backend toggles |
| `PointLight2d` | Radial light authoring component with explicit light height and finite source radius |
| `SpotLight2d` | Cone light authoring component with explicit light height and source width |
| `TextureLight2d` | Cookie / textured light authoring component with explicit light height and source radius |
| `LightOccluder2d` | Occluder authoring component with geometry, transmission, and occluder-group controls |
| `OccluderShape2d` | Occluder shape enum including rectangle, circle, polygon, and image-mask variants |
| `LightShadowMode2d` | Per-light shadow behavior selector for lit, fully occluded, or solid shadow treatment |
| `NormalMappedSprite2d` | Normal-map authoring component for sprite receivers |
| `EmissiveSprite2d` | Emissive contribution authoring component |
| `Lighting2dDiagnostics` | Runtime counts for cameras, lights, occluders, emissive sprites, and normal-mapped sprites |

## Backend Notes

The current backend uses:

- a per-view offscreen lightmap target with optional separable blur
- a multiplicative fullscreen ambient overlay per lit camera
- additive world-space light meshes for point, spot, and texture lights feeding the offscreen lightmap
- a dedicated composite pass that presents the finished lightmap back into the main view
- CPU-packed occluder geometry consumed directly by the light shader
- cached image-mask occluder extraction for authored alpha masks
- additive receiver proxies for normal-mapped and emissive sprites with per-pixel cookie sampling

Current limitations:

- the current receiver path still prioritizes a bounded number of nearby lights per sprite for cost control
- mask occluders are derived from alpha silhouettes rather than arbitrary artist-authored polygon data
- `ExperimentalGi` is still a future alternate backend

The detailed technical decisions live in [`docs/architecture.md`](docs/architecture.md) and [`docs/configuration.md`](docs/configuration.md).

## Examples

The example workspace now exercises the real backend with live motion, pane-driven tuning, and reference-inspired comparisons. Every example keeps gizmo overlays enabled so the authoring data stays easy to inspect while the scene is actually moving.

| Example | Purpose | Run |
|---------|---------|-----|
| `basic` | Baseline camera + point + spot light authoring | `cd examples && cargo run -p saddle-rendering-2d-lighting-example-basic` |
| `occluders` | Occluder authoring shapes and debug visualization | `cd examples && cargo run -p saddle-rendering-2d-lighting-example-occluders` |
| `normal_maps` | Normal-map and emissive authoring surface | `cd examples && cargo run -p saddle-rendering-2d-lighting-example-normal-maps` |
| `mixed_lights` | Point, spot, and texture light composition | `cd examples && cargo run -p saddle-rendering-2d-lighting-example-mixed-lights` |
| `dungeon` | Candlelit room inspired by `bevy_light_2d/examples/dungeon` | `cd examples && cargo run -p saddle-rendering-2d-lighting-example-dungeon` |
| `texture_cursor` | Interactive textured-light demo inspired by `bevy_lit/examples/texture_light` | `cd examples && cargo run -p saddle-rendering-2d-lighting-example-texture-cursor` |
| `mask_occluder` | Moving-light alpha-mask blocker demo inspired by `bevy_lit/examples/occluder_mask` | `cd examples && cargo run -p saddle-rendering-2d-lighting-example-mask-occluder` |
| `road_convoy` | Scrolling vehicle-and-headlights demo inspired by `bevy_2d_screen_space_lightmaps` moving truck | `cd examples && cargo run -p saddle-rendering-2d-lighting-example-road-convoy` |
| `game_demo` | Integration-oriented dungeon-like composition | `cd examples && cargo run -p saddle-rendering-2d-lighting-example-game-demo` |
| `stress` | Many authored lights and occluders for diagnostics | `cd examples && cargo run -p saddle-rendering-2d-lighting-example-stress` |

All examples include `saddle-pane` controls for both lighting parameters and motion playback, and they use only procedurally generated assets so the workspace stays standalone.

## Crate-Local Lab

The richer verification app lives at `examples/lab`:

```bash
cd examples && cargo run -p saddle-rendering-2d-lighting-lab
```

Current scenarios:

```bash
cd examples && cargo run -p saddle-rendering-2d-lighting-lab --features e2e -- smoke_launch
cd examples && cargo run -p saddle-rendering-2d-lighting-lab --features e2e -- lighting_overview
cd examples && cargo run -p saddle-rendering-2d-lighting-lab --features e2e -- lighting_occluders
cd examples && cargo run -p saddle-rendering-2d-lighting-lab --features e2e -- lighting_normal_maps
cd examples && cargo run -p saddle-rendering-2d-lighting-lab --features e2e -- lighting_mixed_lights
cd examples && cargo run -p saddle-rendering-2d-lighting-lab --features e2e -- lighting_blurred_lightmap
cd examples && cargo run -p saddle-rendering-2d-lighting-lab --features e2e -- lighting_receiver_cookies
cd examples && cargo run -p saddle-rendering-2d-lighting-lab --features e2e -- lighting_mask_occluders
cd examples && cargo run -p saddle-rendering-2d-lighting-lab --features e2e -- lighting_soft_shadows
cd examples && cargo run -p saddle-rendering-2d-lighting-lab --features e2e -- lighting_multi_camera
cd examples && cargo run -p saddle-rendering-2d-lighting-lab --features e2e -- lighting_stress
cd examples && cargo run -p saddle-rendering-2d-lighting-lab --features e2e -- reference_dungeon
cd examples && cargo run -p saddle-rendering-2d-lighting-lab --features e2e -- reference_texture_cursor
cd examples && cargo run -p saddle-rendering-2d-lighting-lab --features e2e -- reference_mask_occluder
cd examples && cargo run -p saddle-rendering-2d-lighting-lab --features e2e -- reference_road_convoy
```

## Design Notes

- The public API is intentionally component-driven.
- The production backend is implemented with per-view lightmaps, optional blur, additive light meshes, colored hard/soft shadowing, receiver proxies, occluder masks, and isolated lit-camera state.
- The public authoring API is intentionally stable even if the internal backend evolves later.
- Experimental GI is explicitly deferred until the reusable baseline is solid.

More detail lives in [`docs/architecture.md`](docs/architecture.md) and [`docs/configuration.md`](docs/configuration.md).
