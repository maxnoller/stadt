use bevy::prelude::*;
use fastnoise_lite::{FastNoiseLite, FractalType, NoiseType};
use std::collections::HashMap;

pub struct TerrainPlugin;

impl Plugin for TerrainPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TerrainConfig>()
            .init_resource::<ChunkMap>()
            .insert_resource(TerrainNoise::default())
            .add_systems(Update, update_chunks);
    }
}

#[derive(Resource)]
pub struct TerrainConfig {
    pub chunk_size: f32,
    pub render_distance: i32,
    pub max_height: f32,
    pub water_level: f32,
    pub mountain_threshold: f32,
    pub warp_strength: f32,
}

impl Default for TerrainConfig {
    fn default() -> Self {
        Self {
            chunk_size: 100.0,
            render_distance: 50, // Doubled from 25 to 50 for huge view distance
            max_height: 180.0,
            water_level: 15.0,
            mountain_threshold: 0.6,
            warp_strength: 60.0,
        }
    }
}

#[derive(Resource, Default)]
pub struct ChunkMap {
    pub chunks: HashMap<IVec2, Entity>,
}

/// Multi-layer noise system for realistic terrain generation
#[derive(Resource)]
pub struct TerrainNoise {
    /// Continental noise - large-scale landmass shapes
    pub continental: FastNoiseLite,
    /// Erosion noise - medium-scale rolling hills and valleys
    pub erosion: FastNoiseLite,
    /// Ridge noise - mountain ridges and sharp features
    pub ridges: FastNoiseLite,
    /// Domain warping noise - organic coordinate distortion
    pub warp: FastNoiseLite,
    /// Moisture noise - wetness/rainfall map for biomes
    pub moisture: FastNoiseLite,
    /// Detail noise - small-scale surface variation
    pub detail: FastNoiseLite,
}

impl Default for TerrainNoise {
    fn default() -> Self {
        // Continental noise - define large flat areas vs ocean/mountains
        let mut continental = FastNoiseLite::with_seed(42);
        continental.set_noise_type(Some(NoiseType::OpenSimplex2S));
        continental.set_frequency(Some(0.0004)); // Slightly lower for broader features
        continental.set_fractal_type(Some(FractalType::FBm));
        continental.set_fractal_octaves(Some(4));

        // Erosion noise - gentle rolling hills
        let mut erosion = FastNoiseLite::with_seed(123);
        erosion.set_noise_type(Some(NoiseType::OpenSimplex2S));
        erosion.set_frequency(Some(0.0015)); // Lower freq for smoother hills
        erosion.set_fractal_type(Some(FractalType::FBm));
        erosion.set_fractal_octaves(Some(4)); // Fewer octaves = less jagged
        erosion.set_fractal_lacunarity(Some(2.0));
        erosion.set_fractal_gain(Some(0.4)); // Lower gain = smoother

        // Ridge noise - distinct mountain ranges
        let mut ridges = FastNoiseLite::with_seed(456);
        ridges.set_noise_type(Some(NoiseType::OpenSimplex2S));
        ridges.set_frequency(Some(0.003)); // Lower frequency for larger mountains
        ridges.set_fractal_type(Some(FractalType::Ridged));
        ridges.set_fractal_octaves(Some(5));
        ridges.set_fractal_lacunarity(Some(2.0));
        ridges.set_fractal_gain(Some(0.4)); // Lower gain to reduce spikes

        // Domain warp noise
        let mut warp = FastNoiseLite::with_seed(789);
        warp.set_noise_type(Some(NoiseType::OpenSimplex2S));
        warp.set_frequency(Some(0.001));
        warp.set_fractal_type(Some(FractalType::FBm));
        warp.set_fractal_octaves(Some(3));

        // Moisture noise
        let mut moisture = FastNoiseLite::with_seed(999);
        moisture.set_noise_type(Some(NoiseType::OpenSimplex2S));
        moisture.set_frequency(Some(0.0005)); // Very broad climate zones
        moisture.set_fractal_type(Some(FractalType::FBm));
        moisture.set_fractal_octaves(Some(3));

        // Detail noise
        let mut detail = FastNoiseLite::with_seed(1011);
        detail.set_noise_type(Some(NoiseType::OpenSimplex2S));
        detail.set_frequency(Some(0.05)); // High freq for texture
        detail.set_fractal_type(Some(FractalType::FBm));
        detail.set_fractal_octaves(Some(2));

        Self {
            continental,
            erosion,
            ridges,
            warp,
            moisture,
            detail,
        }
    }
}

#[derive(Component)]
pub struct Chunk {
    pub coords: IVec2,
}

pub mod mesh;
mod systems;

use systems::update_chunks;
