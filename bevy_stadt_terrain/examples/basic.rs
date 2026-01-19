//! Basic terrain example
//!
//! Demonstrates minimal setup for procedural terrain using bevy_stadt_terrain.
//!
//! Run with: `cargo run -p bevy_stadt_terrain --example basic`

use bevy::prelude::*;
use bevy_stadt_terrain::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(TerrainPlugin::default())
        .add_systems(Startup, setup)
        .add_systems(Update, camera_controller)
        .run();
}

fn setup(mut commands: Commands) {
    // Spawn terrain with Stadt-style multi-layer noise
    let noise = bevy_stadt_terrain::heightmap::TerrainNoise::default();
    let config = TerrainConfig::default();
    commands.spawn(TerrainBundle::noise(noise, &config));

    // Camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 200.0, 300.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // Directional light (sun)
    commands.spawn((
        DirectionalLight {
            shadows_enabled: true,
            illuminance: 15_000.0,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(
            EulerRot::XYZ,
            -std::f32::consts::FRAC_PI_3,
            -std::f32::consts::FRAC_PI_4,
            0.0,
        )),
    ));

    // Ambient light
    commands.insert_resource(AmbientLight {
        color: Color::srgb(0.9, 0.95, 1.0),
        brightness: 200.0,
    });
}

/// Simple fly camera controller
fn camera_controller(
    time: Res<Time>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut camera_query: Query<&mut Transform, With<Camera>>,
) {
    let Ok(mut transform) = camera_query.single_mut() else {
        return;
    };

    let mut velocity = Vec3::ZERO;
    let speed = 200.0;

    // WASD movement
    if keyboard.pressed(KeyCode::KeyW) {
        velocity += *transform.forward();
    }
    if keyboard.pressed(KeyCode::KeyS) {
        velocity -= *transform.forward();
    }
    if keyboard.pressed(KeyCode::KeyA) {
        velocity -= *transform.right();
    }
    if keyboard.pressed(KeyCode::KeyD) {
        velocity += *transform.right();
    }

    // Up/Down
    if keyboard.pressed(KeyCode::Space) {
        velocity += Vec3::Y;
    }
    if keyboard.pressed(KeyCode::ShiftLeft) {
        velocity -= Vec3::Y;
    }

    // Rotation
    let rotation_speed = 1.0;
    if keyboard.pressed(KeyCode::KeyQ) {
        transform.rotate_y(rotation_speed * time.delta_secs());
    }
    if keyboard.pressed(KeyCode::KeyE) {
        transform.rotate_y(-rotation_speed * time.delta_secs());
    }

    if velocity != Vec3::ZERO {
        transform.translation += velocity.normalize() * speed * time.delta_secs();
    }
}
