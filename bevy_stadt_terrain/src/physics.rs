//! Physics integration for terrain using Rapier
//!
//! This module is only available when the `rapier` feature is enabled.
//! It provides automatic heightfield collider generation for terrain chunks.

use crate::config::TerrainConfig;
use crate::heightmap::{HeightmapHandle, TerrainNoise, sample_terrain_height};
use crate::{Chunk, Terrain};
use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

/// Marker component indicating a chunk has a physics collider
#[derive(Component)]
pub struct TerrainCollider;

/// System to spawn heightfield colliders for terrain chunks
pub fn spawn_terrain_colliders(
    mut commands: Commands,
    config: Res<TerrainConfig>,
    terrain_query: Query<&HeightmapHandle, With<Terrain>>,
    chunks_without_colliders: Query<(Entity, &Chunk, &Transform), Without<TerrainCollider>>,
) {
    // Get the heightmap source
    let default_noise = TerrainNoise::default();

    for (entity, chunk, transform) in chunks_without_colliders.iter() {
        // Calculate chunk bounds
        let chunk_size = config.chunk_size;
        let subdivisions = chunk.current_lod;

        // Sample heights for the heightfield collider
        let num_rows = subdivisions as usize + 1;
        let num_cols = subdivisions as usize + 1;
        let step = chunk_size / subdivisions as f32;

        let start_x = transform.translation.x - chunk_size / 2.0;
        let start_z = transform.translation.z - chunk_size / 2.0;

        let mut heights = Vec::with_capacity(num_rows * num_cols);

        for z in 0..num_rows {
            for x in 0..num_cols {
                let world_x = start_x + x as f32 * step;
                let world_z = start_z + z as f32 * step;

                let height = if let Ok(heightmap) = terrain_query.single() {
                    heightmap.sample(world_x, world_z)
                } else {
                    sample_terrain_height(world_x, world_z, &default_noise, &config)
                };

                heights.push(height);
            }
        }

        // Create the heightfield collider
        let collider = Collider::heightfield(
            heights,
            num_rows,
            num_cols,
            Vec3::new(chunk_size, 1.0, chunk_size),
        );

        commands.entity(entity).insert((
            collider,
            TerrainCollider,
            // Terrain is static
            RigidBody::Fixed,
            // Adjust collider position to match mesh
            ColliderMassProperties::Mass(0.0),
        ));
    }
}

/// System to update colliders when chunk LOD changes
pub fn update_terrain_colliders(
    mut commands: Commands,
    config: Res<TerrainConfig>,
    terrain_query: Query<&HeightmapHandle, With<Terrain>>,
    chunks_with_colliders: Query<(Entity, &Chunk, &Transform, &Collider), With<TerrainCollider>>,
) {
    let default_noise = TerrainNoise::default();

    for (entity, chunk, transform, _collider) in chunks_with_colliders.iter() {
        // Check if LOD changed (would need to track previous LOD)
        // For now, this is a placeholder for future LOD-aware collider updates
        let _ = (
            entity,
            chunk,
            transform,
            &default_noise,
            &config,
            &terrain_query,
        );
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_physics_module_exists() {
        // Just verify the module compiles
        assert!(true);
    }
}
