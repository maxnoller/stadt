use super::{TerrainConfig, TerrainNoise};
use bevy::asset::RenderAssetUsages;
use bevy::mesh::Indices;
use bevy::prelude::*;
use bevy::render::render_resource::PrimitiveTopology;

/// Generate terrain mesh with smooth normals and biome-based vertex colors
pub fn generate_chunk_mesh(
    coords: IVec2,
    size: f32,
    subdivisions: u32,
    noise: &TerrainNoise,
    config: &TerrainConfig,
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
            let height = sample_terrain_height(world_x, world_z, noise, config);
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

            // Biome color based on height, slope, and moisture
            let normal_vec = Vec3::from_array(normal);
            // Re-calculate world pos for moisture/noise sampling
            // (Precision match with height generation is important)
            let world_x = start_x + (x as f32) * step - size / 2.0;
            let world_z = start_z + (z as f32) * step - size / 2.0;

            let moisture = sample_moisture(world_x, world_z, noise);
            let detail_noise_val = sample_detail_noise(world_x, world_z, noise);
            let color = terrain_to_color(
                height,
                moisture,
                normal_vec,
                world_x,
                world_z,
                config,
                detail_noise_val,
            );
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

    // Add skirts to hide LOD cracks
    add_skirts(
        &mut positions,
        &mut normals,
        &mut colors,
        &mut uvs,
        &mut indices,
        vertices_per_side as usize,
        size,
    );

    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices));

    mesh
}

/// Helper to add skirts on chunk edges to hide LOD gaps
fn add_skirts(
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    colors: &mut Vec<[f32; 4]>,
    uvs: &mut Vec<[f32; 2]>,
    indices: &mut Vec<u32>,
    vertices_per_side: usize,
    _size: f32, // Unused but kept for API if needed later
) {
    let skirt_height = -50.0; // Deeper skirt for new height scale
    let start_vertex = positions.len() as u32;

    // Collect edge indices (top, right, bottom, left)
    let mut edge_indices: Vec<u32> = Vec::new();

    // Top edge (z=0)
    for x in 0..vertices_per_side {
        edge_indices.push(x as u32);
    }
    // Right edge (x=last)
    for z in 0..vertices_per_side {
        edge_indices.push((z * vertices_per_side + (vertices_per_side - 1)) as u32);
    }
    // Bottom edge (z=last)
    for x in (0..vertices_per_side).rev() {
        edge_indices.push(((vertices_per_side - 1) * vertices_per_side + x) as u32);
    }
    // Left edge (x=0)
    for z in (0..vertices_per_side).rev() {
        edge_indices.push((z * vertices_per_side) as u32);
    }

    // Generate skirt vertices
    for &idx in &edge_indices {
        let p = positions[idx as usize];
        let n = normals[idx as usize];
        let c = colors[idx as usize];
        let uv = uvs[idx as usize];

        positions.push([p[0], p[1] + skirt_height, p[2]]);
        normals.push(n);
        colors.push(c);
        uvs.push(uv);
    }

    // Generate skirt indices (quads)
    let skirt_vertex_count = edge_indices.len();
    for i in 0..skirt_vertex_count {
        let curr_orig = edge_indices[i];
        let next_orig = edge_indices[(i + 1) % skirt_vertex_count];

        let curr_skirt = start_vertex + i as u32;
        let next_skirt = start_vertex + ((i + 1) % skirt_vertex_count) as u32;

        // Quad 1
        indices.push(curr_orig);
        indices.push(next_orig);
        indices.push(curr_skirt);

        // Quad 2
        indices.push(next_orig);
        indices.push(next_skirt);
        indices.push(curr_skirt);
    }
}

/// Sample terrain height using multi-layer noise
pub fn sample_terrain_height(
    world_x: f32,
    world_z: f32,
    noise: &TerrainNoise,
    config: &TerrainConfig,
) -> f32 {
    let warp_x = noise.warp.get_noise_2d(world_x, world_z) * config.warp_strength;
    let warp_z = noise.warp.get_noise_2d(world_x + 1000.0, world_z + 1000.0) * config.warp_strength;
    let wx = world_x + warp_x;
    let wz = world_z + warp_z;

    // Continental: -1 to 1 range, normalized to 0-1
    let continental = (noise.continental.get_noise_2d(wx, wz) + 1.0) * 0.5;
    let erosion = (noise.erosion.get_noise_2d(wx, wz) + 1.0) * 0.5;

    // Ridges: Sharp features
    let ridge = noise.ridges.get_noise_2d(wx, wz);
    // Mask ridges to only appear on "high" areas of continental noise
    let mountain_mask = (continental - config.mountain_threshold * 0.5).max(0.0) * 2.5;
    let ridge_masked = ridge.max(0.0) * mountain_mask.powf(1.2);

    // Detail noise for surface roughness (reduced frequency and amplitude)
    let detail = noise.detail.get_noise_2d(wx, wz) * 0.02; // Reduced from 0.05 to 0.02

    // Combined: Continental sets baseline (land vs sea). Erosion adds hills. Ridges add peaks.
    // Weights tweaked for less spiky average terrain
    let combined = continental * 0.30 + erosion * 0.45 + ridge_masked * 0.25 + detail;

    let curved = apply_height_curve(combined);
    (curved * config.max_height) - config.water_level
}

fn sample_moisture(x: f32, z: f32, noise: &TerrainNoise) -> f32 {
    // Moisture map: 0 = dry (desert), 1 = wet (rainforest)
    let val = noise.moisture.get_noise_2d(x * 0.5, z * 0.5); // Low freq
    (val + 1.0) * 0.5
}

fn sample_detail_noise(x: f32, z: f32, noise: &TerrainNoise) -> f32 {
    noise.detail.get_noise_2d(x, z)
}

fn apply_height_curve(value: f32) -> f32 {
    let t = value.clamp(0.0, 1.0);
    // Smoother curve, less aggressive steps
    if t < 0.3 {
        t * 0.7 // Lowlands
    } else if t < 0.7 {
        0.21 + (t - 0.3) * 1.1 // Hills
    } else {
        0.65 + (t - 0.7) * 2.0 // Mountains (but less steep increase than before)
    }
}

fn calculate_smooth_normal(heights: &[Vec<f32>], x: usize, z: usize, step: f32) -> [f32; 3] {
    let left = heights[z][x.saturating_sub(1)];
    let right = heights[z][(x + 1).min(heights[z].len() - 1)];
    let down = heights[z.saturating_sub(1)][x];
    let up = heights[(z + 1).min(heights.len() - 1)][x];

    let dx = (right - left) / (2.0 * step);
    let dz = (up - down) / (2.0 * step);

    Vec3::new(-dx, 1.0, -dz).normalize().to_array()
}

/// Convert terrain properties to biome color
fn terrain_to_color(
    height: f32,
    moisture: f32,
    normal: Vec3,
    _x: f32,
    _z: f32,
    config: &TerrainConfig,
    detail_noise: f32, // Passed in from mesh generation loop
) -> [f32; 4] {
    let normalized_height =
        ((height + config.water_level) / (config.max_height + config.water_level)).clamp(0.0, 1.0);

    let slope = normal.y; // 1.0 = flat, 0.0 = vertical

    // --- Colors ---
    // Water
    let color_deep_water = [0.05, 0.15, 0.35, 1.0];
    let color_shallow_water = [0.15, 0.30, 0.50, 1.0];

    // Lowlands
    let color_sand = [0.82, 0.76, 0.58, 1.0];
    let color_grass_dry = [0.55, 0.60, 0.30, 1.0];
    let color_grass_lush = [0.22, 0.50, 0.12, 1.0];
    let color_forest_tropical = [0.08, 0.35, 0.08, 1.0];

    // Highlands
    let color_tundra = [0.50, 0.53, 0.40, 1.0];
    let color_forest_boreal = [0.12, 0.30, 0.18, 1.0];

    // Mountain
    let color_rock_dark = [0.25, 0.23, 0.21, 1.0];
    let color_rock_grey = [0.45, 0.45, 0.47, 1.0];
    let color_snow = [0.93, 0.93, 0.96, 1.0];

    // --- Texture Variation ---
    // Use coherent noise instead of random dither
    let variation = detail_noise * 0.08; // +/- 8% variation

    // --- Biome Selection ---

    // 1. Water
    if normalized_height < 0.1 {
        let t = normalized_height / 0.1;
        return lerp_color(color_deep_water, color_shallow_water, t + variation * 0.5);
    }

    // 2. Beach
    if normalized_height < 0.12 {
        return lerp_color(
            color_sand,
            color_grass_lush,
            ((normalized_height - 0.1) / 0.02) + variation,
        );
    }

    // 3. Slope Check (Rock)
    if slope < 0.7 {
        let rock_mix = lerp_color(
            color_rock_dark,
            color_rock_grey,
            normalized_height + variation * 2.0,
        );
        return rock_mix;
    }

    // 4. Land Biomes
    let base_color = if normalized_height < 0.4 {
        if moisture < 0.3 {
            color_sand
        } else if moisture < 0.6 {
            color_grass_dry
        } else if moisture < 0.8 {
            color_grass_lush
        } else {
            color_forest_tropical
        }
    } else if normalized_height < 0.75 {
        if moisture < 0.4 {
            color_rock_grey
        } else if moisture < 0.7 {
            color_tundra
        } else {
            color_forest_boreal
        }
    } else if normalized_height < 0.85 {
        color_rock_grey
    } else {
        color_snow
    };

    // Apply color variation
    [
        (base_color[0] + variation).clamp(0.0, 1.0),
        (base_color[1] + variation).clamp(0.0, 1.0),
        (base_color[2] + variation).clamp(0.0, 1.0),
        1.0,
    ]
}

fn lerp_color(a: [f32; 4], b: [f32; 4], t: f32) -> [f32; 4] {
    let t = t.clamp(0.0, 1.0);
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
        1.0,
    ]
}
