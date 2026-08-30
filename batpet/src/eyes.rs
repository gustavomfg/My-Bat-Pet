use std::cmp::Ordering;

use crate::bat::{Bat, EyeDirection, Position};

pub const DEFAULT_CENTER_RADIUS: u16 = 1;

// Coordinates are expressed in the logical pixel grid of the terminal sprite.
pub const LEFT_EYE_BASE: (usize, usize) = (5, 8);
pub const RIGHT_EYE_BASE: (usize, usize) = (12, 8);
pub const EYE_BASE_SIZE: usize = 3;

pub const MAX_PUPIL_OFFSET_X: i32 = 1;
pub const MAX_PUPIL_OFFSET_Y: i32 = 1;
pub const EYE_SMOOTHING_FACTOR: f32 = 0.35;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct EyeOffset {
    pub x: f32,
    pub y: f32,
}

impl EyeOffset {
    pub const CENTER: Self = Self { x: 0.0, y: 0.0 };

    pub fn rounded(self) -> (i32, i32) {
        (self.x.round() as i32, self.y.round() as i32)
    }
}

pub struct Eyes {
    center_radius: u16,
    target_offset: EyeOffset,
    current_offset: EyeOffset,
}

impl Eyes {
    pub fn new() -> Self {
        Self::with_center_radius(DEFAULT_CENTER_RADIUS)
    }

    pub fn with_center_radius(center_radius: u16) -> Self {
        Self {
            center_radius,
            target_offset: EyeOffset::CENTER,
            current_offset: EyeOffset::CENTER,
        }
    }

    pub fn update(&mut self, bat: &mut Bat) -> bool {
        let next_direction = self.direction_for(bat.position, bat.mouse_position);
        let next_target = offset_for_direction(next_direction);
        let direction_changed = bat.eye_direction != next_direction;
        let target_changed = self.target_offset != next_target;

        bat.eye_direction = next_direction;
        self.target_offset = next_target;

        let next_offset = smooth_offset(self.current_offset, next_target);
        let offset_changed = self.current_offset != next_offset;
        self.current_offset = next_offset;

        direction_changed || target_changed || offset_changed
    }

    pub fn target_offset(&self) -> EyeOffset {
        self.target_offset
    }

    pub fn current_offset(&self) -> EyeOffset {
        self.current_offset
    }

    pub fn direction_for(
        &self,
        bat_position: Position,
        mouse_position: Option<Position>,
    ) -> EyeDirection {
        let Some(mouse_position) = mouse_position else {
            return EyeDirection::Center;
        };

        let dx = mouse_position.x as i32 - bat_position.x as i32;
        let dy = mouse_position.y as i32 - bat_position.y as i32;
        let distance_x = dx.abs() as u16;
        let distance_y = dy.abs() as u16;

        if distance_x <= self.center_radius && distance_y <= self.center_radius {
            return EyeDirection::Center;
        }

        let horizontal = match dx.cmp(&0) {
            Ordering::Less => EyeDirection::Left,
            Ordering::Greater => EyeDirection::Right,
            Ordering::Equal => EyeDirection::Center,
        };
        let vertical = match dy.cmp(&0) {
            Ordering::Less => EyeDirection::Up,
            Ordering::Greater => EyeDirection::Down,
            Ordering::Equal => EyeDirection::Center,
        };

        if distance_x > distance_y * 2 {
            return horizontal;
        }

        if distance_y > distance_x * 2 {
            return vertical;
        }

        match (horizontal, vertical) {
            (EyeDirection::Left, EyeDirection::Up) => EyeDirection::UpLeft,
            (EyeDirection::Right, EyeDirection::Up) => EyeDirection::UpRight,
            (EyeDirection::Left, EyeDirection::Down) => EyeDirection::DownLeft,
            (EyeDirection::Right, EyeDirection::Down) => EyeDirection::DownRight,
            (EyeDirection::Center, direction) | (direction, EyeDirection::Center) => direction,
            _ => EyeDirection::Center,
        }
    }
}

fn offset_for_direction(direction: EyeDirection) -> EyeOffset {
    let horizontal = MAX_PUPIL_OFFSET_X as f32;
    let vertical = MAX_PUPIL_OFFSET_Y as f32;

    match direction {
        EyeDirection::Center => EyeOffset::CENTER,
        EyeDirection::Up => EyeOffset {
            x: 0.0,
            y: -vertical,
        },
        EyeDirection::UpRight => EyeOffset {
            x: horizontal,
            y: -vertical,
        },
        EyeDirection::Right => EyeOffset {
            x: horizontal,
            y: 0.0,
        },
        EyeDirection::DownRight => EyeOffset {
            x: horizontal,
            y: vertical,
        },
        EyeDirection::Down => EyeOffset {
            x: 0.0,
            y: vertical,
        },
        EyeDirection::DownLeft => EyeOffset {
            x: -horizontal,
            y: vertical,
        },
        EyeDirection::Left => EyeOffset {
            x: -horizontal,
            y: 0.0,
        },
        EyeDirection::UpLeft => EyeOffset {
            x: -horizontal,
            y: -vertical,
        },
    }
}

fn smooth_offset(current: EyeOffset, target: EyeOffset) -> EyeOffset {
    EyeOffset {
        x: smooth_component(current.x, target.x),
        y: smooth_component(current.y, target.y),
    }
}

fn smooth_component(current: f32, target: f32) -> f32 {
    let factor = EYE_SMOOTHING_FACTOR.clamp(0.0, 1.0);
    let delta = target - current;
    if delta.abs() < 0.01 || factor >= 1.0 {
        target
    } else {
        current + delta * factor
    }
}

impl Default for Eyes {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BAT_POSITION: Position = Position { x: 10, y: 10 };

    fn direction(x: u16, y: u16) -> EyeDirection {
        Eyes::new().direction_for(BAT_POSITION, Some(Position { x, y }))
    }

    #[test]
    fn detects_the_eight_directions() {
        assert_eq!(direction(10, 0), EyeDirection::Up);
        assert_eq!(direction(20, 10), EyeDirection::Right);
        assert_eq!(direction(10, 20), EyeDirection::Down);
        assert_eq!(direction(0, 10), EyeDirection::Left);
        assert_eq!(direction(0, 0), EyeDirection::UpLeft);
        assert_eq!(direction(20, 0), EyeDirection::UpRight);
        assert_eq!(direction(0, 20), EyeDirection::DownLeft);
        assert_eq!(direction(20, 20), EyeDirection::DownRight);
    }

    #[test]
    fn keeps_the_eyes_centered_when_the_mouse_is_near() {
        assert_eq!(direction(10, 10), EyeDirection::Center);
        assert_eq!(direction(11, 11), EyeDirection::Center);
        assert_eq!(direction(9, 10), EyeDirection::Center);
    }

    #[test]
    fn uses_the_nearest_cardinal_or_diagonal_direction() {
        assert_eq!(direction(16, 12), EyeDirection::Right);
        assert_eq!(direction(18, 11), EyeDirection::Right);
        assert_eq!(direction(11, 18), EyeDirection::Down);
    }

    #[test]
    fn smooths_the_pupil_toward_the_direction_target() {
        let mut eyes = Eyes::new();
        let mut bat = Bat::new(BAT_POSITION.x, BAT_POSITION.y);
        bat.set_mouse_position(Position { x: 20, y: 10 });

        eyes.update(&mut bat);

        assert_eq!(
            eyes.target_offset(),
            EyeOffset {
                x: MAX_PUPIL_OFFSET_X as f32,
                y: 0.0,
            }
        );
        assert!(eyes.current_offset().x > 0.0);
        assert!(eyes.current_offset().x < eyes.target_offset().x);

        let previous = eyes.current_offset();
        eyes.update(&mut bat);
        assert!(eyes.current_offset().x > previous.x);
    }

    #[test]
    fn centers_the_pupil_target_when_the_mouse_is_near() {
        let mut eyes = Eyes::new();
        let mut bat = Bat::new(BAT_POSITION.x, BAT_POSITION.y);
        bat.set_mouse_position(Position { x: 20, y: 10 });
        eyes.update(&mut bat);

        bat.set_mouse_position(BAT_POSITION);
        eyes.update(&mut bat);

        assert_eq!(eyes.target_offset(), EyeOffset::CENTER);
        assert!(eyes.current_offset().x > 0.0);
    }
}
