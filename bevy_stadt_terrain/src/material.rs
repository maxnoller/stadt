//! Terrain material with vertex morphing and texture splatting support
//!
//! Extends Bevy's StandardMaterial with:
//! - Vertex morphing for smooth LOD transitions
//! - 4-layer texture splatting (optional)
//! - Auto-splatting based on height/slope

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

/// Shared material handle for all terrain chunks (reduces GPU memory)
#[derive(Resource, Default)]
pub struct TerrainMaterialHandle {
    pub handle: Option<Handle<TerrainMaterial>>,
}

/// Material extension that adds vertex morphing to StandardMaterial
/// Uses Bevy's view uniform for camera position and hardcoded morph distances in the shader
#[derive(Asset, AsBindGroup, Reflect, Debug, Clone, Default)]
pub struct TerrainMaterialExtension {
    // Future: Add splatmap and layer textures here
    // #[texture(100)]
    // #[sampler(101)]
    // pub splatmap: Option<Handle<Image>>,
    //
    // #[texture(102, dimension = "2d_array")]
    // #[sampler(103)]
    // pub layer_textures: Option<Handle<Image>>,
}

impl MaterialExtension for TerrainMaterialExtension {
    fn vertex_shader() -> ShaderRef {
        "shaders/terrain.wgsl".into()
    }

    // Use default fragment shader - vertex colors are handled by StandardMaterial

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

/// Initialize the shared terrain material once at startup
pub fn setup_terrain_material(
    mut materials: ResMut<Assets<TerrainMaterial>>,
    mut terrain_material: ResMut<TerrainMaterialHandle>,
) {
    terrain_material.handle = Some(materials.add(ExtendedMaterial {
        base: StandardMaterial {
            base_color: Color::WHITE, // Vertex colors will modulate this
            perceptual_roughness: 0.85,
            metallic: 0.0,
            reflectance: 0.25,
            ..default()
        },
        extension: TerrainMaterialExtension::default(),
    }));
}

/// Configuration for terrain texture layers (for future splatting support)
#[derive(Clone, Debug)]
pub struct TerrainLayer {
    /// Name of this layer
    pub name: String,
    /// Texture handle for this layer
    pub texture: Handle<Image>,
    /// Height range where this layer appears (normalized 0-1)
    pub height_range: std::ops::Range<f32>,
    /// Slope range where this layer appears (0 = flat, 1 = vertical)
    pub slope_range: std::ops::Range<f32>,
    /// Texture tiling scale
    pub tiling: f32,
}

/// Builder for configuring terrain layers
#[derive(Default, Clone)]
pub struct TerrainLayers {
    layers: Vec<TerrainLayer>,
}

impl TerrainLayers {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a texture layer
    pub fn add(
        mut self,
        name: impl Into<String>,
        texture: Handle<Image>,
        height_range: std::ops::Range<f32>,
        slope_range: std::ops::Range<f32>,
    ) -> Self {
        self.layers.push(TerrainLayer {
            name: name.into(),
            texture,
            height_range,
            slope_range,
            tiling: 1.0,
        });
        self
    }

    /// Add a texture layer with custom tiling
    pub fn add_with_tiling(
        mut self,
        name: impl Into<String>,
        texture: Handle<Image>,
        height_range: std::ops::Range<f32>,
        slope_range: std::ops::Range<f32>,
        tiling: f32,
    ) -> Self {
        self.layers.push(TerrainLayer {
            name: name.into(),
            texture,
            height_range,
            slope_range,
            tiling,
        });
        self
    }

    /// Get the layers
    pub fn layers(&self) -> &[TerrainLayer] {
        &self.layers
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_terrain_layers_builder() {
        // Can't test with actual textures, but verify the builder works
        let layers = TerrainLayers::new();
        assert!(layers.layers().is_empty());
    }
}
