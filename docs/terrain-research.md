# Terrain Generation Research Report

> Research on procedural terrain techniques for improving Stadt's terrain system.

## Executive Summary

This report analyzes industry-standard terrain generation and rendering techniques, comparing them to our current implementation. We identify 6 key areas for improvement, prioritized by effort and impact.

---

## Current Implementation

| Feature | Status |
|---------|--------|
| Multi-layer noise (continental, erosion, ridges) | ✅ |
| Distance-based chunk LOD | ✅ |
| Biome coloring with smooth blending | ✅ |
| Mesh skirts for LOD seams | ✅ |
| Atmospheric fog (DistanceFog) | ✅ |
| LOD hysteresis (15% buffer) | ✅ |
| Erosion approximation (valleys, plateaus, coastal shelves) | ✅ |
| Multi-stage height curve | ✅ |

---

## Industry Techniques Analysis

### 1. Vertex Morphing (Geomorphing)

**Problem:** LOD transitions cause visible "popping" as meshes change resolution.

**Solution:** Interpolate vertex heights over distance, typically in the vertex shader:

```wgsl
let morph_factor = saturate((distance - lod_near) / (lod_far - lod_near));
let height = mix(high_lod_height, low_lod_height, morph_factor);
```

**Status:** Not implemented. Requires custom material with vertex shader.

---

### 2. Quadtree LOD Structure

**Current:** Flat grid with uniform LOD thresholds per chunk.

**Industry standard:** Hierarchical quadtree where nodes subdivide based on camera distance and screen-space error.

**Benefits:**
- Optimal triangle distribution across viewing distances
- Natural integration with frustum culling
- Smoother LOD transitions

**Status:** Not implemented. Would require refactoring chunk management.

---

### 3. GPU Heightmap Sampling

**Current:** CPU-based `FastNoiseLite` sampling per vertex.

**Industry standard:** Store heightmap as GPU texture, sample in vertex shader.

**Benefits:**
- Decouples mesh generation from height sampling
- Enables true async chunk generation
- Required for vertex morphing
- Supports heightmap streaming/caching

**Status:** Not implemented. FastNoiseLite is not `Send+Sync`.

---

### 4. Erosion Simulation

**Current:** Erosion approximation via noise-based effects.

**Industry standard:** Post-process noise with erosion algorithms:

| Type | Effect |
|------|--------|
| **Hydraulic** | Water flow carves valleys, deposits sediment |
| **Thermal** | Loose material slides down steep slopes |

**Algorithm (hydraulic):**
1. Place water droplet at random position
2. Flow downhill following gradient
3. Erode based on velocity, deposit when slowing
4. Repeat 100,000+ iterations

**Status:** ⚡ Partially implemented via approximation:
- Valley carving in lowlands
- Plateau smoothing on highlands
- Coastal shelf transitions
- Multi-stage height curve with natural zones

Full particle-based erosion remains a future enhancement.

---

### 5. Triplanar Texturing

**Current:** Vertex colors only (flat appearance).

**Industry standard:** Project textures from X/Y/Z axes, blend based on surface normal.

```wgsl
let blend = abs(normal);
let color = tex_x * blend.x + tex_y * blend.y + tex_z * blend.z;
```

**Benefits:**
- No texture stretching on cliffs
- Real texture detail on all surfaces
- Combine with splatmapping for biome variety

**Status:** Not implemented. Requires custom shader.

---

### 6. Atmospheric Rendering

**Current:** Using Bevy's `DistanceFog` with atmospheric falloff.

**Industry standard:** Exponential height fog and/or atmospheric scattering.

```wgsl
let fog = exp(-distance * density);
let color = mix(sky_color, terrain_color, fog);
```

**Benefits:**
- Hides distant LOD artifacts
- Creates depth and scale
- Cheap to implement

**Status:** ✅ Implemented using `FogFalloff::from_visibility_colors()` with 4km visibility.

---

## Prioritized Recommendations

| Priority | Technique | Effort | Impact | Notes |
|:--------:|-----------|:------:|:------:|-------|
| 1 | Atmospheric fog | Low | High | Quick win, hides many issues |
| 2 | Vertex morphing | Medium | High | Eliminates LOD pop-in |
| 3 | GPU heightmap | Medium | Medium | Unlocks async + morphing |
| 4 | Triplanar texturing | Medium | High | Major visual upgrade |
| 5 | Quadtree LOD | High | Medium | Better perf at scale |
| 6 | Erosion simulation | High | High | Realistic landforms |

---

## Gap Analysis

| Current Approach | Industry Standard | Status |
|------------------|-------------------|--------|
| Mesh regeneration on LOD change | Vertex morphing in shader | ⚡ Mitigated with hysteresis |
| CPU noise sampling (blocking) | GPU texture sampling | Pending |
| Flat chunk grid | Hierarchical quadtree | Pending |
| Vertex colors | Texture splatmaps + triplanar | Pending |
| Noise + erosion approximation | Full erosion simulation | ⚡ Partially addressed |
| DistanceFog atmospheric | Full scattering model | ✅ Done |

---

## References

- Chunked LOD Algorithm (Ulrich, 2002)
- CDLOD: Continuous Distance-Dependent LOD
- Hydraulic Erosion (Musgrave et al.)
- Triplanar Mapping (GPU Gems)
