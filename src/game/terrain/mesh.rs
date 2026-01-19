use super::{ATTRIBUTE_MORPH_HEIGHT, TerrainConfig, TerrainNoise};
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

    // Generate vertices with smooth normals and morph heights
    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut normals: Vec<[f32; 3]> = Vec::new();
    let mut colors: Vec<[f32; 4]> = Vec::new();
    let mut uvs: Vec<[f32; 2]> = Vec::new();
    let mut morph_heights: Vec<f32> = Vec::new();

    for z in 0..vertices_per_side {
        for x in 0..vertices_per_side {
            let local_x = x as f32 * step - size / 2.0;
            let local_z = z as f32 * step - size / 2.0;
            let height = heights[(z + 1) as usize][(x + 1) as usize];

            positions.push([local_x, height, local_z]);

            // Calculate morph height for LOD transitions
            // Vertices at even positions exist at lower LOD, keep their actual height
            // Vertices at odd positions are removed at lower LOD, interpolate from corners
            let morph_height = calculate_morph_height(&heights, x, z);
            morph_heights.push(morph_height);

            // Calculate smooth normal from neighboring heights
            let normal =
                calculate_smooth_normal(&heights, (x + 1) as usize, (z + 1) as usize, step);
            normals.push(normal);

            // Biome color based on height, slope, and moisture
            let normal_vec = Vec3::from_array(normal);
            // World position for moisture/noise sampling
            // Must match actual vertex world position (start + local offset)
            let world_x = start_x + local_x;
            let world_z = start_z + local_z;

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
        &mut morph_heights,
        &mut indices,
        vertices_per_side as usize,
        config.skirt_depth,
    );

    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_attribute(ATTRIBUTE_MORPH_HEIGHT, morph_heights);
    mesh.insert_indices(Indices::U32(indices));

    mesh
}

/// Helper to add skirts on chunk edges to hide LOD gaps
#[allow(clippy::too_many_arguments)]
fn add_skirts(
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    colors: &mut Vec<[f32; 4]>,
    uvs: &mut Vec<[f32; 2]>,
    morph_heights: &mut Vec<f32>,
    indices: &mut Vec<u32>,
    vertices_per_side: usize,
    skirt_depth: f32,
) {
    let skirt_height = -skirt_depth;
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
        let mh = morph_heights[idx as usize];

        positions.push([p[0], p[1] + skirt_height, p[2]]);
        normals.push(n);
        colors.push(c);
        uvs.push(uv);
        // Skirt vertices morph to the same relative depth below their source vertex
        morph_heights.push(mh + skirt_height);
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

/// Sample terrain height using multi-layer noise with erosion approximation
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
    let erosion_raw = noise.erosion.get_noise_2d(wx, wz);
    let erosion = (erosion_raw + 1.0) * 0.5;

    // Ridges: Sharp features
    let ridge = noise.ridges.get_noise_2d(wx, wz);
    // Mask ridges to only appear on "high" areas of continental noise
    let mountain_mask = (continental - config.mountain_threshold * 0.5).max(0.0) * 2.5;
    let ridge_masked = ridge.max(0.0) * mountain_mask.powf(1.2);

    // Detail noise for surface roughness
    let detail = noise.detail.get_noise_2d(wx, wz) * 0.02;

    // --- Erosion approximation ---
    // 1. Valley carving: In low areas, use erosion noise to carve deeper channels
    //    This simulates water collecting and eroding valleys
    let valley_factor = (1.0 - continental).powf(2.0); // Stronger in lowlands
    let valley_carve = erosion_raw.min(0.0).abs() * valley_factor * 0.15;

    // 2. Plateau effect: High continental areas get flattened tops
    //    This simulates weathering and sediment settling on peaks
    let plateau_factor = (continental - 0.7).max(0.0) * 3.0; // Active above 0.7
    let plateau_smoothing = plateau_factor * (1.0 - erosion) * 0.1;

    // 3. Coastal shelves: Create gradual slopes near water level
    let coastal_factor =
        smoothstep(0.1, 0.25, continental) * (1.0 - smoothstep(0.25, 0.4, continental));
    let coastal_flatten = coastal_factor * 0.05;

    // Combined height with erosion effects
    let base_combined = continental * 0.30 + erosion * 0.45 + ridge_masked * 0.25 + detail;
    let combined =
        (base_combined - valley_carve + plateau_smoothing - coastal_flatten).clamp(0.0, 1.0);

    let curved = apply_height_curve(combined);
    (curved * config.max_height) - config.water_level
}

fn sample_moisture(x: f32, z: f32, noise: &TerrainNoise) -> f32 {
    // Moisture map: 0 = dry (desert), 1 = wet (rainforest)
    // Coordinates are scaled by 0.5 to create broader climate zones.
    // Combined with the base frequency of 0.0005, effective frequency is ~0.00025.
    // This produces large-scale biome regions (deserts, rainforests) spanning many chunks.
    let val = noise.moisture.get_noise_2d(x * 0.5, z * 0.5);
    (val + 1.0) * 0.5
}

fn sample_detail_noise(x: f32, z: f32, noise: &TerrainNoise) -> f32 {
    noise.detail.get_noise_2d(x, z)
}

fn apply_height_curve(value: f32) -> f32 {
    let t = value.clamp(0.0, 1.0);
    // Multi-stage curve for natural terrain:
    // - Deep ocean (0.0-0.15): Gentle slope
    // - Continental shelf (0.15-0.25): Steeper transition to land
    // - Coastal lowlands (0.25-0.40): Flat plains
    // - Rolling hills (0.40-0.60): Gradual rise
    // - Highlands (0.60-0.75): Steeper foothills
    // - Mountains (0.75-1.0): Dramatic peaks
    if t < 0.15 {
        // Deep ocean floor
        t * 0.5
    } else if t < 0.25 {
        // Continental shelf rise
        0.075 + (t - 0.15) * 1.5
    } else if t < 0.40 {
        // Coastal lowlands (flatter)
        0.225 + (t - 0.25) * 0.8
    } else if t < 0.60 {
        // Rolling hills
        0.345 + (t - 0.40) * 1.2
    } else if t < 0.75 {
        // Highland foothills
        0.585 + (t - 0.60) * 1.4
    } else {
        // Mountain peaks (exponential rise for drama)
        let mountain_t = (t - 0.75) / 0.25; // 0 to 1 in mountain range
        0.795 + mountain_t.powf(0.7) * 0.205
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

/// Calculate the morph height for a vertex for smooth LOD transitions.
///
/// For CDLOD geomorphing:
/// - Vertices at even grid positions (exist at lower LOD): morph_height = actual height
/// - Vertices at odd positions (removed at lower LOD): bilinear interpolate from 4 corners
///
/// The `heights` array has a 1-cell border for normal calculation, so actual vertex
/// data is at indices [1..subdivisions+2]. The x,z parameters are grid positions (0..subdivisions).
fn calculate_morph_height(heights: &[Vec<f32>], x: u32, z: u32) -> f32 {
    // Convert to usize with offset for the heights array border
    let hx = (x + 1) as usize;
    let hz = (z + 1) as usize;
    let actual_height = heights[hz][hx];

    // Check if this vertex would exist at the next lower LOD (half resolution)
    // At lower LOD, only vertices at even positions remain
    let x_even = x.is_multiple_of(2);
    let z_even = z.is_multiple_of(2);

    if x_even && z_even {
        // This vertex exists at lower LOD - no morphing needed
        actual_height
    } else if x_even {
        // Vertex is on an even column but odd row - interpolate between z neighbors
        let z_prev = hz.saturating_sub(1);
        let z_next = (hz + 1).min(heights.len() - 1);
        (heights[z_prev][hx] + heights[z_next][hx]) * 0.5
    } else if z_even {
        // Vertex is on an odd column but even row - interpolate between x neighbors
        let x_prev = hx.saturating_sub(1);
        let x_next = (hx + 1).min(heights[hz].len() - 1);
        (heights[hz][x_prev] + heights[hz][x_next]) * 0.5
    } else {
        // Vertex is at odd column AND odd row - bilinear interpolate from 4 corners
        let x_prev = hx.saturating_sub(1);
        let x_next = (hx + 1).min(heights[hz].len() - 1);
        let z_prev = hz.saturating_sub(1);
        let z_next = (hz + 1).min(heights.len() - 1);

        let h00 = heights[z_prev][x_prev];
        let h10 = heights[z_prev][x_next];
        let h01 = heights[z_next][x_prev];
        let h11 = heights[z_next][x_next];

        (h00 + h10 + h01 + h11) * 0.25
    }
}

/// Convert terrain properties to biome color with smooth blending
fn terrain_to_color(
    height: f32,
    moisture: f32,
    normal: Vec3,
    _x: f32,
    _z: f32,
    config: &TerrainConfig,
    detail_noise: f32,
) -> [f32; 4] {
    let normalized_height =
        ((height + config.water_level) / (config.max_height + config.water_level)).clamp(0.0, 1.0);

    let slope = normal.y; // 1.0 = flat, 0.0 = vertical

    // --- Colors ---
    let color_deep_water = [0.05, 0.15, 0.35, 1.0];
    let color_shallow_water = [0.15, 0.30, 0.50, 1.0];
    let color_sand = [0.82, 0.76, 0.58, 1.0];
    let color_grass_dry = [0.55, 0.60, 0.30, 1.0];
    let color_grass_lush = [0.22, 0.50, 0.12, 1.0];
    let color_forest_tropical = [0.08, 0.35, 0.08, 1.0];
    let color_tundra = [0.50, 0.53, 0.40, 1.0];
    let color_forest_boreal = [0.12, 0.30, 0.18, 1.0];
    let color_rock_dark = [0.25, 0.23, 0.21, 1.0];
    let color_rock_grey = [0.45, 0.45, 0.47, 1.0];
    let color_snow = [0.93, 0.93, 0.96, 1.0];

    // Texture variation from detail noise
    let variation = detail_noise * 0.06;

    // --- Smooth blending with gradients ---

    // Water gradient (deep -> shallow)
    let water_color = lerp_color(
        color_deep_water,
        color_shallow_water,
        smoothstep(0.0, 0.1, normalized_height),
    );

    // Shore transition (water -> land)
    let shore_blend = smoothstep(0.08, 0.14, normalized_height);

    // Lowland biome based on moisture (smooth transitions)
    let lowland_color = {
        let dry_to_moderate = smoothstep(0.2, 0.4, moisture);
        let moderate_to_lush = smoothstep(0.5, 0.7, moisture);
        let lush_to_forest = smoothstep(0.75, 0.9, moisture);

        let c1 = lerp_color(color_sand, color_grass_dry, dry_to_moderate);
        let c2 = lerp_color(c1, color_grass_lush, moderate_to_lush);
        lerp_color(c2, color_forest_tropical, lush_to_forest)
    };

    // Highland biome based on moisture
    let highland_color = {
        let dry_to_tundra = smoothstep(0.3, 0.5, moisture);
        let tundra_to_boreal = smoothstep(0.6, 0.8, moisture);

        let c1 = lerp_color(color_rock_grey, color_tundra, dry_to_tundra);
        lerp_color(c1, color_forest_boreal, tundra_to_boreal)
    };

    // Mountain/snow gradient
    let mountain_color = lerp_color(
        color_rock_grey,
        color_snow,
        smoothstep(0.75, 0.90, normalized_height),
    );

    // Blend lowland -> highland -> mountain based on height
    let lowland_to_highland = smoothstep(0.30, 0.50, normalized_height);
    let highland_to_mountain = smoothstep(0.60, 0.80, normalized_height);

    let land_color = {
        let c1 = lerp_color(lowland_color, highland_color, lowland_to_highland);
        lerp_color(c1, mountain_color, highland_to_mountain)
    };

    // Blend water -> land
    let base_color = lerp_color(water_color, land_color, shore_blend);

    // Steep slope -> rock (smooth blend)
    let rock_blend = smoothstep(0.75, 0.60, slope); // Note: inverted range for steep
    let rock_color = lerp_color(color_rock_dark, color_rock_grey, normalized_height);
    let final_color = lerp_color(base_color, rock_color, rock_blend);

    // Apply subtle variation
    [
        (final_color[0] + variation).clamp(0.0, 1.0),
        (final_color[1] + variation).clamp(0.0, 1.0),
        (final_color[2] + variation).clamp(0.0, 1.0),
        1.0,
    ]
}

/// Smooth interpolation (ease in/out)
fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
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
