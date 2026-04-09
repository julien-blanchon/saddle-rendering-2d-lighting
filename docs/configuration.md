# Configuration

## Runtime Notes

The production backend now consumes nearly the entire public surface. The only intentionally future-facing toggle is `LightingBackend2d::ExperimentalGi`.

## `Lighting2dSettings`

Attach to a `Camera2d`.

| Field | Type | Default | Effect Today |
|------|------|---------|-----------------|
| `lighting_enabled` | `bool` | `true` | Master toggle for the camera lighting path |
| `ambient_color` | `Color` | soft blue-gray | Multiplies the lit camera view |
| `ambient_intensity` | `f32` | `0.15` | Strength of the ambient multiply factor |
| `lightmap_scale` | `f32` | `0.5` | Scales the per-view offscreen lightmap resolution relative to the camera viewport |
| `normal_map_scale` | `f32` | `1.0` | Scales receiver normal-map strength globally |
| `blur_radius` | `u32` | `0` | Enables the separable blur pass on the offscreen lightmap when greater than zero |
| `enable_occlusion` | `bool` | `true` | Enables occluder segment packing and shadowing for the current camera |
| `enable_normal_maps` | `bool` | `true` | Enables normal-mapped receiver proxies |
| `enable_emissive` | `bool` | `true` | Enables emissive receiver proxies |
| `backend` | `LightingBackend2d` | `ScreenSpace` | `ScreenSpace` is implemented; `ExperimentalGi` remains a future backend switch |
| `shadow_filter` | `ShadowFiltering2d` | `Hard` | Selects hard vs multi-sample soft shadow filtering |
| `composite_mode` | `LightingCompositeMode2d` | `Multiply` | `Multiply` keeps the ambient overlay; `Additive` skips ambient multiplication and only composites the lightmap |

Convenience presets:

- `Lighting2dSettings::showcase_soft()` enables the current showcase defaults with soft shadows
- `Lighting2dSettings::fast_unshadowed()` disables occlusion for a cheaper additive path

## Lights

### `PointLight2d`

| Field | Type | Default | Effect Today |
|------|------|---------|-----------------|
| `color` | `Color` | white | Light tint |
| `intensity` | `f32` | `1.0` | Brightness multiplier |
| `radius` | `f32` | `96.0` | Effective range |
| `falloff` | `f32` | `1.0` | Controls edge attenuation |
| `height` | `f32` | `48.0` | Height used by the receiver normal-map shader |
| `source_radius` | `f32` | `12.0` | Finite emitter size used by the soft shadow path |
| `shadow_mode` | `LightShadowMode2d` | `Illuminated` | Controls whether the light remains visible in shadowed regions, is fully occluded, or produces solid shadowing |
| `occluder_mask` | `u32` | `u32::MAX` | Bitmask used to filter which occluder groups affect this light |

### `SpotLight2d`

Adds:

- `source_width`
- `direction_radians`
- `inner_angle_radians`
- `outer_angle_radians`
- `height`
- `shadow_mode`
- `occluder_mask`

### `TextureLight2d`

Adds:

- `texture`
- `size`
- `height`
- `source_radius`
- `rotation_radians`
- `shadow_mode`
- `occluder_mask`

### `LightShadowMode2d`

| Variant | Effect Today |
|------|-----------------|
| `Illuminated` | Default mode; lit pixels remain visible and shadowing subtracts from the light contribution |
| `Occluded` | Treats blocker hits as fully occluding the light contribution behind the segment |
| `Solid` | Produces solid shadow treatment that is useful for punchier masked or stylized lights |

## Occluders

### `LightOccluder2d`

| Field | Type | Default | Effect Today |
|------|------|---------|-----------------|
| `shape` | `OccluderShape2d` | small rectangle | Public occluder geometry |
| `casts_shadows` | `bool` | `true` | Enables segment emission into the current light pass |
| `absorption` | `f32` | `1.0` | Controls how strongly the occluder modulates light |
| `shadow_tint` | `Color` | black | Transmission color used when the light passes through the occluder |
| `groups` | `u32` | `1` | Bitmask groups that lights can include or exclude with their `occluder_mask` |

### `OccluderShape2d`

Supported public variants:

- `Rectangle { half_size }`
- `Circle { radius, segments }`
- `Polygon { points }`
- `Mask { mask, alpha_threshold }`

Mask occluders derive edge geometry from the alpha silhouette of the supplied image and cache the extracted segments for reuse.

## Receivers

### `NormalMappedSprite2d`

| Field | Type | Default | Effect Today |
|------|------|---------|-----------------|
| `normal_map` | `Handle<Image>` | required by user | Normal texture source |
| `strength` | `f32` | `1.0` | Normal contribution multiplier |
| `height` | `f32` | `0.0` | Receiver surface height hint for lighting |

Receiver shading consumes nearby point, spot, and texture lights. Texture lights are sampled per pixel in receiver space so cookie alignment follows the world transform instead of a single scalar strength.

### `EmissiveSprite2d`

| Field | Type | Default | Effect Today |
|------|------|---------|-----------------|
| `color` | `Color` | warm white | Emissive tint |
| `intensity` | `f32` | `1.0` | Emissive brightness |
| `mask` | `Option<Handle<Image>>` | `None` | Optional emissive mask; `None` means the whole sprite emits |

## Diagnostics

### `Lighting2dDiagnostics`

Tracks counts for:

- lit cameras
- point lights
- spot lights
- texture lights
- occluders
- normal-mapped sprites
- emissive sprites

This is the main cheap runtime summary for the current backend.
