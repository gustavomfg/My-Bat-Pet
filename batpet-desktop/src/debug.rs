use bevy::prelude::Resource;

#[derive(Debug, Clone, Copy, Resource)]
pub struct DebugOptions {
    pub enabled: bool,
}

impl DebugOptions {
    pub fn from_args() -> Self {
        Self {
            enabled: std::env::args().any(|argument| argument == "--debug"),
        }
    }
}
