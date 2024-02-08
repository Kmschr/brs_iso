mod aabb;
mod asset_loader;
mod bvh;
mod cam;
mod chat;
mod components;
mod faces;
mod icon;
mod pos;
mod state;
mod settings;
mod fps;
mod lit;
mod utils;

use std::{path::PathBuf, io::BufReader, fs::File, sync::mpsc::{Receiver, self}, thread};

use aabb::AABB;
use asset_loader::{AssetLoaderPlugin, SceneAssets};
use bevy::{diagnostic::FrameTimeDiagnosticsPlugin, pbr::DefaultOpaqueRendererMethod, prelude::*, render::mesh::shape::Plane, window::{PrimaryWindow,  WindowResolution}, winit::WinitWindows};
use bevy_egui::{egui, EguiContexts, EguiPlugin};
use bevy_embedded_assets::EmbeddedAssetPlugin;
use brickadia::{save::SaveData, read::SaveReader};
use bvh::{BVHNode, BVH};
use cam::{IsoCamera, IsoCameraPlugin};
use chat::ChatPlugin;
use fps::FPSPlugin;
use lit::LightPlugin;
use settings::SettingsPlugin;
use state::{BVHView, GameState, InputState};
use winit::window::Icon;

use crate::{components::{gen_point_lights, gen_spot_lights, Light}, bvh::BVHMeshGenerator};

#[derive(Component, Debug)]
struct ChunkEntity {
    meshes: Vec<Mesh>,
    material: Handle<StandardMaterial>,
}

#[derive(Component)]
struct SaveBVH {
    save_data: SaveData,
    pub bvh: BVH,
    aabbs: Vec<AABB>,
    com: Vec3
}


#[derive(Component)]
struct Water;

#[derive(Component)]
struct Ground;

#[derive(Component)]
struct ChunkMesh;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Brickadia Isometric Viewer".into(),
                // present_mode: PresentMode::Immediate,
                resolution: WindowResolution::new(1600., 900.),
                resize_constraints: WindowResizeConstraints {
                    min_width: 854.,
                    min_height: 480.,
                    ..default()
                },
                ..default()
            }),
            ..default()
        }))
        // Disable MSAA as it is incompatible with deferred rendering, use FXAA instead
        .insert_resource(Msaa::Off)
        .insert_resource(DefaultOpaqueRendererMethod::deferred())
        .insert_resource(GameState::default())
        .insert_resource(GlobalVolume::new(0.2))
        .add_plugins((LightPlugin, AssetLoaderPlugin, ChatPlugin, SettingsPlugin, IsoCameraPlugin))
        .add_plugins((FrameTimeDiagnosticsPlugin::default(), FPSPlugin))
        .add_plugins(EguiPlugin)
        .add_plugins(EmbeddedAssetPlugin::default())
        .add_systems(Startup, set_window_icon)
        .add_systems(PostStartup, setup)
        .add_systems(Update, (pick_path, load_brs, load_save, spawn_chunks, move_water))
        .add_systems(Update, (bvh_gizmos, change_depth, spotlight_gizmos, light_gizmos, brick_info))
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    assets: Res<SceneAssets>,
) {
    commands.spawn(AudioBundle {
        source: assets.sounds.startup.clone(),
        ..default()
    });

    // spawn water mesh
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Plane::from_size(1000000.).into()),
            material: assets.materials.water.clone(),
            visibility: Visibility::Hidden,
            ..default()
        },
        Water,
    ));

    // spawn ground plane
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Plane::from_size(1000000.).into()),
            material: assets.materials.ground.clone(),
            visibility: Visibility::Hidden,
            ..default()
        },
        Ground,
    ));
}

fn brick_info(
    window_query: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform)>,
    bvh_query: Query<&SaveBVH>,
    mouse: Res<Input<MouseButton>>,
    mut contexts: EguiContexts,
    mut gizmos: Gizmos,
) {
    if !mouse.pressed(MouseButton::Right) {
        return;
    }

    for save_bvh in bvh_query.iter() {
        let window = match window_query.get_single() {
            Ok(window) => window,
            Err(_) => return,
        };
        let mouse_pos: Vec2 = match window.cursor_position() {
            Some(pos) => pos,
            None => return,
        };
    
        for (camera, camera_transform) in cameras.iter() {
            let ray = camera.viewport_to_world(camera_transform, mouse_pos);
    
            if let Some(ray) = ray {
                let brick_index = save_bvh.bvh.intersection(ray, &save_bvh.aabbs);
                if let Some(brick_index) = brick_index {
                    let brick = &save_bvh.save_data.bricks[brick_index];
                    let asset_name = &save_bvh.save_data.header2.brick_assets[brick.asset_name_index as usize];

                    let owner = if brick.owner_index == 0 {
                        "PUBLIC"
                    } else {
                        save_bvh.save_data.header2.brick_owners[brick.owner_index as usize - 1].name.as_str()
                    };

                    egui::Window::new("Brick Info").show(contexts.ctx_mut(), |ui| {
                        ui.label(format!("Brick index: {}", brick_index));
                        ui.label(format!("Brick position: {:?}", brick.position));
                        ui.label(format!("Brick size: {:?}", brick.size));
                        ui.label(format!("Brick asset: {}", asset_name));
                        ui.label(format!("Brick owner: {}", owner));
                    });
    
                    let aabb = save_bvh.aabbs[brick_index];
                    gizmos.cuboid(Transform {
                        translation: aabb.center.as_vec3(),
                        rotation: Quat::IDENTITY,
                        scale: aabb.halfwidths.as_vec3() * 2.0,
                    }, Color::WHITE);
                }
            }
        }
    }
}

fn set_window_icon(
    windows: NonSend<WinitWindows>,
) {
    let rgba = icon::ICON.to_vec();
    let icon = Icon::from_rgba(rgba, 32, 32).unwrap();

    for window in windows.windows.values() {
        window.set_window_icon(Some(icon.clone()));
    }
}

fn move_water(
    mut query: Query<&mut Transform, With<Water>>,
    keycode: Res<Input<KeyCode>>,
    game_state: Res<GameState>,
    time: Res<Time>,
) {
    if !game_state.input_listening() {
        return;
    }

    let mut movement = Vec3::ZERO;
    if keycode.pressed(KeyCode::I) {
        movement += Vec3::Y;
    } else if keycode.pressed(KeyCode::K) {
        movement += Vec3::NEG_Y;
    }

    if keycode.pressed(KeyCode::ShiftLeft) {
        movement *= 10.0;
    }

    let mut transform = query.get_single_mut().unwrap();
    transform.translation += movement * time.delta_seconds() * 50.0;
}

fn pick_path(
    world: &mut World
) {
    let game_state = world.resource::<GameState>();
    match game_state.input {
        InputState::Listen => {},
        InputState::Typing => {
            return;
        }
    }
    let keycode = world.resource::<Input<KeyCode>>();
    if keycode.just_pressed(KeyCode::L) || keycode.just_pressed(KeyCode::O) {
        let (tx, rx) = mpsc::channel();
        world.insert_non_send_resource(rx);
        thread::spawn(move || {
            tx.send(ask_save_path()).unwrap();
        });
    }
}

fn load_brs(
    world: &mut World
) {
    let path_receiver = world.get_non_send_resource::<Receiver<PathBuf>>();
    if let Some(path_receiver) = path_receiver {
        let path = path_receiver.try_recv();
        if path.is_err() {
            return;
        }

        let assets = world.resource::<SceneAssets>();
        world.spawn(AudioBundle {
            source: assets.sounds.upload_start.clone(),
            ..default()
        });

        let path = path.unwrap();
        let (tx, rx) = mpsc::channel();
        world.insert_non_send_resource(rx);
        thread::spawn(move || {
            let save_data = load_save_data(path);
            tx.send(save_data).unwrap();
        });
    }
}

fn load_save(
    mut commands: Commands,
    mut cam_query: Query<&mut IsoCamera>,
    assets: Res<SceneAssets>,
    save_receiver: Option<NonSend<Receiver<SaveData>>>,
) {
    if save_receiver.is_none() {
        return;
    }

    let save_receiver = save_receiver.unwrap();
    let save_data = save_receiver.try_recv();
    if save_data.is_err() {
        return;
    }

    let save_data = save_data.unwrap();
    info!("Loaded {:?} bricks", &save_data.bricks.len());

    let point_lights = gen_point_lights(&save_data);
    info!("Spawning {} point lights", point_lights.len());
    for light in point_lights {
        commands.spawn((light, Light));
    }

    let spot_lights = gen_spot_lights(&save_data);
    info!("Spawning {} spot lights", spot_lights.len());
    for light in spot_lights {
        commands.spawn((light, Light));
    }

    // todo: remove after meshes for most assets are generated
    info!("{:?}", &save_data.header2.brick_assets);
    
    let generator = BVHMeshGenerator::new(&save_data);
    let material_meshes = generator.gen_mesh();
    let com = generator.center_of_mass();

    if let Ok(mut cam) = cam_query.get_single_mut() {
        cam.target = com;
    }

    let mut i = 0;
    for meshes in material_meshes.into_iter() {
        let material = match i {
            0 => assets.materials.plastic.clone(),
            1 => assets.materials.glow.clone(),
            2 => assets.materials.glass.clone(),
            3 => assets.materials.metal.clone(),
            _ => assets.materials.plastic.clone(),
        };
        commands.spawn(ChunkEntity {
            meshes,
            material,
        });
        i += 1;
    }

    let bvh = generator.bvh;
    let aabbs = generator.aabbs;

    commands.spawn(SaveBVH {
        bvh,
        save_data,
        aabbs,
        com,
    });

    commands.spawn(AudioBundle {
        source: assets.sounds.upload_end.clone(),
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
                    ChunkMesh
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
    query: Query<(&PointLight, &Transform)>,
    game_state: Res<GameState>,
) {
    if !game_state.light_debug {
        return;
    }

    for (light, transform) in &query {
        gizmos.sphere(transform.translation, transform.rotation, light.radius, light.color);
    }
}

fn spotlight_gizmos(
    mut gizmos: Gizmos,
    query: Query<(&SpotLight, &Transform)>,
    game_state: Res<GameState>,
) {
    if !game_state.light_debug {
        return;
    }

    for (light, transform) in &query {
        gizmos.line(transform.translation, transform.translation + transform.forward() * light.radius, light.color);
    }
}

fn bvh_gizmos (
    mut gizmos: Gizmos,
    query: Query<&SaveBVH>,
    game_state: Res<GameState>
) {
    for save_bvh in &query {
        match game_state.bvh_view {
            BVHView::On(depth) => {
                aabb_gizmos_recursive(&save_bvh.bvh, 0, &mut gizmos, 0, depth);
            },
            BVHView::Off => {}
        }
    }
}

fn aabb_gizmos_recursive(bvh: &BVH, node: usize, gizmos: &mut Gizmos, depth: u8, target_depth: u8) {
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

    match &bvh[node] {
        BVHNode::Internal { aabb, left, right } => {
            if depth == target_depth {
                gizmos.cuboid(Transform {
                    translation: aabb.center.as_vec3(),
                    rotation: Quat::IDENTITY,
                    scale: aabb.halfwidths.as_vec3() * 2.0,
                }, color);
                return;
            }

            aabb_gizmos_recursive(bvh, *left, gizmos, depth + 1, target_depth);
            aabb_gizmos_recursive(bvh, *right, gizmos, depth + 1, target_depth);
        },
        _ => {}
    }
}

fn change_depth(
    keycode: Res<Input<KeyCode>>,
    mut game_state: ResMut<GameState>,
) {
    match game_state.input {
        InputState::Listen => {
            match &mut game_state.bvh_view {
                BVHView::On(depth) => {
                    if keycode.just_pressed(KeyCode::X) {
                        *depth += 1;
                    }
                    if keycode.just_pressed(KeyCode::Z) {
                        *depth -= 1;
                    }
                }
                BVHView::Off => {}
            }

        },
        InputState::Typing => {}
    }
}
