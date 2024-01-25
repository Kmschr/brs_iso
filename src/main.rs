mod asset_loader;
mod cam;
mod components;
mod pos;
mod tri;
mod fps;
mod lit;

use std::{path::PathBuf, io::BufReader, fs::File, time::Duration};

use asset_loader::{AssetLoaderPlugin, SceneAssets};
use bevy::{prelude::*, diagnostic::FrameTimeDiagnosticsPlugin, pbr::DefaultOpaqueRendererMethod};
//use bevy_editor_pls::EditorPlugin;
use brickadia::{save::SaveData, read::SaveReader};
use cam::IsoCameraPlugin;
use fps::FPSPlugin;
use lit::LightPlugin;

use crate::components::{gen_point_lights, gen_spot_lights};

#[derive(Component, Debug)]
struct ChunkEntity {
    meshes: Vec<Mesh>,
    material: Handle<StandardMaterial>,
}

#[derive(Component)]
struct Chat;

#[derive(Resource)]
struct BackTimer {
    timer: Timer
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Brickadia Isometric Viewer".into(),
                ..default()
            }),
            ..default()
        }))
        // Disable MSAA as it is incompatible with deferred rendering, use FXAA instead
        .insert_resource(Msaa::Off)
        .insert_resource(BackTimer {
            timer: Timer::new(Duration::from_millis(50), TimerMode::Repeating)
        })
        .insert_resource(DefaultOpaqueRendererMethod::deferred())
        .add_plugins((LightPlugin, AssetLoaderPlugin))
        .add_plugins((FrameTimeDiagnosticsPlugin::default(), FPSPlugin))
        //.add_plugins(EditorPlugin::default())
        .add_plugins(IsoCameraPlugin)
        .add_systems(PostStartup, setup)
        .add_systems(Update, (load_save, spawn_chunks, chat))
        .add_systems(Update, light_gizmos)
        //.add_systems(Update, spotlight_gizmos)
        .run();
}

fn setup(mut commands: Commands,
         asset_server: Res<AssetServer>) {
    commands.spawn(AudioBundle {
        source: asset_server.load("sounds/playerConnect.wav"),
        ..default()
    });

    commands.spawn((
        TextBundle::from("test").with_style(
            Style {
                position_type: PositionType::Absolute,
                bottom: Val::Px(10.0),
                left: Val::Px(15.0),
                ..default()
            }
        ),
        Chat
    ));
}

fn chat(
    mut query: Query<&mut Text, With<Chat>>,
    keycode: Res<Input<KeyCode>>,
    time: Res<Time>,
    mut backspace_timer: ResMut<BackTimer>,
) {
    backspace_timer.timer.tick(time.delta());

    let mut text = query.get_single_mut().unwrap();
    for key in keycode.get_just_pressed() {
        match key {
            KeyCode::Back => {
                text.sections[0].value.pop();
            },
            KeyCode::Space => {
                text.sections[0].value.push(' ');
            },
            KeyCode::Tab => {
                text.sections[0].value.push_str("    ");
            },
            KeyCode::Slash => {
                text.sections[0].value.push('/');
            },
            KeyCode::ShiftLeft => {},
            KeyCode::Underline => {
                text.sections[0].value.push('_');
            },
            KeyCode::Period => {
                text.sections[0].value.push('.');
            },
            _ => {
                let mut key = format!("{:?}", key);
                if !keycode.pressed(KeyCode::ShiftLeft) {
                    key = key.to_lowercase();
                };
                text.sections[0].value.push_str(&key);
            }
        }
    }

    let blink_duration = time.elapsed_seconds_f64() % 1.0;
    if blink_duration < 0.5 && text.sections.len() == 1 {
        text.sections.push(TextSection {
            value: "|".into(),
            ..default()
        });
    } else if text.sections.len() == 2 {
        text.sections.pop();
    }

    if keycode.pressed(KeyCode::Back) && backspace_timer.timer.finished() {
        text.sections[0].value.pop();
    }
}

fn load_save(
    mut commands: Commands,
    scene_assets: Res<SceneAssets>,
    keycode: Res<Input<KeyCode>>
) {
    if !keycode.just_pressed(KeyCode::L) {
        return;
    }

    let path = ask_save_path();
    let save_data = load_save_data(path);
    info!("Loaded {:?} bricks", &save_data.bricks.len());

    let point_lights = gen_point_lights(&save_data);
    info!("Spawning {} point lights", point_lights.len());
    for light in point_lights {
        commands.spawn(light);
    }

    let spot_lights = gen_spot_lights(&save_data);
    info!("Spawning {} spot lights", spot_lights.len());
    for light in spot_lights {
        commands.spawn(light);
    }

    // todo: remove after meshes for most assets are generated
    info!("{:?}", &save_data.header2.brick_assets);

    commands.spawn(ChunkEntity {
        meshes: tri::gen_save_mesh(&save_data, "BMC_Plastic"),
        material: scene_assets.plastic_material.clone()
    });
    commands.spawn(ChunkEntity {
        meshes: tri::gen_save_mesh(&save_data, "BMC_Glass"),
        material: scene_assets.glass_material.clone()
    });
    commands.spawn(ChunkEntity {
        meshes: tri::gen_save_mesh(&save_data, "BMC_Glow"),
        material: scene_assets.glow_material.clone()
    });
    commands.spawn(ChunkEntity {
        meshes: tri::gen_save_mesh(&save_data, "BMC_Metallic"),
        material: scene_assets.metal_material.clone()
    });
}

fn spawn_chunks(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut query: Query<(Entity, &mut ChunkEntity)>
) {
    for (entity, mut chunk_entity) in &mut query {
        if chunk_entity.meshes.is_empty() {
            commands.entity(entity).despawn();
            continue;
        }

        for _ in 0..10 {
            if let Some(mesh) = chunk_entity.meshes.pop() {
                commands.spawn((
                    PbrBundle {
                        mesh: meshes.add(mesh),
                        material: chunk_entity.material.clone(),
                        ..default()
                    },
                ));
            }
        }
    }
}

fn ask_save_path() -> PathBuf {
    rfd::FileDialog::new()
        .add_filter("Brickadia Save", &["brs"])
        .set_directory(default_build_directory().unwrap())
        .pick_file()
        .unwrap()
}

fn load_save_data(path: PathBuf) -> SaveData {
    SaveReader::new(BufReader::new(File::open(path).unwrap()))
        .unwrap()
        .read_all()
        .unwrap()
}

fn default_build_directory() -> Option<PathBuf> {
    match std::env::consts::OS {
        "windows" => dirs::data_local_dir().and_then(|path| {
            Some(PathBuf::from(
                path.to_string_lossy().to_string() + "\\Brickadia\\Saved\\Builds",
            ))
        }),
        "linux" => dirs::config_dir().and_then(|path| {
            Some(PathBuf::from(
                path.to_string_lossy().to_string() + "/Epic/Brickadia/Saved/Builds",
            ))
        }),
        _ => None,
    }
}

fn light_gizmos(
    mut gizmos: Gizmos,
    query: Query<(&PointLight, &Transform)>
) {
    for (light, transform) in &query {
        gizmos.sphere(transform.translation, transform.rotation, light.radius, light.color);
    }
}

fn spotlight_gizmos(
    mut gizmos: Gizmos,
    query: Query<(&SpotLight, &Transform)>
) {
    for (light, transform) in &query {
        gizmos.line(transform.translation, transform.translation + transform.forward() * light.radius, light.color);
    }
}
