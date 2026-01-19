use super::mesh::generate_chunk_mesh;
use super::{Chunk, ChunkMap, TerrainConfig, TerrainNoise};
use bevy::prelude::*;

pub fn update_chunks(
    mut commands: Commands,
    camera_query: Query<&Transform, With<Camera>>,
    config: Res<TerrainConfig>,
    noise: Res<TerrainNoise>,
    mut chunk_map: ResMut<ChunkMap>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let Some(camera_transform) = camera_query.iter().next() else {
        return;
    };

    let camera_pos = camera_transform.translation;
    let chunk_x = (camera_pos.x / config.chunk_size).round() as i32;
    let chunk_z = (camera_pos.z / config.chunk_size).round() as i32;
    let center_chunk = IVec2::new(chunk_x, chunk_z);

    // Spawn needed chunks
    for z in -config.render_distance..=config.render_distance {
        for x in -config.render_distance..=config.render_distance {
            let offset = IVec2::new(x, z);
            let chunk_coords = center_chunk + offset;

            if let std::collections::hash_map::Entry::Vacant(e) =
                chunk_map.chunks.entry(chunk_coords)
            {
                // Calculate LOD based on distance from camera
                let chunk_center = Vec3::new(
                    chunk_coords.x as f32 * config.chunk_size,
                    0.0,
                    chunk_coords.y as f32 * config.chunk_size,
                );
                let distance = (chunk_center - camera_pos).length();
                let subdivisions = calculate_lod(distance);

                // Generate chunk mesh with new noise system
                let mesh = generate_chunk_mesh(
                    chunk_coords,
                    config.chunk_size,
                    subdivisions,
                    &noise,
                    &config,
                );

                let mesh_handle = meshes.add(mesh);
                let material_handle = materials.add(StandardMaterial {
                    base_color: Color::WHITE, // Vertex colors will modulate this
                    perceptual_roughness: 0.85,
                    metallic: 0.0,
                    reflectance: 0.25,
                    ..default()
                });

                let entity = commands
                    .spawn((
                        Mesh3d(mesh_handle),
                        MeshMaterial3d(material_handle),
                        Transform::from_translation(Vec3::new(
                            chunk_coords.x as f32 * config.chunk_size,
                            0.0,
                            chunk_coords.y as f32 * config.chunk_size,
                        )),
                        Chunk {
                            coords: chunk_coords,
                        },
                    ))
                    .id();

                e.insert(entity);
            }
        }
    }

    // Despawn far chunks
    let mut to_remove = Vec::new();
    for (&coords, &entity) in chunk_map.chunks.iter() {
        if (coords.x - center_chunk.x).abs() > config.render_distance + 1
            || (coords.y - center_chunk.y).abs() > config.render_distance + 1
        {
            commands.entity(entity).despawn();
            to_remove.push(coords);
        }
    }

    for coords in to_remove {
        chunk_map.chunks.remove(&coords);
    }
}

/// Calculate mesh subdivisions based on distance from camera (LOD)
fn calculate_lod(distance: f32) -> u32 {
    if distance < 500.0 {
        32 // High detail for nearby chunks (increased range)
    } else if distance < 1500.0 {
        20 // Medium detail
    } else if distance < 3000.0 {
        12 // Lower detail for mid-range
    } else {
        4 // Minimum detail for very distant chunks (reduced for performance)
    }
}
