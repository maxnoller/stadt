//! Terrain configuration and builder pattern

use bevy::prelude::*;

/// Main configuration for the terrain system
#[derive(Resource, Clone, Debug)]
pub struct TerrainConfig {
    /// Size of each terrain chunk in world units
    pub chunk_size: f32,
    /// Number of chunks to render in each direction from camera
    pub render_distance: i32,
    /// Maximum terrain height
    pub max_height: f32,
    /// Sea level height (terrain below this may be considered underwater)
    pub water_level: f32,
    /// Height threshold for mountain biome (0.0-1.0 normalized)
    pub mountain_threshold: f32,
    /// Domain warp strength for organic terrain shapes
    pub warp_strength: f32,
    /// Depth of skirts below chunk edges to hide LOD seams
    pub skirt_depth: f32,
    /// Distance thresholds for LOD transitions [near, mid, far]
    pub lod_distances: [f32; 3],
    /// Mesh subdivisions for each LOD level [highest, high, medium, low]
    pub lod_subdivisions: [u32; 4],
    /// Maximum number of concurrent mesh generation tasks
    pub max_concurrent_tasks: usize,
    /// Hysteresis buffer for LOD transitions (percentage of distance threshold)
    pub lod_hysteresis: f32,
    /// Maximum quadtree depth
    pub max_quadtree_depth: u8,
}

impl Default for TerrainConfig {
    fn default() -> Self {
        Self {
            chunk_size: 100.0,
            render_distance: 50,
            max_height: 180.0,
            water_level: 15.0,
            mountain_threshold: 0.6,
            warp_strength: 60.0,
            skirt_depth: 50.0,
            lod_distances: [300.0, 1000.0, 2500.0],
            lod_subdivisions: [64, 32, 16, 8],
            max_concurrent_tasks: 8,
            lod_hysteresis: 0.15,
            max_quadtree_depth: 8,
        }
    }
}

impl TerrainConfig {
    /// Create a new TerrainConfig with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a builder for customizing terrain configuration
    pub fn builder() -> TerrainConfigBuilder {
        TerrainConfigBuilder::default()
    }
}

/// Builder for creating customized TerrainConfig
#[derive(Default)]
pub struct TerrainConfigBuilder {
    config: TerrainConfig,
}

impl TerrainConfigBuilder {
    /// Set the chunk size in world units
    pub fn chunk_size(mut self, size: f32) -> Self {
        self.config.chunk_size = size;
        self
    }

    /// Set the render distance (chunks in each direction from camera)
    pub fn render_distance(mut self, distance: i32) -> Self {
        self.config.render_distance = distance;
        self
    }

    /// Set the maximum terrain height
    pub fn max_height(mut self, height: f32) -> Self {
        self.config.max_height = height;
        self
    }

    /// Set the water level
    pub fn water_level(mut self, level: f32) -> Self {
        self.config.water_level = level;
        self
    }

    /// Set the mountain threshold (0.0-1.0 normalized)
    pub fn mountain_threshold(mut self, threshold: f32) -> Self {
        self.config.mountain_threshold = threshold;
        self
    }

    /// Set the domain warp strength
    pub fn warp_strength(mut self, strength: f32) -> Self {
        self.config.warp_strength = strength;
        self
    }

    /// Set the skirt depth for hiding LOD seams
    pub fn skirt_depth(mut self, depth: f32) -> Self {
        self.config.skirt_depth = depth;
        self
    }

    /// Set the LOD distance thresholds [near, mid, far]
    pub fn lod_distances(mut self, distances: [f32; 3]) -> Self {
        self.config.lod_distances = distances;
        self
    }

    /// Set the LOD subdivisions [highest, high, medium, low]
    pub fn lod_subdivisions(mut self, subdivisions: [u32; 4]) -> Self {
        self.config.lod_subdivisions = subdivisions;
        self
    }

    /// Set the maximum number of concurrent mesh generation tasks
    pub fn max_concurrent_tasks(mut self, max: usize) -> Self {
        self.config.max_concurrent_tasks = max;
        self
    }

    /// Set the LOD hysteresis buffer (percentage of distance threshold)
    pub fn lod_hysteresis(mut self, hysteresis: f32) -> Self {
        self.config.lod_hysteresis = hysteresis;
        self
    }

    /// Set the maximum quadtree depth
    pub fn max_quadtree_depth(mut self, depth: u8) -> Self {
        self.config.max_quadtree_depth = depth;
        self
    }

    /// Build the TerrainConfig
    pub fn build(self) -> TerrainConfig {
        self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = TerrainConfig::default();
        assert_eq!(config.chunk_size, 100.0);
        assert_eq!(config.render_distance, 50);
    }

    #[test]
    fn test_builder() {
        let config = TerrainConfig::builder()
            .chunk_size(200.0)
            .render_distance(100)
            .max_height(500.0)
            .build();

        assert_eq!(config.chunk_size, 200.0);
        assert_eq!(config.render_distance, 100);
        assert_eq!(config.max_height, 500.0);
    }
}
