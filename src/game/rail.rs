use crate::game::train::TrainSpawnQueue;
use crate::game::village::Village;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

pub struct RailPlugin;

impl Plugin for RailPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RailBuildState>()
            .add_systems(Update, rail_click_system);
    }
}

#[derive(Resource, Default)]
struct RailBuildState {
    selected_start: Option<Entity>,
}

#[derive(Component)]
pub struct Rail {
    pub start: Entity,
    pub end: Entity,
}

#[allow(clippy::too_many_arguments)]
fn rail_click_system(
    mouse_input: Res<ButtonInput<MouseButton>>,
    window_query: Query<&Window, With<PrimaryWindow>>,
    camera_query: Query<(&Camera, &GlobalTransform)>,
    mut commands: Commands,
    mut state: ResMut<RailBuildState>,
    mut train_queue: ResMut<TrainSpawnQueue>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    transforms: Query<&GlobalTransform>,
    villages: Query<Entity, With<Village>>,
) {
    if !mouse_input.just_pressed(MouseButton::Left) {
        return;
    }

    let Some(window) = window_query.iter().next() else {
        return;
    };
    let Some(cursor_position) = window.cursor_position() else {
        return;
    };

    let Some((camera, camera_transform)) = camera_query.iter().next() else {
        return;
    };

    let Ok(ray) = camera.viewport_to_world(camera_transform, cursor_position) else {
        return;
    };

    // Simple ray-sphere intersection with villages
    // Iterate all villages and find closest one intersected

    let mut closest_hit: Option<(Entity, f32)> = None;
    let village_radius = 4.0; // Slightly larger than visual

    for village_entity in villages.iter() {
        if let Ok(village_transform) = transforms.get(village_entity) {
            let village_pos = village_transform.translation();

            // Sphere intersection approximation: distance from point to ray
            // Or just check distance from camera if ray passes close enough

            // Vector from ray origin to village center
            let to_village = village_pos - ray.origin;
            // Project onto ray direction
            let t = to_village.dot(ray.direction.normalize_or_zero());

            if t > 0.0 {
                let text_pos = ray.origin + ray.direction.normalize_or_zero() * t;
                let dist_sq = text_pos.distance_squared(village_pos);

                if dist_sq < village_radius * village_radius {
                    // Hit!
                    let dist_from_cam = t;
                    if closest_hit.is_none_or(|(_, d)| dist_from_cam < d) {
                        closest_hit = Some((village_entity, dist_from_cam));
                    }
                }
            }
        }
    }

    if let Some((clicked_entity, _)) = closest_hit {
        info!("Clicked village: {:?}", clicked_entity);

        if let Some(start_entity) = state.selected_start {
            if start_entity == clicked_entity {
                info!("Cancelled selection");
                state.selected_start = None;
                return;
            }

            // Build Rail
            let start_pos = transforms.get(start_entity).unwrap().translation();
            let end_pos = transforms.get(clicked_entity).unwrap().translation();

            let mid_point = (start_pos + end_pos) / 2.0;
            let distance = start_pos.distance(end_pos);
            let direction = (end_pos - start_pos).normalize();
            let rotation = Quat::from_rotation_arc(Vec3::Z, direction);

            let mesh_handle = meshes.add(Cuboid::new(1.0, 1.0, distance));
            let material_handle = materials.add(StandardMaterial {
                base_color: Color::srgb(0.3, 0.3, 0.3),
                ..default()
            });

            let rail = commands
                .spawn((
                    Mesh3d(mesh_handle),
                    MeshMaterial3d(material_handle),
                    Transform {
                        translation: mid_point,
                        rotation,
                        ..default()
                    },
                    Rail {
                        start: start_entity,
                        end: clicked_entity,
                    },
                ))
                .id();

            train_queue.0.push(rail); // Add to queue
            state.selected_start = None;
        } else {
            info!("Selected start village: {:?}", clicked_entity);
            state.selected_start = Some(clicked_entity);
        }
    }
}
