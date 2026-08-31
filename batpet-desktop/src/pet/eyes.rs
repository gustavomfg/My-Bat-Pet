use bevy::{
    log::info,
    prelude::{Query, Res, ResMut, Resource, Time, Transform, Vec2, With, Without},
    window::{PrimaryWindow, Window},
};

use crate::{
    debug::DebugOptions,
    pet::{Bat, CursorState, EyePupil},
    rendering::DISPLAY_HEIGHT,
};

pub const EYE_CENTER_DEAD_ZONE: f32 = 28.0;
pub const MAX_PUPIL_OFFSET_X: f32 = 8.0;
pub const MAX_PUPIL_OFFSET_Y: f32 = 8.0;
pub const EYE_SMOOTHING: f32 = 14.0;

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

impl EyeDirection {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Center => "center",
            Self::Up => "up",
            Self::UpRight => "up-right",
            Self::Right => "right",
            Self::DownRight => "down-right",
            Self::Down => "down",
            Self::DownLeft => "down-left",
            Self::Left => "left",
            Self::UpLeft => "up-left",
        }
    }

    pub const fn offset(self) -> Vec2 {
        match self {
            Self::Center => Vec2::ZERO,
            Self::Up => Vec2::new(0.0, MAX_PUPIL_OFFSET_Y),
            Self::UpRight => Vec2::new(MAX_PUPIL_OFFSET_X, MAX_PUPIL_OFFSET_Y),
            Self::Right => Vec2::new(MAX_PUPIL_OFFSET_X, 0.0),
            Self::DownRight => Vec2::new(MAX_PUPIL_OFFSET_X, -MAX_PUPIL_OFFSET_Y),
            Self::Down => Vec2::new(0.0, -MAX_PUPIL_OFFSET_Y),
            Self::DownLeft => Vec2::new(-MAX_PUPIL_OFFSET_X, -MAX_PUPIL_OFFSET_Y),
            Self::Left => Vec2::new(-MAX_PUPIL_OFFSET_X, 0.0),
            Self::UpLeft => Vec2::new(-MAX_PUPIL_OFFSET_X, MAX_PUPIL_OFFSET_Y),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Resource)]
pub struct EyeState {
    pub direction: EyeDirection,
    pub target_offset: Vec2,
    pub offset: Vec2,
}

impl Default for EyeState {
    fn default() -> Self {
        Self {
            direction: EyeDirection::Center,
            target_offset: Vec2::ZERO,
            offset: Vec2::ZERO,
        }
    }
}

pub fn update_eyes(
    time: Res<Time>,
    cursor: Res<CursorState>,
    windows: Query<&Window, With<PrimaryWindow>>,
    bats: Query<&Transform, (With<Bat>, Without<EyePupil>)>,
    mut eye_state: ResMut<EyeState>,
    mut pupils: Query<(&EyePupil, &mut Transform)>,
    debug: Res<DebugOptions>,
) {
    let Some(window) = windows.iter().next() else {
        return;
    };
    let Some(bat_transform) = bats.iter().next() else {
        return;
    };

    let delta = cursor
        .position
        .map(|position| {
            let window_center = Vec2::new(
                window.resolution.width() * 0.5,
                window.resolution.height() * 0.5,
            );
            let cursor_world =
                Vec2::new(position.x - window_center.x, window_center.y - position.y);
            let bat_center =
                bat_transform.translation.truncate() + Vec2::new(0.0, -DISPLAY_HEIGHT * 0.5);
            cursor_world - bat_center
        })
        .unwrap_or(Vec2::ZERO);

    let direction = direction_for(delta);
    let target_offset = direction.offset();
    let blend = (1.0 - (-EYE_SMOOTHING * time.delta_secs()).exp()).clamp(0.0, 1.0);
    let direction_changed = eye_state.direction != direction;

    eye_state.direction = direction;
    eye_state.target_offset = target_offset;
    eye_state.offset = eye_state.offset.lerp(target_offset, blend);

    if direction_changed && debug.enabled {
        info!("eye direction={}", direction.label());
    }

    for (pupil, mut transform) in &mut pupils {
        transform.translation.x = pupil.base_position.x + eye_state.offset.x;
        transform.translation.y = pupil.base_position.y + eye_state.offset.y;
    }
}

pub fn direction_for(delta: Vec2) -> EyeDirection {
    if delta.length_squared() <= EYE_CENTER_DEAD_ZONE * EYE_CENTER_DEAD_ZONE {
        return EyeDirection::Center;
    }

    let diagonal_threshold = 2.414_214;
    let x = delta.x;
    let y = delta.y;

    if x.abs() > y.abs() * diagonal_threshold {
        return if x.is_sign_positive() {
            EyeDirection::Right
        } else {
            EyeDirection::Left
        };
    }

    if y.abs() > x.abs() * diagonal_threshold {
        return if y.is_sign_positive() {
            EyeDirection::Up
        } else {
            EyeDirection::Down
        };
    }

    match (x.is_sign_positive(), y.is_sign_positive()) {
        (true, true) => EyeDirection::UpRight,
        (true, false) => EyeDirection::DownRight,
        (false, true) => EyeDirection::UpLeft,
        (false, false) => EyeDirection::DownLeft,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn centers_when_cursor_is_near_the_bat() {
        assert_eq!(direction_for(Vec2::new(10.0, 10.0)), EyeDirection::Center);
    }

    #[test]
    fn detects_cardinal_and_diagonal_directions() {
        assert_eq!(direction_for(Vec2::X * 100.0), EyeDirection::Right);
        assert_eq!(direction_for(Vec2::NEG_X * 100.0), EyeDirection::Left);
        assert_eq!(direction_for(Vec2::Y * 100.0), EyeDirection::Up);
        assert_eq!(direction_for(Vec2::NEG_Y * 100.0), EyeDirection::Down);
        assert_eq!(
            direction_for(Vec2::new(100.0, 100.0)),
            EyeDirection::UpRight
        );
        assert_eq!(
            direction_for(Vec2::new(-100.0, 100.0)),
            EyeDirection::UpLeft
        );
        assert_eq!(
            direction_for(Vec2::new(100.0, -100.0)),
            EyeDirection::DownRight
        );
        assert_eq!(
            direction_for(Vec2::new(-100.0, -100.0)),
            EyeDirection::DownLeft
        );
    }

    #[test]
    fn pupil_offsets_stay_within_configured_limits() {
        for direction in [
            EyeDirection::Center,
            EyeDirection::Up,
            EyeDirection::UpRight,
            EyeDirection::Right,
            EyeDirection::DownRight,
            EyeDirection::Down,
            EyeDirection::DownLeft,
            EyeDirection::Left,
            EyeDirection::UpLeft,
        ] {
            let offset = direction.offset();
            assert!(offset.x.abs() <= MAX_PUPIL_OFFSET_X);
            assert!(offset.y.abs() <= MAX_PUPIL_OFFSET_Y);
        }
    }

    #[test]
    fn smoothing_moves_toward_target_without_teleporting() {
        let current = Vec2::ZERO;
        let target = EyeDirection::Right.offset();
        let blend = 1.0 - (-EYE_SMOOTHING * (1.0 / 60.0)).exp();
        let next = current.lerp(target, blend);

        assert!(next.x > 0.0);
        assert!(next.x < target.x);
    }
}
