use bevy::{core_pipeline::{prepass::{MotionVectorPrepass, DepthPrepass, DeferredPrepass}, fxaa::Fxaa}, input::mouse::{MouseMotion, MouseWheel}, pbr::ClusterConfig, prelude::*, render::{camera::ScalingMode, view::screenshot::ScreenshotManager}, window::PrimaryWindow};

use crate::state::GameState;

const DEFAULT_CAMERA_ZOOM: f32 = 800.0;
const ISO_SCALING_MODE: f32 = 1.0;
const CAMERA_CLIP_DISTANCE: f32 = 4000000.0;
const CAMERA_DISTANCE: f32 = 100000.0;
const ZOOM_SPEED: f32 = 12.0;
const MIN_ZOOM: f32 = 1.0;
const MAX_ZOOM: f32 = 100000.0;

pub struct IsoCameraPlugin;

#[derive(Component)]
struct MainCamera;

impl Plugin for IsoCameraPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(Startup, spawn_camera)
            .add_systems(Update, (screenshot_on_f2, move_cam_keyboard, move_cam_mouse))
            .add_systems(FixedUpdate, zoom_cam);
    }
}

fn spawn_camera(
    mut commands: Commands,
) {
    let default_translation = Vec3::new(CAMERA_DISTANCE, CAMERA_DISTANCE, CAMERA_DISTANCE);
    let default_transform = Transform::from_translation(default_translation).looking_at(Vec3::ZERO, Vec3::Y);

    commands.spawn((
        MainCamera,
        Camera3dBundle {
            projection: OrthographicProjection {
                scale: DEFAULT_CAMERA_ZOOM,
                scaling_mode: ScalingMode::FixedVertical(ISO_SCALING_MODE),
                far: CAMERA_CLIP_DISTANCE,
                ..default()
            }.into(),
            transform: default_transform,
            ..default()
        },
        ClusterConfig::Single,
        DepthPrepass,
        MotionVectorPrepass,
        DeferredPrepass,
        Fxaa::default(),
)   );
}

fn screenshot_on_f2(
    input: Res<Input<KeyCode>>,
    main_window: Query<Entity, With<PrimaryWindow>>,
    mut screenshot_manager: ResMut<ScreenshotManager>,
    mut counter: Local<u32>,
) {
    if input.just_pressed(KeyCode::F2) {
        let path = format!("./screenshot-{}.png", *counter);
        *counter += 1;
        screenshot_manager
            .save_screenshot_to_disk(main_window.single(), path)
            .unwrap();
    }
}

fn move_cam_mouse(
    mut cam_query: Query<&mut Transform, With<MainCamera>>,
    projection_query: Query<&Projection>,
    mut motion_evr: EventReader<MouseMotion>,
    mouse: Res<Input<MouseButton>>,
    time: Res<Time>,
) {
    if !mouse.pressed(MouseButton::Left) || mouse.just_pressed(MouseButton::Left) {
        return;
    }

    let projection = projection_query.get_single().unwrap();
    let scale = match projection {
        Projection::Orthographic(projection) => {
            projection.scale
        },
        _ => { 1.0 }
    };

    let mut motion = Vec3::ZERO;

    for ev in motion_evr.read() {
        motion += Vec3::new(-ev.delta.x, 0.0, ev.delta.y);
    }

    let mut transform = cam_query.get_single_mut().unwrap();

    let move_x = transform.local_x() * motion.x;
    let move_z = transform.local_y() * motion.z;

    transform.translation += (move_x + move_z) * time.delta_seconds() * 0.35 * scale;
}

fn move_cam_keyboard(
    mut cam_query: Query<&mut Transform, With<MainCamera>>,
    keyboard: Res<Input<KeyCode>>,
    game_state: Res<GameState>,
    time: Res<Time>,
) {
    if !game_state.input_listening() {
        return;
    }

    let mut movement = Vec3::ZERO;

    if keyboard.pressed(KeyCode::W) {
        movement += Vec3::NEG_Z;
    } else if keyboard.pressed(KeyCode::S) {
        movement += Vec3::Z;
    }
    if keyboard.pressed(KeyCode::D) {
        movement += Vec3::X;
    } else if keyboard.pressed(KeyCode::A) {
        movement += Vec3::NEG_X;
    }

    movement = movement.normalize_or_zero();

    if keyboard.pressed(KeyCode::ShiftLeft) {
        movement *= 10.0;
    }

    let mut transform = cam_query.get_single_mut().unwrap();
    transform.translation += movement * time.delta_seconds() * 500.0;
}

fn zoom_cam(
    mut scroll_evr: EventReader<MouseWheel>,
    mut query: Query<&mut Projection>,
    time: Res<Time>,
) {
    let mut zoom_delta = 0.;

    use bevy::input::mouse::MouseScrollUnit;
    for ev in scroll_evr.read() {
        match ev.unit {
            MouseScrollUnit::Line => {
                // TODO: test on laptop and adjust sensitivity
                zoom_delta += ev.y * 1.0;
            },
            MouseScrollUnit::Pixel => {
                zoom_delta += ev.y;
            }
        }
    }

    if zoom_delta == 0. {
        return;
    }

    for mut projection in query.iter_mut() {
        match projection.as_mut() {
            Projection::Orthographic(projection) => {
                let mut log_scale = projection.scale.ln();
                log_scale -= zoom_delta * time.delta_seconds() * ZOOM_SPEED;
                projection.scale = log_scale.exp().clamp(MIN_ZOOM, MAX_ZOOM);
            },
            _ => {}
        }
    }
}
