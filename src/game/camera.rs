use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::input::mouse::AccumulatedMouseScroll;
use bevy::prelude::*;

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_camera)
            .add_systems(Update, camera_movement);
    }
}

fn setup_camera(mut commands: Commands) {
    // City builder / Civ style camera: high up, looking almost straight down
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 350.0, 150.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

fn camera_movement(
    time: Res<Time>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    scroll: Res<AccumulatedMouseScroll>,
    mouse_motion: Res<AccumulatedMouseMotion>,
    mut query: Query<&mut Transform, With<Camera3d>>,
) {
    let speed = 50.0;
    let zoom_speed = 10.0; // Increased zoom speed for larger scale
    let rotation_speed = 0.005;

    for mut transform in &mut query {
        let mut velocity = Vec3::ZERO;

        let forward = transform.forward();
        let right = transform.right();

        // Flatten forward/right to XZ plane for movement
        let forward_xz = Vec3::new(forward.x, 0.0, forward.z).normalize_or_zero();
        let right_xz = Vec3::new(right.x, 0.0, right.z).normalize_or_zero();

        if keyboard.pressed(KeyCode::KeyW) {
            velocity += forward_xz;
        }
        if keyboard.pressed(KeyCode::KeyS) {
            velocity -= forward_xz;
        }
        if keyboard.pressed(KeyCode::KeyA) {
            velocity -= right_xz;
        }
        if keyboard.pressed(KeyCode::KeyD) {
            velocity += right_xz;
        }

        // Zoom/Height control with keyboard
        if keyboard.pressed(KeyCode::KeyE) {
            velocity += Vec3::Y;
        }
        if keyboard.pressed(KeyCode::KeyQ) {
            velocity -= Vec3::Y;
        }

        transform.translation += velocity * speed * time.delta_secs();

        // Mouse wheel zoom
        if scroll.delta.y != 0.0 {
            transform.translation.y -= scroll.delta.y * zoom_speed;
            transform.translation.y = transform.translation.y.clamp(30.0, 800.0);
        }

        // Middle mouse button - horizontal rotation (orbit around look-at point)
        if mouse_buttons.pressed(MouseButton::Middle) {
            let delta = mouse_motion.delta;

            if delta.x != 0.0 {
                // Get current look-at point (project forward from camera to ground plane)
                let look_distance = transform.translation.y / (-transform.forward().y).max(0.1);
                let look_at = transform.translation + transform.forward() * look_distance;

                // Rotate around Y axis (horizontal mouse movement)
                let yaw = Quat::from_rotation_y(-delta.x * rotation_speed);
                let offset = transform.translation - look_at;
                let rotated_offset = yaw * offset;
                transform.translation = look_at + rotated_offset;

                // Make camera look at the same point
                transform.look_at(look_at, Vec3::Y);
            }
        }

        // Right mouse button - Orbit (Yaw + Pitch)
        if mouse_buttons.pressed(MouseButton::Right) {
            let delta = mouse_motion.delta;

            // Get current look-at point on the ground
            let look_distance = transform.translation.y / (-transform.forward().y).max(0.1);
            let look_at = transform.translation + transform.forward() * look_distance;

            // 1. Handle Yaw (Left/Right)
            if delta.x != 0.0 {
                // Rotate around Y axis
                let yaw = Quat::from_rotation_y(-delta.x * rotation_speed);
                let offset = transform.translation - look_at;
                let rotated_offset = yaw * offset;
                transform.translation = look_at + rotated_offset;
                transform.look_at(look_at, Vec3::Y);
            }

            // 2. Handle Pitch (Up/Down) - with limits
            if delta.y != 0.0 {
                // Recalculate look-at and offset in case Yaw changed them
                // (Though we just updated transform, so we can re-derive or reuse if careful.
                // Re-deriving is safer to avoid drift)
                let look_distance = transform.translation.y / (-transform.forward().y).max(0.1);
                let look_at = transform.translation + transform.forward() * look_distance;

                let to_camera = transform.translation - look_at;
                let horizontal_dist = Vec2::new(to_camera.x, to_camera.z).length();
                let current_pitch = (transform.translation.y - look_at.y).atan2(horizontal_dist);

                // Apply pitch change (inverted: drag down = look more down/camera higher)
                let new_pitch = (current_pitch + delta.y * rotation_speed).clamp(0.15, 1.4);

                let distance = to_camera.length();
                let new_height = distance * new_pitch.sin();
                let new_horizontal = distance * new_pitch.cos();

                let horizontal_dir = Vec2::new(to_camera.x, to_camera.z).normalize_or_zero();

                transform.translation = look_at
                    + Vec3::new(
                        horizontal_dir.x * new_horizontal,
                        new_height,
                        horizontal_dir.y * new_horizontal,
                    );
                transform.look_at(look_at, Vec3::Y);
            }
        }
    }
}
