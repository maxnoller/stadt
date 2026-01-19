use bevy::asset::RenderAssetUsages;
use bevy::mesh::Indices;
use bevy::prelude::*;
use bevy::render::render_resource::PrimitiveTopology;

/// Generate terrain mesh with smooth normals and height-based vertex colors
pub fn generate_chunk_mesh(
    coords: IVec2,
    size: f32,
    subdivisions: u32,
    max_height: f32,
    noise: &fastnoise_lite::FastNoiseLite,
) -> Mesh {
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );

    let vertices_per_side = subdivisions + 1;
    let step = size / subdivisions as f32;
    let start_x = coords.x as f32 * size;
    let start_z = coords.y as f32 * size;

    // Generate height map for this chunk (with 1 extra on each side for normal calculation)
    let mut heights: Vec<Vec<f32>> = Vec::new();
    for z in 0..=subdivisions + 2 {
        let mut row = Vec::new();
        for x in 0..=subdivisions + 2 {
            let world_x = start_x + (x as f32 - 1.0) * step - size / 2.0;
            let world_z = start_z + (z as f32 - 1.0) * step - size / 2.0;
            let height = sample_terrain_height(world_x, world_z, noise, max_height);
            row.push(height);
        }
        heights.push(row);
    }

    // Generate vertices with smooth normals
    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut normals: Vec<[f32; 3]> = Vec::new();
    let mut colors: Vec<[f32; 4]> = Vec::new();
    let mut uvs: Vec<[f32; 2]> = Vec::new();

    for z in 0..vertices_per_side {
        for x in 0..vertices_per_side {
            let local_x = x as f32 * step - size / 2.0;
            let local_z = z as f32 * step - size / 2.0;
            let height = heights[(z + 1) as usize][(x + 1) as usize];

            positions.push([local_x, height, local_z]);

            // Calculate smooth normal from neighboring heights
            let normal =
                calculate_smooth_normal(&heights, (x + 1) as usize, (z + 1) as usize, step);
            normals.push(normal);

            // Height-based color for realistic biomes
            let color = height_to_color(height, max_height);
            colors.push(color);

            // UV coordinates
            uvs.push([
                x as f32 / subdivisions as f32,
                z as f32 / subdivisions as f32,
            ]);
        }
    }

    // Generate indices for triangles
    let mut indices: Vec<u32> = Vec::new();
    for z in 0..subdivisions {
        for x in 0..subdivisions {
            let top_left = z * vertices_per_side + x;
            let top_right = top_left + 1;
            let bottom_left = (z + 1) * vertices_per_side + x;
            let bottom_right = bottom_left + 1;

            // Triangle 1
            indices.push(top_left);
            indices.push(bottom_left);
            indices.push(top_right);

            // Triangle 2
            indices.push(top_right);
            indices.push(bottom_left);
            indices.push(bottom_right);
        }
    }

    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices));

    mesh
}

/// Sample terrain height using multi-octave noise (FBM) for more realistic terrain
fn sample_terrain_height(
    world_x: f32,
    world_z: f32,
    noise: &fastnoise_lite::FastNoiseLite,
    max_height: f32,
) -> f32 {
    // Base terrain noise
    let base = noise.get_noise_2d(world_x, world_z);

    // Add detail octaves for more natural variation
    let detail1 = noise.get_noise_2d(world_x * 2.0, world_z * 2.0) * 0.5;
    let detail2 = noise.get_noise_2d(world_x * 4.0, world_z * 4.0) * 0.25;

    let combined = base + detail1 + detail2;

    // Normalize and scale
    combined * max_height
}

/// Calculate smooth normal by averaging gradients from neighboring heights
fn calculate_smooth_normal(heights: &[Vec<f32>], x: usize, z: usize, step: f32) -> [f32; 3] {
    let left = heights[z][x.saturating_sub(1)];
    let right = heights[z][(x + 1).min(heights[z].len() - 1)];
    let down = heights[z.saturating_sub(1)][x];
    let up = heights[(z + 1).min(heights.len() - 1)][x];

    // Gradient in X and Z directions
    let dx = (right - left) / (2.0 * step);
    let dz = (up - down) / (2.0 * step);

    // Normal from cross product of tangent vectors
    let normal = Vec3::new(-dx, 1.0, -dz).normalize();
    normal.to_array()
}

/// Convert height to realistic biome color
fn height_to_color(height: f32, max_height: f32) -> [f32; 4] {
    // Define biome thresholds
    let water_level = -2.0;
    let beach_level = 0.5;
    let grass_level = max_height * 0.4;
    let rock_level = max_height * 0.7;

    // Colors (linear RGB)
    let deep_water = [0.02, 0.1, 0.3, 1.0];
    let shallow_water = [0.1, 0.3, 0.5, 1.0];
    let sand = [0.76, 0.7, 0.5, 1.0];
    let grass = [0.2, 0.5, 0.15, 1.0];
    let forest = [0.1, 0.35, 0.1, 1.0];
    let rock = [0.4, 0.38, 0.35, 1.0];
    let snow = [0.95, 0.95, 0.97, 1.0];

    if height < water_level {
        deep_water
    } else if height < 0.0 {
        lerp_color(
            shallow_water,
            sand,
            (height - water_level) / (0.0 - water_level),
        )
    } else if height < beach_level {
        lerp_color(sand, grass, height / beach_level)
    } else if height < grass_level {
        let t = (height - beach_level) / (grass_level - beach_level);
        lerp_color(grass, forest, t)
    } else if height < rock_level {
        let t = (height - grass_level) / (rock_level - grass_level);
        lerp_color(forest, rock, t)
    } else {
        let t = ((height - rock_level) / (max_height - rock_level)).min(1.0);
        lerp_color(rock, snow, t)
    }
}

/// Linear interpolation between two colors
fn lerp_color(a: [f32; 4], b: [f32; 4], t: f32) -> [f32; 4] {
    let t = t.clamp(0.0, 1.0);
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
        a[3] + (b[3] - a[3]) * t,
    ]
}
