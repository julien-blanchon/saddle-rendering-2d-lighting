# Architecture

## Current Runtime Shape

The current backend is a hybrid runtime built from a few intentionally simple layers:

- per-view offscreen lightmap targets sized from the authored camera viewport
- optional separable blur passes that soften the lightmap before composition
- camera-scoped ambient overlays with multiplicative blending
- world-space additive light proxies for point, spot, and texture lights
- CPU-packed occluder segments with per-segment transmission tint and occluder-group metadata consumed directly by the light shader
- additive receiver proxies for normal-mapped and emissive sprites
- cached image-mask occluder extraction for alpha-driven blocker authoring

This keeps the public authoring surface small while still giving each lit camera an isolated runtime and a real light accumulation path.

## Data Flow

```text
camera settings + light authoring + occluder authoring + receiver metadata
    ->
global runtime context + internal asset prep
    ->
per-view offscreen lightmap targets
    ->
additive light proxies
    ->
receiver proxies
    ->
optional blur passes
    ->
ambient overlay + composite quad in the owning camera view
    ->
final 2D composition in the standard transparent pass
```

## Why This Shape

### Stable authoring layer

Most downstream games should interact only with:

- `Lighting2dSettings`
- light components
- occluder components
- receiver metadata components

### Swappable internal implementation

We want to keep room for:

- cheaper or richer light accumulation internals
- richer shadow paths
- normal-map support
- per-view lightmap quality tuning
- possible experimental GI later

without forcing users to rewrite gameplay-side authoring code.

## Internal Modules

- `config`: camera-scoped settings and backend toggles
- `components`: light, occluder, and receiver authoring
- `materials`: internal 2D materials for ambient overlays, light proxies, and receiver proxies
- `geometry`: world-space occluder packing, mask extraction, and sprite UV / size helpers
- `systems`: runtime context, per-view lightmap management, bounds, proxy sync, cleanup, and diagnostics

## Current Ordering

`Lighting2dSystems` is intentionally public:

1. `Prepare`
2. `SyncAuthoring`
3. `UpdateBounds`
4. `UpdateProxies`
5. `Diagnostics`
6. `Debug`

The current backend uses `Prepare`, `UpdateBounds`, `UpdateProxies`, and `Diagnostics`.

## View Runtime Model

Each camera with `Lighting2dSettings` gets its own internal `LightingViewRuntime` that owns:

- a dedicated render layer group
- a lightmap render target
- optional blur intermediates
- a composite quad and ambient overlay bound to the owning camera
- per-view light, receiver, and auxiliary camera proxies

This is what prevents cross-view leakage when multiple lit cameras coexist with different settings.

## Occluder Packing

Occluders are converted into segment lists on the CPU. Rectangle, circle, and polygon shapes are generated procedurally. Mask shapes derive boundary edges from the alpha silhouette of the source image and store the result in a cache keyed by image asset id plus threshold. Packed segment metadata carries:

- edge endpoints
- shadow tint and absorption
- occluder group bits for per-light filtering

The light and receiver shaders read those packed segments directly.

## Known Limits

- light and receiver shaders still cap the number of nearby lights and occluders they consume per proxy for predictable runtime cost
- mask occluders are silhouette-derived, not author-authored concave decomposition data
- `ExperimentalGi` is still a future alternate backend, not part of the baseline runtime
