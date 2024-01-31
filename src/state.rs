use bevy::prelude::*;

#[derive(Resource, Default)]
pub struct GameState {
    pub input: InputState,
    pub bvh_view: BVHView,
}

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
