//! Heightmap abstraction for terrain generation
//!
//! Provides the `HeightmapSource` trait and implementations for:
//! - Procedural generation via closures
//! - Multi-layer noise (Stadt-style terrain)
//! - Image-based heightmaps (16-bit PNG)

use crate::config::TerrainConfig;
use bevy::prelude::*;
use fastnoise_lite::{FastNoiseLite, FractalType, NoiseType};
use std::sync::Arc;

/// Trait for height sampling at any world coordinate
pub trait HeightmapSource: Send + Sync + 'static {
    /// Sample the height at a given world position
    fn sample(&self, x: f32, z: f32) -> f32;

    /// Sample the surface normal at a given position
    fn sample_normal(&self, x: f32, z: f32, step: f32) -> Vec3 {
        let left = self.sample(x - step, z);
        let right = self.sample(x + step, z);
        let down = self.sample(x, z - step);
        let up = self.sample(x, z + step);

        let dx = (right - left) / (2.0 * step);
        let dz = (up - down) / (2.0 * step);

        Vec3::new(-dx, 1.0, -dz).normalize()
    }

    /// Sample the slope angle (0 = flat, 1 = vertical)
    fn sample_slope(&self, x: f32, z: f32, step: f32) -> f32 {
        let normal = self.sample_normal(x, z, step);
        1.0 - normal.y
    }
}

/// Component/Resource for storing the active heightmap
#[derive(Component)]
pub enum HeightmapHandle {
    /// Procedural heightmap using a closure or struct
    Procedural(Box<dyn HeightmapSource>),
    /// Multi-layer noise heightmap (Stadt-style)
    Noise(Box<TerrainNoise>, TerrainConfig),
    /// Image-based heightmap
    Image(Arc<ImageHeightmap>),
}

impl HeightmapHandle {
    pub fn sample(&self, x: f32, z: f32) -> f32 {
        match self {
            HeightmapHandle::Procedural(source) => source.sample(x, z),
            HeightmapHandle::Noise(noise, config) => {
                sample_terrain_height(x, z, noise.as_ref(), config)
            }
            HeightmapHandle::Image(img) => img.sample(x, z),
        }
    }

    pub fn sample_normal(&self, x: f32, z: f32, step: f32) -> Vec3 {
        match self {
            HeightmapHandle::Procedural(source) => source.sample_normal(x, z, step),
            HeightmapHandle::Noise(noise, config) => {
                // Use finite differences for noise-based terrain
                let left = sample_terrain_height(x - step, z, noise.as_ref(), config);
                let right = sample_terrain_height(x + step, z, noise.as_ref(), config);
                let down = sample_terrain_height(x, z - step, noise.as_ref(), config);
                let up = sample_terrain_height(x, z + step, noise.as_ref(), config);

                let dx = (right - left) / (2.0 * step);
                let dz = (up - down) / (2.0 * step);

                Vec3::new(-dx, 1.0, -dz).normalize()
            }
            HeightmapHandle::Image(img) => img.sample_normal(x, z, step),
        }
    }
}

/// Simple procedural heightmap using a closure
pub struct ProceduralHeightmap<F>
where
    F: Fn(f32, f32) -> f32 + Send + Sync + 'static,
{
    height_fn: F,
}

impl<F> ProceduralHeightmap<F>
where
    F: Fn(f32, f32) -> f32 + Send + Sync + 'static,
{
    pub fn new(height_fn: F) -> Self {
        Self { height_fn }
    }
}

impl<F> HeightmapSource for ProceduralHeightmap<F>
where
    F: Fn(f32, f32) -> f32 + Send + Sync + 'static,
{
    fn sample(&self, x: f32, z: f32) -> f32 {
        (self.height_fn)(x, z)
    }
}

/// Image-based heightmap from 16-bit PNG data
pub struct ImageHeightmap {
    /// Height data normalized to 0-1 range
    pub heights: Vec<f32>,
    /// Image width in pixels
    pub width: u32,
    /// Image height in pixels
    pub height: u32,
    /// World-space size of the heightmap
    pub world_size: Vec2,
    /// World-space origin offset
    pub origin: Vec2,
    /// Height scale multiplier
    pub height_scale: f32,
}

impl ImageHeightmap {
    pub fn new(
        heights: Vec<f32>,
        width: u32,
        height: u32,
        world_size: Vec2,
        height_scale: f32,
    ) -> Self {
        Self {
            heights,
            width,
            height,
            world_size,
            origin: Vec2::ZERO,
            height_scale,
        }
    }

    pub fn with_origin(mut self, origin: Vec2) -> Self {
        self.origin = origin;
        self
    }

    /// Sample with bilinear interpolation
    fn sample_bilinear(&self, u: f32, v: f32) -> f32 {
        let u = u.clamp(0.0, 1.0);
        let v = v.clamp(0.0, 1.0);

        let x = u * (self.width - 1) as f32;
        let y = v * (self.height - 1) as f32;

        let x0 = x.floor() as usize;
        let y0 = y.floor() as usize;
        let x1 = (x0 + 1).min(self.width as usize - 1);
        let y1 = (y0 + 1).min(self.height as usize - 1);

        let fx = x.fract();
        let fy = y.fract();

        let h00 = self.heights[y0 * self.width as usize + x0];
        let h10 = self.heights[y0 * self.width as usize + x1];
        let h01 = self.heights[y1 * self.width as usize + x0];
        let h11 = self.heights[y1 * self.width as usize + x1];

        let h0 = h00 * (1.0 - fx) + h10 * fx;
        let h1 = h01 * (1.0 - fx) + h11 * fx;

        h0 * (1.0 - fy) + h1 * fy
    }
}

impl HeightmapSource for ImageHeightmap {
    fn sample(&self, x: f32, z: f32) -> f32 {
        let u = (x - self.origin.x) / self.world_size.x;
        let v = (z - self.origin.y) / self.world_size.y;

        self.sample_bilinear(u, v) * self.height_scale
    }
}

/// Multi-layer noise system for realistic terrain generation (Stadt-style)
pub struct TerrainNoise {
    /// Continental noise - large-scale landmass shapes
    pub continental: FastNoiseLite,
    /// Erosion noise - medium-scale rolling hills and valleys
    pub erosion: FastNoiseLite,
    /// Ridge noise - mountain ridges and sharp features
    pub ridges: FastNoiseLite,
    /// Domain warping noise - organic coordinate distortion
    pub warp: FastNoiseLite,
    /// Moisture noise - wetness/rainfall map for biomes
    pub moisture: FastNoiseLite,
    /// Detail noise - small-scale surface variation
    pub detail: FastNoiseLite,
}

impl Default for TerrainNoise {
    fn default() -> Self {
        Self::with_seed(42)
    }
}

impl TerrainNoise {
    /// Create terrain noise with a specific seed
    pub fn with_seed(seed: i32) -> Self {
        // Continental noise - define large flat areas vs ocean/mountains
        let mut continental = FastNoiseLite::with_seed(seed);
        continental.set_noise_type(Some(NoiseType::OpenSimplex2S));
        continental.set_frequency(Some(0.0004));
        continental.set_fractal_type(Some(FractalType::FBm));
        continental.set_fractal_octaves(Some(4));

        // Erosion noise - gentle rolling hills
        let mut erosion = FastNoiseLite::with_seed(seed + 81);
        erosion.set_noise_type(Some(NoiseType::OpenSimplex2S));
        erosion.set_frequency(Some(0.0015));
        erosion.set_fractal_type(Some(FractalType::FBm));
        erosion.set_fractal_octaves(Some(4));
        erosion.set_fractal_lacunarity(Some(2.0));
        erosion.set_fractal_gain(Some(0.4));

        // Ridge noise - distinct mountain ranges
        let mut ridges = FastNoiseLite::with_seed(seed + 414);
        ridges.set_noise_type(Some(NoiseType::OpenSimplex2S));
        ridges.set_frequency(Some(0.003));
        ridges.set_fractal_type(Some(FractalType::Ridged));
        ridges.set_fractal_octaves(Some(5));
        ridges.set_fractal_lacunarity(Some(2.0));
        ridges.set_fractal_gain(Some(0.4));

        // Domain warp noise
        let mut warp = FastNoiseLite::with_seed(seed + 747);
        warp.set_noise_type(Some(NoiseType::OpenSimplex2S));
        warp.set_frequency(Some(0.001));
        warp.set_fractal_type(Some(FractalType::FBm));
        warp.set_fractal_octaves(Some(3));

        // Moisture noise
        let mut moisture = FastNoiseLite::with_seed(seed + 957);
        moisture.set_noise_type(Some(NoiseType::OpenSimplex2S));
        moisture.set_frequency(Some(0.0005));
        moisture.set_fractal_type(Some(FractalType::FBm));
        moisture.set_fractal_octaves(Some(3));

        // Detail noise
        let mut detail = FastNoiseLite::with_seed(seed + 969);
        detail.set_noise_type(Some(NoiseType::OpenSimplex2S));
        detail.set_frequency(Some(0.05));
        detail.set_fractal_type(Some(FractalType::FBm));
        detail.set_fractal_octaves(Some(2));

        Self {
            continental,
            erosion,
            ridges,
            warp,
            moisture,
            detail,
        }
    }

    /// Sample moisture at a world position (0 = dry, 1 = wet)
    pub fn sample_moisture(&self, x: f32, z: f32) -> f32 {
        let val = self.moisture.get_noise_2d(x * 0.5, z * 0.5);
        (val + 1.0) * 0.5
    }

    /// Sample detail noise at a world position
    pub fn sample_detail(&self, x: f32, z: f32) -> f32 {
        self.detail.get_noise_2d(x, z)
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
    let valley_factor = (1.0 - continental).powf(2.0);
    let valley_carve = erosion_raw.min(0.0).abs() * valley_factor * 0.15;

    // 2. Plateau effect: High continental areas get flattened tops
    let plateau_factor = (continental - 0.7).max(0.0) * 3.0;
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

/// Apply a multi-stage height curve for natural terrain
fn apply_height_curve(value: f32) -> f32 {
    let t = value.clamp(0.0, 1.0);
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
        let mountain_t = (t - 0.75) / 0.25;
        0.795 + mountain_t.powf(0.7) * 0.205
    }
}

/// Smooth interpolation (ease in/out)
fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_procedural_heightmap() {
        let heightmap = ProceduralHeightmap::new(|x, z| x + z);
        assert_eq!(heightmap.sample(1.0, 2.0), 3.0);
    }

    #[test]
    fn test_terrain_noise() {
        let noise = TerrainNoise::default();
        let config = TerrainConfig::default();

        // Should produce reasonable heights
        let height = sample_terrain_height(0.0, 0.0, &noise, &config);
        assert!(height > -config.water_level);
        assert!(height < config.max_height);
    }

    #[test]
    fn test_smoothstep() {
        assert_eq!(smoothstep(0.0, 1.0, 0.0), 0.0);
        assert_eq!(smoothstep(0.0, 1.0, 1.0), 1.0);
        assert!((smoothstep(0.0, 1.0, 0.5) - 0.5).abs() < 0.01);
    }
}
