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
    // RTS camera position: high up, looking down-ish
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 50.0, 50.0).looking_at(Vec3::ZERO, Vec3::Y),
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
    let zoom_speed = 5.0;
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
            transform.translation.y = transform.translation.y.clamp(10.0, 200.0);
        }

        // Middle mouse button rotation (orbit around look-at point)
        if mouse_buttons.pressed(MouseButton::Middle) {
            let delta = mouse_motion.delta;

            if delta.x != 0.0 || delta.y != 0.0 {
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
    }
}
