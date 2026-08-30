use bevy::prelude::States;

#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum BatState {
    #[default]
    HangingIdle,
}

impl BatState {
    pub const fn label(self) -> &'static str {
        match self {
            Self::HangingIdle => "HangingIdle",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starts_in_hanging_idle() {
        assert_eq!(BatState::default(), BatState::HangingIdle);
        assert_eq!(BatState::HangingIdle.label(), "HangingIdle");
    }
}
