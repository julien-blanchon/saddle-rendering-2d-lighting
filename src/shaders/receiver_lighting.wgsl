#import bevy_sprite::mesh2d_vertex_output::VertexOutput

const SOFT_SHADOW_SAMPLE_COUNT: i32 = 5;
const SOFT_SHADOW_OFFSETS: array<f32, 5> = array<f32, 5>(-1.0, -0.5, 0.0, 0.5, 1.0);

struct SegmentUniform {
    segment: vec4<f32>,
    transmission: vec4<f32>,
    metadata: vec4<u32>,
}

struct ReceiverLight {
    color: vec4<f32>,
    position_and_kind: vec4<f32>,
    radius_and_angles: vec4<f32>,
    direction_and_source: vec4<f32>,
    cookie_size_and_rotation: vec4<f32>,
    metadata: vec4<u32>,
}

struct ReceiverLightingMaterial {
    base_color: vec4<f32>,
    emissive_color: vec4<f32>,
    uv_rect: vec4<f32>,
    params: vec4<f32>,
    flags: vec4<f32>,
    lights: array<ReceiverLight, 8>,
    segments: array<SegmentUniform, 48>,
}

@group(2) @binding(0) var<uniform> material: ReceiverLightingMaterial;
@group(2) @binding(1) var diffuse_texture: texture_2d<f32>;
@group(2) @binding(2) var diffuse_sampler: sampler;
@group(2) @binding(3) var normal_texture: texture_2d<f32>;
@group(2) @binding(4) var normal_sampler: sampler;
@group(2) @binding(5) var emissive_mask_texture: texture_2d<f32>;
@group(2) @binding(6) var emissive_mask_sampler: sampler;
@group(2) @binding(7) var cookie_texture_0: texture_2d<f32>;
@group(2) @binding(8) var cookie_sampler_0: sampler;
@group(2) @binding(9) var cookie_texture_1: texture_2d<f32>;
@group(2) @binding(10) var cookie_sampler_1: sampler;
@group(2) @binding(11) var cookie_texture_2: texture_2d<f32>;
@group(2) @binding(12) var cookie_sampler_2: sampler;
@group(2) @binding(13) var cookie_texture_3: texture_2d<f32>;
@group(2) @binding(14) var cookie_sampler_3: sampler;

fn cross2(a: vec2<f32>, b: vec2<f32>) -> f32 {
    return a.x * b.y - a.y * b.x;
}

fn square(x: f32) -> f32 {
    return x * x;
}

fn segment_intersection(origin: vec2<f32>, ray_target: vec2<f32>, segment: vec4<f32>) -> vec2<f32> {
    let p = origin;
    let r = ray_target - origin;
    let q = segment.xy;
    let s = segment.zw - segment.xy;
    let denom = cross2(r, s);
    if abs(denom) < 0.0001 {
        return vec2(-1.0, 0.0);
    }

    let qp = q - p;
    let t = cross2(qp, s) / denom;
    let u = cross2(qp, r) / denom;
    return vec2(t, u);
}

fn attenuation(distance_to_light: f32, radius: f32, falloff: f32) -> f32 {
    let s = distance_to_light / max(radius, 0.001);
    if s > 1.0 {
        return 0.0;
    }

    let s2 = square(s);
    return square(max(1.0 - s2, 0.0)) / (1.0 + falloff * s2);
}

fn effective_spot_light_center(
    light_position: vec2<f32>,
    direction: vec2<f32>,
    source_half_width: f32,
    point: vec2<f32>,
) -> vec2<f32> {
    if source_half_width <= 0.001 {
        return light_position;
    }

    let bar_direction = normalize(vec2(-direction.y, direction.x));
    let projection = dot(point - light_position, bar_direction);
    return light_position + bar_direction * clamp(projection, -source_half_width, source_half_width);
}

fn apply_segment_shadow(segment: SegmentUniform, shadow_mode: u32) -> vec3<f32> {
    if shadow_mode == 0u {
        return vec3(0.0, 0.0, 0.0);
    }
    if shadow_mode == 1u {
        return vec3(segment.transmission.a, segment.transmission.a, segment.transmission.a);
    }

    return clamp(segment.transmission.rgb, vec3(0.0, 0.0, 0.0), vec3(1.0, 1.0, 1.0));
}

fn segment_transmission(
    origin: vec2<f32>,
    ray_target: vec2<f32>,
    segment_count: i32,
    shadow_mode: u32,
    occluder_mask: u32,
) -> vec3<f32> {
    var transmission = vec3(1.0, 1.0, 1.0);
    for (var index = 0; index < 48; index = index + 1) {
        if index >= segment_count {
            break;
        }

        let segment = material.segments[index];
        if (segment.metadata.x & occluder_mask) == 0u {
            continue;
        }
        let hit = segment_intersection(origin, ray_target, segment.segment);
        let t = hit.x;
        let u = hit.y;
        if t > 0.0005 && t < 0.9995 && u >= 0.0 && u <= 1.0 {
            transmission *= apply_segment_shadow(segment, shadow_mode);
        }
    }

    return transmission;
}

fn soft_shadow_origin(
    origin: vec2<f32>,
    ray_target: vec2<f32>,
    light_kind: i32,
    direction: vec2<f32>,
    source_extent: f32,
    sample_index: i32,
) -> vec2<f32> {
    if source_extent <= 0.001 {
        return origin;
    }

    let offset = SOFT_SHADOW_OFFSETS[sample_index] * source_extent;
    if light_kind == 1 {
        let bar_direction = normalize(vec2(-direction.y, direction.x));
        return origin + bar_direction * offset;
    }

    let ray_direction = normalize(ray_target - origin);
    let perpendicular = normalize(vec2(-ray_direction.y, ray_direction.x));
    return origin + perpendicular * offset;
}

fn shadow_transmission(
    origin: vec2<f32>,
    ray_target: vec2<f32>,
    light_kind: i32,
    direction: vec2<f32>,
    source_extent: f32,
    segment_count: i32,
    shadow_mode: u32,
    occluder_mask: u32,
    soft_shadows: bool,
) -> vec3<f32> {
    if !soft_shadows || source_extent <= 0.001 {
        return segment_transmission(origin, ray_target, segment_count, shadow_mode, occluder_mask);
    }

    var accumulated = vec3(0.0, 0.0, 0.0);
    for (var index = 0; index < SOFT_SHADOW_SAMPLE_COUNT; index = index + 1) {
        let sample_origin = soft_shadow_origin(
            origin,
            ray_target,
            light_kind,
            direction,
            source_extent,
            index,
        );
        accumulated += segment_transmission(
            sample_origin,
            ray_target,
            segment_count,
            shadow_mode,
            occluder_mask,
        );
    }

    return accumulated / f32(SOFT_SHADOW_SAMPLE_COUNT);
}

fn unpack_normal(sampled: vec3<f32>, strength: f32) -> vec3<f32> {
    let mapped = sampled * 2.0 - vec3(1.0, 1.0, 1.0);
    return normalize(vec3(mapped.xy * strength, max(mapped.z, 0.0001)));
}

fn sample_cookie(slot: u32, uv: vec2<f32>) -> vec4<f32> {
    switch slot {
        case 1u: {
            return textureSample(cookie_texture_0, cookie_sampler_0, uv);
        }
        case 2u: {
            return textureSample(cookie_texture_1, cookie_sampler_1, uv);
        }
        case 3u: {
            return textureSample(cookie_texture_2, cookie_sampler_2, uv);
        }
        case 4u: {
            return textureSample(cookie_texture_3, cookie_sampler_3, uv);
        }
        default: {
            return vec4(1.0, 1.0, 1.0, 1.0);
        }
    }
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    var uv = mix(material.uv_rect.xy, material.uv_rect.zw, in.uv);
    if material.flags.x > 0.5 {
        uv.x = material.uv_rect.z - (uv.x - material.uv_rect.x);
    }
    if material.flags.y > 0.5 {
        uv.y = material.uv_rect.w - (uv.y - material.uv_rect.y);
    }

    let base_sample = textureSample(diffuse_texture, diffuse_sampler, uv) * material.base_color;
    let normal_sample = textureSample(normal_texture, normal_sampler, uv);
    let mask_sample = textureSample(emissive_mask_texture, emissive_mask_sampler, uv);

    let strength = material.params.x;
    let sprite_height = material.params.y;
    let emissive_intensity = material.params.z;
    let soft_shadows = material.params.w > 0.5;
    let light_count = i32(material.flags.z + 0.5);
    let segment_count = i32(material.flags.w + 0.5);
    let normal = unpack_normal(normal_sample.xyz, strength);

    var lighting = vec3(0.0, 0.0, 0.0);
    for (var index = 0; index < 8; index = index + 1) {
        if index >= light_count {
            break;
        }

        let light = material.lights[index];
        let light_position = light.position_and_kind.xy;
        let light_kind = i32(light.position_and_kind.w + 0.5);
        let direction = normalize(light.direction_and_source.xy);
        let source_extent = light.direction_and_source.z;
        let cookie_strength = light.direction_and_source.w;
        let shadow_mode = light.metadata.x;
        let occluder_mask = light.metadata.y;
        let cookie_slot = light.metadata.z;

        var emitter_position = light_position;
        if light_kind == 1 {
            emitter_position = effective_spot_light_center(
                light_position,
                direction,
                source_extent,
                in.world_position.xy,
            );
        }

        var light_attenuation = attenuation(
            distance(emitter_position, in.world_position.xy),
            light.radius_and_angles.x,
            light.radius_and_angles.y,
        );

        if light_kind == 1 {
            let to_frag = normalize(in.world_position.xy - emitter_position);
            let cone_cos = dot(direction, to_frag);
            let inner_cos = light.radius_and_angles.z;
            let outer_cos = light.radius_and_angles.w;
            light_attenuation *= smoothstep(outer_cos, inner_cos, cone_cos);
        }

        if light_kind == 2 {
            let size = max(light.cookie_size_and_rotation.xy, vec2(0.001, 0.001));
            let cos_angle = light.cookie_size_and_rotation.z;
            let sin_angle = light.cookie_size_and_rotation.w;
            let local = in.world_position.xy - light_position;
            let rotated = vec2(
                cos_angle * local.x + sin_angle * local.y,
                -sin_angle * local.x + cos_angle * local.y,
            );
            let uv = rotated / size + vec2(0.5, 0.5);
            if any(uv < vec2(0.0, 0.0)) || any(uv > vec2(1.0, 1.0)) {
                light_attenuation = 0.0;
            } else {
                let cookie = sample_cookie(cookie_slot, uv);
                light_attenuation *= cookie_strength * cookie.a * (cookie.r + cookie.g + cookie.b) / 3.0;
            }
        }

        if light_attenuation <= 0.0001 {
            continue;
        }

        let transmission = shadow_transmission(
            emitter_position,
            in.world_position.xy,
            light_kind,
            direction,
            source_extent,
            segment_count,
            shadow_mode,
            occluder_mask,
            soft_shadows,
        );
        if max(max(transmission.r, transmission.g), transmission.b) <= 0.0001 {
            continue;
        }

        let to_light = normalize(vec3(
            emitter_position - in.world_position.xy,
            max(light.position_and_kind.z - sprite_height, 1.0),
        ));
        let lambert = max(dot(normal, to_light), 0.0);
        lighting += light.color.rgb * light_attenuation * lambert * transmission;
    }

    let emissive = material.emissive_color.rgb * emissive_intensity * mask_sample.r * base_sample.a;
    let lit = base_sample.rgb * lighting + emissive;
    return vec4(lit, max(base_sample.a, mask_sample.a * emissive_intensity));
}
