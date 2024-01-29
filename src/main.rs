mod aabb;
mod asset_loader;
mod bvh;
mod cam;
mod chat;
mod components;
mod faces;
mod pos;
mod fps;
mod lit;
mod utils;

use std::{path::PathBuf, io::BufReader, fs::File, sync::mpsc::{Receiver, self}, thread};

use asset_loader::{AssetLoaderPlugin, SceneAssets};
use bevy::{prelude::*, diagnostic::FrameTimeDiagnosticsPlugin, pbr::DefaultOpaqueRendererMethod};
use brickadia::{save::SaveData, read::SaveReader};
use bvh::BVHNode;
use cam::IsoCameraPlugin;
use chat::ChatPlugin;
use fps::FPSPlugin;
use lit::LightPlugin;

use crate::{components::{gen_point_lights, gen_spot_lights}, bvh::BVHMeshGenerator};

#[derive(Component, Debug)]
struct ChunkEntity {
    meshes: Vec<Mesh>,
    material: Handle<StandardMaterial>,
}

#[derive(Component)]
struct SaveBVH {
    bvh: BVHNode,
}

#[derive(Resource, Default)]
struct BVHDepth {
    value: u8,
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
        .insert_resource(BVHDepth::default())
        .add_plugins((LightPlugin, AssetLoaderPlugin, ChatPlugin))
        .add_plugins((FrameTimeDiagnosticsPlugin::default(), FPSPlugin))
        .add_plugins(IsoCameraPlugin)
        .add_systems(PostStartup, setup)
        .add_systems(Update, (pick_path, load_save, spawn_chunks))
        .add_systems(Update, (bvh_gizmos, change_depth))
        //.add_systems(Update, spotlight_gizmos)
        //.add_systems(Update, light_gizmos)
        .run();
}

fn setup(mut commands: Commands,
         asset_server: Res<AssetServer>) {
    commands.spawn(AudioBundle {
        source: asset_server.load("sounds/playerConnect.wav"),
        ..default()
    });
}

fn pick_path(
    world: &mut World
) {
    let keycode = world.resource::<Input<KeyCode>>();
    if keycode.just_pressed(KeyCode::L) {
        let (tx, rx) = mpsc::channel();
        world.insert_non_send_resource(rx);
        thread::spawn(move || {
            tx.send(ask_save_path()).unwrap();
        });
    }
}

fn load_save(
    mut commands: Commands,
    scene_assets: Res<SceneAssets>,
    path_receiver: Option<NonSend<Receiver<PathBuf>>>,
) {
    if path_receiver.is_none() {
        return;
    }

    let path_receiver = path_receiver.unwrap();
    let path = path_receiver.try_recv();
    if path.is_err() {
        return;
    }

    let path = path.unwrap();

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
    
    let generator = BVHMeshGenerator::new(&save_data);
    let material_meshes = generator.gen_mesh();

    let mut i = 0;
    for meshes in material_meshes.into_iter() {
        let material = match i {
            0 => scene_assets.plastic_material.clone(),
            1 => scene_assets.glow_material.clone(),
            2 => scene_assets.glass_material.clone(),
            3 => scene_assets.metal_material.clone(),
            _ => scene_assets.plastic_material.clone(),
        };
        commands.spawn(ChunkEntity {
            meshes,
            material,
        });
        i += 1;
    }

 

    commands.spawn(SaveBVH {
        bvh: generator.bvh
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

fn _light_gizmos(
    mut gizmos: Gizmos,
    query: Query<(&PointLight, &Transform)>
) {
    for (light, transform) in &query {
        gizmos.sphere(transform.translation, transform.rotation, light.radius, light.color);
    }
}

fn _spotlight_gizmos(
    mut gizmos: Gizmos,
    query: Query<(&SpotLight, &Transform)>
) {
    for (light, transform) in &query {
        gizmos.line(transform.translation, transform.translation + transform.forward() * light.radius, light.color);
    }
}

fn bvh_gizmos (
    mut gizmos: Gizmos,
    query: Query<&SaveBVH>,
    bvh_depth: Res<BVHDepth>
) {
    for save_bvh in &query {
        aabb_gizmos_recursive(&save_bvh.bvh, &mut gizmos, 0, bvh_depth.value);
    }
}

fn aabb_gizmos_recursive(bvh: &BVHNode, gizmos: &mut Gizmos, depth: u8, target_depth: u8) {
    let color = match depth {
        0 => Color::WHITE,
        1 => Color::BLUE,
        2 => Color::GREEN,
        3 => Color::YELLOW,
        4 => Color::MAROON,
        5 => Color::GOLD,
        6 => Color::VIOLET,
        _ => Color::WHITE,
    };

    match bvh {
        BVHNode::Internal { aabb, left, right } => {
            if depth == target_depth {
                gizmos.cuboid(Transform {
                    translation: aabb.center.as_vec3(),
                    rotation: Quat::IDENTITY,
                    scale: aabb.halfwidths.as_vec3() * 2.0,
                }, color);
                return;
            }

            aabb_gizmos_recursive(&left, gizmos, depth + 1, target_depth);
            aabb_gizmos_recursive(&right, gizmos, depth + 1, target_depth);
        },
        _ => {}
    }
}

fn change_depth(
    keycode: Res<Input<KeyCode>>,
    mut bvh_depth: ResMut<BVHDepth>,
) {
    if keycode.just_pressed(KeyCode::W) {
        bvh_depth.value += 1;
    }
    if keycode.just_pressed(KeyCode::S) {
        bvh_depth.value -= 1;
    }
}
