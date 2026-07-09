mod aabb;
mod asset_loader;
mod brdb_load;
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
mod viewcube;

use std::{path::PathBuf, io::BufReader, fs::File, sync::mpsc::{Receiver, self}, thread};

use aabb::AABB;
use asset_loader::{AssetLoaderPlugin, SceneAssets};
use bevy::{diagnostic::FrameTimeDiagnosticsPlugin, pbr::DefaultOpaqueRendererMethod, prelude::*, tasks::{block_on, futures_lite::future, AsyncComputeTaskPool, Task}, window::{PrimaryWindow, WindowResolution}, winit::WinitWindows};
use bevy_egui::{egui, EguiContexts, EguiPlugin, EguiPrimaryContextPass};
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


// Result of the off-thread mesh-gen task; consumed by `poll_gen_task`.
struct LoadedBuild {
    save_data: SaveData,
    material_meshes: Vec<Vec<Mesh>>,
    com: Vec3,
    bvh: BVH,
    aabbs: Vec<AABB>,
}

// Holds the in-flight gen task so the main thread stays responsive (spinner
// animates) while faces/BVH/meshes are built on the compute pool.
#[derive(Resource)]
struct GenTask(Task<LoadedBuild>);

#[derive(Component)]
struct Water;

#[derive(Component)]
struct Ground;

#[derive(Component)]
struct ChunkMesh;

#[derive(Component)]
struct LoadPrompt;

// The prompt's text child: shows "Press L..." when idle, an animated spinner
// while a build loads.
#[derive(Component)]
struct LoadPromptText;

// ASCII spinner frames (braille glyphs aren't in the default font), cycled while loading.
const SPINNER_FRAMES: [&str; 4] = ["|", "/", "-", "\\"];
const LOAD_PROMPT_IDLE: &str = "Press L to load a build";

// Bottom-left controls hint, shown once a build is loaded.
#[derive(Component)]
struct ControlsHint;

const CONTROLS_HINT: &str = "\
WASD  move
Q / E or right-drag  rotate
scroll  zoom
shift  faster
H  recenter
F2  screenshot
F3  hi-res screenshot";

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Brickadia Isometric Viewer".into(),
                // present_mode: PresentMode::Immediate,
                resolution: WindowResolution::new(1600, 900),
                resize_constraints: WindowResizeConstraints {
                    min_width: 854.,
                    min_height: 480.,
                    ..default()
                },
                ..default()
            }),
            ..default()
        }))
        // MSAA is disabled per-camera (Msaa::Off) since it's incompatible with
        // deferred rendering; FXAA is used instead.
        .insert_resource(DefaultOpaqueRendererMethod::deferred())
        .insert_resource(GameState::default())
        .init_resource::<state::BuildLoaded>()
        .init_resource::<state::Loading>()
        .init_resource::<state::BrickInfoEnabled>()
        .init_resource::<state::Screenshotting>()
        .insert_resource(GlobalVolume::new(bevy::audio::Volume::Linear(0.2)))
        .add_plugins((LightPlugin, AssetLoaderPlugin, ChatPlugin, SettingsPlugin, IsoCameraPlugin, viewcube::ViewCubePlugin))
        .add_plugins((FrameTimeDiagnosticsPlugin::default(), FPSPlugin))
        .add_plugins(EguiPlugin::default())
        .add_plugins(EmbeddedAssetPlugin::default())
        .add_systems(Update, set_window_icon)
        .add_systems(PostStartup, setup)
        .add_systems(Update, (pick_path, load_brs, load_save, poll_gen_task, spawn_chunks, move_water))
        .add_systems(Update, (bvh_gizmos, change_depth, spotlight_gizmos, light_gizmos, toggle_load_prompt))
        // egui UI must run in the primary-context pass under bevy_egui's multi-pass mode
        .add_systems(EguiPrimaryContextPass, brick_info)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    assets: Res<SceneAssets>,
) {
    commands.spawn((
        AudioPlayer::new(assets.sounds.startup.clone()),
        PlaybackSettings::DESPAWN,
    ));

    // spawn water mesh
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(1000000., 1000000.))),
        MeshMaterial3d(assets.materials.water.clone()),
        Visibility::Hidden,
        Water,
    ));

    // spawn ground plane
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(1000000., 1000000.))),
        MeshMaterial3d(assets.materials.ground.clone()),
        Visibility::Hidden,
        Ground,
    ));

    // Centered prompt shown until a build is loaded.
    commands.spawn((
        LoadPrompt,
        state::HideOnScreenshot,
        Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.),
            height: Val::Percent(100.),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
        Pickable::IGNORE,
    )).with_child((
        LoadPromptText,
        Text::new(LOAD_PROMPT_IDLE),
        TextFont { font_size: FontSize::Px(28.0), ..default() },
        TextColor(Color::WHITE),
    ));

    // Controls hint, bottom-left; hidden until a build is loaded.
    commands.spawn((
        ControlsHint,
        state::HideOnScreenshot,
        Visibility::Hidden,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(12.),
            bottom: Val::Px(12.),
            ..default()
        },
        Pickable::IGNORE,
        Text::new(CONTROLS_HINT),
        TextFont { font_size: FontSize::Px(14.0), ..default() },
        TextColor(Color::srgba(1., 1., 1., 0.65)),
    ));
}

// Drive the centered prompt: hidden once a build is loaded, an animated spinner
// while one is loading, and the "Press L..." hint otherwise.
fn toggle_load_prompt(
    build_loaded: Res<state::BuildLoaded>,
    loading: Res<state::Loading>,
    time: Res<Time>,
    mut prompt: Query<&mut Visibility, (With<LoadPrompt>, Without<ControlsHint>)>,
    mut label: Query<&mut Text, With<LoadPromptText>>,
    mut controls: Query<&mut Visibility, (With<ControlsHint>, Without<LoadPrompt>)>,
) {
    let Ok(mut vis) = prompt.single_mut() else { return; };
    let Ok(mut text) = label.single_mut() else { return; };

    if let Ok(mut cvis) = controls.single_mut() {
        *cvis = if build_loaded.0 { Visibility::Visible } else { Visibility::Hidden };
    }

    if build_loaded.0 {
        *vis = Visibility::Hidden;
        return;
    }

    *vis = Visibility::Visible;
    if loading.0 {
        let frame = (time.elapsed_secs() * 12.0) as usize % SPINNER_FRAMES.len();
        text.0 = SPINNER_FRAMES[frame].to_string();
    } else {
        text.0 = LOAD_PROMPT_IDLE.to_string();
    }
}

fn brick_info(
    window_query: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform), With<IsoCamera>>,
    bvh_query: Query<&SaveBVH>,
    viewcube_hover: Res<viewcube::ViewCubeHover>,
    brick_info_enabled: Res<state::BrickInfoEnabled>,
    screenshotting: Res<state::Screenshotting>,
    mut contexts: EguiContexts,
    mut gizmos: Gizmos,
) {
    // Off by default; toggled via the `/brickinfo` console command. Also hidden
    // while a screenshot is being captured.
    if !brick_info_enabled.0 || screenshotting.0 {
        return;
    }

    // The view cube owns the cursor while hovered.
    if viewcube_hover.0 {
        return;
    }

    for save_bvh in bvh_query.iter() {
        let window = match window_query.single() {
            Ok(window) => window,
            Err(_) => return,
        };
        let mouse_pos: Vec2 = match window.cursor_position() {
            Some(pos) => pos,
            None => return,
        };
    
        for (camera, camera_transform) in cameras.iter() {
            let ray = camera.viewport_to_world(camera_transform, mouse_pos);

            if let Ok(ray) = ray {
                let brick_index = save_bvh.bvh.intersection(ray, &save_bvh.aabbs);
                if let Some(brick_index) = brick_index {
                    let brick = &save_bvh.save_data.bricks[brick_index];
                    let asset_name = &save_bvh.save_data.header2.brick_assets[brick.asset_name_index as usize];

                    let owner = if brick.owner_index == 0 {
                        "PUBLIC"
                    } else {
                        save_bvh.save_data.header2.brick_owners[brick.owner_index as usize - 1].name.as_str()
                    };

                    if let Ok(ctx) = contexts.ctx_mut() {
                        egui::Window::new("Brick Info").show(ctx, |ui| {
                            ui.label(format!("Brick index: {}", brick_index));
                            ui.label(format!("Brick position: {:?}", brick.position));
                            ui.label(format!("Brick size: {:?}", brick.size));
                            ui.label(format!("Brick asset: {}", asset_name));
                            ui.label(format!("Brick owner: {}", owner));
                        });
                    }
    
                    let aabb = save_bvh.aabbs[brick_index];
                    gizmos.primitive_3d(
                        &Cuboid { half_size: aabb.halfwidths.as_vec3() },
                        aabb.center.as_vec3(),
                        Color::WHITE,
                    );
                }
            }
        }
    }
}

fn set_window_icon(
    // WinitWindows is a non-send resource that isn't available until winit has
    // created the window, so guard it and only run until the icon is set.
    windows: Option<NonSend<WinitWindows>>,
    mut done: Local<bool>,
) {
    if *done {
        return;
    }
    let Some(windows) = windows else { return; };

    let rgba = icon::ICON.to_vec();
    let icon = Icon::from_rgba(rgba, 32, 32).unwrap();

    for window in windows.windows.values() {
        window.set_window_icon(Some(icon.clone()));
    }
    *done = true;
}

fn move_water(
    mut query: Query<&mut Transform, With<Water>>,
    keycode: Res<ButtonInput<KeyCode>>,
    game_state: Res<GameState>,
    time: Res<Time>,
) {
    if !game_state.input_listening() {
        return;
    }

    let mut movement = Vec3::ZERO;
    if keycode.pressed(KeyCode::KeyI) {
        movement += Vec3::Y;
    } else if keycode.pressed(KeyCode::KeyK) {
        movement += Vec3::NEG_Y;
    }

    if keycode.pressed(KeyCode::ShiftLeft) {
        movement *= 10.0;
    }

    let Ok(mut transform) = query.single_mut() else { return; };
    transform.translation += movement * time.delta_secs() * 50.0;
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
    let keycode = world.resource::<ButtonInput<KeyCode>>();
    if keycode.just_pressed(KeyCode::KeyL) || keycode.just_pressed(KeyCode::KeyO) {
        let (tx, rx) = mpsc::channel();
        world.insert_non_send(rx);
        thread::spawn(move || {
            tx.send(ask_save_path()).unwrap();
        });
    }
}

fn load_brs(
    world: &mut World
) {
    let path_receiver = world.get_non_send::<Receiver<PathBuf>>();
    if let Some(path_receiver) = path_receiver {
        let path = path_receiver.try_recv();
        if path.is_err() {
            return;
        }

        let assets = world.resource::<SceneAssets>();
        world.spawn((
            AudioPlayer::new(assets.sounds.upload_start.clone()),
            PlaybackSettings::DESPAWN,
        ));
        world.resource_mut::<state::Loading>().0 = true;

        let path = path.unwrap();
        let (tx, rx) = mpsc::channel();
        world.insert_non_send(rx);
        thread::spawn(move || {
            let save_data = load_save_data(path);
            tx.send(save_data).unwrap();
        });
    }
}

// Kick mesh gen onto the async compute pool so the main thread (and spinner)
// stays live during the heavy face/BVH/mesh build.
fn load_save(
    mut commands: Commands,
    save_receiver: Option<NonSend<Receiver<SaveData>>>,
) {
    let Some(save_receiver) = save_receiver else {
        return;
    };
    let Ok(save_data) = save_receiver.try_recv() else {
        return;
    };
    info!("Loaded {:?} bricks", &save_data.bricks.len());

    let task = AsyncComputeTaskPool::get().spawn(async move {
        // generator borrows save_data; move the owned results out and let it
        // drop before handing save_data back to the main world
        let (material_meshes, com, bvh, aabbs) = {
            let generator = BVHMeshGenerator::new(&save_data);
            let material_meshes = generator.gen_mesh();
            let com = generator.center_of_mass();
            (material_meshes, com, generator.bvh, generator.aabbs)
        };
        LoadedBuild { save_data, material_meshes, com, bvh, aabbs }
    });
    commands.insert_resource(GenTask(task));
}

// Drain the finished gen task: spawn lights, chunk entities, and the BVH, then
// clear the loading state.
fn poll_gen_task(
    mut commands: Commands,
    mut cam_query: Query<&mut IsoCamera>,
    mut build_loaded: ResMut<state::BuildLoaded>,
    mut loading: ResMut<state::Loading>,
    assets: Res<SceneAssets>,
    task: Option<ResMut<GenTask>>,
) {
    let Some(mut task) = task else {
        return;
    };
    let Some(loaded) = block_on(future::poll_once(&mut task.0)) else {
        return;
    };
    commands.remove_resource::<GenTask>();

    let LoadedBuild { save_data, material_meshes, com, bvh, aabbs } = loaded;

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

    if let Ok(mut cam) = cam_query.single_mut() {
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

    commands.spawn(SaveBVH {
        bvh,
        save_data,
        aabbs,
        com,
    });

    commands.spawn((
        AudioPlayer::new(assets.sounds.upload_end.clone()),
        PlaybackSettings::DESPAWN,
    ));

    build_loaded.0 = true;
    loading.0 = false;
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
                    Mesh3d(meshes.add(mesh)),
                    MeshMaterial3d(chunk_entity.material.clone()),
                    ChunkMesh,
                ));
            }
        }
    }
}

fn ask_save_path() -> PathBuf {
    rfd::FileDialog::new()
        .add_filter("Brickadia Save", &["brs", "brdb", "brz"])
        .set_directory(default_build_directory().unwrap())
        .pick_file()
        .unwrap()
}

fn load_save_data(path: PathBuf) -> SaveData {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    match ext.as_str() {
        "brdb" => brdb_load::load_brdb_world(&path).expect("failed to load .brdb world"),
        "brz" => brdb_load::load_brz_prefab(&path).expect("failed to load .brz prefab"),
        _ => SaveReader::new(BufReader::new(File::open(path).unwrap()))
            .unwrap()
            .read_all()
            .unwrap(),
    }
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
        gizmos.sphere(transform.translation, light.radius, light.color);
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
        1 => Color::srgb(0.0, 0.0, 1.0),
        2 => Color::srgb(0.0, 1.0, 0.0),
        3 => Color::srgb(1.0, 1.0, 0.0),
        4 => Color::srgb(0.5, 0.0, 0.0),
        5 => Color::srgb(1.0, 0.84, 0.0),
        6 => Color::srgb(0.93, 0.51, 0.93),
        _ => Color::WHITE,
    };

    match &bvh[node] {
        BVHNode::Internal { aabb, left, right } => {
            if depth == target_depth {
                gizmos.primitive_3d(
                    &Cuboid { half_size: aabb.halfwidths.as_vec3() },
                    aabb.center.as_vec3(),
                    color,
                );
                return;
            }

            aabb_gizmos_recursive(bvh, *left, gizmos, depth + 1, target_depth);
            aabb_gizmos_recursive(bvh, *right, gizmos, depth + 1, target_depth);
        },
        _ => {}
    }
}

fn change_depth(
    keycode: Res<ButtonInput<KeyCode>>,
    mut game_state: ResMut<GameState>,
) {
    match game_state.input {
        InputState::Listen => {
            match &mut game_state.bvh_view {
                BVHView::On(depth) => {
                    if keycode.just_pressed(KeyCode::KeyX) {
                        *depth += 1;
                    }
                    if keycode.just_pressed(KeyCode::KeyZ) {
                        *depth -= 1;
                    }
                }
                BVHView::Off => {}
            }

        },
        InputState::Typing => {}
    }
}
