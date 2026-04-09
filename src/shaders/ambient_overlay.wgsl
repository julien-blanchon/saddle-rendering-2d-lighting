#import bevy_sprite::mesh2d_vertex_output::VertexOutput

struct AmbientOverlayMaterial {
    tint: vec4<f32>,
}

@group(2) @binding(0) var<uniform> material: AmbientOverlayMaterial;

@fragment
fn fragment(_in: VertexOutput) -> @location(0) vec4<f32> {
    return material.tint;
}
