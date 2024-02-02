use bevy::{core_pipeline::{prepass::{MotionVectorPrepass, DepthPrepass, DeferredPrepass}, fxaa::Fxaa}, input::mouse::{MouseMotion, MouseWheel}, pbr::ClusterConfig, prelude::*, render::{camera::ScalingMode, view::screenshot::ScreenshotManager}, window::PrimaryWindow};

use crate::{state::GameState, SaveBVH};

const DEFAULT_CAMERA_ZOOM: f32 = 800.0;
const ISO_SCALING_MODE: f32 = 2.0;
const CAM_CLIP_DIST: f32 = 4000000.0;
const CAM_DIST: f32 = 100000.0;
const ZOOM_SPEED: f32 = 12.0;
const MIN_ZOOM: f32 = 1.0;
const MAX_ZOOM: f32 = 100000.0;

const CAM_Y: Vec3 = Vec3::new(0.0, CAM_DIST, 0.0);

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

const NORMAL_BUTTON: Color = Color::rgba(0.25, 0.25, 0.25, 0.5);
const HOVERED_BUTTON: Color = Color::rgba(0.35, 0.35, 0.35, 0.5);
const PRESSED_BUTTON: Color = Color::rgb(0.45, 0.45, 0.45);

impl Plugin for IsoCameraPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(Startup, spawn_camera)
            .add_systems(Update, (screenshot_on_f2, move_cam_keyboard, move_cam_mouse, camera_buttons, jump_home, update_transform, rotate_keyboard))
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
        Camera3dBundle {
            projection: OrthographicProjection {
                scale: DEFAULT_CAMERA_ZOOM,
                scaling_mode: ScalingMode::FixedVertical(ISO_SCALING_MODE),
                far: CAM_CLIP_DIST,
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
    ));

    commands.spawn((
        ButtonBundle {
            style: Style {
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
            border_color: BorderColor(Color::BLACK),
            background_color: BackgroundColor(NORMAL_BUTTON),
            ..default()
        },
        CamButton::new(ViewType::Top),
    )).with_children(|parent| {
        parent.spawn(TextBundle::from_section("View", TextStyle {
            color: Color::BLACK,
            ..default()
        }));
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
        ButtonBundle {
            style: Style {
                width: Val::Px(width),
                height: Val::Px(height),
                border: UiRect::all(Val::Px(1.0)),
                position_type: PositionType::Absolute,
                right: Val::Px(right),
                bottom: Val::Px(bottom),
                ..default()
            },
            border_color: BorderColor(Color::BLACK),
            background_color: BackgroundColor(NORMAL_BUTTON),
            ..default()
        },
        CamButton::new(view_type),
    ));
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
    mut cam_query: Query<(&Transform, &mut IsoCamera)>,
    projection_query: Query<&Projection>,
    mut motion_evr: EventReader<MouseMotion>,
    mouse: Res<Input<MouseButton>>,
) {
    if !mouse.pressed(MouseButton::Left) || mouse.just_pressed(MouseButton::Left) {
        return;
    }

    let projection = projection_query.get_single().unwrap();
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

    let (transform, mut cam) = cam_query.get_single_mut().unwrap();

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

                let mut cam = cam_query.get_single_mut().unwrap();
                cam.horizontal_angle = horizontal_angle;
                cam.vertical_angle = vertical_angle;

                *color = PRESSED_BUTTON.into();
                border_color.0 = Color::RED;
            }
            Interaction::Hovered => {
                *color = HOVERED_BUTTON.into();
                border_color.0 = Color::WHITE;
            }
            Interaction::None => {
                *color = NORMAL_BUTTON.into();
                border_color.0 = Color::BLACK;
            }
        }
    }
}

fn move_cam_keyboard(
    mut cam_query: Query<&mut IsoCamera>,
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

    let mut main_cam = cam_query.get_single_mut().unwrap();
    let delta = movement * time.delta_seconds() * 500.0;
    main_cam.target += delta;
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

fn jump_home(
    mut query: Query<&mut IsoCamera>,
    bvh_query: Query<&SaveBVH>,
    keyboard: Res<Input<KeyCode>>,
    game_state: Res<GameState>,
) {
    if !keyboard.just_pressed(KeyCode::H) || !game_state.input_listening() {
        return;
    }

    if let Ok(mut cam) = query.get_single_mut() {
        if let Some(bvh) = bvh_query.iter().last() {
            cam.target = bvh.com;
        } else {
            cam.target = Vec3::ZERO;
        }
    }
}

// Process changes to camera target
fn update_transform(
    mut query: Query<(&mut Transform, &mut IsoCamera), Changed<IsoCamera>>
) {
    for (mut transform, mut cam) in query.iter_mut() {
        let rotate_z = Quat::from_axis_angle(Vec3::NEG_Z, cam.vertical_angle.to_radians());
        let rotate_y = Quat::from_axis_angle(Vec3::Y, -cam.horizontal_angle.to_radians());
        let rotation = rotate_y * rotate_z;
    
        let translation = rotation.mul_vec3(CAM_Y) + cam.target;
    
        let up = if cam.vertical_angle == 0.0 {
            rotate_y.mul_vec3(Vec3::NEG_Z)
        } else {
            Vec3::Y
        };
    
        *transform = Transform::from_translation(translation).looking_at(cam.target, up);
    
        if cam.horizontal_angle >= 360. || cam.horizontal_angle < -360. {
            cam.horizontal_angle = 0.0;
        }
    }
}

fn rotate_keyboard(
    mut query: Query<&mut IsoCamera>,
    keyboard: Res<Input<KeyCode>>,
    game_state: Res<GameState>,
    time: Res<Time>,
) {
    if !game_state.input_listening() {
        return;
    }

    let mut delta: f32 = 0.0;
    if keyboard.pressed(KeyCode::Q) {
        delta += 1.0;
    } 
    if keyboard.pressed(KeyCode::E) {
        delta -= 1.0;
    }
    if keyboard.pressed(KeyCode::ShiftLeft) {
        delta *= 10.0;
    }

    if delta.abs() < f32::EPSILON {
        return;
    }

    let mut cam = query.get_single_mut().unwrap();
    cam.horizontal_angle += delta * time.delta_seconds() * 20.0;
}
