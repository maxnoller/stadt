use bevy::{
    mesh::{MeshVertexAttribute, MeshVertexBufferLayoutRef},
    pbr::{
        ExtendedMaterial, MaterialExtension, MaterialExtensionKey, MaterialExtensionPipeline,
        MeshPipelineKey, StandardMaterial,
    },
    prelude::*,
    render::render_resource::{AsBindGroup, SpecializedMeshPipelineError, VertexFormat},
    shader::ShaderRef,
};

/// Custom vertex attribute for morph height (what height this vertex would have at lower LOD)
pub const ATTRIBUTE_MORPH_HEIGHT: MeshVertexAttribute =
    MeshVertexAttribute::new("MorphHeight", 988540917, VertexFormat::Float32);

/// Type alias for the terrain material
pub type TerrainMaterial = ExtendedMaterial<StandardMaterial, TerrainMaterialExtension>;

/// Material extension that adds vertex morphing to StandardMaterial
/// Uses Bevy's view uniform for camera position and hardcoded morph distances in the shader
#[derive(Asset, AsBindGroup, Reflect, Debug, Clone, Default)]
pub struct TerrainMaterialExtension {}

impl MaterialExtension for TerrainMaterialExtension {
    fn vertex_shader() -> ShaderRef {
        "shaders/terrain.wgsl".into()
    }

    fn specialize(
        _pipeline: &MaterialExtensionPipeline,
        descriptor: &mut bevy::render::render_resource::RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayoutRef,
        key: MaterialExtensionKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        // Check if this is a prepass pipeline - if so, don't use our custom vertex layout
        let is_prepass = key.mesh_key.intersects(
            MeshPipelineKey::DEPTH_PREPASS
                | MeshPipelineKey::NORMAL_PREPASS
                | MeshPipelineKey::MOTION_VECTOR_PREPASS
                | MeshPipelineKey::DEFERRED_PREPASS,
        );

        if is_prepass {
            // For prepass, use standard vertex layout without morph_height
            // The prepass doesn't need morphing since we don't have a custom prepass shader
            return Ok(());
        }

        // Configure vertex buffer layout with our custom morph_height attribute for forward pass
        let vertex_layout = layout.0.get_layout(&[
            Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
            Mesh::ATTRIBUTE_NORMAL.at_shader_location(1),
            Mesh::ATTRIBUTE_UV_0.at_shader_location(2),
            Mesh::ATTRIBUTE_COLOR.at_shader_location(5),
            ATTRIBUTE_MORPH_HEIGHT.at_shader_location(17),
        ])?;

        descriptor.vertex.buffers = vec![vertex_layout];
        Ok(())
    }
}
