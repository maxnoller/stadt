//! # bevy_stadt_terrain
//!
//! A standalone Bevy terrain plugin featuring:
//! - CDLOD quadtree-based level of detail
//! - Async chunk streaming with priority queue
//! - HeightmapSource abstraction for procedural/image-based terrain
//! - Texture splatting with automatic slope/height-based layer blending
//! - Height query API for gameplay systems
//! - Optional Rapier physics integration (feature-gated)

use bevy::prelude::*;

pub mod config;
pub mod heightmap;
pub mod material;
pub mod mesh;
#[cfg(feature = "rapier")]
pub mod physics;
pub mod quadtree;
pub mod streaming;

pub mod prelude {
    pub use crate::config::{TerrainConfig, TerrainConfigBuilder};
    pub use crate::heightmap::{HeightmapSource, ImageHeightmap, ProceduralHeightmap};
    pub use crate::material::{TerrainLayers, TerrainMaterial, TerrainMaterialExtension};
    pub use crate::quadtree::{QuadtreeNode, TerrainQuadtree};
    pub use crate::streaming::TerrainHeightQuery;
    pub use crate::{TerrainBundle, TerrainPlugin};

    #[cfg(feature = "rapier")]
    pub use crate::physics::TerrainCollider;
}

/// Main terrain plugin that sets up all terrain systems
#[derive(Default)]
pub struct TerrainPlugin {
    /// Configuration for terrain generation
    pub config: config::TerrainConfig,
}

impl TerrainPlugin {
    /// Create a new terrain plugin with the given configuration
    pub fn new(config: config::TerrainConfig) -> Self {
        Self { config }
    }

    /// Create a terrain plugin using a builder pattern
    pub fn builder() -> TerrainPluginBuilder {
        TerrainPluginBuilder::default()
    }
}

impl Plugin for TerrainPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(bevy::pbr::MaterialPlugin::<material::TerrainMaterial>::default())
            .insert_resource(self.config.clone())
            .init_resource::<quadtree::TerrainQuadtree>()
            .init_resource::<streaming::TerrainStreaming>()
            .init_resource::<material::TerrainMaterialHandle>()
            .add_systems(Startup, material::setup_terrain_material)
            .add_systems(
                Update,
                (
                    streaming::update_quadtree,
                    streaming::spawn_mesh_tasks,
                    streaming::poll_mesh_tasks,
                    streaming::spawn_chunk_entities,
                )
                    .chain(),
            );

        #[cfg(feature = "rapier")]
        {
            app.add_systems(Update, physics::spawn_terrain_colliders);
        }
    }
}

/// Builder for constructing a TerrainPlugin with custom settings
#[derive(Default)]
pub struct TerrainPluginBuilder {
    config: config::TerrainConfig,
}

impl TerrainPluginBuilder {
    pub fn chunk_size(mut self, size: f32) -> Self {
        self.config.chunk_size = size;
        self
    }

    pub fn render_distance(mut self, distance: i32) -> Self {
        self.config.render_distance = distance;
        self
    }

    pub fn max_height(mut self, height: f32) -> Self {
        self.config.max_height = height;
        self
    }

    pub fn lod_distances(mut self, distances: [f32; 3]) -> Self {
        self.config.lod_distances = distances;
        self
    }

    pub fn lod_subdivisions(mut self, subdivisions: [u32; 4]) -> Self {
        self.config.lod_subdivisions = subdivisions;
        self
    }

    pub fn build(self) -> TerrainPlugin {
        TerrainPlugin::new(self.config)
    }
}

/// Marker component for terrain entities
#[derive(Component)]
pub struct Terrain;

/// Component storing chunk metadata
#[derive(Component)]
pub struct Chunk {
    /// Grid coordinates of this chunk
    pub coords: IVec2,
    /// Current LOD level (subdivisions) for this chunk
    pub current_lod: u32,
    /// Quadtree node ID this chunk belongs to
    pub node_id: u64,
}

/// Bundle for spawning a terrain entity
#[derive(Bundle)]
pub struct TerrainBundle {
    pub terrain: Terrain,
    pub heightmap: heightmap::HeightmapHandle,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    pub visibility: Visibility,
    pub inherited_visibility: InheritedVisibility,
    pub view_visibility: ViewVisibility,
}

impl TerrainBundle {
    /// Create terrain with a procedural heightmap using a closure
    pub fn procedural<F>(height_fn: F) -> Self
    where
        F: Fn(f32, f32) -> f32 + Send + Sync + 'static,
    {
        Self {
            terrain: Terrain,
            heightmap: heightmap::HeightmapHandle::Procedural(Box::new(
                heightmap::ProceduralHeightmap::new(height_fn),
            )),
            transform: Transform::default(),
            global_transform: GlobalTransform::default(),
            visibility: Visibility::default(),
            inherited_visibility: InheritedVisibility::default(),
            view_visibility: ViewVisibility::default(),
        }
    }

    /// Create terrain with a multi-layer noise heightmap (Stadt-style)
    pub fn noise(noise: heightmap::TerrainNoise, config: &config::TerrainConfig) -> Self {
        Self {
            terrain: Terrain,
            heightmap: heightmap::HeightmapHandle::Noise(Box::new(noise), config.clone()),
            transform: Transform::default(),
            global_transform: GlobalTransform::default(),
            visibility: Visibility::default(),
            inherited_visibility: InheritedVisibility::default(),
            view_visibility: ViewVisibility::default(),
        }
    }
}
