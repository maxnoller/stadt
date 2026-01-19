use bevy::prelude::*;

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(camera::CameraPlugin)
            .add_plugins(terrain::TerrainPlugin)
            .add_plugins(village::VillagePlugin)
            .add_plugins(rail::RailPlugin)
            .add_plugins(train::TrainPlugin)
            .add_plugins(MeshPickingPlugin) // Ensure picking works on meshes
            .add_systems(Startup, setup_lights);
    }
}

mod camera;
mod rail;
mod terrain;
mod train;
mod village;

fn setup_lights(mut commands: Commands) {
    // Directional light (sun)
    commands.spawn((
        DirectionalLight {
            shadows_enabled: true,
            illuminance: 15_000.0,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(
            EulerRot::XYZ,
            -std::f32::consts::FRAC_PI_3, // Steeper angle for better shadows
            -std::f32::consts::FRAC_PI_4,
            0.0,
        )),
    ));
}
