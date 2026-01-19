//! Stadt terrain integration
//!
//! This module wraps bevy_stadt_terrain for Stadt-specific configuration.

use bevy::prelude::*;
pub use bevy_stadt_terrain::prelude::*;

pub struct TerrainPlugin;

impl Plugin for TerrainPlugin {
    fn build(&self, app: &mut App) {
        // Use the standalone terrain plugin with Stadt-specific defaults
        app.add_plugins(bevy_stadt_terrain::TerrainPlugin::default())
            .add_systems(Startup, spawn_terrain);
    }
}

/// Spawn the terrain entity with Stadt-style noise
fn spawn_terrain(mut commands: Commands) {
    let noise = bevy_stadt_terrain::heightmap::TerrainNoise::with_seed(42);
    let config = TerrainConfig::default();
    commands.spawn(TerrainBundle::noise(noise, &config));
}
