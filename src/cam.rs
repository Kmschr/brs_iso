use bevy::{prelude::*, render::{camera::ScalingMode, view::screenshot::ScreenshotManager}, core_pipeline::{prepass::{MotionVectorPrepass, DepthPrepass, DeferredPrepass}, fxaa::Fxaa}, window::PrimaryWindow, pbr::ClusterConfig};
use bevy_panorbit_camera::{PanOrbitCameraPlugin, PanOrbitCamera};

const DEFAULT_CAMERA_ZOOM: f32 = 800.0;
const ISO_SCALING_MODE: f32 = 1.0;
const CAMERA_CLIP_DISTANCE: f32 = 4000000.0;
const CAMERA_DISTANCE: f32 = 100000.0;

pub struct IsoCameraPlugin;

#[derive(Component)]
struct OrientationCube;

impl Plugin for IsoCameraPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_plugins(PanOrbitCameraPlugin)
            .add_systems(Startup, spawn_camera)
            .add_systems(Update, screenshot_on_f2);
    }
}

fn spawn_camera(
    mut commands: Commands,
) {
    commands.spawn((Camera3dBundle {
        projection: OrthographicProjection {
            scale: DEFAULT_CAMERA_ZOOM,
            scaling_mode: ScalingMode::FixedVertical(ISO_SCALING_MODE),
            far: CAMERA_CLIP_DISTANCE,
            ..default()
        }.into(),
        transform: Transform::from_translation(Vec3::new(CAMERA_DISTANCE, CAMERA_DISTANCE, CAMERA_DISTANCE)),
        ..default()
    },
    ClusterConfig::Single,
    PanOrbitCamera::default(),
    DepthPrepass,
    MotionVectorPrepass,
    DeferredPrepass,
    Fxaa::default()));
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
