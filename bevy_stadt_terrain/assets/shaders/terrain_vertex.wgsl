#import bevy_pbr::{
    mesh_bindings::mesh,
    mesh_functions,
    mesh_view_bindings::view,
    forward_io::VertexOutput,
    view_transformations::position_world_to_clip,
}

// Morph distances - these could be made configurable via push constants or global uniform
// Morphing starts at MORPH_START and completes at MORPH_END
const MORPH_START: f32 = 150.0;
const MORPH_END: f32 = 300.0;

// Custom vertex input with morph_height attribute
struct TerrainVertex {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(5) color: vec4<f32>,
    @location(17) morph_height: f32,
}

@vertex
fn vertex(vertex: TerrainVertex) -> VertexOutput {
    var out: VertexOutput;

    let mesh_world_from_local = mesh_functions::get_world_from_local(vertex.instance_index);

    // Calculate world position of the vertex (before morphing, for distance calc)
    let world_pos = (mesh_world_from_local * vec4<f32>(vertex.position, 1.0)).xyz;

    // Get camera position from view uniform
    let camera_position = view.world_position;

    // Calculate distance from camera to vertex
    let distance = length(world_pos - camera_position);

    // Calculate morph factor: 0 at MORPH_START, 1 at MORPH_END
    let morph_range = max(MORPH_END - MORPH_START, 0.001);
    let morph_factor = clamp((distance - MORPH_START) / morph_range, 0.0, 1.0);

    // Interpolate between actual height and morph height
    var morphed_position = vertex.position;
    morphed_position.y = mix(vertex.position.y, vertex.morph_height, morph_factor);

    // Transform normal to world space
    out.world_normal = mesh_functions::mesh_normal_local_to_world(
        vertex.normal,
        vertex.instance_index
    );

    // Transform morphed position to world space
    out.world_position = mesh_functions::mesh_position_local_to_world(
        mesh_world_from_local,
        vec4<f32>(morphed_position, 1.0)
    );
    out.position = position_world_to_clip(out.world_position.xyz);

    out.uv = vertex.uv;
    out.color = vertex.color;

#ifdef VERTEX_OUTPUT_INSTANCE_INDEX
    out.instance_index = vertex.instance_index;
#endif

#ifdef VISIBILITY_RANGE_DITHER
    out.visibility_range_dither = mesh_functions::get_visibility_range_dither_level(
        vertex.instance_index, mesh_world_from_local[3]);
#endif

    return out;
}
