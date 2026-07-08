use bevy::{anti_alias::fxaa::Fxaa, camera::ScalingMode, core_pipeline::prepass::{MotionVectorPrepass, DepthPrepass, DeferredPrepass}, input::mouse::{MouseMotion, MouseWheel}, light::cluster::ClusterConfig, prelude::*, render::view::screenshot::{save_to_disk, Screenshot}};

use crate::{bvh::BVHNode, state::GameState, SaveBVH};

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

#[derive(Component)]
struct CamButton {
    view: ViewType,
}

impl CamButton {
    fn new(view: ViewType) -> Self {
        Self {
            view,
        }
    }
}

#[derive(Default)]
enum ViewType {
    Top,
    Left,
    Right,
    Back,
    Front,
    #[default]
    BottomRight,
    BottomLeft,
    TopRight,
    TopLeft,
}

const NORMAL_BUTTON: Color = Color::srgba(0.25, 0.25, 0.25, 0.5);
const HOVERED_BUTTON: Color = Color::srgba(0.35, 0.35, 0.35, 0.5);
const PRESSED_BUTTON: Color = Color::srgb(0.45, 0.45, 0.45);

impl Plugin for IsoCameraPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(Startup, spawn_camera)
            .add_systems(Update, (screenshot_on_f2, move_cam_keyboard, move_cam_mouse, camera_buttons, jump_home, update_transform, rotate_keyboard, rotate_mouse))
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
        // Disable MSAA as it is incompatible with deferred rendering, use FXAA instead
        Msaa::Off,
        ClusterConfig::Single,
        DepthPrepass,
        MotionVectorPrepass,
        DeferredPrepass,
        Fxaa::default(),
    ));

    commands.spawn((
        Button,
        Node {
            width: Val::Px(50.),
            height: Val::Px(50.),
            border: UiRect::all(Val::Px(1.0)),
            position_type: PositionType::Absolute,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            right: Val::Px(40.),
            bottom: Val::Px(40.),
            ..default()
        },
        BorderColor::all(Color::BLACK),
        BackgroundColor(NORMAL_BUTTON),
        CamButton::new(ViewType::Top),
    )).with_children(|parent| {
        parent.spawn((
            Text::new("View"),
            TextColor(Color::BLACK),
        ));
    });

    spawn_view_button(&mut commands, 20., 50., 20., 40., ViewType::Right);
    spawn_view_button(&mut commands, 20., 50., 90., 40., ViewType::Left);
    spawn_view_button(&mut commands, 50., 20., 40., 20., ViewType::Front);
    spawn_view_button(&mut commands, 50., 20., 40., 90., ViewType::Back);
    spawn_view_button(&mut commands, 20., 20., 20., 20., ViewType::BottomRight);
    spawn_view_button(&mut commands, 20., 20., 20., 90., ViewType::TopRight);
    spawn_view_button(&mut commands, 20., 20., 90., 90., ViewType::TopLeft);
    spawn_view_button(&mut commands, 20., 20., 90., 20., ViewType::BottomLeft);
}

fn spawn_view_button(commands: &mut Commands, width: f32, height: f32, right: f32, bottom: f32, view_type: ViewType) {
    commands.spawn((
        Button,
        Node {
            width: Val::Px(width),
            height: Val::Px(height),
            border: UiRect::all(Val::Px(1.0)),
            position_type: PositionType::Absolute,
            right: Val::Px(right),
            bottom: Val::Px(bottom),
            ..default()
        },
        BorderColor::all(Color::BLACK),
        BackgroundColor(NORMAL_BUTTON),
        CamButton::new(view_type),
    ));
}

fn screenshot_on_f2(
    input: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    mut counter: Local<u32>,
) {
    if input.just_pressed(KeyCode::F2) {
        let path = format!("./screenshot-{}.png", *counter);
        *counter += 1;
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(path));
    }
}

fn move_cam_mouse(
    mut cam_query: Query<(&Transform, &mut IsoCamera)>,
    projection_query: Query<&Projection>,
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

fn camera_buttons(
    mut interaction_query: Query<
        (
            &Interaction,
            &mut BackgroundColor,
            &mut BorderColor,
            &CamButton,
        ),
        (Changed<Interaction>, With<Button>),
    >,
    mut cam_query: Query<&mut IsoCamera>,
) {
    for (interaction, mut color, mut border_color, cam_button) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                let (horizontal_angle, vertical_angle) = match cam_button.view {
                    ViewType::Top => {
                        (0.0, 0.0)
                    },
                    ViewType::Front => {
                        (90.0, 90.0)
                    },
                    ViewType::Back => {
                        (270.0, 90.0)
                    },
                    ViewType::Right => {
                        (0.0, 90.0)
                    },
                    ViewType::Left => {
                        (180.0, 90.0)
                    },
                    ViewType::BottomRight => {
                        (45.0, 45.0)
                    },
                    ViewType::TopRight => {
                        (315.0, 45.0)
                    },
                    ViewType::TopLeft => {
                        (225.0, 45.0)
                    },
                    ViewType::BottomLeft => {
                        (135.0, 45.0)
                    },
                };

                let mut cam = cam_query.single_mut().unwrap();
                cam.horizontal_angle = horizontal_angle;
                cam.vertical_angle = vertical_angle;

                *color = PRESSED_BUTTON.into();
                *border_color = BorderColor::all(Color::srgb(1.0, 0.0, 0.0));
            }
            Interaction::Hovered => {
                *color = HOVERED_BUTTON.into();
                *border_color = BorderColor::all(Color::WHITE);
            }
            Interaction::None => {
                *color = NORMAL_BUTTON.into();
                *border_color = BorderColor::all(Color::BLACK);
            }
        }
    }
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
