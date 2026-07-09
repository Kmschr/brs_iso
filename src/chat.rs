use std::time::Duration;

use bevy::{input::{keyboard::KeyboardInput, ButtonState}, prelude::*};

use crate::{asset_loader::SceneAssets, components::Light, lit::Sun, state::{BVHView, BuildLoaded, GameState, InputState}, ChunkMesh, Ground, SaveBVH, Water};

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

fn text_font() -> TextFont {
    TextFont {
        font_size: FontSize::Px(14.0),
        ..default()
    }
}

fn spawn_chat(
    mut commands: Commands
) {
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(0.0),
            left: Val::Px(0.0),
            width: Val::Percent(100.),
            height: Val::Px(30.),
            border: UiRect::top(Val::Px(1.)),
            ..default()
        },
        BackgroundColor(Color::BLACK),
        BorderColor::all(Color::WHITE),
        Visibility::Hidden,
        Console,
    )).with_children(|parent| {
        // Chat text: root ">" section, then spans for
        // [1] input before cursor, [2] cursor, [3] input after cursor.
        parent.spawn((
            Text::new(">"),
            text_font(),
            TextColor(Color::WHITE),
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(10.0),
                left: Val::Px(15.0),
                ..default()
            },
            Chat,
        )).with_children(|text| {
            text.spawn((TextSpan::new(""), text_font(), TextColor(Color::WHITE)));
            text.spawn((TextSpan::new(""), text_font(), TextColor(Color::WHITE)));
            text.spawn((TextSpan::new(""), text_font(), TextColor(Color::WHITE)));
        });
    });
}

fn blink_cursor(
    chat_query: Query<Entity, With<Chat>>,
    mut writer: TextUiWriter,
    time: Res<Time>,
    mut timers: ResMut<Timers>,
) {
    timers.tenth_second.tick(time.delta());
    timers.half_second.tick(time.delta());

    let Ok(entity) = chat_query.single() else { return; };
    if timers.half_second.just_finished() {
        let mut cursor = writer.text(entity, 2);
        if cursor.is_empty() {
            cursor.push('|');
        } else {
            cursor.pop();
        }
    }
}

fn enable_chat(
    mut query: Query<&mut Visibility, With<Console>>,
    mut game_state: ResMut<GameState>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    match game_state.input {
        InputState::Listen => {
            if !keyboard.just_pressed(KeyCode::Slash) {
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

    game_state.input = InputState::Typing;
}

fn keyboard_system(
    chat_query: Query<Entity, With<Chat>>,
    mut writer: TextUiWriter,
    mut console_query: Query<&mut Visibility, With<Console>>,
    mut rd: MessageReader<KeyboardInput>,
    mut game_state: ResMut<GameState>,
    mut build_loaded: ResMut<BuildLoaded>,
    mut commands: Commands,
    mesh_query: Query<Entity, With<ChunkMesh>>,
    mut light_query: Query<(Entity, &mut Visibility), (With<Light>, Without<Water>, Without<Console>)>,
    bvh_query: Query<Entity, With<SaveBVH>>,
    mut water_query: Query<&mut Visibility, (With<Water>, Without<Console>)>,
    mut ground_query: Query<&mut Visibility, (With<Ground>, Without<Console>, Without<Water>, Without<Light>)>,
    mut sun_query: Query<&mut DirectionalLight, With<Sun>>,
    assets: Res<SceneAssets>,
) {
    if game_state.input_listening() || game_state.is_changed() {
        return;
    }

    let Ok(entity) = chat_query.single() else { return; };

    for ev in rd.read() {
        if ev.state != ButtonState::Pressed {
            continue;
        }
        match ev.key_code {
            KeyCode::Escape => {
                game_state.input = InputState::Listen;
                for mut visibility in console_query.iter_mut() {
                    *visibility = Visibility::Hidden;
                }
                writer.text(entity, 1).clear();
                writer.text(entity, 3).clear();
            },
            KeyCode::Backspace => {
                writer.text(entity, 1).pop();
            },
            KeyCode::ArrowLeft => {
                if let Some(c) = writer.text(entity, 1).pop() {
                    writer.text(entity, 3).insert(0, c);
                }
            },
            KeyCode::ArrowRight => {
                if !writer.text(entity, 3).is_empty() {
                    let c = writer.text(entity, 3).remove(0);
                    writer.text(entity, 1).push(c);
                }
            },
            KeyCode::Enter => {
                let before = writer.text(entity, 1).clone();
                let after = writer.text(entity, 3).clone();
                let command = format!("{before}{after}");

                match command.as_str() {
                    "/clear" | "/clearbricks" | "/clearallbricks" => {
                        commands.spawn((
                            AudioPlayer::new(assets.sounds.clear_bricks.clone()),
                            PlaybackSettings::DESPAWN,
                        ));
                        for entity in mesh_query.iter() {
                            commands.entity(entity).despawn();
                        }
                        for (entity, _) in light_query.iter() {
                            commands.entity(entity).despawn();
                        }
                        for entity in bvh_query.iter() {
                            commands.entity(entity).despawn();
                        }
                        build_loaded.0 = false;
                    },
                    "/water" => {
                        if let Ok(mut visibility) = water_query.single_mut() {
                            toggle_visibility(&mut visibility);
                        }
                    },
                    "/ground" => {
                        if let Ok(mut visibility) = ground_query.single_mut() {
                            toggle_visibility(&mut visibility);
                        }
                    },
                    "/bvh" => {
                        match game_state.bvh_view {
                            BVHView::Off => {
                                info!("Toggled BVH On");
                                game_state.bvh_view = BVHView::On(0);
                            },
                            BVHView::On(_) => {
                                info!("Toggled BVH Off");
                                game_state.bvh_view = BVHView::Off;
                            }
                        }
                    },
                    "/shadows" => {
                        if let Ok(mut sun) = sun_query.single_mut() {
                            sun.shadow_maps_enabled = !sun.shadow_maps_enabled;
                        }
                    },
                    "/lights" => {
                        for (_, mut visibility) in light_query.iter_mut() {
                            toggle_visibility(&mut visibility);
                        }
                    },
                    "/debuglights" | "/lightdebug" => {
                        game_state.light_debug = !game_state.light_debug;
                    }
                    _ => {}
                }

                writer.text(entity, 1).clear();
                writer.text(entity, 3).clear();
                game_state.input = InputState::Listen;

                for mut visibility in console_query.iter_mut() {
                    *visibility = Visibility::Hidden;
                }
            },
            _ => {
                // Printable characters come through the event's `text` field,
                // already accounting for keyboard layout and shift state.
                if let Some(text) = &ev.text {
                    for c in text.chars() {
                        if !c.is_control() {
                            writer.text(entity, 1).push(c);
                        }
                    }
                }
            }
        }
    }
}

fn toggle_visibility(visibility: &mut Visibility) {
    *visibility = match *visibility {
        Visibility::Hidden => Visibility::Visible,
        Visibility::Visible => Visibility::Hidden,
        Visibility::Inherited => Visibility::Hidden,
    };
}
