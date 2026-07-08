use bevy::prelude::*;

pub struct SettingsPlugin;

impl Plugin for SettingsPlugin {
    fn build(&self, _app: &mut App) {
        //app.add_systems(Startup, settings_ui);
    }
}

fn _settings_ui(
    mut commands: Commands,
) {
    commands.spawn((
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
    )).with_children(|parent| {
        parent.spawn((
            Node {
                width: Val::Px(800.0),
                height: Val::Px(600.0),
                align_items: AlignItems::Start,
                justify_content: JustifyContent::Start,
                ..default()
            },
            BorderColor::all(Color::BLACK),
            BackgroundColor(Color::srgba(0.2, 0.2, 0.2, 0.7)),
        )).with_children(|parent| {
            parent.spawn((
                Text::new("Settings"),
                TextFont { font_size: FontSize::Px(16.0), ..default() },
                TextColor(Color::BLACK),
                Node {
                    position_type: PositionType::Absolute,
                    top: Val::Px(10.),
                    left: Val::Px(10.),
                    ..default()
                },
            ));
        });
    });
}
