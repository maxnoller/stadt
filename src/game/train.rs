use crate::game::rail::Rail;
use bevy::prelude::*;

pub struct TrainPlugin;

impl Plugin for TrainPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TrainSpawnQueue>()
            .add_systems(Update, (spawn_train, move_train));
    }
}

#[derive(Resource, Default)]
pub struct TrainSpawnQueue(pub Vec<Entity>);

#[derive(Component)]
pub struct Train {
    pub rail: Entity,
    pub progress: f32,  // 0.0 to 1.0
    pub speed: f32,     // units per second
    pub direction: f32, // 1.0 or -1.0
}

fn spawn_train(
    mut queue: ResMut<TrainSpawnQueue>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for rail_entity in queue.0.drain(..) {
        // Spawn a train on the new rail
        let mesh_handle = meshes.add(Cuboid::new(0.8, 0.8, 2.0));
        let material_handle = materials.add(StandardMaterial {
            base_color: Color::srgb(0.9, 0.8, 0.1), // Yellow train
            ..default()
        });

        commands.spawn((
            Mesh3d(mesh_handle),
            MeshMaterial3d(material_handle),
            Transform::from_xyz(0.0, 0.0, 0.0), // Pos set by system
            Train {
                rail: rail_entity,
                progress: 0.0,
                speed: 0.5,
                direction: 1.0,
            },
        ));
    }
}

fn move_train(
    time: Res<Time>,
    mut trains: Query<(&mut Transform, &mut Train)>,
    rails: Query<&Rail>,
    transforms: Query<&GlobalTransform>, // To get village positions
) {
    for (mut train_transform, mut train_comp) in trains.iter_mut() {
        let Ok(rail) = rails.get(train_comp.rail) else {
            continue;
        };

        // Get start/end positions
        // Rail stores Entities, so we look them up
        let Ok(start_tf) = transforms.get(rail.start) else {
            continue;
        };
        let Ok(end_tf) = transforms.get(rail.end) else {
            continue;
        };

        let start_pos = start_tf.translation();
        let end_pos = end_tf.translation();

        let rail_len = start_pos.distance(end_pos);
        if rail_len < 0.001 {
            continue;
        }

        // Speed in normalized coords = real_speed / length
        let norm_speed = train_comp.speed / rail_len;

        train_comp.progress += norm_speed * train_comp.direction * time.delta_secs();

        // Bounce
        if train_comp.progress >= 1.0 {
            train_comp.progress = 1.0;
            train_comp.direction = -1.0;
        } else if train_comp.progress <= 0.0 {
            train_comp.progress = 0.0;
            train_comp.direction = 1.0;
        }

        // Update Transform
        let pos = start_pos.lerp(end_pos, train_comp.progress);

        // Oriental
        let dir_vec = (end_pos - start_pos).normalize();
        let rotation = Quat::from_rotation_arc(Vec3::Z, dir_vec); // Train faces Z?

        train_transform.translation = pos + Vec3::new(0.0, 1.0, 0.0); // Sit on top
        train_transform.rotation = rotation;
    }
}
