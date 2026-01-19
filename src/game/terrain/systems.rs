use super::material::TerrainMaterialExtension;
use super::mesh::generate_chunk_mesh;
use super::{
    Chunk, ChunkMap, LOD_HYSTERESIS, TerrainConfig, TerrainMaterial, TerrainMaterialHandle,
    TerrainNoise,
};
use bevy::pbr::{ExtendedMaterial, StandardMaterial};
use bevy::prelude::*;

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

pub fn update_chunks(
    mut commands: Commands,
    camera_query: Query<&Transform, With<Camera>>,
    config: Res<TerrainConfig>,
    noise: Res<TerrainNoise>,
    mut chunk_map: ResMut<ChunkMap>,
    mut meshes: ResMut<Assets<Mesh>>,
    terrain_material: Res<TerrainMaterialHandle>,
) {
    // Use single() for single camera - returns Result for graceful handling
    let Ok(camera_transform) = camera_query.single() else {
        return;
    };

    let Some(material_handle) = terrain_material.handle.clone() else {
        return; // Material not yet initialized
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
                let subdivisions = calculate_lod(distance, &config);

                // Generate chunk mesh
                let mesh = generate_chunk_mesh(
                    chunk_coords,
                    config.chunk_size,
                    subdivisions,
                    &noise,
                    &config,
                );

                let mesh_handle = meshes.add(mesh);

                let entity = commands
                    .spawn((
                        Mesh3d(mesh_handle),
                        MeshMaterial3d(material_handle.clone()),
                        Transform::from_translation(Vec3::new(
                            chunk_coords.x as f32 * config.chunk_size,
                            0.0,
                            chunk_coords.y as f32 * config.chunk_size,
                        )),
                        Chunk {
                            coords: chunk_coords,
                            current_lod: subdivisions,
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

/// Update LOD for existing chunks when camera moves significantly
pub fn update_chunk_lod(
    mut commands: Commands,
    camera_query: Query<&Transform, With<Camera>>,
    config: Res<TerrainConfig>,
    noise: Res<TerrainNoise>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut chunk_query: Query<(Entity, &Chunk, &Transform, &mut Mesh3d)>,
) {
    let Ok(camera_transform) = camera_query.single() else {
        return;
    };

    let camera_pos = camera_transform.translation;

    for (entity, chunk, transform, mut mesh3d) in chunk_query.iter_mut() {
        let distance = (transform.translation - camera_pos).length();
        let desired_lod = calculate_lod_with_hysteresis(distance, chunk.current_lod, &config);

        // Only regenerate if LOD changed
        if desired_lod != chunk.current_lod {
            // Regenerate mesh with new LOD
            let mesh = generate_chunk_mesh(
                chunk.coords,
                config.chunk_size,
                desired_lod,
                &noise,
                &config,
            );

            let new_mesh_handle = meshes.add(mesh);
            *mesh3d = Mesh3d(new_mesh_handle);

            // Update chunk's current LOD
            commands.entity(entity).insert(Chunk {
                coords: chunk.coords,
                current_lod: desired_lod,
            });
        }
    }
}

/// Calculate mesh subdivisions based on distance from camera (LOD)
fn calculate_lod(distance: f32, config: &TerrainConfig) -> u32 {
    if distance < config.lod_distances[0] {
        config.lod_subdivisions[0] // High detail for nearby chunks
    } else if distance < config.lod_distances[1] {
        config.lod_subdivisions[1] // Medium detail
    } else if distance < config.lod_distances[2] {
        config.lod_subdivisions[2] // Lower detail for mid-range
    } else {
        config.lod_subdivisions[3] // Minimum detail for very distant chunks
    }
}

/// Calculate LOD with hysteresis to prevent rapid switching at boundaries.
/// When moving away (to lower detail), requires crossing threshold + hysteresis buffer.
/// When moving closer (to higher detail), requires crossing threshold - hysteresis buffer.
fn calculate_lod_with_hysteresis(distance: f32, current_lod: u32, config: &TerrainConfig) -> u32 {
    let current_lod_index = config
        .lod_subdivisions
        .iter()
        .position(|&s| s == current_lod)
        .unwrap_or(0);

    // Check each threshold with hysteresis
    let thresholds = &config.lod_distances;
    let subdivisions = &config.lod_subdivisions;

    // Determine target LOD based on distance with hysteresis
    for (i, &threshold) in thresholds.iter().enumerate() {
        let buffer = threshold * LOD_HYSTERESIS;

        // If we're at a higher detail level (lower index), require moving further to drop detail
        // If we're at a lower detail level (higher index), require moving closer to increase detail
        let effective_threshold = if current_lod_index <= i {
            threshold + buffer // Moving away: need to go further
        } else {
            threshold - buffer // Moving closer: need to come closer
        };

        if distance < effective_threshold {
            return subdivisions[i];
        }
    }

    subdivisions[3] // Fallback to lowest detail
}
