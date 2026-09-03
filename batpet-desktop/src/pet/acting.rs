use bevy::{
    log::info,
    prelude::{Component, Query, Res, ResMut, Time, Transform, Vec2, With},
    window::{PrimaryWindow, Window},
};

use crate::{
    debug::DebugOptions,
    pet::{
        Bat, CursorState,
        eyes::{EyeDirection, cursor_delta, direction_for},
        visual::{AnimationIntent, AttentionBodyFrame, AttentionIntent, AttentionReaction},
    },
};

use super::idle::IdleScheduler;

pub const ATTENTION_FAR_DISTANCE: f32 = 260.0;
pub const VERY_NEAR_START_DISTANCE: f32 = 104.0;
pub const VERY_NEAR_FULL_DISTANCE: f32 = 64.0;

pub const ATTENTION_LEVEL_SMOOTHING: f32 = 5.0;
pub const HEAD_FOLLOW_SMOOTHING: f32 = 7.5;
pub const BODY_FOLLOW_SMOOTHING: f32 = 2.8;
pub const EAR_FOLLOW_SMOOTHING: f32 = 5.0;

pub const MAX_HEAD_OFFSET_X: f32 = 1.35;
pub const MAX_HEAD_OFFSET_Y: f32 = 0.45;
pub const MAX_BODY_OFFSET_X: f32 = 0.55;
pub const MAX_BODY_OFFSET_Y: f32 = 0.18;

pub const ATTENTION_MEDIUM_THRESHOLD: f32 = 0.22;
pub const ATTENTION_NEAR_THRESHOLD: f32 = 0.55;
pub const EAR_ATTENTIVE_THRESHOLD: f32 = 0.30;
pub const BACKGROUND_ACTION_MAX_ATTENTION: f32 = 0.28;
pub const IDLE_GAZE_MAX_ATTENTION: f32 = 0.16;
pub const BODY_DIRECTION_DEAD_ZONE: f32 = 0.12;

pub const VERY_NEAR_TRIGGER_FACTOR: f32 = 0.55;
pub const VERY_NEAR_ANTICIPATION_DURATION: f32 = 0.10;
pub const VERY_NEAR_RECOIL_DURATION: f32 = 0.18;
pub const VERY_NEAR_RECOVERY_DURATION: f32 = 0.55;
pub const VERY_NEAR_REACTION_COOLDOWN: f32 = 2.8;
pub const VERY_NEAR_RECOIL_OFFSET: f32 = 0.65;
pub const VERY_NEAR_COMPRESSION: f32 = 0.008;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AttentionProfile {
    pub level: f32,
    pub very_near_factor: f32,
    pub direction: EyeDirection,
    pub head_target: Vec2,
    pub body_target: Vec2,
    pub ear_target: f32,
}

impl Default for AttentionProfile {
    fn default() -> Self {
        Self {
            level: 0.0,
            very_near_factor: 0.0,
            direction: EyeDirection::Center,
            head_target: Vec2::ZERO,
            body_target: Vec2::ZERO,
            ear_target: 0.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ReactionPhase {
    #[default]
    Idle,
    Anticipating,
    Recoiling,
    Recovering,
}

#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct AttentionMotion {
    pub level: f32,
    pub target_level: f32,
    pub head_offset: Vec2,
    pub target_head_offset: Vec2,
    pub body_offset: Vec2,
    pub target_body_offset: Vec2,
    pub ear_alertness: f32,
    pub target_ear_alertness: f32,
    pub direction: EyeDirection,
    pub reaction_phase: ReactionPhase,
    pub reaction_elapsed: f32,
    pub reaction_cooldown: f32,
    pub reaction_direction: Vec2,
    pub was_very_near: bool,
}

impl Default for AttentionMotion {
    fn default() -> Self {
        Self {
            level: 0.0,
            target_level: 0.0,
            head_offset: Vec2::ZERO,
            target_head_offset: Vec2::ZERO,
            body_offset: Vec2::ZERO,
            target_body_offset: Vec2::ZERO,
            ear_alertness: 0.0,
            target_ear_alertness: 0.0,
            direction: EyeDirection::Center,
            reaction_phase: ReactionPhase::Idle,
            reaction_elapsed: 0.0,
            reaction_cooldown: 0.0,
            reaction_direction: Vec2::ZERO,
            was_very_near: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum IdleGazePhase {
    #[default]
    Resting,
    Looking,
    Returning,
}

#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct IdleGazeMotion {
    pub phase: IdleGazePhase,
    pub elapsed: f32,
    pub duration: f32,
    pub target_offset: Vec2,
}

impl Default for IdleGazeMotion {
    fn default() -> Self {
        Self {
            phase: IdleGazePhase::Resting,
            elapsed: 0.0,
            duration: 0.0,
            target_offset: Vec2::ZERO,
        }
    }
}

pub fn attention_profile(delta: Vec2) -> AttentionProfile {
    if !delta.x.is_finite() || !delta.y.is_finite() {
        return AttentionProfile::default();
    }

    let distance = delta.length();
    let direction_vector = if distance > f32::EPSILON {
        delta / distance
    } else {
        Vec2::ZERO
    };
    let level = attention_level_for_distance(distance);
    let very_near_factor = very_near_factor_for_distance(distance);
    let body_influence = smoothstep(level);

    AttentionProfile {
        level,
        very_near_factor,
        direction: direction_for(delta),
        head_target: Vec2::new(
            direction_vector.x * MAX_HEAD_OFFSET_X * level,
            direction_vector.y * MAX_HEAD_OFFSET_Y * level,
        ),
        body_target: Vec2::new(
            direction_vector.x * MAX_BODY_OFFSET_X * body_influence,
            direction_vector.y * MAX_BODY_OFFSET_Y * body_influence,
        ),
        ear_target: smoothstep(
            ((level - EAR_ATTENTIVE_THRESHOLD) / (1.0 - EAR_ATTENTIVE_THRESHOLD)).clamp(0.0, 1.0),
        ),
    }
}

pub fn attention_level_for_distance(distance: f32) -> f32 {
    if !distance.is_finite() {
        return 0.0;
    }

    1.0 - smoothstep(
        ((distance - VERY_NEAR_FULL_DISTANCE) / (ATTENTION_FAR_DISTANCE - VERY_NEAR_FULL_DISTANCE))
            .clamp(0.0, 1.0),
    )
}

pub fn very_near_factor_for_distance(distance: f32) -> f32 {
    if !distance.is_finite() {
        return 0.0;
    }

    smoothstep(
        ((VERY_NEAR_START_DISTANCE - distance)
            / (VERY_NEAR_START_DISTANCE - VERY_NEAR_FULL_DISTANCE))
            .clamp(0.0, 1.0),
    )
}

pub fn attention_body_frame(level: f32, direction: EyeDirection) -> AttentionBodyFrame {
    if !level.is_finite() || level < ATTENTION_MEDIUM_THRESHOLD {
        return AttentionBodyFrame::Relaxed;
    }

    if level < ATTENTION_NEAR_THRESHOLD {
        return AttentionBodyFrame::Attentive;
    }

    match direction {
        EyeDirection::Left | EyeDirection::UpLeft | EyeDirection::DownLeft => {
            AttentionBodyFrame::Left
        }
        EyeDirection::Right | EyeDirection::UpRight | EyeDirection::DownRight => {
            AttentionBodyFrame::Right
        }
        EyeDirection::Center | EyeDirection::Up | EyeDirection::Down => {
            AttentionBodyFrame::Attentive
        }
    }
}

pub fn update_attention(
    time: Res<Time>,
    cursor: Res<CursorState>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut bats: Query<(&Transform, &mut AttentionMotion, &mut AnimationIntent), With<Bat>>,
    debug: Res<DebugOptions>,
) {
    let Some(window) = windows.iter().next() else {
        return;
    };

    let delta_secs = safe_delta(time.delta_secs());

    for (transform, mut attention, mut intent) in &mut bats {
        let delta = cursor_delta(cursor.position, window, transform);
        let profile = if cursor.position.is_some() {
            attention_profile(delta)
        } else {
            AttentionProfile::default()
        };
        let direction_changed = attention.direction != profile.direction;

        attention.target_level = profile.level;
        attention.level = damp(
            attention.level,
            profile.level,
            ATTENTION_LEVEL_SMOOTHING,
            delta_secs,
        );
        attention.target_head_offset = profile.head_target;
        attention.head_offset = damp_vec2(
            attention.head_offset,
            profile.head_target,
            HEAD_FOLLOW_SMOOTHING,
            delta_secs,
        );
        attention.target_body_offset = profile.body_target;
        attention.body_offset = damp_vec2(
            attention.body_offset,
            profile.body_target,
            BODY_FOLLOW_SMOOTHING,
            delta_secs,
        );
        attention.target_ear_alertness = profile.ear_target;
        attention.ear_alertness = damp(
            attention.ear_alertness,
            profile.ear_target,
            EAR_FOLLOW_SMOOTHING,
            delta_secs,
        );
        attention.direction = profile.direction;

        let reaction_started =
            update_reaction(&mut attention, profile.very_near_factor, delta, delta_secs);
        let (reaction_offset, body_compression) = reaction_visual(
            attention.reaction_phase,
            attention.reaction_elapsed,
            attention.reaction_direction,
        );
        let reaction = if matches!(
            attention.reaction_phase,
            ReactionPhase::Recoiling | ReactionPhase::Recovering
        ) {
            AttentionReaction::VeryNear
        } else {
            AttentionReaction::None
        };

        intent.attention = AttentionIntent {
            level: attention.level,
            head_offset: attention.head_offset,
            body_offset: attention.body_offset + reaction_offset,
            ear_alertness: attention.ear_alertness,
            body_compression,
            body_frame: attention_body_frame_with_follow(
                attention.level,
                attention.direction,
                attention.body_offset,
            ),
            reaction,
        };

        if direction_changed && debug.enabled && attention.level >= ATTENTION_MEDIUM_THRESHOLD {
            info!("attention direction={}", attention.direction.label());
        }
        if reaction_started && debug.enabled {
            info!("very-near reaction started");
        }
    }
}

fn update_reaction(
    attention: &mut AttentionMotion,
    very_near_factor: f32,
    delta: Vec2,
    delta_secs: f32,
) -> bool {
    attention.reaction_cooldown = (attention.reaction_cooldown - delta_secs).max(0.0);
    let entered_very_near = very_near_factor >= VERY_NEAR_TRIGGER_FACTOR
        && !attention.was_very_near
        && attention.reaction_cooldown <= 0.0;
    let mut started = false;

    if entered_very_near {
        attention.reaction_phase = ReactionPhase::Anticipating;
        attention.reaction_elapsed = 0.0;
        attention.reaction_cooldown = VERY_NEAR_REACTION_COOLDOWN;
        attention.reaction_direction = if delta.length_squared() > f32::EPSILON {
            delta.normalize()
        } else {
            Vec2::ZERO
        };
        started = true;
    } else {
        tick_reaction(attention, delta_secs);
    }

    attention.was_very_near = very_near_factor >= VERY_NEAR_TRIGGER_FACTOR;
    started
}

fn tick_reaction(attention: &mut AttentionMotion, delta_secs: f32) {
    if matches!(attention.reaction_phase, ReactionPhase::Idle) {
        return;
    }

    let mut remaining = delta_secs;
    while remaining > 0.0 && !matches!(attention.reaction_phase, ReactionPhase::Idle) {
        let phase_duration = match attention.reaction_phase {
            ReactionPhase::Idle => 0.0,
            ReactionPhase::Anticipating => VERY_NEAR_ANTICIPATION_DURATION,
            ReactionPhase::Recoiling => VERY_NEAR_RECOIL_DURATION,
            ReactionPhase::Recovering => VERY_NEAR_RECOVERY_DURATION,
        };
        let phase_remaining = (phase_duration - attention.reaction_elapsed).max(0.0);

        if remaining < phase_remaining {
            attention.reaction_elapsed += remaining;
            break;
        }

        remaining -= phase_remaining;
        attention.reaction_elapsed = 0.0;
        attention.reaction_phase = match attention.reaction_phase {
            ReactionPhase::Idle => ReactionPhase::Idle,
            ReactionPhase::Anticipating => ReactionPhase::Recoiling,
            ReactionPhase::Recoiling => ReactionPhase::Recovering,
            ReactionPhase::Recovering => ReactionPhase::Idle,
        };
    }
}

pub fn reaction_visual(phase: ReactionPhase, elapsed: f32, direction: Vec2) -> (Vec2, f32) {
    let (progress, direction_multiplier) = match phase {
        ReactionPhase::Recoiling => ((elapsed / VERY_NEAR_RECOIL_DURATION).clamp(0.0, 1.0), -1.0),
        ReactionPhase::Recovering => (
            (elapsed / VERY_NEAR_RECOVERY_DURATION).clamp(0.0, 1.0),
            -1.0,
        ),
        ReactionPhase::Idle | ReactionPhase::Anticipating => (0.0, 0.0),
    };
    let intensity = match phase {
        ReactionPhase::Recoiling => ease_out(progress),
        ReactionPhase::Recovering => 1.0 - ease_in_out(progress),
        ReactionPhase::Idle | ReactionPhase::Anticipating => 0.0,
    };

    (
        direction * VERY_NEAR_RECOIL_OFFSET * intensity * direction_multiplier,
        VERY_NEAR_COMPRESSION * intensity,
    )
}

pub fn update_idle_gaze(
    time: Res<Time>,
    mut scheduler: ResMut<IdleScheduler>,
    attention: Query<&AttentionMotion, With<Bat>>,
    mut gazes: Query<&mut IdleGazeMotion, With<Bat>>,
    debug: Res<DebugOptions>,
) {
    let attention_level = attention
        .iter()
        .next()
        .map_or(0.0, |motion| motion.target_level);
    let delta_secs = safe_delta(time.delta_secs());

    for mut gaze in &mut gazes {
        if attention_level > IDLE_GAZE_MAX_ATTENTION {
            scheduler.defer_idle_gaze();
            if matches!(gaze.phase, IdleGazePhase::Looking) {
                begin_gaze_return(&mut gaze);
            }
        }

        match gaze.phase {
            IdleGazePhase::Resting => {
                if attention_level <= IDLE_GAZE_MAX_ATTENTION && scheduler.try_start_idle_gaze() {
                    gaze.phase = IdleGazePhase::Looking;
                    gaze.elapsed = 0.0;
                    gaze.duration = scheduler.next_idle_gaze_duration();
                    gaze.target_offset = scheduler.next_idle_gaze_offset();

                    if debug.enabled {
                        info!("idle gaze started");
                    }
                }
            }
            IdleGazePhase::Looking => {
                gaze.elapsed += delta_secs;
                if gaze.elapsed >= gaze.duration {
                    begin_gaze_return(&mut gaze);
                }
            }
            IdleGazePhase::Returning => {
                gaze.elapsed += delta_secs;
                if gaze.elapsed >= gaze.duration {
                    gaze.phase = IdleGazePhase::Resting;
                    gaze.elapsed = 0.0;
                    gaze.duration = 0.0;
                    gaze.target_offset = Vec2::ZERO;
                    scheduler.finish_idle_gaze();
                }
            }
        }
    }
}

fn attention_body_frame_with_follow(
    level: f32,
    direction: EyeDirection,
    body_offset: Vec2,
) -> AttentionBodyFrame {
    if level < ATTENTION_NEAR_THRESHOLD {
        return attention_body_frame(level, direction);
    }

    if body_offset.x.abs() <= BODY_DIRECTION_DEAD_ZONE {
        return AttentionBodyFrame::Attentive;
    }

    if body_offset.x.is_sign_negative() {
        AttentionBodyFrame::Left
    } else {
        AttentionBodyFrame::Right
    }
}

fn begin_gaze_return(gaze: &mut IdleGazeMotion) {
    gaze.phase = IdleGazePhase::Returning;
    gaze.elapsed = 0.0;
    gaze.duration = super::idle::GAZE_RETURN_DURATION;
}

pub fn idle_gaze_target(gaze: &IdleGazeMotion) -> Vec2 {
    match gaze.phase {
        IdleGazePhase::Resting => Vec2::ZERO,
        IdleGazePhase::Looking => gaze.target_offset,
        IdleGazePhase::Returning => {
            let remaining = 1.0 - (gaze.elapsed / gaze.duration.max(f32::EPSILON)).clamp(0.0, 1.0);
            gaze.target_offset * remaining
        }
    }
}

fn damp(current: f32, target: f32, response: f32, delta_secs: f32) -> f32 {
    current + (target - current) * smoothing_blend(response, delta_secs)
}

fn damp_vec2(current: Vec2, target: Vec2, response: f32, delta_secs: f32) -> Vec2 {
    current.lerp(target, smoothing_blend(response, delta_secs))
}

fn smoothing_blend(response: f32, delta_secs: f32) -> f32 {
    (1.0 - (-response * delta_secs.max(0.0)).exp()).clamp(0.0, 1.0)
}

fn safe_delta(delta_secs: f32) -> f32 {
    if delta_secs.is_finite() {
        delta_secs.max(0.0)
    } else {
        0.0
    }
}

fn smoothstep(value: f32) -> f32 {
    value * value * (3.0 - 2.0 * value)
}

fn ease_out(value: f32) -> f32 {
    1.0 - (1.0 - value).powi(2)
}

fn ease_in_out(value: f32) -> f32 {
    value * value * (3.0 - 2.0 * value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attention_is_high_near_and_fades_to_zero_far_away() {
        assert!(attention_level_for_distance(0.0) > 0.99);
        assert!(attention_level_for_distance(140.0) > attention_level_for_distance(220.0));
        assert_eq!(attention_level_for_distance(ATTENTION_FAR_DISTANCE), 0.0);
    }

    #[test]
    fn very_near_curve_has_a_soft_boundary_and_clamp() {
        assert_eq!(very_near_factor_for_distance(VERY_NEAR_START_DISTANCE), 0.0);
        assert_eq!(very_near_factor_for_distance(VERY_NEAR_FULL_DISTANCE), 1.0);
        assert_eq!(very_near_factor_for_distance(0.0), 1.0);
        assert_eq!(very_near_factor_for_distance(10_000.0), 0.0);
    }

    #[test]
    fn attention_profile_keeps_body_response_smaller_than_head_response() {
        let profile = attention_profile(Vec2::new(180.0, 0.0));

        assert!(profile.head_target.x.abs() > profile.body_target.x.abs());
        assert!(profile.body_target.x.abs() <= MAX_BODY_OFFSET_X);
        assert!(profile.head_target.x.abs() <= MAX_HEAD_OFFSET_X);
    }

    #[test]
    fn attention_body_frame_changes_only_after_relevance_thresholds() {
        assert_eq!(
            attention_body_frame(0.1, EyeDirection::Right),
            AttentionBodyFrame::Relaxed
        );
        assert_eq!(
            attention_body_frame(0.35, EyeDirection::Right),
            AttentionBodyFrame::Attentive
        );
        assert_eq!(
            attention_body_frame(0.8, EyeDirection::Right),
            AttentionBodyFrame::Right
        );
    }

    #[test]
    fn body_attention_waits_for_the_slower_follow_response() {
        assert_eq!(
            attention_body_frame_with_follow(0.8, EyeDirection::Right, Vec2::ZERO),
            AttentionBodyFrame::Attentive
        );
        assert_eq!(
            attention_body_frame_with_follow(0.8, EyeDirection::Right, Vec2::new(-0.2, 0.0)),
            AttentionBodyFrame::Left
        );
    }

    #[test]
    fn reaction_has_anticipation_recoil_and_recovery_shapes() {
        assert_eq!(
            reaction_visual(ReactionPhase::Anticipating, 0.0, Vec2::X),
            (Vec2::ZERO, 0.0)
        );

        let (recoil, compression) =
            reaction_visual(ReactionPhase::Recoiling, VERY_NEAR_RECOIL_DURATION, Vec2::X);
        assert!(recoil.x < 0.0);
        assert_eq!(compression, VERY_NEAR_COMPRESSION);

        let (restored, restored_compression) = reaction_visual(
            ReactionPhase::Recovering,
            VERY_NEAR_RECOVERY_DURATION,
            Vec2::X,
        );
        assert!(restored.length() < 0.000_001);
        assert!(restored_compression < 0.000_001);
    }

    #[test]
    fn very_near_reaction_is_edge_triggered_and_cooldown_protected() {
        let mut attention = AttentionMotion::default();

        assert!(update_reaction(
            &mut attention,
            VERY_NEAR_TRIGGER_FACTOR,
            Vec2::X,
            0.0,
        ));
        assert_eq!(attention.reaction_phase, ReactionPhase::Anticipating);

        attention.was_very_near = false;
        assert!(!update_reaction(
            &mut attention,
            VERY_NEAR_TRIGGER_FACTOR,
            Vec2::X,
            0.0,
        ));
    }

    #[test]
    fn returning_idle_gaze_decays_to_center() {
        let gaze = IdleGazeMotion {
            phase: IdleGazePhase::Returning,
            elapsed: crate::pet::idle::GAZE_RETURN_DURATION,
            duration: crate::pet::idle::GAZE_RETURN_DURATION,
            target_offset: Vec2::new(3.0, 0.2),
        };

        assert_eq!(idle_gaze_target(&gaze), Vec2::ZERO);
    }
}
