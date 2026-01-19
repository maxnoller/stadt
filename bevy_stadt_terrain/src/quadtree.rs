//! Quadtree-based LOD system for terrain chunks
//!
//! Implements CDLOD (Continuous Distance-Dependent Level of Detail) using a quadtree
//! structure. The quadtree is traversed each frame to determine which nodes need
//! to be rendered and at what LOD level.

use crate::config::TerrainConfig;
use bevy::math::bounding::{Aabb2d, BoundingVolume};
use bevy::prelude::*;
use std::collections::HashMap;

/// A node in the terrain quadtree
#[derive(Clone, Debug)]
pub struct QuadtreeNode {
    /// Unique identifier for this node
    pub id: u64,
    /// Axis-aligned bounding box in world coordinates (XZ plane)
    pub bounds: Aabb2d,
    /// LOD level (0 = highest detail, higher = lower detail)
    pub lod_level: u8,
    /// Depth in the quadtree (0 = root)
    pub depth: u8,
    /// Grid coordinates for this node
    pub coords: IVec2,
    /// Entity handle if this node has a spawned chunk
    pub entity: Option<Entity>,
    /// Whether this node is currently selected for rendering
    pub selected: bool,
    /// Children nodes (None if leaf node)
    pub children: Option<Box<[QuadtreeNode; 4]>>,
}

impl QuadtreeNode {
    /// Create a new quadtree node
    pub fn new(id: u64, bounds: Aabb2d, depth: u8, coords: IVec2) -> Self {
        Self {
            id,
            bounds,
            lod_level: depth,
            depth,
            coords,
            entity: None,
            selected: false,
            children: None,
        }
    }

    /// Get the center point of this node in world coordinates
    pub fn center(&self) -> Vec2 {
        self.bounds.center()
    }

    /// Get the size of this node (width = height since it's square)
    pub fn size(&self) -> f32 {
        self.bounds.half_size().x * 2.0
    }

    /// Check if this node is a leaf (has no children)
    pub fn is_leaf(&self) -> bool {
        self.children.is_none()
    }

    /// Subdivide this node into 4 children
    pub fn subdivide(&mut self, next_id: &mut u64) {
        if self.children.is_some() {
            return;
        }

        let center = self.center();
        let half = self.bounds.half_size();
        let quarter = half * 0.5;
        let new_depth = self.depth + 1;

        let mut create_child = |offset: Vec2, coords_offset: IVec2| {
            let child_center = center + offset * quarter;
            let child_bounds = Aabb2d::new(child_center, quarter);
            *next_id += 1;
            QuadtreeNode::new(
                *next_id,
                child_bounds,
                new_depth,
                self.coords * 2 + coords_offset,
            )
        };

        // Children are ordered: NW, NE, SW, SE (top-left, top-right, bottom-left, bottom-right)
        self.children = Some(Box::new([
            create_child(Vec2::new(-1.0, -1.0), IVec2::new(0, 0)), // NW
            create_child(Vec2::new(1.0, -1.0), IVec2::new(1, 0)),  // NE
            create_child(Vec2::new(-1.0, 1.0), IVec2::new(0, 1)),  // SW
            create_child(Vec2::new(1.0, 1.0), IVec2::new(1, 1)),   // SE
        ]));
    }

    /// Calculate the distance from camera to the closest point on this node's bounds
    /// Considers terrain height for more accurate 3D distance
    pub fn distance_to_camera(&self, camera_pos: Vec3, estimated_height: f32) -> f32 {
        let center = self.center();
        let half = self.bounds.half_size();

        // Find closest point on the 2D bounds to camera's XZ position
        let closest_x = camera_pos.x.clamp(center.x - half.x, center.x + half.x);
        let closest_z = camera_pos.z.clamp(center.y - half.y, center.y + half.y);

        // Use estimated terrain height at closest point for 3D distance
        let closest_point = Vec3::new(closest_x, estimated_height, closest_z);
        closest_point.distance(camera_pos)
    }

    /// Recursively select nodes for rendering based on camera distance
    pub fn select_for_rendering(
        &mut self,
        camera_pos: Vec3,
        config: &TerrainConfig,
        height_sampler: impl Fn(f32, f32) -> f32 + Copy,
        max_depth: u8,
    ) {
        // Reset selection
        self.selected = false;

        // Estimate height at node center for distance calculation
        let center = self.center();
        let estimated_height = height_sampler(center.x, center.y);
        let distance = self.distance_to_camera(camera_pos, estimated_height);

        // Determine if we should subdivide based on distance and current depth
        let should_subdivide = self.should_subdivide(distance, config, max_depth);

        if should_subdivide && self.depth < max_depth {
            // Ensure children exist
            if self.children.is_none() {
                let mut next_id = self.id * 4;
                self.subdivide(&mut next_id);
            }

            // Recursively select children
            if let Some(children) = &mut self.children {
                for child in children.iter_mut() {
                    child.select_for_rendering(camera_pos, config, height_sampler, max_depth);
                }
            }
        } else {
            // This node is selected for rendering
            self.selected = true;
            self.lod_level = self.calculate_lod(distance, config);
        }
    }

    /// Determine if this node should be subdivided based on distance
    fn should_subdivide(&self, distance: f32, config: &TerrainConfig, max_depth: u8) -> bool {
        if self.depth >= max_depth {
            return false;
        }

        // Use the LOD distances to determine subdivision
        // Closer nodes need more subdivision (higher detail)
        let lod_threshold = match self.depth {
            0 => config.lod_distances[2] * 2.0, // Very large nodes
            1 => config.lod_distances[2],
            2 => config.lod_distances[1],
            3 => config.lod_distances[0],
            _ => config.lod_distances[0] * 0.5,
        };

        distance < lod_threshold
    }

    /// Calculate the LOD level for this node based on distance
    fn calculate_lod(&self, distance: f32, config: &TerrainConfig) -> u8 {
        if distance < config.lod_distances[0] {
            0 // Highest detail
        } else if distance < config.lod_distances[1] {
            1
        } else if distance < config.lod_distances[2] {
            2
        } else {
            3 // Lowest detail
        }
    }

    /// Get the mesh subdivisions for this node's LOD level
    pub fn subdivisions(&self, config: &TerrainConfig) -> u32 {
        config.lod_subdivisions[self.lod_level as usize]
    }

    /// Collect all selected nodes into a vector
    pub fn collect_selected(&self, selected: &mut Vec<SelectedNode>) {
        if self.selected {
            selected.push(SelectedNode {
                id: self.id,
                bounds: self.bounds,
                lod_level: self.lod_level,
                coords: self.coords,
                entity: self.entity,
            });
        } else if let Some(children) = &self.children {
            for child in children.iter() {
                child.collect_selected(selected);
            }
        }
    }
}

/// A node that has been selected for rendering
#[derive(Clone, Debug)]
pub struct SelectedNode {
    pub id: u64,
    pub bounds: Aabb2d,
    pub lod_level: u8,
    pub coords: IVec2,
    pub entity: Option<Entity>,
}

/// The terrain quadtree resource that manages all terrain nodes
#[derive(Resource)]
pub struct TerrainQuadtree {
    /// Root nodes of the quadtree (grid of top-level nodes)
    pub roots: HashMap<IVec2, QuadtreeNode>,
    /// Maximum depth of the quadtree
    pub max_depth: u8,
    /// Size of each root node
    pub root_size: f32,
    /// Next available node ID
    next_id: u64,
}

impl Default for TerrainQuadtree {
    fn default() -> Self {
        Self {
            roots: HashMap::new(),
            max_depth: 4,
            root_size: 800.0, // 8x the default chunk size of 100
            next_id: 0,
        }
    }
}

impl TerrainQuadtree {
    /// Create a new quadtree with the given configuration
    pub fn new(max_depth: u8, root_size: f32) -> Self {
        Self {
            roots: HashMap::new(),
            max_depth,
            root_size,
            next_id: 0,
        }
    }

    /// Update the quadtree based on camera position
    pub fn update(
        &mut self,
        camera_pos: Vec3,
        config: &TerrainConfig,
        height_sampler: impl Fn(f32, f32) -> f32 + Copy,
    ) {
        // Determine which root nodes should exist based on render distance
        let root_x = (camera_pos.x / self.root_size).round() as i32;
        let root_z = (camera_pos.z / self.root_size).round() as i32;

        // Calculate how many root nodes we need based on render distance
        let roots_needed =
            (config.render_distance as f32 * config.chunk_size / self.root_size).ceil() as i32 + 1;

        // Create/update root nodes
        for z in -roots_needed..=roots_needed {
            for x in -roots_needed..=roots_needed {
                let coords = IVec2::new(root_x + x, root_z + z);
                let root = self.roots.entry(coords).or_insert_with(|| {
                    let center = Vec2::new(
                        coords.x as f32 * self.root_size,
                        coords.y as f32 * self.root_size,
                    );
                    let bounds = Aabb2d::new(center, Vec2::splat(self.root_size * 0.5));
                    self.next_id += 1;
                    QuadtreeNode::new(self.next_id, bounds, 0, coords)
                });

                root.select_for_rendering(camera_pos, config, height_sampler, self.max_depth);
            }
        }

        // Remove root nodes that are too far away
        let max_dist = roots_needed + 2;
        self.roots.retain(|coords, _| {
            (coords.x - root_x).abs() <= max_dist && (coords.y - root_z).abs() <= max_dist
        });
    }

    /// Collect all nodes that should be rendered
    pub fn collect_selected_nodes(&self) -> Vec<SelectedNode> {
        let mut selected = Vec::new();
        for root in self.roots.values() {
            root.collect_selected(&mut selected);
        }
        selected
    }

    /// Find a node by its ID
    pub fn find_node(&self, id: u64) -> Option<&QuadtreeNode> {
        for root in self.roots.values() {
            if let Some(node) = Self::find_in_node(root, id) {
                return Some(node);
            }
        }
        None
    }

    /// Find a node by its ID (mutable)
    pub fn find_node_mut(&mut self, id: u64) -> Option<&mut QuadtreeNode> {
        for root in self.roots.values_mut() {
            if let Some(node) = Self::find_in_node_mut(root, id) {
                return Some(node);
            }
        }
        None
    }

    fn find_in_node(node: &QuadtreeNode, id: u64) -> Option<&QuadtreeNode> {
        if node.id == id {
            return Some(node);
        }
        if let Some(children) = &node.children {
            for child in children.iter() {
                if let Some(found) = Self::find_in_node(child, id) {
                    return Some(found);
                }
            }
        }
        None
    }

    fn find_in_node_mut(node: &mut QuadtreeNode, id: u64) -> Option<&mut QuadtreeNode> {
        if node.id == id {
            return Some(node);
        }
        if let Some(children) = &mut node.children {
            for child in children.iter_mut() {
                if let Some(found) = Self::find_in_node_mut(child, id) {
                    return Some(found);
                }
            }
        }
        None
    }
}

/// Calculate LOD with hysteresis to prevent rapid switching at boundaries
pub fn calculate_lod_with_hysteresis(
    distance: f32,
    current_lod: u32,
    config: &TerrainConfig,
) -> u32 {
    let current_lod_index = config
        .lod_subdivisions
        .iter()
        .position(|&s| s == current_lod)
        .unwrap_or(0);

    let thresholds = &config.lod_distances;
    let subdivisions = &config.lod_subdivisions;

    for (i, &threshold) in thresholds.iter().enumerate() {
        let buffer = threshold * config.lod_hysteresis;

        // If we're at a higher detail level (lower index), require moving further to drop detail
        // If we're at a lower detail level (higher index), require moving closer to increase detail
        let effective_threshold = if current_lod_index <= i {
            threshold + buffer
        } else {
            threshold - buffer
        };

        if distance < effective_threshold {
            return subdivisions[i];
        }
    }

    subdivisions[3]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quadtree_node_creation() {
        let bounds = Aabb2d::new(Vec2::ZERO, Vec2::splat(100.0));
        let node = QuadtreeNode::new(1, bounds, 0, IVec2::ZERO);

        assert_eq!(node.id, 1);
        assert_eq!(node.depth, 0);
        assert!(node.is_leaf());
    }

    #[test]
    fn test_quadtree_subdivision() {
        let bounds = Aabb2d::new(Vec2::ZERO, Vec2::splat(100.0));
        let mut node = QuadtreeNode::new(1, bounds, 0, IVec2::ZERO);
        let mut next_id = 1;

        node.subdivide(&mut next_id);

        assert!(!node.is_leaf());
        assert!(node.children.is_some());

        if let Some(children) = &node.children {
            assert_eq!(children.len(), 4);
            for child in children.iter() {
                assert!(child.size() < node.size());
            }
        }
    }

    #[test]
    fn test_distance_calculation() {
        let bounds = Aabb2d::new(Vec2::new(100.0, 100.0), Vec2::splat(50.0));
        let node = QuadtreeNode::new(1, bounds, 0, IVec2::ZERO);

        let camera_pos = Vec3::new(0.0, 100.0, 0.0);
        let distance = node.distance_to_camera(camera_pos, 0.0);

        // Should be approximately sqrt((50)^2 + (100)^2 + (50)^2) for corner case
        assert!(distance > 0.0);
    }
}
