#import bevy_pbr::{
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::alpha_discard,
    forward_io::VertexOutput,
    pbr_types::PbrInput,
}

#ifdef PREPASS_PIPELINE
#import bevy_pbr::{
    prepass_io::{VertexOutput, FragmentOutput},
    pbr_prepass_functions,
}
#else
#import bevy_pbr::{
    forward_io::{VertexOutput, FragmentOutput},
    pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing},
}
#endif

// Future splatmap support:
// @group(2) @binding(100) var splatmap_texture: texture_2d<f32>;
// @group(2) @binding(101) var splatmap_sampler: sampler;
// @group(2) @binding(102) var layer_textures: texture_2d_array<f32>;
// @group(2) @binding(103) var layer_sampler: sampler;

// Texture tiling scale for world-space UV
const TEXTURE_SCALE: f32 = 0.1;

@fragment
fn fragment(
    in: VertexOutput,
    @builtin(front_facing) is_front: bool,
) -> FragmentOutput {
    // Generate standard PBR input from the material
    var pbr_input = pbr_input_from_standard_material(in, is_front);

    // Use vertex color as the base color
    // The vertex colors are already calculated based on biome rules
    pbr_input.material.base_color = in.color;

    // Future texture splatting would go here:
    // let world_uv = in.world_position.xz * TEXTURE_SCALE;
    // let weights = textureSample(splatmap_texture, splatmap_sampler, in.uv);
    //
    // var color = vec3(0.0);
    // for (var i = 0u; i < 4u; i++) {
    //     let layer_color = textureSample(layer_textures, layer_sampler, world_uv, i);
    //     color += layer_color.rgb * weights[i];
    // }
    // pbr_input.material.base_color = vec4(color, 1.0);

    // Apply alpha discard if needed
    pbr_input.material.base_color = alpha_discard(pbr_input.material, pbr_input.material.base_color);

#ifdef PREPASS_PIPELINE
    // For prepass, we don't do full PBR lighting
    let out = pbr_prepass_functions::deferred_output(in, pbr_input);
#else
    // Apply PBR lighting
    var out: FragmentOutput;
    out.color = apply_pbr_lighting(pbr_input);
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
#endif

    return out;
}
