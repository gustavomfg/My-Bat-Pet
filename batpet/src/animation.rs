use std::time::{Duration, Instant};

use crate::bat::{Bat, WingFrame};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AnimationFrame {
    #[default]
    Flight00,
    Flight01,
    Flight02,
    Flight03,
}

impl AnimationFrame {
    pub const FLIGHT: [Self; 4] = [
        Self::Flight00,
        Self::Flight01,
        Self::Flight02,
        Self::Flight03,
    ];

    pub fn from_index(index: usize) -> Self {
        Self::FLIGHT[index % Self::FLIGHT.len()]
    }

    pub fn index(self) -> usize {
        match self {
            Self::Flight00 => 0,
            Self::Flight01 => 1,
            Self::Flight02 => 2,
            Self::Flight03 => 3,
        }
    }

    pub fn wing_frame(self) -> WingFrame {
        match self {
            Self::Flight00 => WingFrame::Open,
            Self::Flight01 | Self::Flight03 => WingFrame::Mid,
            Self::Flight02 => WingFrame::Closed,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Flight00 => "open",
            Self::Flight01 | Self::Flight03 => "mid",
            Self::Flight02 => "closed",
        }
    }
}

const WING_FRAMES: [WingFrame; 4] = [
    WingFrame::Open,
    WingFrame::Mid,
    WingFrame::Closed,
    WingFrame::Mid,
];

pub const WING_FPS: u32 = 8;
pub const WING_FRAME_DURATION: Duration = Duration::from_millis(1000 / WING_FPS as u64);

pub struct WingAnimation {
    frame_index: usize,
    last_update: Instant,
}

impl WingAnimation {
    pub fn new() -> Self {
        Self {
            frame_index: 0,
            last_update: Instant::now(),
        }
    }

    pub fn current_frame(&self) -> WingFrame {
        WING_FRAMES[self.frame_index]
    }

    pub fn current_animation_frame(&self) -> AnimationFrame {
        AnimationFrame::from_index(self.frame_index)
    }

    pub fn update(&mut self, bat: &mut Bat) -> bool {
        self.update_at(Instant::now(), bat)
    }

    fn update_at(&mut self, now: Instant, bat: &mut Bat) -> bool {
        let elapsed = now.saturating_duration_since(self.last_update);
        let elapsed_frames = elapsed.as_nanos() / WING_FRAME_DURATION.as_nanos();

        if elapsed_frames == 0 {
            return false;
        }

        self.frame_index = (self.frame_index + elapsed_frames as usize) % WING_FRAMES.len();

        let consumed_nanos = WING_FRAME_DURATION
            .as_nanos()
            .saturating_mul(elapsed_frames)
            .min(u64::MAX as u128) as u64;
        let consumed = Duration::from_nanos(consumed_nanos);
        self.last_update = self.last_update.checked_add(consumed).unwrap_or(now);

        let next_frame = self.current_frame();
        let changed = bat.wing_frame != next_frame;
        bat.wing_frame = next_frame;
        changed
    }
}

impl Default for WingAnimation {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bat::Bat;

    #[test]
    fn preserves_the_four_logical_frame_sequence() {
        assert_eq!(AnimationFrame::from_index(0), AnimationFrame::Flight00);
        assert_eq!(AnimationFrame::from_index(1), AnimationFrame::Flight01);
        assert_eq!(AnimationFrame::from_index(2), AnimationFrame::Flight02);
        assert_eq!(AnimationFrame::from_index(3), AnimationFrame::Flight03);
        assert_eq!(AnimationFrame::from_index(4), AnimationFrame::Flight00);
    }

    #[test]
    fn maps_the_duplicate_mid_wing_states() {
        assert_eq!(AnimationFrame::Flight01.wing_frame(), WingFrame::Mid);
        assert_eq!(AnimationFrame::Flight03.wing_frame(), WingFrame::Mid);
    }

    #[test]
    fn advances_through_the_four_wing_frames() {
        let start = Instant::now();
        let mut animation = WingAnimation {
            frame_index: 0,
            last_update: start,
        };
        let mut bat = Bat::new(0, 0);

        assert_eq!(animation.current_frame(), WingFrame::Open);

        animation.update_at(start + WING_FRAME_DURATION, &mut bat);
        assert_eq!(bat.wing_frame, WingFrame::Mid);

        animation.update_at(start + WING_FRAME_DURATION * 2, &mut bat);
        assert_eq!(bat.wing_frame, WingFrame::Closed);

        animation.update_at(start + WING_FRAME_DURATION * 3, &mut bat);
        assert_eq!(bat.wing_frame, WingFrame::Mid);

        animation.update_at(start + WING_FRAME_DURATION * 4, &mut bat);
        assert_eq!(bat.wing_frame, WingFrame::Open);
    }

    #[test]
    fn catches_up_when_the_main_loop_is_slow() {
        let start = Instant::now();
        let mut animation = WingAnimation {
            frame_index: 0,
            last_update: start,
        };
        let mut bat = Bat::new(0, 0);

        animation.update_at(start + WING_FRAME_DURATION * 3, &mut bat);

        assert_eq!(bat.wing_frame, WingFrame::Mid);
    }
}
