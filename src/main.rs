mod asset_loader;
mod cam;
mod pos;
mod tri;
mod fps;
mod lit;

use std::{path::PathBuf, io::BufReader, fs::File};

use asset_loader::{AssetLoaderPlugin, SceneAssets};
use bevy::{prelude::*, diagnostic::FrameTimeDiagnosticsPlugin, pbr::DefaultOpaqueRendererMethod};
//use bevy_editor_pls::EditorPlugin;
use brickadia::{save::SaveData, read::SaveReader};
use cam::IsoCameraPlugin;
use fps::FPSPlugin;
use lit::LightPlugin;

#[derive(Component, Debug)]
struct ChunkEntity {
    meshes: Vec<Mesh>,
    material: Handle<StandardMaterial>,
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
        .insert_resource(DefaultOpaqueRendererMethod::deferred())
        .add_plugins((LightPlugin, AssetLoaderPlugin))
        .add_plugins((FrameTimeDiagnosticsPlugin::default(), FPSPlugin))
        //.add_plugins(EditorPlugin::default())
        .add_plugins(IsoCameraPlugin)
        .add_systems(PostStartup, setup)
        .add_systems(Update, spawn_chunks)
        .run();
}

fn setup(mut commands: Commands,
         asset_server: Res<AssetServer>,
         scene_assets: Res<SceneAssets>) {
    let path = ask_save_path();
    let save_data = load_save_data(path);
    info!("Loaded {:?} bricks", &save_data.bricks.len());

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

    commands.spawn(AudioBundle {
        source: asset_server.load("sounds/playerConnect.wav"),
        ..default()
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
