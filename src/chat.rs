use std::time::Duration;

use bevy::prelude::*;

pub struct ChatPlugin;

#[derive(Component)]
struct Chat;

#[derive(Resource)]
struct BackTimer {
    timer: Timer
}

impl Plugin for ChatPlugin {
    fn build(&self, app: &mut App) {
        app
            .insert_resource(BackTimer {
                timer: Timer::new(Duration::from_millis(50), TimerMode::Repeating)
            })
            .add_systems(Startup, spawn_chat)
            .add_systems(Update, chat);
    }
}

fn spawn_chat(
    mut commands: Commands
) {
    commands.spawn((
        TextBundle::from("test").with_style(
            Style {
                position_type: PositionType::Absolute,
                bottom: Val::Px(10.0),
                left: Val::Px(15.0),
                ..default()
            }
        ),
        Chat
    ));
}

fn chat(
    mut query: Query<&mut Text, With<Chat>>,
    keycode: Res<Input<KeyCode>>,
    time: Res<Time>,
    mut backspace_timer: ResMut<BackTimer>,
) {
    backspace_timer.timer.tick(time.delta());

    let mut text = query.get_single_mut().unwrap();
    for key in keycode.get_just_pressed() {
        match key {
            KeyCode::Back => {
                text.sections[0].value.pop();
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
            KeyCode::ShiftLeft => {},
            KeyCode::Underline => {
                text.sections[0].value.push('_');
            },
            KeyCode::Period => {
                text.sections[0].value.push('.');
            },
            _ => {
                let mut key = format!("{:?}", key);
                if !keycode.pressed(KeyCode::ShiftLeft) {
                    key = key.to_lowercase();
                };
                text.sections[0].value.push_str(&key);
            }
        }
    }

    let blink_duration = time.elapsed_seconds_f64() % 1.0;
    if blink_duration < 0.5 && text.sections.len() == 1 {
        text.sections.push(TextSection {
            value: "|".into(),
            ..default()
        });
    } else if text.sections.len() == 2 {
        text.sections.pop();
    }

    if keycode.pressed(KeyCode::Back) && backspace_timer.timer.finished() {
        text.sections[0].value.pop();
    }
}
