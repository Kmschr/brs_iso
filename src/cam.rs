use bevy::{core_pipeline::{prepass::{MotionVectorPrepass, DepthPrepass, DeferredPrepass}, fxaa::Fxaa}, input::mouse::{MouseMotion, MouseWheel}, pbr::ClusterConfig, prelude::*, render::{camera::ScalingMode, view::screenshot::ScreenshotManager}, window::PrimaryWindow};

use crate::{bvh::BVHNode, state::GameState, SaveBVH};

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
    Custom
}

const NORMAL_BUTTON: Color = Color::rgba(0.25, 0.25, 0.25, 0.5);
const HOVERED_BUTTON: Color = Color::rgba(0.35, 0.35, 0.35, 0.5);
const PRESSED_BUTTON: Color = Color::rgb(0.45, 0.45, 0.45);

impl Plugin for IsoCameraPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(Startup, spawn_camera)
            .add_systems(Update, (screenshot_on_f2, move_cam_keyboard, move_cam_mouse, camera_buttons))
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

    commands.spawn((
        ButtonBundle {
            style: Style {
                width: Val::Px(20.),
                height: Val::Px(50.),
                border: UiRect::all(Val::Px(1.0)),
                position_type: PositionType::Absolute,
                right: Val::Px(20.),
                bottom: Val::Px(40.),
                ..default()
            },
            border_color: BorderColor(Color::BLACK),
            background_color: BackgroundColor(NORMAL_BUTTON),
            ..default()
        },
        CamButton::new(ViewType::Right),
    ));

    commands.spawn((
        ButtonBundle {
            style: Style {
                width: Val::Px(20.),
                height: Val::Px(50.),
                border: UiRect::all(Val::Px(1.0)),
                position_type: PositionType::Absolute,
                right: Val::Px(90.),
                bottom: Val::Px(40.),
                ..default()
            },
            border_color: BorderColor(Color::BLACK),
            background_color: BackgroundColor(NORMAL_BUTTON),
            ..default()
        },
        CamButton::new(ViewType::Left),
    ));

    commands.spawn((
        ButtonBundle {
            style: Style {
                width: Val::Px(50.),
                height: Val::Px(20.),
                border: UiRect::all(Val::Px(1.0)),
                position_type: PositionType::Absolute,
                right: Val::Px(40.),
                bottom: Val::Px(20.),
                ..default()
            },
            border_color: BorderColor(Color::BLACK),
            background_color: BackgroundColor(NORMAL_BUTTON),
            ..default()
        },
        CamButton::new(ViewType::Front),
    ));

    commands.spawn((
        ButtonBundle {
            style: Style {
                width: Val::Px(50.),
                height: Val::Px(20.),
                border: UiRect::all(Val::Px(1.0)),
                position_type: PositionType::Absolute,
                right: Val::Px(40.),
                bottom: Val::Px(90.),
                ..default()
            },
            border_color: BorderColor(Color::BLACK),
            background_color: BackgroundColor(NORMAL_BUTTON),
            ..default()
        },
        CamButton::new(ViewType::Back),
    ));

    commands.spawn((
        ButtonBundle {
            style: Style {
                width: Val::Px(20.),
                height: Val::Px(20.),
                border: UiRect::all(Val::Px(1.0)),
                position_type: PositionType::Absolute,
                right: Val::Px(20.),
                bottom: Val::Px(20.),
                ..default()
            },
            border_color: BorderColor(Color::BLACK),
            background_color: BackgroundColor(NORMAL_BUTTON),
            ..default()
        },
        CamButton::new(ViewType::BottomRight),
    ));

    commands.spawn((
        ButtonBundle {
            style: Style {
                width: Val::Px(20.),
                height: Val::Px(20.),
                border: UiRect::all(Val::Px(1.0)),
                position_type: PositionType::Absolute,
                right: Val::Px(20.),
                bottom: Val::Px(90.),
                ..default()
            },
            border_color: BorderColor(Color::BLACK),
            background_color: BackgroundColor(NORMAL_BUTTON),
            ..default()
        },
        CamButton::new(ViewType::TopRight),
    ));

    commands.spawn((
        ButtonBundle {
            style: Style {
                width: Val::Px(20.),
                height: Val::Px(20.),
                border: UiRect::all(Val::Px(1.0)),
                position_type: PositionType::Absolute,
                right: Val::Px(90.),
                bottom: Val::Px(90.),
                ..default()
            },
            border_color: BorderColor(Color::BLACK),
            background_color: BackgroundColor(NORMAL_BUTTON),
            ..default()
        },
        CamButton::new(ViewType::TopLeft),
    ));

    commands.spawn((
        ButtonBundle {
            style: Style {
                width: Val::Px(20.),
                height: Val::Px(20.),
                border: UiRect::all(Val::Px(1.0)),
                position_type: PositionType::Absolute,
                right: Val::Px(90.),
                bottom: Val::Px(20.),
                ..default()
            },
            border_color: BorderColor(Color::BLACK),
            background_color: BackgroundColor(NORMAL_BUTTON),
            ..default()
        },
        CamButton::new(ViewType::BottomLeft),
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
    let (scale, area) = match projection {
        Projection::Orthographic(projection) => {
            (projection.scale, projection.area)
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
    if motion.length() > 20. {
        return;
    }

    let mut transform = cam_query.get_single_mut().unwrap();

    let mut forward = transform.local_y();
    forward.y = 0.;
    forward = forward.normalize();

    let move_x = transform.local_x() * motion.x;
    let move_z = forward * motion.y;

    transform.translation += (move_x + move_z) * time.delta_seconds() * 0.6 * scale;
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
    bvh_query: Query<&SaveBVH>,
    mut cam_query: Query<&mut Transform, With<MainCamera>>,
) {
    for (interaction, mut color, mut border_color, cam_button) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                let mut transform = cam_query.get_single_mut().unwrap();

                let target = match bvh_query.get_single() {
                    Ok(save_bvh) => {
                        match save_bvh.bvh {
                            BVHNode::Internal { aabb, left: _, right: _ } => {
                                aabb.center.as_vec3()
                            },
                            _ => Vec3::ZERO
                        }
                    },
                    _ => Vec3::ZERO,
                };

                match cam_button.view {
                    ViewType::Top => {
                        let translation = target + Vec3::new(0.0, CAMERA_DISTANCE, 0.0);
                        let new_transform = Transform::from_translation(translation);
                        *transform = new_transform.looking_at(target, Vec3::NEG_Z);
                    },
                    ViewType::Front => {
                        let translation = target + Vec3::new(0.0, 0.0, CAMERA_DISTANCE);
                        let new_transform = Transform::from_translation(translation);
                        *transform = new_transform.looking_at(target, Vec3::Y);
                    },
                    ViewType::Back => {
                        let translation = target + Vec3::new(0.0, 0.0, -CAMERA_DISTANCE);
                        let new_transform = Transform::from_translation(translation);
                        *transform = new_transform.looking_at(target, Vec3::Y);
                    },
                    ViewType::Right => {
                        let translation = target + Vec3::new(CAMERA_DISTANCE, 0.0, 0.0);
                        let new_transform = Transform::from_translation(translation);
                        *transform = new_transform.looking_at(target, Vec3::Y);
                    },
                    ViewType::Left => {
                        let translation = target + Vec3::new(-CAMERA_DISTANCE, 0.0, 0.0);
                        let new_transform = Transform::from_translation(translation);
                        *transform = new_transform.looking_at(target, Vec3::Y);
                    },
                    ViewType::BottomRight => {
                        let translation = target + Vec3::new(CAMERA_DISTANCE, CAMERA_DISTANCE, CAMERA_DISTANCE);
                        let new_transform = Transform::from_translation(translation);
                        *transform = new_transform.looking_at(target, Vec3::Y);
                    },
                    ViewType::TopRight => {
                        let translation = target + Vec3::new(CAMERA_DISTANCE, CAMERA_DISTANCE, -CAMERA_DISTANCE);
                        let new_transform = Transform::from_translation(translation);
                        *transform = new_transform.looking_at(target, Vec3::Y);
                    },
                    ViewType::TopLeft => {
                        let translation = target + Vec3::new(-CAMERA_DISTANCE, CAMERA_DISTANCE, -CAMERA_DISTANCE);
                        let new_transform = Transform::from_translation(translation);
                        *transform = new_transform.looking_at(target, Vec3::Y);
                    },
                    ViewType::BottomLeft => {
                        let translation = target + Vec3::new(-CAMERA_DISTANCE, CAMERA_DISTANCE, CAMERA_DISTANCE);
                        let new_transform = Transform::from_translation(translation);
                        *transform = new_transform.looking_at(target, Vec3::Y);
                    },
                    _ => {}
                }


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
