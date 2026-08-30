use bevy::prelude::{Component, Resource, Vec2};

#[derive(Component, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Bat;

#[derive(Debug, Default, Clone, Copy, PartialEq, Resource)]
pub struct CursorState {
    pub position: Option<Vec2>,
}
