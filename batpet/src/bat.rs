#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Position {
    pub x: u16,
    pub y: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EyeDirection {
    #[default]
    Center,
    Up,
    UpRight,
    Right,
    DownRight,
    Down,
    DownLeft,
    Left,
    UpLeft,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BatState {
    #[default]
    HangingIdle,
    // Reserved for future behavior; only HangingIdle is active for now.
    HangingAlert,
    TakeOff,
    Flying,
    Landing,
}

impl BatState {
    pub const fn label(self) -> &'static str {
        match self {
            Self::HangingIdle => "HangingIdle",
            Self::HangingAlert => "HangingAlert",
            Self::TakeOff => "TakeOff",
            Self::Flying => "Flying",
            Self::Landing => "Landing",
        }
    }
}

impl EyeDirection {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Center => "Center",
            Self::Up => "Up",
            Self::UpRight => "UpRight",
            Self::Right => "Right",
            Self::DownRight => "DownRight",
            Self::Down => "Down",
            Self::DownLeft => "DownLeft",
            Self::Left => "Left",
            Self::UpLeft => "UpLeft",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WingFrame {
    #[default]
    Open,
    Mid,
    Closed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Bat {
    pub position: Position,
    pub mouse_position: Option<Position>,
    pub eye_direction: EyeDirection,
    pub state: BatState,
    pub wing_frame: WingFrame,
}

impl Bat {
    pub fn new(x: u16, y: u16) -> Self {
        Self {
            position: Position { x, y },
            mouse_position: None,
            eye_direction: EyeDirection::default(),
            state: BatState::default(),
            wing_frame: WingFrame::default(),
        }
    }

    pub fn set_position(&mut self, position: Position) {
        self.position = position;
    }

    pub fn set_mouse_position(&mut self, position: Position) {
        self.mouse_position = Some(position);
    }
}
