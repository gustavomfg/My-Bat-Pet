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
pub const HEAD_FOLLOW_SMOOTHING: f32 = 4.0;
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
pub const VERY_NEAR_RECOIL_DURATION: f32 = 0.24;
pub const VERY_NEAR_RECOVERY_DURATION: f32 = 0.85;
pub const VERY_NEAR_REACTION_COOLDOWN: f32 = 4.2;
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
    pub perceived_delta: Vec2,
    pub absent_secs: f32,
    pub still_secs: f32,
    pub settle_blink: bool,
    last_cursor: Option<Vec2>,
    candidate_direction: EyeDirection,
    direction_age: f32,
    followed_head: Vec2,
    head_step: f32,
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
            perceived_delta: Vec2::ZERO,
            absent_secs: 10.0,
            still_secs: 0.0,
            settle_blink: false,
            last_cursor: None,
            candidate_direction: EyeDirection::Center,
            direction_age: 0.0,
            followed_head: Vec2::ZERO,
            head_step: 0.0,
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
    mut scheduler: ResMut<IdleScheduler>,
) {
    let Some(window) = windows.iter().next() else {
        return;
    };

    let delta_secs = safe_delta(time.delta_secs());

    for (transform, mut attention, mut intent) in &mut bats {
        let delta = cursor_delta(cursor.position, window, transform);
        let profile = perceive(&mut attention, cursor.position, delta, delta_secs);
        let direction_changed = attention.direction != profile.direction;

        // Do not release a queue of overdue idle gestures when the user leaves.
        if profile.level > IDLE_GAZE_MAX_ATTENTION || attention.absent_secs < 1.8 {
            scheduler.adjustment_timer = scheduler.adjustment_timer.max(2.4);
            scheduler.ear_twitch_timer = scheduler.ear_twitch_timer.max(3.2);
            scheduler.gaze_timer = scheduler.gaze_timer.max(4.0);
        }

        attention.target_level = profile.level;
        attention.level = damp(
            attention.level,
            profile.level,
            ATTENTION_LEVEL_SMOOTHING,
            delta_secs,
        );
        if profile.direction != attention.candidate_direction {
            attention.candidate_direction = profile.direction;
            attention.direction_age = 0.0;
        } else {
            attention.direction_age += delta_secs;
        }
        // Eyes get the live target; the head commits only after a short look.
        if attention.direction_age >= 0.22 {
            attention.followed_head = profile.head_target;
        }
        attention.target_head_offset = attention.followed_head;
        attention.head_offset = damp_vec2(
            attention.head_offset,
            attention.target_head_offset,
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
        attention.head_step = stable_step(attention.head_offset.x, attention.head_step);

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
            head_offset: Vec2::new(attention.head_step, attention.head_offset.y),
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

/// Tiny perceptual memory: ignore hand tremor, become comfortable with a still
/// cursor, and keep watching the last position briefly after it disappears.
fn perceive(
    motion: &mut AttentionMotion,
    cursor: Option<Vec2>,
    delta: Vec2,
    dt: f32,
) -> AttentionProfile {
    motion.settle_blink = false;
    if let Some(position) = cursor.filter(|p| p.is_finite()) {
        let moved = motion
            .last_cursor
            .is_none_or(|last| last.distance(position) > 6.0);
        let old_still = motion.still_secs;
        if moved {
            motion.still_secs = 0.0;
            motion.last_cursor = Some(position);
        } else {
            motion.still_secs += dt;
        }
        motion.absent_secs = 0.0;
        motion.perceived_delta = delta;
        let mut profile = attention_profile(delta);
        motion.settle_blink = old_still < 2.8 && motion.still_secs >= 2.8 && profile.level > 0.22;
        let comfort = smoothstep(((motion.still_secs - 1.2) / 3.0).clamp(0.0, 1.0));
        profile.head_target *= 1.0 - comfort * 0.72;
        profile.body_target *= 1.0 - comfort * 0.72;
        profile.ear_target *= 1.0 - comfort * 0.88;
        profile.level *= 1.0 - comfort * 0.65;
        profile
    } else {
        let old_absent = motion.absent_secs;
        motion.absent_secs += dt;
        motion.still_secs = 0.0;
        motion.last_cursor = None;
        motion.settle_blink = old_absent < 1.0 && motion.absent_secs >= 1.0 && motion.level > 0.05;
        let mut profile = attention_profile(motion.perceived_delta);
        let memory = 1.0 - smoothstep(((motion.absent_secs - 0.55) / 1.1).clamp(0.0, 1.0));
        profile.level *= memory;
        profile.head_target *= memory;
        profile.body_target *= memory;
        profile.ear_target *= memory * 0.6;
        profile.very_near_factor = 0.0;
        profile
    }
}

/// Different enter/leave thresholds prevent pixel chatter at pose boundaries.
pub(crate) fn stable_step(value: f32, previous: f32) -> f32 {
    if value > 0.65 {
        1.0
    } else if value < -0.65 {
        -1.0
    } else if value.abs() < 0.30 || value * previous < 0.0 {
        0.0
    } else {
        previous
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
    fn still_cursor_becomes_familiar_and_does_not_repeat_the_settle_blink() {
        let mut motion = AttentionMotion::default();
        let position = Vec2::new(240., 180.);
        let initial = perceive(&mut motion, Some(position), Vec2::new(80., 0.), 0.016);
        let mut last = initial.clone();
        let mut blinks = 0;
        for i in 0..360 {
            let tremor = Vec2::new(if i % 2 == 0 { 1. } else { -1. }, 0.);
            last = perceive(
                &mut motion,
                Some(position + tremor),
                Vec2::new(80., 0.),
                1. / 60.,
            );
            blinks += usize::from(motion.settle_blink);
        }
        assert!(last.head_target.length() < initial.head_target.length() * 0.4);
        assert!(last.ear_target < initial.ear_target * 0.2);
        assert_eq!(blinks, 1);
        let renewed = perceive(
            &mut motion,
            Some(position + Vec2::X * 30.),
            Vec2::new(110., 0.),
            0.016,
        );
        assert!(renewed.head_target.length() > last.head_target.length());
    }

    #[test]
    fn eyes_lead_the_head_and_departure_has_a_finite_follow_through() {
        use crate::pet::{BreathingMotion, EyePupil, EyeState, VisualPose};
        use bevy::prelude::*;
        use std::time::Duration;
        let mut app = App::new();
        app.insert_resource(Time::<()>::default())
            .insert_resource(DebugOptions { enabled: false })
            .insert_resource(CursorState {
                position: Some(Vec2::new(260., 180.)),
            })
            .init_resource::<IdleScheduler>()
            .init_resource::<EyeState>()
            .add_systems(
                Update,
                (
                    update_attention,
                    crate::pet::resolve_visual_pose,
                    crate::pet::update_eyes,
                )
                    .chain(),
            );
        app.world_mut().spawn((
            Window {
                resolution: bevy::window::WindowResolution::new(320, 320),
                ..default()
            },
            PrimaryWindow,
        ));
        let bat = app
            .world_mut()
            .spawn((
                Bat,
                Transform::from_xyz(0., 152., 0.),
                AttentionMotion::default(),
                AnimationIntent::default(),
                VisualPose::default(),
                BreathingMotion::default(),
            ))
            .id();
        app.world_mut().spawn((
            EyePupil {
                base_position: Vec2::ZERO,
            },
            Transform::default(),
            Visibility::Visible,
        ));
        let tick = |app: &mut App| {
            app.world_mut()
                .resource_mut::<Time>()
                .advance_by(Duration::from_secs_f32(1. / 60.));
            app.update();
        };
        for _ in 0..10 {
            tick(&mut app);
        }
        assert_eq!(app.world().resource::<EyeState>().displayed.x, 8.);
        assert_eq!(
            app.world()
                .get::<VisualPose>(bat)
                .unwrap()
                .attention
                .head_offset
                .x,
            0.
        );
        for _ in 0..40 {
            tick(&mut app);
        }
        assert_eq!(
            app.world()
                .get::<VisualPose>(bat)
                .unwrap()
                .attention
                .head_offset
                .x,
            1.
        );
        app.world_mut().resource_mut::<CursorState>().position = None;
        for _ in 0..15 {
            tick(&mut app);
        }
        assert_eq!(app.world().resource::<EyeState>().displayed.x, 8.);
        for _ in 0..150 {
            tick(&mut app);
        }
        assert_eq!(app.world().resource::<EyeState>().displayed, Vec2::ZERO);
        assert_eq!(
            app.world()
                .get::<VisualPose>(bat)
                .unwrap()
                .attention
                .head_offset
                .x,
            0.
        );
    }

    #[test]
    fn pixel_pose_ignores_small_oscillations_at_the_switch_boundary() {
        let mut step = 0.;
        for value in [0.49, 0.51, 0.48, 0.52] {
            step = stable_step(value, step);
        }
        assert_eq!(step, 0.);
        step = stable_step(0.7, step);
        for value in [0.51, 0.49, 0.53, 0.45] {
            step = stable_step(value, step);
        }
        assert_eq!(step, 1.);
        assert_eq!(stable_step(0.2, step), 0.);
    }

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
