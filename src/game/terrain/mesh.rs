use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::PrimitiveTopology; // Try this path, or bevy::render::render_asset::RenderAssetUsages if public.
// If RenderAssetUsages is problematic, we can use Mesh::from(shape::Plane) but we want custom.
// Actually, check if RenderAssetUsages is in prelude? It usually isn't.

pub fn generate_chunk_mesh(
    coords: IVec2,
    size: f32,
    subdivisions: u32,
    max_height: f32,
    noise: &fastnoise_lite::FastNoiseLite,
) -> Mesh {
    // Try to find the correct RenderAssetUsages path or value.
    // If we can't import it, we might be stuck.
    // BUT usually RenderAssetUsages is re-exported.
    // Let's assume bevy::render::render_asset::RenderAssetUsages is correct but explicit pub needed?
    // Let's try `RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD`.
    // Actually, let's look for where `Mesh` is defined.

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );

    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut normals: Vec<[f32; 3]> = Vec::new();
    let mut uvs: Vec<[f32; 2]> = Vec::new();

    let step = size / subdivisions as f32;
    let start_x = coords.x as f32 * size;
    let start_z = coords.y as f32 * size;

    // Function to get height
    let get_height = |lx: f32, lz: f32| -> f32 {
        let world_x = start_x + lx;
        let world_z = start_z + lz;
        noise.get_noise_2d(world_x, world_z) * max_height
    };

    for z in 0..subdivisions {
        for x in 0..subdivisions {
            let x0 = x as f32 * step;
            let z0 = z as f32 * step;
            let x1 = (x + 1) as f32 * step;
            let z1 = (z + 1) as f32 * step;

            let y_00 = get_height(x0, z0);
            let y_10 = get_height(x1, z0);
            let y_01 = get_height(x0, z1);
            let y_11 = get_height(x1, z1);

            let v_00 = [x0 - size / 2.0, y_00, z0 - size / 2.0];
            let v_10 = [x1 - size / 2.0, y_10, z0 - size / 2.0];
            let v_01 = [x0 - size / 2.0, y_01, z1 - size / 2.0];
            let v_11 = [x1 - size / 2.0, y_11, z1 - size / 2.0];

            // Triangle 1: 00, 01, 10
            let normal1 = calculate_normal(v_00, v_01, v_10);
            positions.push(v_00);
            normals.push(normal1);
            uvs.push([0.0, 0.0]);
            positions.push(v_01);
            normals.push(normal1);
            uvs.push([0.0, 1.0]);
            positions.push(v_10);
            normals.push(normal1);
            uvs.push([1.0, 0.0]);

            // Triangle 2: 10, 01, 11
            let normal2 = calculate_normal(v_10, v_01, v_11);
            positions.push(v_10);
            normals.push(normal2);
            uvs.push([1.0, 0.0]);
            positions.push(v_01);
            normals.push(normal2);
            uvs.push([0.0, 1.0]);
            positions.push(v_11);
            normals.push(normal2);
            uvs.push([1.0, 1.0]);
        }
    }

    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);

    mesh
}

fn calculate_normal(p1: [f32; 3], p2: [f32; 3], p3: [f32; 3]) -> [f32; 3] {
    let v1 = Vec3::from(p1);
    let v2 = Vec3::from(p2);
    let v3 = Vec3::from(p3);

    let edge1 = v2 - v1;
    let edge2 = v3 - v1;

    edge1.cross(edge2).normalize_or_zero().to_array()
}
