use bevy::prelude::*;
use bevy_stadt_terrain::Chunk;
use bevy_stadt_terrain::heightmap::{TerrainNoise, sample_terrain_height};
use bevy_stadt_terrain::prelude::*;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

pub struct VillagePlugin;

impl Plugin for VillagePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, spawn_villages_on_new_chunks);
    }
}

#[derive(Component)]
pub struct Village;

fn spawn_villages_on_new_chunks(
    mut commands: Commands,
    chunk_query: Query<(Entity, &Chunk, &Transform), Added<Chunk>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    config: Res<TerrainConfig>,
) {
    let village_probability = 0.3; // 30% chance per chunk

    // Use default noise for height sampling (same as terrain generation)
    let noise = TerrainNoise::with_seed(42);

    for (chunk_entity, chunk, chunk_transform) in chunk_query.iter() {
        // Deterministic RNG based on chunk coordinates
        let seed = (chunk.coords.x as u64).wrapping_mul(73856093)
            ^ (chunk.coords.y as u64).wrapping_mul(19349663);
        let mut rng = StdRng::seed_from_u64(seed);

        if rng.random_bool(village_probability) {
            // Pick a random position within the chunk
            let local_x = rng.random_range(-config.chunk_size / 2.0..config.chunk_size / 2.0);
            let local_z = rng.random_range(-config.chunk_size / 2.0..config.chunk_size / 2.0);

            // Get height at this position
            let world_x = chunk_transform.translation.x + local_x;
            let world_z = chunk_transform.translation.z + local_z;

            // Use same height calculation as terrain mesh
            let y = sample_terrain_height(world_x, world_z, &noise, &config);

            // Only spawn above water
            if y < config.water_level + 1.0 {
                continue;
            }

            // Spawn Village
            let mesh_handle = meshes.add(Cuboid::new(5.0, 5.0, 5.0));
            let material_handle = materials.add(StandardMaterial {
                base_color: Color::srgb(0.8, 0.2, 0.2), // Red village
                ..default()
            });

            let village = commands
                .spawn((
                    Mesh3d(mesh_handle),
                    MeshMaterial3d(material_handle),
                    Transform::from_xyz(world_x, y + 2.5, world_z), // +2.5 to sit on top (half height)
                    Village,
                ))
                .id();

            // Optional: Parent to chunk?
            // If we parent to chunk, it despawns with chunk automatically.
            commands.entity(chunk_entity).add_child(village);
        }
    }
}
