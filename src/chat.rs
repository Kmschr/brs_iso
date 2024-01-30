use std::time::Duration;

use bevy::{input::{keyboard::KeyboardInput, ButtonState}, prelude::*};

use crate::state::{GameState, InputState};

pub struct ChatPlugin;

#[derive(Component)]
struct Chat;

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
            .add_systems(Update, (blink_cursor, keyboard_system, enable_chat));
    }
}

fn spawn_chat(
    mut commands: Commands
) {
    let mut chatbox = TextBundle::from_sections([
        TextSection::new("", TextStyle::default()),
        TextSection::new("", TextStyle::default()),
        TextSection::new("", TextStyle::default())
    ]).with_style(
        Style {
            position_type: PositionType::Absolute,
            bottom: Val::Px(10.0),
            left: Val::Px(15.0),
            ..default()
        }
    );
    chatbox.visibility = Visibility::Hidden;
    commands.spawn((
        chatbox,
        Chat
    ));
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
        let cursor_section = &mut text.sections[1].value;
        if cursor_section.is_empty() {
            cursor_section.push('|');
        } else {
            cursor_section.pop();
        }
    }
}

fn enable_chat(
    mut query: Query<&mut Visibility, With<Chat>>,
    mut game_state: ResMut<GameState>,
    mut keycode: ResMut<Input<KeyCode>>,
) {
    match game_state.input {
        InputState::Listen => {
            if !keycode.just_pressed(KeyCode::T) {
                return;
            }
        },
        InputState::Typing => {
            return;
        }
    }

    let mut visibility = query.get_single_mut().unwrap();
    *visibility = Visibility::Visible;

    game_state.input = InputState::Typing;
    keycode.reset(KeyCode::T);
}

fn keyboard_system(
    mut query: Query<(&mut Text, &mut Visibility), With<Chat>>,
    keycode: Res<Input<KeyCode>>,
    mut rd: EventReader<KeyboardInput>,
    mut game_state: ResMut<GameState>,
) {
    match game_state.input {
        InputState::Listen => {
            return;
        },
        InputState::Typing => {}
    }

    let (mut text, mut visibility) = query.get_single_mut().unwrap();

    for ev in rd.read() {
        if ev.state != ButtonState::Pressed {
            continue;
        }
        if let Some(key) = ev.key_code {
            match key {
                KeyCode::Back => {
                    text.sections[0].value.pop();
                },
                KeyCode::Left => {
                    let char = text.sections[0].value.pop().unwrap_or_default();
                    text.sections[2].value.insert(0, char);
                },
                KeyCode::Right => {
                    if !text.sections[2].value.is_empty() {
                        let char = text.sections[2].value.remove(0);
                        text.sections[0].value.push(char);
                    }
                },
                KeyCode::Space => {
                    text.sections[0].value.push(' ');
                },
                KeyCode::Tab => {
                    text.sections[0].value.push_str("    ");
                },
                KeyCode::Slash => {
                    text.sections[0].value.push('/');
                },
                KeyCode::Underline => {
                    text.sections[0].value.push('_');
                },
                KeyCode::Period => {
                    text.sections[0].value.push('.');
                },
                KeyCode::Return => {
                    text.sections[0].value = String::new();
                    text.sections[2].value = String::new();
                    game_state.input = InputState::Listen;
                    *visibility = Visibility::Hidden;
                },
                _ => {
                    let mut key = format!("{:?}", key);
                    if key.len() > 1 {
                        continue;
                    }
    
                    if !keycode.pressed(KeyCode::ShiftLeft) && !keycode.pressed(KeyCode::ShiftRight) {
                        key = key.to_lowercase();
                    };
                    text.sections[0].value.push_str(&key);
                }
            }
        }
    }
}
