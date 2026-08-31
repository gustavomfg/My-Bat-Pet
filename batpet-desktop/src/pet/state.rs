use bevy::prelude::States;

#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum BatState {
    #[default]
    HangingIdle,
    Flying,
}

impl BatState {
    pub const fn label(self) -> &'static str {
        match self {
            Self::HangingIdle => "HangingIdle",
            Self::Flying => "Flying",
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
        assert_eq!(BatState::Flying.label(), "Flying");
    }
}
