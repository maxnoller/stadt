use bevy::prelude::*;
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
}

impl Default for TerrainConfig {
    fn default() -> Self {
        Self {
            chunk_size: 100.0,
            render_distance: 3,
            max_height: 20.0,
        }
    }
}

#[derive(Resource, Default)]
pub struct ChunkMap {
    pub chunks: HashMap<IVec2, Entity>,
}

#[derive(Resource)]
pub struct TerrainNoise {
    pub noise: fastnoise_lite::FastNoiseLite,
}

impl Default for TerrainNoise {
    fn default() -> Self {
        let mut noise = fastnoise_lite::FastNoiseLite::new();
        noise.set_noise_type(Some(fastnoise_lite::NoiseType::OpenSimplex2));
        noise.set_frequency(Some(0.01));
        Self { noise }
    }
}

#[derive(Component)]
pub struct Chunk {
    pub coords: IVec2,
}

mod mesh;
mod systems;

use systems::update_chunks;
