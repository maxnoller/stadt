# AGENTS.md

## Overview

Stadt is a Bevy 0.18 game featuring procedural terrain with CDLOD quadtree LOD, train/rail systems, and village generation. The terrain system is extracted into a reusable `bevy_stadt_terrain` plugin.

## Build & Test

```bash
# Build the entire workspace
cargo build --workspace

# Run the game
cargo run

# Run terrain plugin example
cargo run -p bevy_stadt_terrain --example basic

# Run tests
cargo test --workspace

# Lint (required to pass before commit)
cargo clippy -- -D warnings

# Format code
cargo fmt
```

## Project Structure

```
stadt/
├── src/
│   ├── main.rs              # App entry point
│   └── game/
│       ├── mod.rs           # GamePlugin (orchestrates all systems)
│       ├── camera.rs        # Fly camera controller
│       ├── terrain/mod.rs   # Thin wrapper around bevy_stadt_terrain
│       ├── village.rs       # Village spawning on terrain
│       ├── rail.rs          # Rail track system
│       └── train.rs         # Train entities
├── bevy_stadt_terrain/      # Standalone terrain plugin (workspace member)
│   ├── src/
│   │   ├── lib.rs           # Plugin entry, TerrainBundle
│   │   ├── config.rs        # TerrainConfig with builder pattern
│   │   ├── heightmap.rs     # HeightmapSource trait, TerrainNoise
│   │   ├── quadtree.rs      # CDLOD quadtree LOD system
│   │   ├── streaming.rs     # Async chunk mesh generation
│   │   ├── mesh.rs          # Mesh generation with morph heights
│   │   ├── material.rs      # TerrainMaterial (vertex morphing shader)
│   │   └── physics.rs       # Rapier heightfield colliders (feature-gated)
│   └── examples/basic.rs
└── assets/
    └── shaders/terrain.wgsl # Vertex morphing shader
```

## Architecture

The game uses Bevy's ECS architecture. `GamePlugin` adds all subsystem plugins. Terrain is handled by `bevy_stadt_terrain` which uses a quadtree for LOD selection, async mesh generation via `AsyncComputeTaskPool`, and vertex morphing shaders for smooth LOD transitions. Villages spawn on terrain chunks using deterministic RNG based on chunk coordinates.

## Code Style

- **Rust Edition**: 2024
- **Formatting**: `cargo fmt` (enforced by pre-commit hook)
- **Linting**: `cargo clippy -- -D warnings` (enforced by pre-commit hook)
- **Commits**: Conventional commits (`feat:`, `fix:`, `refactor:`, etc.)
- **Bevy patterns**: Use `Query`, `Res`, `Commands` for systems; derive `Component`, `Resource`

## Git Workflow

- **Pre-commit hooks**: Format check + clippy (via husky)
- **Commit format**: Conventional commits - `feat(scope): description`
- **Branch naming**: `feat/description`, `fix/description`

## Important Constraints

- **Bevy version**: 0.18 - do not upgrade without testing all systems
- **Shader paths**: Shaders must be in `assets/shaders/` and referenced as `"shaders/name.wgsl"`
- **Terrain seed**: Use seed 42 for `TerrainNoise::with_seed()` to maintain deterministic terrain
- **Never modify**: `Cargo.lock` manually, `node_modules/`, `.husky/` internals
- **Physics feature**: Rapier integration requires `--features rapier` flag
- **Dynamic linking**: Dev builds use `dynamic_linking` for faster iteration - don't remove

## Key APIs

```rust
// Spawn terrain
let noise = TerrainNoise::with_seed(42);
commands.spawn(TerrainBundle::noise(noise, &config));

// Query terrain height
let height = sample_terrain_height(x, z, &noise, &config);

// TerrainConfig defaults: chunk_size=100, render_distance=50, max_height=180
```
