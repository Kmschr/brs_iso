use bevy::prelude::*;

#[derive(Resource, Default)]
pub struct GameState {
    pub input: InputState,
    pub bvh_view: BVHView,
    pub light_debug: bool,
}

/// True once a build has been loaded; false again after it's cleared.
#[derive(Resource, Default)]
pub struct BuildLoaded(pub bool);

/// Whether the brick-hover info window is enabled (off by default, toggled via console).
#[derive(Resource, Default)]
pub struct BrickInfoEnabled(pub bool);

/// True while a screenshot is being captured, so overlay UI can hide itself.
#[derive(Resource, Default)]
pub struct Screenshotting(pub bool);

/// UI overlays that should be hidden from screenshots (FPS, console, prompt, ...).
#[derive(Component)]
pub struct HideOnScreenshot;

impl GameState {
    pub fn input_listening(&self) -> bool {
        match self.input {
            InputState::Listen => true,
            InputState::Typing => false,
        }
    }
}

#[derive(Default)]
pub enum InputState {
    #[default]
    Listen,
    Typing,
}

#[derive(Default)]
pub enum BVHView {
    #[default]
    Off,
    On(u8)
}
