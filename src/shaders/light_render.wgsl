#import bevy_sprite::mesh2d_vertex_output::VertexOutput

const SOFT_SHADOW_SAMPLE_COUNT: i32 = 5;
const SOFT_SHADOW_OFFSETS: array<f32, 5> = array<f32, 5>(-1.0, -0.5, 0.0, 0.5, 1.0);

struct SegmentUniform {
    segment: vec4<f32>,
    transmission: vec4<f32>,
    metadata: vec4<u32>,
}

struct LightMaterial {
    color: vec4<f32>,
    position_and_kind: vec4<f32>,
    radius_and_angles: vec4<f32>,
    direction_and_flags: vec4<f32>,
    texture_size_and_rotation: vec4<f32>,
    source_params: vec4<f32>,
    metadata: vec4<u32>,
    segments: array<SegmentUniform, 48>,
}

@group(2) @binding(0) var<uniform> material: LightMaterial;
@group(2) @binding(1) var cookie_texture: texture_2d<f32>;
@group(2) @binding(2) var cookie_sampler: sampler;

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

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let light_position = material.position_and_kind.xy;
    let light_kind = i32(material.position_and_kind.w + 0.5);
    let point = in.world_position.xy;
    let radius = material.radius_and_angles.x;
    let falloff = material.radius_and_angles.y;
    let direction = normalize(material.direction_and_flags.xy);
    let segment_count = i32(material.direction_and_flags.z + 0.5);
    let occlusion_mode = material.direction_and_flags.w;
    let use_occlusion = occlusion_mode > 0.5;
    let soft_shadows = occlusion_mode > 1.5;
    let source_extent = material.source_params.x;
    let shadow_mode = material.metadata.x;
    let occluder_mask = material.metadata.y;

    var emitter_position = light_position;
    if light_kind == 1 {
        emitter_position = effective_spot_light_center(light_position, direction, source_extent, point);
    }

    var light_attenuation = attenuation(distance(emitter_position, point), radius, falloff);

    if light_kind == 1 {
        let vector_to_point = normalize(point - emitter_position);
        let cone_cos = dot(direction, vector_to_point);
        let inner_cos = material.radius_and_angles.z;
        let outer_cos = material.radius_and_angles.w;
        light_attenuation *= smoothstep(outer_cos, inner_cos, cone_cos);
    }

    if light_kind == 2 {
        let size = material.texture_size_and_rotation.xy;
        let cos_angle = material.texture_size_and_rotation.z;
        let sin_angle = material.texture_size_and_rotation.w;
        let local = point - light_position;
        let rotated = vec2(
            cos_angle * local.x + sin_angle * local.y,
            -sin_angle * local.x + cos_angle * local.y,
        );
        let uv = rotated / max(size, vec2(0.001, 0.001)) + vec2(0.5, 0.5);
        if any(uv < vec2(0.0, 0.0)) || any(uv > vec2(1.0, 1.0)) {
            light_attenuation = 0.0;
        } else {
            let cookie = textureSample(cookie_texture, cookie_sampler, uv);
            light_attenuation *= cookie.a * (cookie.r + cookie.g + cookie.b) / 3.0;
        }
    }

    if light_attenuation <= 0.0001 {
        return vec4(0.0);
    }

    var transmission = vec3(1.0, 1.0, 1.0);
    if use_occlusion {
        transmission = shadow_transmission(
            emitter_position,
            point,
            light_kind,
            direction,
            source_extent,
            segment_count,
            shadow_mode,
            occluder_mask,
            soft_shadows,
        );
    }

    let alpha = light_attenuation * max(max(transmission.r, transmission.g), transmission.b);
    return vec4(material.color.rgb * light_attenuation * transmission, alpha);
}
