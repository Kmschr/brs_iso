use bevy::{anti_alias::fxaa::Fxaa, camera::ScalingMode, core_pipeline::prepass::{MotionVectorPrepass, DepthPrepass, DeferredPrepass}, input::mouse::{MouseMotion, MouseWheel}, light::cluster::ClusterConfig, prelude::*, render::view::screenshot::{save_to_disk, Screenshot}};

use crate::{bvh::BVHNode, state::{GameState, HideOnScreenshot, Screenshotting}, SaveBVH};

const DEFAULT_CAMERA_ZOOM: f32 = 800.0;
const ISO_SCALING_MODE: f32 = 2.0;
const CAM_CLIP_DIST: f32 = 4000000.0;
const CAM_DIST: f32 = 100000.0;
const ZOOM_SPEED: f32 = 12.0;
const MIN_ZOOM: f32 = 1.0;
const MAX_ZOOM: f32 = 100000.0;

pub struct IsoCameraPlugin;

#[derive(Component, Default)]
pub struct IsoCamera {
    pub target: Vec3,
    pub horizontal_angle: f32,
    pub vertical_angle: f32,
}

impl Plugin for IsoCameraPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(Startup, spawn_camera)
            .init_resource::<ScreenshotSeq>()
            .add_systems(Update, (screenshot_sequence, move_cam_keyboard, move_cam_mouse, jump_home, update_transform, rotate_keyboard, rotate_mouse))
            .add_systems(FixedUpdate, zoom_cam);
    }
}

fn spawn_camera(
    mut commands: Commands,
) {
    let default_translation = Vec3::new(CAM_DIST, CAM_DIST, CAM_DIST);
    let default_transform = Transform::from_translation(default_translation).looking_at(Vec3::ZERO, Vec3::Y);

    commands.spawn((
        IsoCamera {
            horizontal_angle: 45.0,
            vertical_angle: 45.0,
            ..default()
        },
        Camera3d::default(),
        Projection::Orthographic(OrthographicProjection {
            scale: DEFAULT_CAMERA_ZOOM,
            scaling_mode: ScalingMode::FixedVertical { viewport_height: ISO_SCALING_MODE },
            far: CAM_CLIP_DIST,
            ..OrthographicProjection::default_3d()
        }),
        default_transform,
        // Gentle ambient fill lifts shadows off pure black without washing them out.
        // (A second *directional* fill light would flatten/erase the sun's shadows.)
        AmbientLight {
            color: Color::WHITE,
            brightness: 600.0,
            ..default()
        },
        // The view cube adds a second camera; UI must anchor to this one.
        bevy::ui::IsDefaultUiCamera,
        // Disable MSAA as it is incompatible with deferred rendering, use FXAA instead
        Msaa::Off,
        ClusterConfig::Single,
        DepthPrepass,
        MotionVectorPrepass,
        DeferredPrepass,
        Fxaa::default(),
    ));
}

#[derive(Default)]
enum ShotPhase {
    #[default]
    Idle,
    // UI hidden this frame; capture on the next so the hide has rendered.
    Capture,
    // Capture spawned; restore UI visibility next frame.
    Restore,
}

#[derive(Resource, Default)]
struct ScreenshotSeq {
    phase: ShotPhase,
    counter: u32,
    // Prior visibility of each hidden overlay, restored after capture.
    saved: Vec<(Entity, Visibility)>,
}

// F2 hides overlay UI, waits a frame, captures, then restores. `Screenshotting`
// lets non-Visibility UI (view cube, egui brick info) opt out during capture.
fn screenshot_sequence(
    input: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    mut seq: ResMut<ScreenshotSeq>,
    mut screenshotting: ResMut<Screenshotting>,
    mut overlays: Query<(Entity, &mut Visibility), With<HideOnScreenshot>>,
) {
    match seq.phase {
        ShotPhase::Idle => {
            if input.just_pressed(KeyCode::F2) {
                seq.saved.clear();
                for (entity, mut vis) in overlays.iter_mut() {
                    seq.saved.push((entity, *vis));
                    *vis = Visibility::Hidden;
                }
                seq.phase = ShotPhase::Capture;
                screenshotting.0 = true;
            }
        }
        ShotPhase::Capture => {
            let path = format!("./screenshot-{}.png", seq.counter);
            seq.counter += 1;
            commands
                .spawn(Screenshot::primary_window())
                .observe(save_to_disk(path));
            seq.phase = ShotPhase::Restore;
        }
        ShotPhase::Restore => {
            for (entity, vis) in seq.saved.drain(..) {
                if let Ok((_, mut current)) = overlays.get_mut(entity) {
                    *current = vis;
                }
            }
            screenshotting.0 = false;
            seq.phase = ShotPhase::Idle;
        }
    }
}

fn move_cam_mouse(
    mut cam_query: Query<(&Transform, &mut IsoCamera)>,
    projection_query: Query<&Projection, With<IsoCamera>>,
    mut motion_evr: MessageReader<MouseMotion>,
    mouse: Res<ButtonInput<MouseButton>>,
) {
    if !mouse.pressed(MouseButton::Left) || mouse.just_pressed(MouseButton::Left) {
        return;
    }

    let projection = projection_query.single().unwrap();
    let scale = match projection {
        Projection::Orthographic(projection) => {
            projection.scale
        },
        _ => {
            return;
        }
    };

    let mut motion = Vec2::ZERO;

    for ev in motion_evr.read() {
        motion += Vec2::new(-ev.delta.x, ev.delta.y);
    }

    // filter out big jumps
    if motion.length() > 100. {
        return;
    }

    let (transform, mut cam) = cam_query.single_mut().unwrap();

    let move_x = transform.local_x() * motion.x;
    let move_z = transform.local_y() * motion.y;

    cam.target += (move_x + move_z) * 0.0015 * scale;
}

fn move_cam_keyboard(
    mut cam_query: Query<&mut IsoCamera>,
    keyboard: Res<ButtonInput<KeyCode>>,
    game_state: Res<GameState>,
    time: Res<Time>,
) {
    if !game_state.input_listening() {
        return;
    }

    let mut movement = Vec3::ZERO;

    if keyboard.pressed(KeyCode::KeyW) {
        movement += Vec3::NEG_Z;
    } else if keyboard.pressed(KeyCode::KeyS) {
        movement += Vec3::Z;
    }
    if keyboard.pressed(KeyCode::KeyD) {
        movement += Vec3::X;
    } else if keyboard.pressed(KeyCode::KeyA) {
        movement += Vec3::NEG_X;
    }

    movement = movement.normalize_or_zero();

    if keyboard.pressed(KeyCode::ShiftLeft) {
        movement *= 10.0;
    }

    let mut main_cam = cam_query.single_mut().unwrap();
    let delta = movement * time.delta_secs() * 500.0;
    main_cam.target += delta;
}

fn zoom_cam(
    mut scroll_evr: MessageReader<MouseWheel>,
    mut query: Query<&mut Projection, With<IsoCamera>>,
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
                log_scale -= zoom_delta * time.delta_secs() * ZOOM_SPEED;
                projection.scale = log_scale.exp().clamp(MIN_ZOOM, MAX_ZOOM);
            },
            _ => {}
        }
    }
}

fn jump_home(
    mut query: Query<&mut IsoCamera>,
    bvh_query: Query<&SaveBVH>,
    keyboard: Res<ButtonInput<KeyCode>>,
    game_state: Res<GameState>,
) {
    if !keyboard.just_pressed(KeyCode::KeyH) || !game_state.input_listening() {
        return;
    }

    if let Ok(mut cam) = query.single_mut() {
        if let Some(bvh) = bvh_query.iter().last() {
            cam.target = bvh.com;
        } else {
            cam.target = Vec3::ZERO;
        }
    }
}

// Process changes to camera target
fn update_transform(
    mut query: Query<(&mut Transform, &mut IsoCamera), Changed<IsoCamera>>,
    bvh_query: Query<&SaveBVH>,
) {
    for (mut transform, mut cam) in query.iter_mut() {
        let rotate_z = Quat::from_axis_angle(Vec3::NEG_Z, cam.vertical_angle.to_radians());
        let rotate_y = Quat::from_axis_angle(Vec3::Y, -cam.horizontal_angle.to_radians());
        let rotation = rotate_y * rotate_z;

        let mut max_dist = 0.0;
        for save_bvh in bvh_query.iter() {
            let root = &save_bvh.bvh[0];
            let aabb = match root {
                BVHNode::Internal { aabb, left: _, right: _ } => aabb,
                _ => { continue; }
            };
            let max_side = aabb.halfwidths.x.max(aabb.halfwidths.y).max(aabb.halfwidths.z);
            let dist = max_side as f32 * 2.0;

            if dist > max_dist {
                max_dist = dist;
            }
        }

        let translation = rotation.mul_vec3(Vec3::new(0.0, max_dist, 0.0)) + cam.target;

        let up = if cam.vertical_angle == 0.0 {
            rotate_y.mul_vec3(Vec3::NEG_Z)
        } else {
            Vec3::Y
        };

        *transform = Transform::from_translation(translation).looking_at(cam.target, up);

        if cam.horizontal_angle >= 360. || cam.horizontal_angle < -360. {
            cam.horizontal_angle = 0.0;
        }

        cam.vertical_angle = cam.vertical_angle.clamp(0.0, 90.0);
    }
}

fn rotate_keyboard(
    mut query: Query<&mut IsoCamera>,
    keyboard: Res<ButtonInput<KeyCode>>,
    game_state: Res<GameState>,
    time: Res<Time>,
) {
    if !game_state.input_listening() {
        return;
    }

    let mut delta: f32 = 0.0;
    if keyboard.pressed(KeyCode::KeyQ) {
        delta += 1.0;
    }
    if keyboard.pressed(KeyCode::KeyE) {
        delta -= 1.0;
    }
    if keyboard.pressed(KeyCode::ShiftLeft) {
        delta *= 10.0;
    }

    if delta.abs() < f32::EPSILON {
        return;
    }

    let mut cam = query.single_mut().unwrap();
    cam.horizontal_angle += delta * time.delta_secs() * 20.0;
}

fn rotate_mouse(
    mut query: Query<&mut IsoCamera>,
    mut motion_evr: MessageReader<MouseMotion>,
    mouse: Res<ButtonInput<MouseButton>>,
) {
    if !mouse.pressed(MouseButton::Right) {
        return;
    }

    let mut motion = Vec2::ZERO;

    for ev in motion_evr.read() {
        motion += Vec2::new(ev.delta.x, -ev.delta.y);
    }

    // filter out big jumps
    if motion.length() > 100. {
        return;
    }

    for mut cam in query.iter_mut() {
        cam.vertical_angle += motion.y * 0.1;
        cam.horizontal_angle += motion.x * 0.1;

        cam.vertical_angle = cam.vertical_angle.clamp(0.0, 90.0);
    }
}
