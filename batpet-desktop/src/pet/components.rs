use bevy::prelude::{Component, Resource, Vec2, Vec3};

#[derive(Component, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Bat;

#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct FlightMotion {
    pub elapsed: f32,
    pub origin: Vec3,
}

impl Default for FlightMotion {
    fn default() -> Self {
        Self {
            elapsed: 0.0,
            origin: Vec3::ZERO,
        }
    }
}

#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct EyePupil {
    pub base_position: Vec2,
}

#[derive(Component, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct EyeLid;

#[derive(Debug, Default, Clone, Copy, PartialEq, Resource)]
pub struct CursorState {
    pub position: Option<Vec2>,
}
