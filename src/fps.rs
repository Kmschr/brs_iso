use bevy::prelude::*;
use bevy::diagnostic::DiagnosticsStore;
use bevy::diagnostic::FrameTimeDiagnosticsPlugin;

/// Marker to find the container entity so we can show/hide the FPS counter
#[derive(Component)]
struct FpsRoot;

/// Marker to find the text entity so we can update it
#[derive(Component)]
struct FpsText;

pub struct FPSPlugin;

impl Plugin for FPSPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(Startup, setup_fps_counter)
            .add_systems(Update, (fps_text_update_system, fps_counter_showhide));
    }
}

fn setup_fps_counter(
    mut commands: Commands,
) {
    // create our UI root node (wrapper/container for the text)
    let root = commands.spawn((
        FpsRoot,
        Node {
            position_type: PositionType::Absolute,
            // position it at the top-right corner, 1% away from the edges
            right: Val::Percent(1.),
            top: Val::Percent(1.),
            bottom: Val::Auto,
            left: Val::Auto,
            // give it some padding for readability
            padding: UiRect::all(Val::Px(4.0)),
            ..default()
        },
        // dark background for readability
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.5)),
        // "always on top" by maxing out the global Z index
        GlobalZIndex(i32::MAX),
    )).id();

    // create our text: a root section ("FPS: ") plus a span child for the number
    let text_fps = commands.spawn((
        FpsText,
        Text::new("FPS: "),
        TextFont { font_size: FontSize::Px(16.0), ..default() },
        TextColor(Color::WHITE),
    )).with_child((
        TextSpan::new(" N/A"),
        TextFont { font_size: FontSize::Px(16.0), ..default() },
        TextColor(Color::WHITE),
    )).id();

    commands.entity(root).add_child(text_fps);
}

fn fps_text_update_system(
    diagnostics: Res<DiagnosticsStore>,
    query: Query<Entity, With<FpsText>>,
    mut writer: TextUiWriter,
) {
    for entity in &query {
        // try to get a "smoothed" FPS value from Bevy
        if let Some(value) = diagnostics
            .get(&FrameTimeDiagnosticsPlugin::FPS)
            .and_then(|fps| fps.smoothed())
        {
            // span index 1 = the " N/A"/number span child
            *writer.text(entity, 1) = format!("{value:>4.0}");

            writer.color(entity, 1).0 = if value >= 120.0 {
                Color::srgb(0.0, 1.0, 0.0)
            } else if value >= 60.0 {
                Color::srgb((1.0 - (value - 60.0) / (120.0 - 60.0)) as f32, 1.0, 0.0)
            } else if value >= 30.0 {
                Color::srgb(1.0, ((value - 30.0) / (60.0 - 30.0)) as f32, 0.0)
            } else {
                Color::srgb(1.0, 0.0, 0.0)
            };
        } else {
            *writer.text(entity, 1) = " N/A".into();
            writer.color(entity, 1).0 = Color::WHITE;
        }
    }
}

/// Toggle the FPS counter when pressing F12
fn fps_counter_showhide(
    mut q: Query<&mut Visibility, With<FpsRoot>>,
    kbd: Res<ButtonInput<KeyCode>>,
) {
    if kbd.just_pressed(KeyCode::F12) {
        let Ok(mut vis) = q.single_mut() else { return; };
        *vis = match *vis {
            Visibility::Hidden => Visibility::Visible,
            _ => Visibility::Hidden,
        };
    }
}
