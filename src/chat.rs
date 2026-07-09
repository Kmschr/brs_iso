use std::time::Duration;

use bevy::{ecs::system::SystemParam, input::{keyboard::KeyboardInput, ButtonState}, prelude::*};

use crate::{asset_loader::SceneAssets, components::Light, lit::Sun, state::{BVHView, BrickInfoEnabled, BuildLoaded, GameState, InputState}, ChunkMesh, Ground, SaveBVH, Water};

pub struct ChatPlugin;

/// Max log lines kept in the scrollback before oldest are dropped.
const MAX_LOG_LINES: usize = 200;

// Palette.
const COLOR_ACCENT: Color = Color::srgb(0.45, 0.85, 0.55);
const COLOR_INPUT: Color = Color::srgb(0.92, 0.92, 0.95);
const COLOR_ECHO: Color = Color::srgb(0.55, 0.55, 0.62);
const COLOR_INFO: Color = Color::srgb(0.80, 0.82, 0.86);
const COLOR_ERROR: Color = Color::srgb(0.95, 0.45, 0.45);
const COLOR_PANEL: Color = Color::srgba(0.04, 0.04, 0.06, 0.88);
const COLOR_BORDER: Color = Color::srgba(1.0, 1.0, 1.0, 0.12);

#[derive(Component)]
struct Chat;

/// Root panel that toggles visible/hidden.
#[derive(Component)]
struct Console;

/// Scrollback container; log lines are spawned as its children.
#[derive(Component)]
struct ConsoleLog;

#[derive(Component)]
struct ConsoleLogLine;

/// Persistent console state that outlives an open/close cycle.
#[derive(Resource, Default)]
struct ConsoleState {
    /// Past submitted commands, oldest first.
    history: Vec<String>,
    /// Current position while browsing history via up/down. `None` = editing a fresh line.
    browse: Option<usize>,
    /// In-progress line stashed when the user starts browsing history.
    stash: String,
}

/// Scene entities a command may touch, grouped to stay under the system-param limit.
#[derive(SystemParam)]
struct SceneQueries<'w, 's> {
    mesh: Query<'w, 's, Entity, With<ChunkMesh>>,
    lights: Query<'w, 's, (Entity, &'static mut Visibility), (With<Light>, Without<Water>, Without<Console>)>,
    bvh: Query<'w, 's, Entity, With<SaveBVH>>,
    water: Query<'w, 's, &'static mut Visibility, (With<Water>, Without<Console>)>,
    ground: Query<'w, 's, &'static mut Visibility, (With<Ground>, Without<Console>, Without<Water>, Without<Light>)>,
    sun: Query<'w, 's, &'static mut DirectionalLight, With<Sun>>,
}

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
            .init_resource::<ConsoleState>()
            .add_systems(Startup, spawn_chat)
            .add_systems(Update, (blink_cursor, keyboard_system, trim_log))
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
            height: Val::Px(240.),
            flex_direction: FlexDirection::Column,
            border: UiRect::top(Val::Px(1.)),
            ..default()
        },
        BackgroundColor(COLOR_PANEL),
        BorderColor::all(COLOR_BORDER),
        Visibility::Hidden,
        crate::state::HideOnScreenshot,
        Console,
    )).with_children(|parent| {
        // Scrollback: newest lines pinned to the bottom, overflow clipped off the top.
        parent.spawn((
            Node {
                flex_grow: 1.0,
                // Must be able to shrink below content height, or it squeezes the
                // input row to nothing instead of clipping overflow off the top.
                min_height: Val::Px(0.),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::FlexEnd,
                overflow: Overflow::clip(),
                padding: UiRect::axes(Val::Px(14.), Val::Px(8.)),
                row_gap: Val::Px(2.),
                ..default()
            },
            ConsoleLog,
        )).with_children(|log| {
            log.spawn((
                Text::new("brs console — type /help for commands"),
                text_font(),
                TextColor(COLOR_INFO),
                ConsoleLogLine,
            ));
        });

        // Input row: accent prompt then input spans (before-cursor, cursor, after-cursor).
        parent.spawn((
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                height: Val::Px(28.),
                flex_shrink: 0.,
                padding: UiRect::horizontal(Val::Px(14.)),
                border: UiRect::top(Val::Px(1.)),
                ..default()
            },
            BorderColor::all(COLOR_BORDER),
        )).with_children(|row| {
            row.spawn((
                Text::new("> "),
                text_font(),
                TextColor(COLOR_ACCENT),
                Chat,
            )).with_children(|text| {
                text.spawn((TextSpan::new(""), text_font(), TextColor(COLOR_INPUT)));
                text.spawn((TextSpan::new(""), text_font(), TextColor(COLOR_INPUT)));
                text.spawn((TextSpan::new(""), text_font(), TextColor(COLOR_INPUT)));
            });
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

/// Drop the oldest log lines once the scrollback exceeds the cap.
fn trim_log(
    mut commands: Commands,
    log_query: Query<&Children, With<ConsoleLog>>,
) {
    let Ok(children) = log_query.single() else { return; };
    if children.len() > MAX_LOG_LINES {
        for child in children.iter().take(children.len() - MAX_LOG_LINES) {
            commands.entity(child).despawn();
        }
    }
}

fn keyboard_system(
    chat_query: Query<Entity, With<Chat>>,
    log_query: Query<Entity, With<ConsoleLog>>,
    mut writer: TextUiWriter,
    mut console_query: Query<&mut Visibility, With<Console>>,
    mut rd: MessageReader<KeyboardInput>,
    mut game_state: ResMut<GameState>,
    mut console_state: ResMut<ConsoleState>,
    mut build_loaded: ResMut<BuildLoaded>,
    mut brick_info_enabled: ResMut<BrickInfoEnabled>,
    mut commands: Commands,
    mut scene: SceneQueries,
    assets: Res<SceneAssets>,
) {
    if game_state.input_listening() || game_state.is_changed() {
        return;
    }

    let Ok(entity) = chat_query.single() else { return; };
    let Ok(log_entity) = log_query.single() else { return; };

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
                console_state.browse = None;
            },
            KeyCode::Backspace => {
                writer.text(entity, 1).pop();
                console_state.browse = None;
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
            KeyCode::ArrowUp => {
                recall_history(&mut writer, entity, &mut console_state, -1);
            },
            KeyCode::ArrowDown => {
                recall_history(&mut writer, entity, &mut console_state, 1);
            },
            KeyCode::Enter => {
                let before = writer.text(entity, 1).clone();
                let after = writer.text(entity, 3).clone();
                let command = format!("{before}{after}");
                let command = command.trim();

                writer.text(entity, 1).clear();
                writer.text(entity, 3).clear();
                console_state.browse = None;

                if command.is_empty() {
                    continue;
                }

                console_state.history.push(command.to_string());

                let mut feedback: Vec<(String, Color)> = Vec::new();
                match command {
                    "/clear" | "/clearbricks" | "/clearallbricks" => {
                        commands.spawn((
                            AudioPlayer::new(assets.sounds.clear_bricks.clone()),
                            PlaybackSettings::DESPAWN,
                        ));
                        for entity in scene.mesh.iter() {
                            commands.entity(entity).despawn();
                        }
                        for (entity, _) in scene.lights.iter() {
                            commands.entity(entity).despawn();
                        }
                        for entity in scene.bvh.iter() {
                            commands.entity(entity).despawn();
                        }
                        build_loaded.0 = false;
                        feedback.push(("cleared all bricks".into(), COLOR_INFO));
                    },
                    "/water" => {
                        if let Ok(mut visibility) = scene.water.single_mut() {
                            let on = toggle_visibility(&mut visibility);
                            feedback.push((format!("water {}", on_off(on)), COLOR_INFO));
                        }
                    },
                    "/ground" => {
                        if let Ok(mut visibility) = scene.ground.single_mut() {
                            let on = toggle_visibility(&mut visibility);
                            feedback.push((format!("ground {}", on_off(on)), COLOR_INFO));
                        }
                    },
                    "/bvh" => {
                        match game_state.bvh_view {
                            BVHView::Off => {
                                game_state.bvh_view = BVHView::On(0);
                                feedback.push(("bvh view on".into(), COLOR_INFO));
                            },
                            BVHView::On(_) => {
                                game_state.bvh_view = BVHView::Off;
                                feedback.push(("bvh view off".into(), COLOR_INFO));
                            }
                        }
                    },
                    "/shadows" => {
                        if let Ok(mut sun) = scene.sun.single_mut() {
                            sun.shadow_maps_enabled = !sun.shadow_maps_enabled;
                            feedback.push((format!("shadows {}", on_off(sun.shadow_maps_enabled)), COLOR_INFO));
                        }
                    },
                    "/lights" => {
                        let mut on = false;
                        for (_, mut visibility) in scene.lights.iter_mut() {
                            on = toggle_visibility(&mut visibility);
                        }
                        feedback.push((format!("lights {}", on_off(on)), COLOR_INFO));
                    },
                    "/debuglights" | "/lightdebug" => {
                        game_state.light_debug = !game_state.light_debug;
                        feedback.push((format!("light debug {}", on_off(game_state.light_debug)), COLOR_INFO));
                    }
                    "/brickinfo" => {
                        brick_info_enabled.0 = !brick_info_enabled.0;
                        feedback.push((format!("brick info {}", on_off(brick_info_enabled.0)), COLOR_INFO));
                    }
                    "/help" => {
                        for line in HELP_LINES {
                            feedback.push((line.to_string(), COLOR_INFO));
                        }
                    }
                    _ => {
                        feedback.push((format!("unknown command: {command} (try /help)"), COLOR_ERROR));
                    }
                }

                // Echo the command, then its output, into the scrollback.
                commands.entity(log_entity).with_children(|log| {
                    log.spawn((
                        Text::new(format!("> {command}")),
                        text_font(),
                        TextColor(COLOR_ECHO),
                        ConsoleLogLine,
                    ));
                    for (line, color) in feedback {
                        log.spawn((
                            Text::new(line),
                            text_font(),
                            TextColor(color),
                            ConsoleLogLine,
                        ));
                    }
                });
            },
            _ => {
                // Printable characters come through the event's `text` field,
                // already accounting for keyboard layout and shift state.
                if let Some(text) = &ev.text {
                    for c in text.chars() {
                        if !c.is_control() {
                            writer.text(entity, 1).push(c);
                            console_state.browse = None;
                        }
                    }
                }
            }
        }
    }
}

const HELP_LINES: &[&str] = &[
    "/clear         remove all bricks",
    "/water         toggle water plane",
    "/ground        toggle ground plane",
    "/lights        toggle brick lights",
    "/shadows       toggle sun shadows",
    "/bvh           toggle bvh view",
    "/brickinfo     toggle brick hover info",
    "/debuglights   toggle light debug gizmos",
    "/help          show this list",
];

/// Step through command history. `dir` is -1 for older, +1 for newer.
fn recall_history(
    writer: &mut TextUiWriter,
    entity: Entity,
    state: &mut ConsoleState,
    dir: i32,
) {
    if state.history.is_empty() {
        return;
    }

    match dir {
        -1 => {
            let next = match state.browse {
                None => {
                    // Stash the in-progress line before browsing.
                    let before = writer.text(entity, 1).clone();
                    let after = writer.text(entity, 3).clone();
                    state.stash = format!("{before}{after}");
                    state.history.len() - 1
                }
                Some(0) => 0,
                Some(i) => i - 1,
            };
            state.browse = Some(next);
            set_input(writer, entity, &state.history[next].clone());
        }
        _ => {
            let Some(i) = state.browse else { return; };
            if i + 1 < state.history.len() {
                state.browse = Some(i + 1);
                set_input(writer, entity, &state.history[i + 1].clone());
            } else {
                // Past the newest entry: restore the stashed line.
                state.browse = None;
                let stash = state.stash.clone();
                set_input(writer, entity, &stash);
            }
        }
    }
}

/// Replace the whole input line, cursor at the end.
fn set_input(writer: &mut TextUiWriter, entity: Entity, value: &str) {
    *writer.text(entity, 1) = value.to_string();
    writer.text(entity, 3).clear();
}

fn on_off(on: bool) -> &'static str {
    if on { "on" } else { "off" }
}

/// Toggle visibility; returns true if now visible.
fn toggle_visibility(visibility: &mut Visibility) -> bool {
    *visibility = match *visibility {
        Visibility::Visible => Visibility::Hidden,
        _ => Visibility::Visible,
    };
    matches!(*visibility, Visibility::Visible)
}
