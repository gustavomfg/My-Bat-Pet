use bevy::{
    log::info,
    prelude::{Query, Res, ResMut, Resource, Time, Transform, Vec2, With, Without},
    window::{PrimaryWindow, Window},
};

use crate::{
    debug::DebugOptions,
    pet::{AttentionMotion, Bat, CursorState, EyePupil},
    rendering::EYE_CENTER_LOCAL,
};

use super::acting::{IDLE_GAZE_MAX_ATTENTION, IdleGazeMotion, idle_gaze_target};

pub const EYE_CENTER_DEAD_ZONE: f32 = 28.0;
pub const MAX_PUPIL_OFFSET_X: f32 = 6.0;
pub const MAX_PUPIL_OFFSET_Y: f32 = 6.0;
pub const EYE_SMOOTHING: f32 = 12.0;
pub const EYE_INFLUENCE_DISTANCE: f32 = 144.0;

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
    bats: Query<(&Transform, &AttentionMotion), (With<Bat>, Without<EyePupil>)>,
    gazes: Query<&IdleGazeMotion, With<Bat>>,
    mut eye_state: ResMut<EyeState>,
    mut pupils: Query<(&EyePupil, &mut Transform)>,
    debug: Res<DebugOptions>,
) {
    let Some(window) = windows.iter().next() else {
        return;
    };
    let Some((bat_transform, attention)) = bats.iter().next() else {
        return;
    };

    let delta = cursor_delta(cursor.position, window, bat_transform);

    let direction = direction_for(delta);
    let cursor_target = eye_target_offset(delta);
    let target_offset = if attention.target_level <= IDLE_GAZE_MAX_ATTENTION {
        let gaze_target = gazes.iter().next().map_or(Vec2::ZERO, idle_gaze_target);
        if gaze_target == Vec2::ZERO {
            cursor_target
        } else {
            gaze_target
        }
    } else {
        cursor_target
    };
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

pub(crate) fn cursor_delta(
    cursor_position: Option<Vec2>,
    window: &Window,
    bat_transform: &Transform,
) -> Vec2 {
    cursor_position
        .map(|position| {
            let window_center = Vec2::new(
                window.resolution.width() * 0.5,
                window.resolution.height() * 0.5,
            );
            let cursor_world =
                Vec2::new(position.x - window_center.x, window_center.y - position.y);
            let eye_center = bat_transform.translation.truncate() + EYE_CENTER_LOCAL;
            cursor_world - eye_center
        })
        .unwrap_or(Vec2::ZERO)
}

pub fn eye_target_offset(delta: Vec2) -> Vec2 {
    let distance = delta.length();

    if distance <= EYE_CENTER_DEAD_ZONE || !distance.is_finite() {
        return Vec2::ZERO;
    }

    let direction = delta / distance;
    let influence =
        smoothstep(((distance - EYE_CENTER_DEAD_ZONE) / EYE_INFLUENCE_DISTANCE).clamp(0.0, 1.0));

    Vec2::new(
        (direction.x * MAX_PUPIL_OFFSET_X * influence)
            .clamp(-MAX_PUPIL_OFFSET_X, MAX_PUPIL_OFFSET_X),
        (direction.y * MAX_PUPIL_OFFSET_Y * influence)
            .clamp(-MAX_PUPIL_OFFSET_Y, MAX_PUPIL_OFFSET_Y),
    )
}

pub fn direction_for(delta: Vec2) -> EyeDirection {
    if !delta.x.is_finite()
        || !delta.y.is_finite()
        || delta.length_squared() <= EYE_CENTER_DEAD_ZONE * EYE_CENTER_DEAD_ZONE
    {
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

fn smoothstep(value: f32) -> f32 {
    value * value * (3.0 - 2.0 * value)
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
    fn continuous_pupil_target_has_a_dead_zone_and_smooth_influence() {
        assert_eq!(eye_target_offset(Vec2::new(10.0, 10.0)), Vec2::ZERO);

        let near = eye_target_offset(Vec2::new(EYE_CENTER_DEAD_ZONE + 1.0, 0.0));
        let far = eye_target_offset(Vec2::new(10_000.0, 0.0));

        assert!(near.x > 0.0);
        assert!(near.x < MAX_PUPIL_OFFSET_X);
        assert_eq!(far, Vec2::new(MAX_PUPIL_OFFSET_X, 0.0));
    }

    #[test]
    fn continuous_pupil_target_stays_inside_the_eye_limits() {
        for delta in [
            Vec2::new(1.0, 100.0),
            Vec2::new(-100.0, 1.0),
            Vec2::new(100.0, 100.0),
            Vec2::new(-100.0, -100.0),
        ] {
            let target = eye_target_offset(delta);
            assert!(target.x.abs() <= MAX_PUPIL_OFFSET_X);
            assert!(target.y.abs() <= MAX_PUPIL_OFFSET_Y);
        }
    }

    #[test]
    fn non_finite_cursor_delta_is_treated_as_center() {
        assert_eq!(
            direction_for(Vec2::new(f32::NAN, 1.0)),
            EyeDirection::Center
        );
        assert_eq!(eye_target_offset(Vec2::new(f32::INFINITY, 1.0)), Vec2::ZERO);
    }

    #[test]
    fn smoothing_moves_toward_target_without_teleporting() {
        let current = Vec2::ZERO;
        let target = Vec2::new(MAX_PUPIL_OFFSET_X, 0.0);
        let blend = 1.0 - (-EYE_SMOOTHING * (1.0 / 60.0)).exp();
        let next = current.lerp(target, blend);

        assert!(next.x > 0.0);
        assert!(next.x < target.x);
    }
}
