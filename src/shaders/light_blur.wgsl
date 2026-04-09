#import bevy_sprite::mesh2d_vertex_output::VertexOutput

struct LightBlurMaterial {
    direction_and_radius: vec4<f32>,
}

@group(2) @binding(0) var<uniform> material: LightBlurMaterial;
@group(2) @binding(1) var source_texture: texture_2d<f32>;
@group(2) @binding(2) var source_sampler: sampler;

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let radius = material.direction_and_radius.z;
    if radius <= 0.5 {
        return textureSample(source_texture, source_sampler, in.uv);
    }

    let step_uv = material.direction_and_radius.xy * max(radius, 1.0);
    var color = textureSample(source_texture, source_sampler, in.uv) * 0.227027;
    color += textureSample(source_texture, source_sampler, in.uv + step_uv * 1.384615) * 0.316216;
    color += textureSample(source_texture, source_sampler, in.uv - step_uv * 1.384615) * 0.316216;
    color += textureSample(source_texture, source_sampler, in.uv + step_uv * 3.230769) * 0.070270;
    color += textureSample(source_texture, source_sampler, in.uv - step_uv * 3.230769) * 0.070270;
    return color;
}
