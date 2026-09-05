use bevy::prelude::States;

#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum BatState {
    #[default]
    HangingIdle,
    Reacting,
    Takeoff,
    Flying,
    Returning,
    Landing,
}

impl BatState {
    pub const fn label(self) -> &'static str {
        match self {
            Self::HangingIdle => "HangingIdle",
            Self::Reacting => "Reacting",
            Self::Takeoff => "Takeoff",
            Self::Flying => "Flying",
            Self::Returning => "Return",
            Self::Landing => "Landing",
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
        assert_eq!(BatState::Reacting.label(), "Reacting");
        assert_eq!(BatState::Takeoff.label(), "Takeoff");
        assert_eq!(BatState::Flying.label(), "Flying");
        assert_eq!(BatState::Returning.label(), "Return");
        assert_eq!(BatState::Landing.label(), "Landing");
    }
}
