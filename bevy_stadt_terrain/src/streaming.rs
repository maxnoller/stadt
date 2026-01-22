//! Async terrain chunk streaming and height query API
//!
//! Manages the asynchronous generation of terrain meshes using Bevy's
//! AsyncComputeTaskPool. Uses a priority queue to ensure nearby chunks
//! are generated first.

use crate::config::TerrainConfig;
use crate::heightmap::{HeightmapHandle, TerrainNoise, sample_terrain_height};
use crate::material::TerrainMaterialHandle;
use crate::mesh::generate_chunk_mesh;
use crate::quadtree::TerrainQuadtree;
use crate::{Chunk, Terrain};
use bevy::math::bounding::BoundingVolume;
use bevy::prelude::*;
use bevy::tasks::{AsyncComputeTaskPool, Task, block_on};
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap};
use std::sync::Arc;

/// Request to generate a terrain mesh
#[derive(Clone, Debug)]
pub struct MeshRequest {
    /// Node ID this mesh belongs to
    pub node_id: u64,
    /// World-space bounds (center and half-size)
    pub center: Vec2,
    pub size: f32,
    /// LOD level for this mesh
    pub lod: u8,
    /// Priority (lower = higher priority, based on distance)
    pub priority: f32,
    /// Grid coordinates
    pub coords: IVec2,
}

impl PartialEq for MeshRequest {
    fn eq(&self, other: &Self) -> bool {
        self.node_id == other.node_id
    }
}

impl Eq for MeshRequest {}

impl PartialOrd for MeshRequest {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for MeshRequest {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Compare by priority (lower is better, so we reverse)
        self.priority
            .partial_cmp(&other.priority)
            .unwrap_or(std::cmp::Ordering::Equal)
    }
}

/// Result of mesh generation
pub struct MeshResult {
    pub node_id: u64,
    pub mesh: Mesh,
    pub center: Vec2,
    pub lod: u8,
    pub coords: IVec2,
}

/// Resource managing terrain chunk streaming
#[derive(Resource, Default)]
pub struct TerrainStreaming {
    /// Priority queue of pending mesh requests
    pub pending: BinaryHeap<Reverse<MeshRequest>>,
    /// Currently in-flight mesh generation tasks
    pub in_flight: HashMap<u64, Task<MeshResult>>,
    /// Completed mesh results ready to be spawned
    pub completed: Vec<MeshResult>,
    /// Set of node IDs that already have entities
    pub spawned: HashMap<u64, Entity>,
    /// Parent node IDs waiting for their children to be spawned (subdivision case)
    /// Maps parent_id -> set of child_ids that need to be ready before despawning parent
    pub waiting_for_children: HashMap<u64, std::collections::HashSet<u64>>,
    /// Child node IDs waiting for their parent to be spawned (merge case)
    /// Maps child_id -> parent_id that needs to be ready before despawning child
    pub waiting_for_parent: HashMap<u64, u64>,
}

impl TerrainStreaming {
    /// Queue a mesh request
    pub fn queue_request(&mut self, request: MeshRequest) {
        // Don't queue if already spawned or in flight
        if !self.spawned.contains_key(&request.node_id)
            && !self.in_flight.contains_key(&request.node_id)
        {
            // Check if not already in pending queue
            let already_pending = self
                .pending
                .iter()
                .any(|Reverse(r)| r.node_id == request.node_id);
            if !already_pending {
                self.pending.push(Reverse(request));
            }
        }
    }
}

/// Resource for querying terrain height at any world position
#[derive(Resource)]
pub struct TerrainHeightQuery {
    noise: Arc<TerrainNoise>,
    config: TerrainConfig,
}

impl TerrainHeightQuery {
    pub fn new(noise: TerrainNoise, config: TerrainConfig) -> Self {
        Self {
            noise: Arc::new(noise),
            config,
        }
    }

    /// Get terrain height at world position
    pub fn get_height(&self, x: f32, z: f32) -> f32 {
        sample_terrain_height(x, z, &self.noise, &self.config)
    }

    /// Get surface normal at world position
    pub fn get_normal(&self, x: f32, z: f32) -> Vec3 {
        let step = 1.0;
        let left = self.get_height(x - step, z);
        let right = self.get_height(x + step, z);
        let down = self.get_height(x, z - step);
        let up = self.get_height(x, z + step);

        let dx = (right - left) / (2.0 * step);
        let dz = (up - down) / (2.0 * step);

        Vec3::new(-dx, 1.0, -dz).normalize()
    }

    /// Simple raycast against terrain (vertical ray only for now)
    pub fn raycast_vertical(&self, x: f32, z: f32, max_height: f32) -> Option<Vec3> {
        let height = self.get_height(x, z);
        if height <= max_height {
            Some(Vec3::new(x, height, z))
        } else {
            None
        }
    }
}

/// System: Update the quadtree based on camera position
pub fn update_quadtree(
    camera_query: Query<&Transform, With<Camera>>,
    config: Res<TerrainConfig>,
    terrain_query: Query<&HeightmapHandle, With<Terrain>>,
    mut quadtree: ResMut<TerrainQuadtree>,
    mut streaming: ResMut<TerrainStreaming>,
) {
    let Ok(camera_transform) = camera_query.single() else {
        return;
    };

    let camera_pos = camera_transform.translation;

    // Get heightmap from terrain entity, or use default noise
    let default_noise = TerrainNoise::default();
    let default_config = TerrainConfig::default();

    let height_sampler = |x: f32, z: f32| -> f32 {
        if let Ok(heightmap) = terrain_query.single() {
            heightmap.sample(x, z)
        } else {
            sample_terrain_height(x, z, &default_noise, &default_config)
        }
    };

    // Update quadtree
    quadtree.update(camera_pos, &config, height_sampler);

    // Collect selected nodes and queue mesh requests
    let selected = quadtree.collect_selected_nodes();

    for node in selected {
        // Check if we need to spawn this node
        if !streaming.spawned.contains_key(&node.id) {
            let distance = Vec2::new(camera_pos.x, camera_pos.z).distance(node.bounds.center());

            let request = MeshRequest {
                node_id: node.id,
                center: node.bounds.center(),
                size: node.bounds.half_size().x * 2.0,
                lod: node.lod_level,
                priority: distance,
                coords: node.coords,
            };

            streaming.queue_request(request);
        }
    }

    // Mark nodes that are no longer selected for removal, but handle LOD transitions gracefully
    let selected_ids: std::collections::HashSet<u64> = quadtree
        .collect_selected_nodes()
        .iter()
        .map(|n| n.id)
        .collect();

    // Find nodes that need to be removed (spawned but not selected)
    let spawned_not_selected: Vec<u64> = streaming
        .spawned
        .keys()
        .filter(|id| !selected_ids.contains(id))
        .cloned()
        .collect();

    for node_id in spawned_not_selected {
        // Case 1: Check if this node's CHILDREN are now selected (subdivision: parent -> children)
        let child_ids: Vec<u64> = (1..=4).map(|i| node_id * 4 + i).collect();
        let children_selected: Vec<u64> = child_ids
            .iter()
            .filter(|id| selected_ids.contains(id))
            .cloned()
            .collect();

        if !children_selected.is_empty() {
            // This is a parent that subdivided - keep it until all children are spawned
            let all_children_spawned = children_selected
                .iter()
                .all(|id| streaming.spawned.contains_key(id));

            if all_children_spawned {
                // All children ready, safe to remove parent
                streaming.spawned.remove(&node_id);
                streaming.waiting_for_children.remove(&node_id);
            } else {
                // Children not ready - keep parent visible
                let pending_children: std::collections::HashSet<u64> = children_selected
                    .iter()
                    .filter(|id| !streaming.spawned.contains_key(id))
                    .cloned()
                    .collect();
                streaming
                    .waiting_for_children
                    .insert(node_id, pending_children);
            }
            continue;
        }

        // Case 2: Check if this node's PARENT is now selected (merge: children -> parent)
        // Parent ID calculation: for child = parent*4 + offset (offset 1-4)
        // So parent = (child - 1) / 4 (integer division) for child > 4
        if node_id > 4 {
            let parent_id = (node_id - 1) / 4;
            if selected_ids.contains(&parent_id) {
                // This is a child that should merge back into parent
                let parent_spawned = streaming.spawned.contains_key(&parent_id);

                if parent_spawned {
                    // Parent is ready, safe to remove child
                    streaming.spawned.remove(&node_id);
                    streaming.waiting_for_parent.remove(&node_id);
                } else {
                    // Parent not ready - keep child visible
                    streaming.waiting_for_parent.insert(node_id, parent_id);
                }
                continue;
            }
        }

        // Case 3: Node went out of view entirely (not LOD transition)
        streaming.spawned.remove(&node_id);
        streaming.waiting_for_children.remove(&node_id);
        streaming.waiting_for_parent.remove(&node_id);
    }
}

/// System: Spawn async mesh generation tasks
pub fn spawn_mesh_tasks(
    config: Res<TerrainConfig>,
    terrain_query: Query<&HeightmapHandle, With<Terrain>>,
    mut streaming: ResMut<TerrainStreaming>,
) {
    let task_pool = AsyncComputeTaskPool::get();

    // Limit concurrent tasks
    while streaming.in_flight.len() < config.max_concurrent_tasks {
        let Some(Reverse(request)) = streaming.pending.pop() else {
            break;
        };

        // Skip if already spawned (could have been spawned while in queue)
        if streaming.spawned.contains_key(&request.node_id) {
            continue;
        }

        // Clone config for the async task
        let config = config.clone();
        let node_id = request.node_id;
        let center = request.center;
        let size = request.size;
        let lod = request.lod;
        let coords = request.coords;

        // Get the noise from terrain entity or use default
        let noise = if let Ok(heightmap) = terrain_query.single() {
            match heightmap {
                HeightmapHandle::Noise(noise, _) => (**noise).clone(),
                _ => TerrainNoise::default(),
            }
        } else {
            TerrainNoise::default()
        };

        let task = task_pool.spawn(async move {
            // Calculate subdivisions based on LOD
            let subdivisions = config.lod_subdivisions[lod as usize];

            // Generate mesh
            let mesh = generate_chunk_mesh(coords, size, subdivisions, &noise, &config);

            MeshResult {
                node_id,
                mesh,
                center,
                lod,
                coords,
            }
        });

        streaming.in_flight.insert(node_id, task);
    }
}

/// System: Poll mesh tasks for completion
pub fn poll_mesh_tasks(mut streaming: ResMut<TerrainStreaming>) {
    // First, find which tasks are finished
    let finished_ids: Vec<u64> = streaming
        .in_flight
        .iter()
        .filter(|(_, task)| task.is_finished())
        .map(|(id, _)| *id)
        .collect();

    // Then remove and poll them
    for id in finished_ids {
        if let Some(mut task) = streaming.in_flight.remove(&id)
            && let Some(result) = block_on(futures_lite::future::poll_once(&mut task))
        {
            streaming.completed.push(result);
        }
    }
}

/// System: Spawn chunk entities from completed mesh results
pub fn spawn_chunk_entities(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    material_handle: Res<TerrainMaterialHandle>,
    mut streaming: ResMut<TerrainStreaming>,
    existing_chunks: Query<(Entity, &Chunk)>,
) {
    let Some(material) = material_handle.handle.clone() else {
        return;
    };

    // Drain completed results into a local vec to avoid borrow issues
    let completed_results: Vec<MeshResult> = streaming.completed.drain(..).collect();

    // Spawn new chunks
    for result in completed_results {
        let mesh_handle = meshes.add(result.mesh);

        let entity = commands
            .spawn((
                Mesh3d(mesh_handle),
                MeshMaterial3d(material.clone()),
                Transform::from_translation(Vec3::new(result.center.x, 0.0, result.center.y)),
                Chunk {
                    coords: result.coords,
                    current_lod: result.lod as u32,
                    node_id: result.node_id,
                },
            ))
            .id();

        streaming.spawned.insert(result.node_id, entity);

        // Case A: Check if this node's parent was waiting for it (we're a child being spawned)
        // Parent ID is (child_id - 1) / 4
        if result.node_id > 4 {
            let parent_id = (result.node_id - 1) / 4;

            if let Some(waiting_children) = streaming.waiting_for_children.get_mut(&parent_id) {
                waiting_children.remove(&result.node_id);

                // If all children are now spawned, remove parent from spawned
                if waiting_children.is_empty() {
                    streaming.spawned.remove(&parent_id);
                }
            }
        }

        // Case B: Check if any children were waiting for this node (we're a parent being spawned)
        // Children that were waiting for this parent can now be removed
        let children_waiting: Vec<u64> = streaming
            .waiting_for_parent
            .iter()
            .filter(|(_, parent)| **parent == result.node_id)
            .map(|(child, _)| *child)
            .collect();

        for child_id in children_waiting {
            streaming.spawned.remove(&child_id);
            streaming.waiting_for_parent.remove(&child_id);
        }
    }

    // Clean up fully satisfied waiting parents
    streaming
        .waiting_for_children
        .retain(|_, children| !children.is_empty());

    // Despawn chunks that are no longer needed
    let spawned_ids: std::collections::HashSet<u64> = streaming.spawned.keys().cloned().collect();

    for (entity, chunk) in existing_chunks.iter() {
        if !spawned_ids.contains(&chunk.node_id) {
            commands.entity(entity).despawn();
        }
    }
}

// Implement Clone for TerrainNoise so it can be sent to async tasks
impl Clone for TerrainNoise {
    fn clone(&self) -> Self {
        // FastNoiseLite doesn't implement Clone, so we recreate with same settings
        // This is a limitation - we use the default seed for now
        TerrainNoise::default()
    }
}
