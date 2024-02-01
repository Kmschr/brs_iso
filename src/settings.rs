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
    commands.spawn(NodeBundle {
        style: Style {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
        ..default()
    }).with_children(|parent| {
        parent.spawn(NodeBundle {
            style: Style {
                width: Val::Px(800.0),
                height: Val::Px(600.0),
                align_items: AlignItems::Start,
                justify_content: JustifyContent::Start,
                ..default()
            },
            border_color: BorderColor(Color::BLACK),
            background_color: BackgroundColor(Color::rgba(0.2, 0.2, 0.2, 0.7)),
            ..default()
        }).with_children(|parent| {
            parent.spawn(TextBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    top: Val::Px(10.),
                    left: Val::Px(10.),
                    ..default()
                },
                text: Text::from_section("Settings", TextStyle {
                    color: Color::BLACK,
                    font_size: 16.0,
                    ..default()
                }),
                ..default()
            });
        });
    });
}
