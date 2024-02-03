use bevy::{prelude::*, pbr::{DirectionalLightShadowMap, CascadeShadowConfigBuilder}};

use crate::state::{GameState, InputState};

const SHADOW_MAP_SIZE: usize = 8192;
const AMBIENT_BRIGHTNESS: f32 = 0.5;
const SUN_ILLUMINANCE: f32 = 20000.0;

pub struct LightPlugin;

#[derive(Component)]
pub struct Sun;

impl Plugin for LightPlugin {
    fn build(&self, app: &mut App) {
        app
            .insert_resource(DirectionalLightShadowMap { size: SHADOW_MAP_SIZE })
            .insert_resource(AmbientLight {
                color: Color::WHITE,
                brightness: AMBIENT_BRIGHTNESS,
            })
            .add_systems(Startup, spawn_light)
            .add_systems(Update, animate_light_direction);
    }
}

fn spawn_light(mut commands: Commands) {

    let mut shadow_light_transform = Transform::from_rotation(Quat::from_rotation_x(-1.079));
    shadow_light_transform.rotate_y(0.303);
    shadow_light_transform.rotate_z(0.508);

    commands.spawn((
        DirectionalLightBundle {
            directional_light: DirectionalLight {
                shadows_enabled: true,
                illuminance: SUN_ILLUMINANCE,
                ..default()
            },
            cascade_shadow_config: CascadeShadowConfigBuilder {
                num_cascades: 4,
                maximum_distance: 500000.0,
                first_cascade_far_bound: 1000.0,
                ..default()
            }.into(),
            transform: shadow_light_transform,
            ..default()
        },
        Sun
    ));

    let mut light_transform = Transform::from_rotation(Quat::from_rotation_x(-1.460));
    light_transform.rotate_y(-0.566);
    light_transform.rotate_z(-0.346);

    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            shadows_enabled: false,
            illuminance: SUN_ILLUMINANCE / 2.0,
            ..default()
        },
        transform: light_transform,
        ..default()
    });
}

fn animate_light_direction(
    time: Res<Time>,
    mut query: Query<&mut Transform, With<DirectionalLight>>,
    keycode: Res<Input<KeyCode>>,
    game_state: Res<GameState>,
) {
    match game_state.input {
        InputState::Listen => {},
        InputState::Typing => {
            return;
        }
    }

    let mut dir = 0.0;
    if keycode.pressed(KeyCode::Left) {
        dir = 1.;
    } else if keycode.pressed(KeyCode::Right) {
        dir = -1.;
    }

    if dir == 0.0 {
        return;
    }

    for mut transform in &mut query {
        transform.rotate_y(time.delta_seconds() * dir);
    }
}
