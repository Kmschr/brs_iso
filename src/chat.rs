use std::time::Duration;

use bevy::{input::{keyboard::KeyboardInput, ButtonState}, prelude::*};

use crate::{asset_loader::SceneAssets, components::Light, lit::Sun, state::{BVHView, GameState, InputState}, ChunkMesh, SaveBVH, Water};

pub struct ChatPlugin;

#[derive(Component)]
struct Chat;

#[derive(Component)]
struct Console;

#[derive(Resource)]
struct Timers {
    tenth_second: Timer,
    half_second: Timer,
}

impl Plugin for ChatPlugin {
    fn build(&self, app: &mut App) {
        app
            .insert_resource(Timers {
                tenth_second: Timer::new(Duration::from_millis(100), TimerMode::Repeating),
                half_second: Timer::new(Duration::from_millis(500), TimerMode::Repeating),
            })
            .add_systems(Startup, spawn_chat)
            .add_systems(Update, (blink_cursor, keyboard_system))
            .add_systems(Update, enable_chat.after(keyboard_system));
    }
}

fn spawn_chat(
    mut commands: Commands
) {
    commands.spawn((
        NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                bottom: Val::Px(0.0),
                left: Val::Px(0.0),
                width: Val::Percent(100.),
                height: Val::Px(30.),
                border: UiRect::top(Val::Px(1.)),
                ..default()
            },
            background_color: BackgroundColor(Color::BLACK),
            border_color: BorderColor(Color::WHITE),
            visibility: Visibility::Hidden,
            ..default()
        },
        Console
    )).with_children(|parent| {
        parent.spawn((
            TextBundle {
                visibility: Visibility::Inherited,
                style: Style {
                    position_type: PositionType::Absolute,
                    bottom: Val::Px(10.0),
                    left: Val::Px(15.0),
                    ..default()
                },
                text: Text::from_sections([
                    TextSection::new(">", text_style()),
                    TextSection::new("", text_style()),
                    TextSection::new("", text_style()),
                    TextSection::new("", text_style())
                ]),
                ..default()
            },
            Chat
        ));
    });
}

fn text_style() -> TextStyle {
    TextStyle {
        font_size: 14.,
        ..default()
    }
}

fn blink_cursor(
    mut query: Query<&mut Text, With<Chat>>,
    time: Res<Time>,
    mut timers: ResMut<Timers>,
) {
    timers.tenth_second.tick(time.delta());
    timers.half_second.tick(time.delta());

    let mut text = query.get_single_mut().unwrap();
    if timers.half_second.finished() {
        let cursor_section = &mut text.sections[2].value;
        if cursor_section.is_empty() {
            cursor_section.push('|');
        } else {
            cursor_section.pop();
        }
    }
}

fn enable_chat(
    mut query: Query<&mut Visibility, With<Console>>,
    mut game_state: ResMut<GameState>,
    mut keyboard: ResMut<Input<KeyCode>>,
    mut rd: EventReader<KeyboardInput>,
) {
    match game_state.input {
        InputState::Listen => {
            if !keyboard.just_pressed(KeyCode::T) {
                return;
            }
        },
        InputState::Typing => {
            return;
        }
    }

    for mut visibility in query.iter_mut() {
        *visibility = Visibility::Visible;
    }    

    keyboard.reset(KeyCode::T);
    rd.clear();
    game_state.input = InputState::Typing;
}

fn keyboard_system(
    mut text_query: Query<&mut Text, With<Chat>>,
    mut console_query: Query<&mut Visibility, With<Console>>,
    keyboard: Res<Input<KeyCode>>,
    mut rd: EventReader<KeyboardInput>,
    mut game_state: ResMut<GameState>,
    mut commands: Commands,
    mesh_query: Query<Entity, With<ChunkMesh>>,
    mut light_query: Query<(Entity, &mut Visibility), (With<Light>, Without<Water>, Without<Console>)>,
    bvh_query: Query<Entity, With<SaveBVH>>,
    mut water_query: Query<&mut Visibility, (With<Water>, Without<Console>)>,
    mut sun_query: Query<&mut DirectionalLight, With<Sun>>,
    assets: Res<SceneAssets>,
) {
    if game_state.input_listening() || game_state.is_changed() {
        rd.clear();
        return;
    }

    let mut text = text_query.get_single_mut().unwrap();

    for ev in rd.read() {
        if ev.state != ButtonState::Pressed {
            continue;
        }
        if let Some(key) = ev.key_code {
            match key {
                KeyCode::Back => {
                    text.sections[1].value.pop();
                },
                KeyCode::Left => {
                    let char = text.sections[1].value.pop().unwrap_or_default();
                    text.sections[3].value.insert(0, char);
                },
                KeyCode::Right => {
                    if !text.sections[3].value.is_empty() {
                        let char = text.sections[3].value.remove(0);
                        text.sections[1].value.push(char);
                    }
                },
                KeyCode::Space => {
                    text.sections[1].value.push(' ');
                },
                KeyCode::Tab => {
                    text.sections[1].value.push_str("    ");
                },
                KeyCode::Slash => {
                    text.sections[1].value.push('/');
                },
                KeyCode::Underline => {
                    text.sections[1].value.push('_');
                },
                KeyCode::Period => {
                    text.sections[1].value.push('.');
                },
                KeyCode::Return => {
                    let command = format!("{}{}", text.sections[1].value, text.sections[3].value);

                    match command.as_str() {
                        "/clear" | "/clearbricks" | "/clearallbricks" => {
                            commands.spawn(AudioBundle {
                                source: assets.sounds.clear_bricks.clone(),
                                ..default()
                            }); 
                            for entity in mesh_query.iter() {
                                commands.entity(entity).despawn();
                            }
                            for (entity, _) in light_query.iter() {
                                commands.entity(entity).despawn();
                            }
                            for entity in bvh_query.iter() {
                                commands.entity(entity).despawn();
                            }
                        },
                        "/water" => {
                            let mut visibility = water_query.get_single_mut().unwrap();
                            match *visibility {
                                Visibility::Hidden => {
                                    *visibility = Visibility::Visible;
                                },
                                Visibility::Visible => {
                                    *visibility = Visibility::Hidden;
                                },
                                _ => {}
                            }
                        },
                        "/bvh" => {
                            match game_state.bvh_view {
                                BVHView::Off => {
                                    game_state.bvh_view = BVHView::On(0);
                                },
                                BVHView::On(_) => {
                                    game_state.bvh_view = BVHView::Off;
                                }
                            }
                        },
                        "/shadows" => {
                            let mut sun = sun_query.get_single_mut().unwrap();
                            sun.shadows_enabled = !sun.shadows_enabled;
                        },
                        "/lights" => {
                            for (_, mut visibility) in light_query.iter_mut() {
                                match *visibility {
                                    Visibility::Hidden => {
                                        *visibility = Visibility::Visible;
                                    },
                                    Visibility::Visible => {
                                        *visibility = Visibility::Hidden;
                                    },
                                    _ => {}
                                }
                            }
                        },
                        "/debuglights" | "/lightdebug" => {
                            game_state.light_debug = !game_state.light_debug;
                        }
                        _ => {}
                    }

                    text.sections[1].value = String::new();
                    text.sections[3].value = String::new();
                    game_state.input = InputState::Listen;

                    for mut visibility in console_query.iter_mut() {
                        *visibility = Visibility::Hidden;
                    }
                },
                _ => {
                    let mut key = format!("{:?}", key);
                    if key.len() > 1 {
                        continue;
                    }
    
                    if !keyboard.pressed(KeyCode::ShiftLeft) && !keyboard.pressed(KeyCode::ShiftRight) {
                        key = key.to_lowercase();
                    };
                    text.sections[1].value.push_str(&key);
                }
            }
        }
    }
}
