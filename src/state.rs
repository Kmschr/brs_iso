use bevy::prelude::*;

#[derive(Resource, Default)]
pub struct GameState {
    pub input: InputState,
    pub bvh_view: BVHView,
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
