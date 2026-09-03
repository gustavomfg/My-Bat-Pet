use std::f32::consts::{PI, TAU};

use bevy::{
    log::info,
    prelude::{
        Children, Component, Entity, Query, Res, ResMut, Resource, Time, Transform, Vec2, Vec3,
        Visibility, With,
    },
};

use crate::{
    debug::DebugOptions,
    pet::{
        Bat, EyeLid, EyePupil,
        visual::{
            AnimationIntent, BaseBodyFrame, BodyAction, FaceFrame, VisualAssetAvailability,
            VisualPose,
        },
    },
};

use super::acting::{
    AttentionMotion, BACKGROUND_ACTION_MAX_ATTENTION, IdleGazeMotion, IdleGazePhase,
};

pub const BREATHING_MIN_DURATION: f32 = 4.2;
pub const BREATHING_MAX_DURATION: f32 = 6.1;
pub const BREATHING_MIN_AMPLITUDE: f32 = 0.45;
pub const BREATHING_MAX_AMPLITUDE: f32 = 0.85;

pub const INITIAL_BLINK_DELAY: f32 = 5.0;
pub const BLINK_MIN_INTERVAL: f32 = 4.3;
pub const BLINK_MAX_INTERVAL: f32 = 8.2;
pub const BLINK_MIN_DURATION: f32 = 0.08;
pub const BLINK_MAX_DURATION: f32 = 0.13;
pub const DOUBLE_BLINK_GAP_MIN: f32 = 0.08;
pub const DOUBLE_BLINK_GAP_MAX: f32 = 0.16;
pub const DOUBLE_BLINK_CHANCE: f32 = 0.18;

pub const INITIAL_ADJUSTMENT_DELAY: f32 = 7.5;
pub const ADJUSTMENT_MIN_INTERVAL: f32 = 8.0;
pub const ADJUSTMENT_MAX_INTERVAL: f32 = 14.0;
pub const ADJUSTMENT_MIN_DURATION: f32 = 0.42;
pub const ADJUSTMENT_MAX_DURATION: f32 = 0.78;

pub const INITIAL_GAZE_DELAY: f32 = 8.5;
pub const GAZE_MIN_INTERVAL: f32 = 8.0;
pub const GAZE_MAX_INTERVAL: f32 = 16.0;
pub const GAZE_MIN_DURATION: f32 = 0.4;
pub const GAZE_MAX_DURATION: f32 = 1.2;
pub const GAZE_RETURN_DURATION: f32 = 0.32;
pub const GAZE_START_COOLDOWN: f32 = 0.35;
pub const GAZE_END_COOLDOWN: f32 = 0.45;

pub const INITIAL_EAR_TWITCH_DELAY: f32 = 10.5;
pub const EAR_TWITCH_MIN_INTERVAL: f32 = 9.0;
pub const EAR_TWITCH_MAX_INTERVAL: f32 = 17.0;
pub const EAR_TWITCH_MIN_DURATION: f32 = 0.14;
pub const EAR_TWITCH_MAX_DURATION: f32 = 0.24;

pub const MICRO_ACTION_COOLDOWN: f32 = 0.85;
pub const BLINK_ACTION_COOLDOWN: f32 = 0.55;
pub const BLINK_SLOW_CHANCE: f32 = 0.08;
pub const BLINK_SLOW_MAX_ATTENTION: f32 = 0.22;
pub const BLINK_SLOW_MIN_DURATION: f32 = 0.13;
pub const BLINK_SLOW_MAX_DURATION: f32 = 0.19;
pub const BREATHING_SCALE_X: f32 = 0.004;
pub const BREATHING_SCALE_Y: f32 = 0.008;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IdleRng {
    state: u32,
}

impl IdleRng {
    pub const fn new(seed: u32) -> Self {
        Self {
            state: if seed == 0 { 0x9E37_79B9 } else { seed },
        }
    }

    fn next_u32(&mut self) -> u32 {
        let mut value = self.state;
        value ^= value << 13;
        value ^= value >> 17;
        value ^= value << 5;
        self.state = value;
        value
    }

    fn next_f32(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / (u32::MAX >> 8) as f32
    }

    fn range(&mut self, min: f32, max: f32) -> f32 {
        debug_assert!(min <= max);
        min + (max - min) * self.next_f32()
    }

    fn chance(&mut self, probability: f32) -> bool {
        match probability {
            probability if probability <= 0.0 => false,
            probability if probability >= 1.0 => true,
            probability => self.next_f32() < probability,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Resource)]
pub struct IdleScheduler {
    pub rng: IdleRng,
    pub adjustment_timer: f32,
    pub gaze_timer: f32,
    pub ear_twitch_timer: f32,
    pub action_cooldown: f32,
    pub gaze_active: bool,
}

impl Default for IdleScheduler {
    fn default() -> Self {
        Self::from_seed(0xBA7A_1D1E)
    }
}

impl IdleScheduler {
    pub const fn from_seed(seed: u32) -> Self {
        Self {
            rng: IdleRng::new(seed),
            adjustment_timer: INITIAL_ADJUSTMENT_DELAY,
            gaze_timer: INITIAL_GAZE_DELAY,
            ear_twitch_timer: INITIAL_EAR_TWITCH_DELAY,
            action_cooldown: 0.0,
            gaze_active: false,
        }
    }

    pub fn advance(&mut self, delta_secs: f32) {
        let delta_secs = if delta_secs.is_finite() {
            delta_secs.max(0.0)
        } else {
            0.0
        };

        self.adjustment_timer -= delta_secs;
        self.gaze_timer -= delta_secs;
        self.ear_twitch_timer -= delta_secs;
        self.action_cooldown = (self.action_cooldown - delta_secs).max(0.0);
    }

    pub fn try_start_adjustment(&mut self) -> bool {
        if self.adjustment_timer > 0.0 || self.action_cooldown > 0.0 || self.gaze_active {
            return false;
        }

        self.adjustment_timer = self.next_adjustment_interval();
        self.action_cooldown = MICRO_ACTION_COOLDOWN;
        true
    }

    pub fn try_start_ear_twitch(&mut self) -> bool {
        if self.ear_twitch_timer > 0.0 || self.action_cooldown > 0.0 || self.gaze_active {
            return false;
        }

        self.ear_twitch_timer = self.next_ear_twitch_interval();
        self.action_cooldown = MICRO_ACTION_COOLDOWN;
        true
    }

    pub fn can_start_blink(&self) -> bool {
        self.action_cooldown <= 0.0
    }

    pub fn reserve_blink_action(&mut self) {
        self.action_cooldown = BLINK_ACTION_COOLDOWN;
    }

    pub fn try_start_idle_gaze(&mut self) -> bool {
        if self.gaze_timer > 0.0 || self.gaze_active || self.action_cooldown > 0.0 {
            return false;
        }

        self.gaze_timer = self.next_idle_gaze_interval();
        self.gaze_active = true;
        self.action_cooldown = GAZE_START_COOLDOWN;
        true
    }

    pub fn finish_idle_gaze(&mut self) {
        self.gaze_active = false;
        self.action_cooldown = GAZE_END_COOLDOWN;
    }

    pub fn defer_idle_gaze(&mut self) {
        if self.gaze_timer <= 0.0 {
            self.gaze_timer = self.next_idle_gaze_interval();
        }
    }

    pub fn next_idle_gaze_interval(&mut self) -> f32 {
        self.rng.range(GAZE_MIN_INTERVAL, GAZE_MAX_INTERVAL)
    }

    pub fn next_idle_gaze_duration(&mut self) -> f32 {
        self.rng.range(GAZE_MIN_DURATION, GAZE_MAX_DURATION)
    }

    pub fn next_idle_gaze_offset(&mut self) -> Vec2 {
        match self.rng.next_u32() & 3 {
            0 => Vec2::new(-3.2, 0.35),
            1 => Vec2::new(3.2, 0.35),
            2 => Vec2::new(-2.6, -0.25),
            _ => Vec2::new(2.6, -0.25),
        }
    }

    pub fn next_blink_duration_for_attention(&mut self, attention_level: f32) -> f32 {
        if attention_level <= BLINK_SLOW_MAX_ATTENTION && self.rng.chance(BLINK_SLOW_CHANCE) {
            return self
                .rng
                .range(BLINK_SLOW_MIN_DURATION, BLINK_SLOW_MAX_DURATION);
        }

        self.next_blink_duration()
    }

    pub fn next_ear_twitch_interval(&mut self) -> f32 {
        self.rng
            .range(EAR_TWITCH_MIN_INTERVAL, EAR_TWITCH_MAX_INTERVAL)
    }

    pub fn next_ear_twitch_duration(&mut self) -> f32 {
        self.rng
            .range(EAR_TWITCH_MIN_DURATION, EAR_TWITCH_MAX_DURATION)
    }

    pub fn next_ear_twitch_action(&mut self) -> BodyAction {
        if self.rng.next_u32() & 1 == 0 {
            BodyAction::EarTwitchLeft
        } else {
            BodyAction::EarTwitchRight
        }
    }

    pub fn next_breathing_duration(&mut self) -> f32 {
        self.rng
            .range(BREATHING_MIN_DURATION, BREATHING_MAX_DURATION)
    }

    pub fn next_breathing_amplitude(&mut self) -> f32 {
        self.rng
            .range(BREATHING_MIN_AMPLITUDE, BREATHING_MAX_AMPLITUDE)
    }

    pub fn next_blink_interval(&mut self) -> f32 {
        self.rng.range(BLINK_MIN_INTERVAL, BLINK_MAX_INTERVAL)
    }

    pub fn next_blink_duration(&mut self) -> f32 {
        self.rng.range(BLINK_MIN_DURATION, BLINK_MAX_DURATION)
    }

    pub fn next_double_blink_gap(&mut self) -> f32 {
        self.rng.range(DOUBLE_BLINK_GAP_MIN, DOUBLE_BLINK_GAP_MAX)
    }

    pub fn should_double_blink(&mut self) -> bool {
        self.rng.chance(DOUBLE_BLINK_CHANCE)
    }

    pub fn next_adjustment_interval(&mut self) -> f32 {
        self.rng
            .range(ADJUSTMENT_MIN_INTERVAL, ADJUSTMENT_MAX_INTERVAL)
    }

    pub fn next_adjustment_duration(&mut self) -> f32 {
        self.rng
            .range(ADJUSTMENT_MIN_DURATION, ADJUSTMENT_MAX_DURATION)
    }

    pub fn next_adjustment_offset(&mut self) -> Vec2 {
        match self.rng.next_u32() & 3 {
            0 => Vec2::new(1.0, 0.0),
            1 => Vec2::new(-1.0, 0.0),
            2 => Vec2::new(0.0, 0.55),
            _ => Vec2::new(0.0, -0.45),
        }
    }
}

#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct IdleMotion {
    pub base_translation: Vec3,
    pub breathing_scale: Vec2,
    pub adjustment_offset: Vec2,
}

impl IdleMotion {
    pub const fn new(base_translation: Vec3) -> Self {
        Self {
            base_translation,
            breathing_scale: Vec2::new(1.0, 1.0),
            adjustment_offset: Vec2::ZERO,
        }
    }
}

#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct BreathingMotion {
    pub elapsed: f32,
    pub duration: f32,
    pub amplitude: f32,
}

impl Default for BreathingMotion {
    fn default() -> Self {
        Self {
            elapsed: 0.0,
            duration: 4.8,
            amplitude: 0.65,
        }
    }
}

#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct EarTwitchMotion {
    pub active: bool,
    pub elapsed: f32,
    pub duration: f32,
    pub action: Option<BodyAction>,
}

impl Default for EarTwitchMotion {
    fn default() -> Self {
        Self {
            active: false,
            elapsed: 0.0,
            duration: 0.0,
            action: None,
        }
    }
}

#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct IdleAdjustmentMotion {
    pub active: bool,
    pub elapsed: f32,
    pub duration: f32,
    pub target_offset: Vec2,
}

impl Default for IdleAdjustmentMotion {
    fn default() -> Self {
        Self {
            active: false,
            elapsed: 0.0,
            duration: 0.0,
            target_offset: Vec2::ZERO,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlinkPhase {
    Waiting,
    Closed,
    DoubleBlinkGap,
}

#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct BlinkState {
    pub phase: BlinkPhase,
    pub timer: f32,
    pub phase_duration: f32,
    pub double_blink_pending: bool,
}

impl Default for BlinkState {
    fn default() -> Self {
        Self::waiting(INITIAL_BLINK_DELAY)
    }
}

impl BlinkState {
    pub const fn waiting(delay: f32) -> Self {
        Self {
            phase: BlinkPhase::Waiting,
            timer: delay,
            phase_duration: 0.0,
            double_blink_pending: false,
        }
    }

    pub const fn is_closed(self) -> bool {
        matches!(self.phase, BlinkPhase::Closed)
    }

    pub fn face_frame(self) -> FaceFrame {
        if !self.is_closed() || self.phase_duration <= 0.0 {
            return FaceFrame::Open;
        }

        let progress = ((self.phase_duration - self.timer) / self.phase_duration).clamp(0.0, 1.0);
        if progress < 0.22 || progress > 0.78 {
            FaceFrame::BlinkHalf
        } else {
            FaceFrame::BlinkClosed
        }
    }

    fn begin_closed(&mut self, duration: f32) {
        self.phase = BlinkPhase::Closed;
        self.timer = duration;
        self.phase_duration = duration;
    }

    pub fn tick(
        &mut self,
        delta_secs: f32,
        scheduler: &mut IdleScheduler,
        attention_level: f32,
    ) -> Option<BlinkEvent> {
        if !delta_secs.is_finite() {
            return None;
        }

        let mut remaining = delta_secs.max(0.0);
        let mut event = None;

        loop {
            if matches!(self.phase, BlinkPhase::Waiting)
                && self.timer <= remaining
                && !scheduler.can_start_blink()
            {
                self.timer = 0.0;
                break;
            }

            if self.timer > remaining {
                self.timer -= remaining;
                break;
            }

            remaining -= self.timer;
            self.timer = 0.0;

            match self.phase {
                BlinkPhase::Waiting => {
                    self.begin_closed(scheduler.next_blink_duration_for_attention(attention_level));
                    self.double_blink_pending = scheduler.should_double_blink();
                    scheduler.reserve_blink_action();
                    event.get_or_insert(BlinkEvent::Started {
                        double: self.double_blink_pending,
                    });
                }
                BlinkPhase::Closed => {
                    if self.double_blink_pending {
                        self.phase = BlinkPhase::DoubleBlinkGap;
                        self.timer = scheduler.next_double_blink_gap();
                        self.phase_duration = 0.0;
                    } else {
                        self.phase = BlinkPhase::Waiting;
                        self.timer = scheduler.next_blink_interval();
                        self.phase_duration = 0.0;
                    }
                }
                BlinkPhase::DoubleBlinkGap => {
                    self.begin_closed(scheduler.next_blink_duration_for_attention(attention_level));
                    self.double_blink_pending = false;
                    scheduler.reserve_blink_action();
                    event.get_or_insert(BlinkEvent::DoubleBlinkStarted);
                }
            }

            if remaining <= f32::EPSILON {
                break;
            }
        }

        event
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BlinkEvent {
    Started { double: bool },
    DoubleBlinkStarted,
}

pub fn advance_idle_scheduler(time: Res<Time>, mut scheduler: ResMut<IdleScheduler>) {
    scheduler.advance(time.delta_secs());
}

pub fn update_breathing(
    time: Res<Time>,
    mut scheduler: ResMut<IdleScheduler>,
    mut bats: Query<(&mut BreathingMotion, &mut IdleMotion, &mut AnimationIntent), With<Bat>>,
) {
    let delta_secs = time.delta_secs().max(0.0);

    for (mut breathing, mut idle, mut intent) in &mut bats {
        breathing.elapsed += delta_secs;

        while breathing.elapsed >= breathing.duration {
            breathing.elapsed -= breathing.duration;
            breathing.duration = scheduler.next_breathing_duration();
            breathing.amplitude = scheduler.next_breathing_amplitude();
        }

        let offset = breathing_offset(breathing.elapsed, breathing.duration, breathing.amplitude);
        let normalized_offset = if breathing.amplitude.abs() > f32::EPSILON {
            offset / breathing.amplitude
        } else {
            0.0
        };

        idle.breathing_scale = breathing_scale(normalized_offset);
        intent.base_body = breathing_frame(breathing.elapsed / breathing.duration);
    }
}

pub fn update_idle_adjustment(
    time: Res<Time>,
    mut scheduler: ResMut<IdleScheduler>,
    mut bats: Query<
        (
            &mut IdleAdjustmentMotion,
            &mut IdleMotion,
            &mut AnimationIntent,
            &AttentionMotion,
            &IdleGazeMotion,
        ),
        With<Bat>,
    >,
    debug: Res<DebugOptions>,
) {
    let delta_secs = time.delta_secs().max(0.0);

    for (mut adjustment, mut idle, mut intent, attention, gaze) in &mut bats {
        if !adjustment.active
            && attention.target_level <= BACKGROUND_ACTION_MAX_ATTENTION
            && matches!(gaze.phase, IdleGazePhase::Resting)
            && scheduler.try_start_adjustment()
        {
            adjustment.active = true;
            adjustment.elapsed = 0.0;
            adjustment.duration = scheduler.next_adjustment_duration();
            adjustment.target_offset = scheduler.next_adjustment_offset();
            intent.body_action = Some(BodyAction::WingAdjust);

            if debug.enabled {
                info!("idle adjustment started");
            }
        }

        if !adjustment.active {
            continue;
        }

        adjustment.elapsed += delta_secs;
        idle.adjustment_offset = idle_adjustment_offset(
            adjustment.elapsed / adjustment.duration,
            adjustment.target_offset,
        );

        if adjustment.elapsed >= adjustment.duration {
            adjustment.active = false;
            adjustment.elapsed = 0.0;
            adjustment.duration = 0.0;
            adjustment.target_offset = Vec2::ZERO;
            idle.adjustment_offset = Vec2::ZERO;
            intent.body_action = None;
        }
    }
}

pub fn update_ear_twitch(
    time: Res<Time>,
    mut scheduler: ResMut<IdleScheduler>,
    mut bats: Query<
        (
            &mut EarTwitchMotion,
            &mut AnimationIntent,
            &AttentionMotion,
            &IdleGazeMotion,
        ),
        With<Bat>,
    >,
    debug: Res<DebugOptions>,
) {
    let delta_secs = time.delta_secs().max(0.0);

    for (mut twitch, mut intent, attention, gaze) in &mut bats {
        if !twitch.active
            && attention.target_level <= BACKGROUND_ACTION_MAX_ATTENTION
            && matches!(gaze.phase, IdleGazePhase::Resting)
            && scheduler.try_start_ear_twitch()
        {
            twitch.active = true;
            twitch.elapsed = 0.0;
            twitch.duration = scheduler.next_ear_twitch_duration();
            twitch.action = Some(scheduler.next_ear_twitch_action());
            intent.body_action = twitch.action;

            if debug.enabled {
                if let Some(action) = twitch.action {
                    info!("ear twitch started side={}", action.label());
                }
            }
        }

        if !twitch.active {
            continue;
        }

        twitch.elapsed += delta_secs;

        if twitch.elapsed >= twitch.duration {
            twitch.active = false;
            twitch.elapsed = 0.0;
            twitch.duration = 0.0;
            twitch.action = None;
            intent.body_action = None;
        }
    }
}

pub fn update_blink(
    time: Res<Time>,
    mut scheduler: ResMut<IdleScheduler>,
    mut bats: Query<(&mut BlinkState, &Children, &mut AnimationIntent), With<Bat>>,
    mut pupils: Query<&mut Visibility, With<EyePupil>>,
    face_layers: Query<Entity, With<EyeLid>>,
    debug: Res<DebugOptions>,
) {
    let has_face_layer = !face_layers.is_empty();

    for (mut blink, children, mut intent) in &mut bats {
        let was_closed = blink.is_closed();
        let event = blink.tick(time.delta_secs(), &mut scheduler, intent.attention.level);
        let is_closed = blink.is_closed();
        intent.face = blink.face_frame();

        if was_closed != is_closed && !has_face_layer {
            // Placeholder until a closed-eye sprite exists: hiding the pupils
            // keeps the blink state independent from the final eye artwork.
            let visibility = if is_closed {
                Visibility::Hidden
            } else {
                Visibility::Visible
            };

            for child in children.iter() {
                if let Ok(mut pupil_visibility) = pupils.get_mut(*child) {
                    *pupil_visibility = visibility;
                }
            }
        }

        if debug.enabled {
            match event {
                Some(BlinkEvent::Started { double: true }) => {
                    info!("blink started double=true");
                }
                Some(BlinkEvent::Started { double: false }) => {
                    info!("blink started");
                }
                Some(BlinkEvent::DoubleBlinkStarted) => {
                    info!("double blink");
                }
                None => {}
            }
        }
    }
}

pub fn apply_idle_motion(
    assets: Res<VisualAssetAvailability>,
    mut bats: Query<(&IdleMotion, &VisualPose, &mut Transform), With<Bat>>,
) {
    for (idle, pose, mut transform) in &mut bats {
        let adjustment_offset = if assets.body_atlas {
            Vec2::ZERO
        } else {
            idle.adjustment_offset
        };
        let body_offset = if assets.body_atlas {
            Vec2::ZERO
        } else {
            pose.attention.body_offset
        };
        let compression = if assets.body_atlas {
            0.0
        } else {
            pose.attention.body_compression.clamp(0.0, 0.02)
        };
        let scale = Vec3::new(
            idle.breathing_scale.x * (1.0 + compression * 0.25),
            idle.breathing_scale.y * (1.0 - compression),
            1.0,
        );

        transform.translation = idle.base_translation
            + Vec3::new(
                body_offset.x + adjustment_offset.x,
                body_offset.y + adjustment_offset.y,
                0.0,
            );
        transform.scale = scale;
    }
}

pub fn breathing_offset(elapsed: f32, duration: f32, amplitude: f32) -> f32 {
    if duration <= 0.0 || !duration.is_finite() {
        return 0.0;
    }

    let progress = (elapsed / duration).rem_euclid(1.0);
    amplitude * (progress * TAU).sin()
}

pub fn breathing_scale(normalized_offset: f32) -> Vec2 {
    let normalized_offset = normalized_offset.clamp(-1.0, 1.0);
    Vec2::new(
        1.0 - normalized_offset * BREATHING_SCALE_X,
        1.0 + normalized_offset * BREATHING_SCALE_Y,
    )
}

pub fn breathing_frame(progress: f32) -> BaseBodyFrame {
    match progress.rem_euclid(1.0) {
        progress if progress < 0.36 => BaseBodyFrame::BreatheIn,
        progress if progress < 0.64 => BaseBodyFrame::Neutral,
        _ => BaseBodyFrame::BreatheOut,
    }
}

pub fn idle_adjustment_offset(progress: f32, target_offset: Vec2) -> Vec2 {
    target_offset * (PI * progress.clamp(0.0, 1.0)).sin()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scheduler_intervals_stay_inside_their_ranges() {
        let mut scheduler = IdleScheduler::from_seed(7);

        for _ in 0..32 {
            assert!(
                (BLINK_MIN_INTERVAL..=BLINK_MAX_INTERVAL)
                    .contains(&scheduler.next_blink_interval())
            );
            assert!(
                (BLINK_MIN_DURATION..=BLINK_MAX_DURATION)
                    .contains(&scheduler.next_blink_duration())
            );
            assert!(
                (DOUBLE_BLINK_GAP_MIN..=DOUBLE_BLINK_GAP_MAX)
                    .contains(&scheduler.next_double_blink_gap())
            );
            assert!(
                (BREATHING_MIN_DURATION..=BREATHING_MAX_DURATION)
                    .contains(&scheduler.next_breathing_duration())
            );
            assert!(
                (ADJUSTMENT_MIN_INTERVAL..=ADJUSTMENT_MAX_INTERVAL)
                    .contains(&scheduler.next_adjustment_interval())
            );
            assert!(
                (ADJUSTMENT_MIN_DURATION..=ADJUSTMENT_MAX_DURATION)
                    .contains(&scheduler.next_adjustment_duration())
            );
            assert!(
                (GAZE_MIN_INTERVAL..=GAZE_MAX_INTERVAL)
                    .contains(&scheduler.next_idle_gaze_interval())
            );
            assert!(
                (GAZE_MIN_DURATION..=GAZE_MAX_DURATION)
                    .contains(&scheduler.next_idle_gaze_duration())
            );
            assert!(
                (EAR_TWITCH_MIN_INTERVAL..=EAR_TWITCH_MAX_INTERVAL)
                    .contains(&scheduler.next_ear_twitch_interval())
            );
            assert!(
                (EAR_TWITCH_MIN_DURATION..=EAR_TWITCH_MAX_DURATION)
                    .contains(&scheduler.next_ear_twitch_duration())
            );
        }
    }

    #[test]
    fn adjustment_timer_only_fires_after_its_interval() {
        let mut scheduler = IdleScheduler::from_seed(7);
        let initial_delay = scheduler.adjustment_timer;

        scheduler.advance(initial_delay - 0.01);
        assert!(!scheduler.try_start_adjustment());

        scheduler.advance(0.02);
        assert!(scheduler.try_start_adjustment());
        assert!(
            (ADJUSTMENT_MIN_INTERVAL..=ADJUSTMENT_MAX_INTERVAL)
                .contains(&scheduler.adjustment_timer)
        );
    }

    #[test]
    fn scheduler_keeps_ear_twitch_and_adjustment_apart() {
        let mut scheduler = IdleScheduler::from_seed(7);

        scheduler.advance(INITIAL_ADJUSTMENT_DELAY);
        assert!(scheduler.try_start_adjustment());
        assert!(!scheduler.try_start_ear_twitch());

        scheduler.advance(MICRO_ACTION_COOLDOWN);
        assert!(scheduler.try_start_ear_twitch() || scheduler.ear_twitch_timer > 0.0);
    }

    #[test]
    fn scheduler_locks_background_actions_during_idle_gaze() {
        let mut scheduler = IdleScheduler::from_seed(7);

        scheduler.advance(INITIAL_GAZE_DELAY);
        assert!(scheduler.try_start_idle_gaze());
        assert!(scheduler.gaze_active);
        assert!(!scheduler.try_start_adjustment());
        assert!(!scheduler.try_start_ear_twitch());

        scheduler.finish_idle_gaze();
        assert!(!scheduler.gaze_active);
        assert!(scheduler.action_cooldown > 0.0);
    }

    #[test]
    fn attentive_blink_does_not_use_the_relaxed_slow_variant() {
        let mut scheduler = IdleScheduler::from_seed(7);

        for _ in 0..32 {
            let duration = scheduler.next_blink_duration_for_attention(1.0);
            assert!((BLINK_MIN_DURATION..=BLINK_MAX_DURATION).contains(&duration));
        }
    }

    #[test]
    fn breathing_is_smooth_and_bounded() {
        let amplitude = 0.75;

        assert_eq!(breathing_offset(0.0, 5.0, amplitude), 0.0);
        assert!((breathing_offset(1.25, 5.0, amplitude) - amplitude).abs() < 0.001);
        assert!(breathing_offset(0.7, 5.0, amplitude).abs() <= amplitude);
    }

    #[test]
    fn idle_adjustment_starts_and_ends_at_rest() {
        let target = Vec2::new(1.0, 0.5);

        assert_eq!(idle_adjustment_offset(0.0, target), Vec2::ZERO);
        assert!(idle_adjustment_offset(1.0, target).length() < 0.000_001);
        assert_eq!(idle_adjustment_offset(0.5, target), target);
    }

    #[test]
    fn breathing_scale_expands_vertically_around_the_support_anchor() {
        assert_eq!(breathing_scale(0.0), Vec2::new(1.0, 1.0));

        let expanded = breathing_scale(1.0);
        assert!(expanded.x < 1.0);
        assert!(expanded.y > 1.0);
        assert!(expanded.y - 1.0 <= BREATHING_SCALE_Y + 0.000_001);
    }

    #[test]
    fn breathing_frames_cover_inhale_neutral_and_exhale() {
        assert_eq!(breathing_frame(0.1), BaseBodyFrame::BreatheIn);
        assert_eq!(breathing_frame(0.5), BaseBodyFrame::Neutral);
        assert_eq!(breathing_frame(0.9), BaseBodyFrame::BreatheOut);
    }

    #[test]
    fn blink_schedules_the_next_blink_only_after_finishing() {
        let mut scheduler = IdleScheduler::from_seed(7);
        let mut blink = BlinkState {
            phase: BlinkPhase::Closed,
            timer: 0.1,
            phase_duration: 0.1,
            double_blink_pending: false,
        };

        assert_eq!(blink.tick(0.05, &mut scheduler, 0.0), None);
        assert_eq!(blink.phase, BlinkPhase::Closed);

        assert_eq!(blink.tick(0.05, &mut scheduler, 0.0), None);
        assert_eq!(blink.phase, BlinkPhase::Waiting);
        assert!((BLINK_MIN_INTERVAL..=BLINK_MAX_INTERVAL).contains(&blink.timer));
    }

    #[test]
    fn double_blink_has_an_open_gap_between_closed_phases() {
        let mut scheduler = IdleScheduler::from_seed(7);
        let mut blink = BlinkState {
            phase: BlinkPhase::Closed,
            timer: 0.1,
            phase_duration: 0.1,
            double_blink_pending: true,
        };

        assert_eq!(blink.tick(0.1, &mut scheduler, 0.0), None);
        assert_eq!(blink.phase, BlinkPhase::DoubleBlinkGap);
        assert!((DOUBLE_BLINK_GAP_MIN..=DOUBLE_BLINK_GAP_MAX).contains(&blink.timer));

        let gap = blink.timer;
        assert_eq!(
            blink.tick(gap, &mut scheduler, 0.0),
            Some(BlinkEvent::DoubleBlinkStarted)
        );
        assert_eq!(blink.phase, BlinkPhase::Closed);
    }

    #[test]
    fn blink_face_frame_has_half_closed_edges_and_a_closed_middle() {
        let mut blink = BlinkState {
            phase: BlinkPhase::Closed,
            timer: 0.1,
            phase_duration: 0.1,
            double_blink_pending: false,
        };

        assert_eq!(blink.face_frame(), FaceFrame::BlinkHalf);
        blink.timer = 0.05;
        assert_eq!(blink.face_frame(), FaceFrame::BlinkClosed);
        blink.timer = 0.01;
        assert_eq!(blink.face_frame(), FaceFrame::BlinkHalf);
    }
}
