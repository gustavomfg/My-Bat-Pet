use bevy::{
    input::ButtonInput,
    log::info,
    prelude::{
        Component, KeyCode, NextState, Query, Res, ResMut, Resource, State, Time, Transform, Vec2,
        With,
    },
};

use crate::debug::DebugOptions;

use super::{Bat, BatState};

pub const TAKEOFF_TARGET_OFFSET: Vec2 = Vec2::new(4.0, -32.0);
pub const FLIGHT_TARGET_OFFSET: Vec2 = Vec2::new(0.0, -48.0);
pub const RETURN_APPROACH_OFFSET: Vec2 = Vec2::new(0.0, -24.0);

pub const TAKEOFF_SUPPORT_HOLD: f32 = 0.24;
pub const FLIGHT_DWELL: f32 = 0.85;
pub const FLIGHT_LOOP_DELAY: f32 = 1.6;

pub const DEFAULT_MAX_SPEED: f32 = 190.0;
pub const DEFAULT_MAX_ACCELERATION: f32 = 420.0;
pub const DEFAULT_ARRIVAL_RADIUS: f32 = 24.0;
pub const DEFAULT_SLOW_RADIUS: f32 = 96.0;
const STAGE_ARRIVAL_RADIUS: f32 = 8.0;
const STAGE_SLOW_RADIUS: f32 = 48.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FlightVisualFrame {
    #[default]
    Rest,
    Coil,
    Lift,
    Spread,
    Power,
    Recover,
    Reach,
}

impl FlightVisualFrame {
    pub const fn sheet_index(self) -> u32 {
        match self {
            Self::Lift => 0,
            Self::Spread | Self::Coil | Self::Reach => 1,
            Self::Power => 2,
            Self::Recover | Self::Rest => 3,
        }
    }
}

#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct FlightVisualIntent {
    pub frame: FlightVisualFrame,
    pub facing: i8,
    pub speed: f32,
    pub vertical_speed: f32,
    pub body_offset: Vec2,
    phase: f32,
    contact_elapsed: f32,
}

impl Default for FlightVisualIntent {
    fn default() -> Self {
        Self {
            frame: FlightVisualFrame::Rest,
            facing: 1,
            speed: 0.0,
            vertical_speed: 0.0,
            body_offset: Vec2::ZERO,
            phase: 0.0,
            contact_elapsed: 0.0,
        }
    }
}

#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct Perch {
    pub anchor: Vec2,
    pub approach_offset: Vec2,
}

impl Perch {
    pub const fn hanging(anchor: Vec2) -> Self {
        Self {
            anchor,
            approach_offset: RETURN_APPROACH_OFFSET,
        }
    }

    pub fn approach_target(self) -> Vec2 {
        self.anchor + self.approach_offset
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FlightTarget {
    pub position: Vec2,
    pub arrival_radius: f32,
    pub slow_radius: f32,
}

impl FlightTarget {
    pub const fn new(position: Vec2) -> Self {
        Self {
            position,
            arrival_radius: DEFAULT_ARRIVAL_RADIUS,
            slow_radius: DEFAULT_SLOW_RADIUS,
        }
    }
}

#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct FlightMotion {
    pub position: Vec2,
    pub velocity: Vec2,
    pub acceleration: Vec2,
    pub target: FlightTarget,
    pub max_speed: f32,
    pub max_acceleration: f32,
    pub stage_elapsed: f32,
    pub arrived: bool,
}

impl FlightMotion {
    pub fn at(position: Vec2) -> Self {
        let position = finite_vec2(position, Vec2::ZERO);
        Self {
            position,
            velocity: Vec2::ZERO,
            acceleration: Vec2::ZERO,
            target: FlightTarget::new(position),
            max_speed: DEFAULT_MAX_SPEED,
            max_acceleration: DEFAULT_MAX_ACCELERATION,
            stage_elapsed: 0.0,
            arrived: true,
        }
    }

    pub fn set_target(&mut self, target: FlightTarget) {
        let arrival_radius = if target.arrival_radius.is_finite() {
            target.arrival_radius.max(0.0)
        } else {
            DEFAULT_ARRIVAL_RADIUS
        };
        let slow_radius = if target.slow_radius.is_finite() {
            target.slow_radius.max(arrival_radius)
        } else {
            DEFAULT_SLOW_RADIUS.max(arrival_radius)
        };
        self.target = FlightTarget {
            position: finite_vec2(target.position, self.position),
            arrival_radius,
            slow_radius,
        };
        self.stage_elapsed = 0.0;
        self.arrived = false;
    }
}

#[derive(Resource, Debug, Clone, Copy, PartialEq)]
pub struct FlightDebug {
    pub loop_mode: bool,
    pub once: bool,
    pub delay: f32,
    pub cycles: u32,
}

impl Default for FlightDebug {
    fn default() -> Self {
        Self {
            loop_mode: false,
            once: false,
            delay: 0.0,
            cycles: 0,
        }
    }
}

impl FlightDebug {
    pub fn from_args() -> Self {
        let loop_mode = std::env::args().any(|arg| arg == "--flight-loop");
        let once = std::env::args().any(|arg| arg == "--flight-once");
        Self {
            loop_mode,
            once,
            delay: if loop_mode || once { 1.0 } else { 0.0 },
            ..Default::default()
        }
    }
}

pub fn trigger_flight(
    time: Res<Time>,
    keyboard: Res<ButtonInput<KeyCode>>,
    state: Res<State<BatState>>,
    mut debug: ResMut<FlightDebug>,
    mut next: ResMut<NextState<BatState>>,
    options: Res<DebugOptions>,
) {
    if *state.get() != BatState::HangingIdle {
        return;
    }

    debug.delay = (debug.delay - safe_delta(time.delta_secs())).max(0.0);
    let keyboard_request = keyboard.just_pressed(KeyCode::KeyF);
    let scheduled_request = (debug.loop_mode || debug.once) && debug.delay <= 0.0;

    if keyboard_request || scheduled_request {
        debug.cycles = debug.cycles.saturating_add(1);
        debug.delay = if debug.loop_mode {
            FLIGHT_LOOP_DELAY
        } else {
            f32::INFINITY
        };
        next.set(BatState::Takeoff);
        if options.enabled {
            let cycle = debug.cycles;
            info!(
                "flight cycle requested cycle={} trigger={}",
                cycle,
                if keyboard_request { "key-f" } else { "debug" }
            );
        }
    }
}

pub fn update_flight_visual(
    time: Res<Time>,
    state: Res<State<BatState>>,
    mut bats: Query<(&FlightMotion, &mut FlightVisualIntent), With<Bat>>,
) {
    for (motion, mut intent) in &mut bats {
        intent.advance(*state.get(), motion, time.delta_secs());
    }
}

impl FlightVisualIntent {
    fn advance(&mut self, state: BatState, motion: &FlightMotion, dt: f32) {
        let dt = safe_delta(dt);
        self.speed = (motion.velocity.length() / motion.max_speed.max(1.0)).clamp(0.0, 1.0);
        self.vertical_speed = (motion.velocity.y / motion.max_speed.max(1.0)).clamp(-1.0, 1.0);
        if motion.velocity.x.abs() > 3.0 {
            self.facing = if motion.velocity.x < 0.0 { -1 } else { 1 };
        }
        self.body_offset = Vec2::ZERO;
        if matches!(state, BatState::HangingIdle | BatState::Reacting) {
            self.frame = FlightVisualFrame::Rest;
            self.phase = 0.0;
            self.contact_elapsed = 0.0;
            return;
        }
        if state == BatState::Takeoff && motion.stage_elapsed < TAKEOFF_SUPPORT_HOLD {
            self.frame = FlightVisualFrame::Coil;
            self.phase = 0.0;
            // A single pixel of gathering weight, with the claws still fixed.
            self.body_offset = Vec2::Y * crate::rendering::DISPLAY_SCALE;
            return;
        }
        if state == BatState::Landing && motion.arrived {
            self.contact_elapsed += dt;
            self.frame = FlightVisualFrame::Reach;
            // Absorb contact before relaxing into Alive.
            if self.contact_elapsed < 0.12 {
                self.body_offset = Vec2::NEG_Y * crate::rendering::DISPLAY_SCALE;
            }
            return;
        }
        self.contact_elapsed = 0.0;
        // Preserve phase across flight states. Upward effort shortens recovery;
        // braking and the quiet part of the hop use a slower, readable stroke.
        let effort = (motion.acceleration.y / motion.max_acceleration.max(1.0)).max(0.0);
        let frequency = 2.6 + self.speed * 1.4 + effort.min(1.0) * 0.6;
        self.phase = (self.phase + dt * frequency).rem_euclid(1.0);
        self.frame = wingbeat_frame(self.phase);
        if state == BatState::Landing && motion.position.distance(motion.target.position) <= 12.0 {
            self.frame = FlightVisualFrame::Spread;
        }
    }
}

fn wingbeat_frame(phase: f32) -> FlightVisualFrame {
    match phase {
        p if p < 0.22 => FlightVisualFrame::Lift,
        p if p < 0.36 => FlightVisualFrame::Spread,
        p if p < 0.58 => FlightVisualFrame::Power,
        _ => FlightVisualFrame::Recover,
    }
}

pub fn start_takeoff(
    mut bats: Query<
        (
            &Transform,
            &Perch,
            &mut FlightMotion,
            &mut super::VisualPose,
        ),
        With<Bat>,
    >,
) {
    for (transform, perch, mut motion, mut pose) in &mut bats {
        motion.position = finite_vec2(transform.translation.truncate(), perch.anchor);
        motion.velocity = Vec2::ZERO;
        motion.acceleration = Vec2::ZERO;
        motion.set_target(stage_target(perch.anchor + TAKEOFF_TARGET_OFFSET));
        *pose = super::VisualPose::default();
    }
}

pub fn start_flying(mut bats: Query<(&Perch, &mut FlightMotion), With<Bat>>) {
    for (perch, mut motion) in &mut bats {
        motion.set_target(stage_target(perch.anchor + FLIGHT_TARGET_OFFSET));
    }
}

pub fn start_returning(mut bats: Query<(&Perch, &mut FlightMotion), With<Bat>>) {
    for (perch, mut motion) in &mut bats {
        motion.set_target(stage_target(perch.approach_target()));
    }
}

pub fn start_landing(mut bats: Query<(&Perch, &mut FlightMotion), With<Bat>>) {
    for (perch, mut motion) in &mut bats {
        motion.set_target(stage_target(perch.anchor));
    }
}

fn stage_target(position: Vec2) -> FlightTarget {
    FlightTarget {
        position,
        arrival_radius: STAGE_ARRIVAL_RADIUS,
        slow_radius: STAGE_SLOW_RADIUS,
    }
}

pub fn finish_landing(mut bats: Query<(&Perch, &mut FlightMotion, &mut Transform), With<Bat>>) {
    for (perch, mut motion, mut transform) in &mut bats {
        motion.position = perch.anchor;
        motion.velocity = Vec2::ZERO;
        motion.acceleration = Vec2::ZERO;
        motion.arrived = true;
        transform.translation.x = perch.anchor.x;
        transform.translation.y = perch.anchor.y;
    }
}

pub fn update_takeoff(
    time: Res<Time>,
    mut bats: Query<(&mut FlightMotion, &mut Transform), With<Bat>>,
    mut next: ResMut<NextState<BatState>>,
) {
    for (mut motion, mut transform) in &mut bats {
        let dt = safe_delta(time.delta_secs());
        let hold_remaining = (TAKEOFF_SUPPORT_HOLD - motion.stage_elapsed).max(0.0);
        let held = dt.min(hold_remaining);
        motion.stage_elapsed += held;
        advance_flight(&mut motion, &mut transform, dt - held);
        if motion.arrived {
            next.set(BatState::Flying);
        }
    }
}

pub fn update_flying(
    time: Res<Time>,
    mut bats: Query<(&mut FlightMotion, &mut Transform), With<Bat>>,
    mut next: ResMut<NextState<BatState>>,
) {
    for (mut motion, mut transform) in &mut bats {
        advance_flight(&mut motion, &mut transform, time.delta_secs());
        if motion.arrived && motion.stage_elapsed >= FLIGHT_DWELL {
            next.set(BatState::Returning);
        }
    }
}

pub fn update_returning(
    time: Res<Time>,
    mut bats: Query<(&mut FlightMotion, &mut Transform), With<Bat>>,
    mut next: ResMut<NextState<BatState>>,
) {
    for (mut motion, mut transform) in &mut bats {
        advance_flight(&mut motion, &mut transform, time.delta_secs());
        if motion.arrived {
            next.set(BatState::Landing);
        }
    }
}

pub fn update_landing(
    time: Res<Time>,
    mut bats: Query<(&mut FlightMotion, &mut Transform), With<Bat>>,
    mut next: ResMut<NextState<BatState>>,
) {
    for (mut motion, mut transform) in &mut bats {
        let was_arrived = motion.arrived;
        advance_flight(&mut motion, &mut transform, time.delta_secs());
        if motion.arrived && !was_arrived {
            // Contact starts a short support recovery; physics remains anchored.
            motion.stage_elapsed = 0.0;
        }
        if motion.arrived && motion.stage_elapsed >= 0.22 {
            next.set(BatState::HangingIdle);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlightStep {
    Moving,
    Arrived,
}

pub fn advance_flight(
    motion: &mut FlightMotion,
    transform: &mut Transform,
    delta_secs: f32,
) -> FlightStep {
    let dt = safe_delta(delta_secs);
    motion.stage_elapsed = finite_f32(motion.stage_elapsed + dt);
    if dt <= 0.0 {
        render_position(motion, transform);
        return if motion.arrived {
            FlightStep::Arrived
        } else {
            FlightStep::Moving
        };
    }

    if motion.arrived {
        render_position(motion, transform);
        return FlightStep::Arrived;
    }

    motion.acceleration = arrive_acceleration(
        motion.position,
        motion.velocity,
        motion.target,
        motion.max_speed,
        motion.max_acceleration,
    );
    motion.velocity = finite_vec2(
        clamp_length(motion.velocity + motion.acceleration * dt, motion.max_speed),
        Vec2::ZERO,
    );
    motion.position = finite_vec2(
        motion.position + motion.velocity * dt,
        finite_vec2(motion.target.position, Vec2::ZERO),
    );

    let distance = motion.position.distance(motion.target.position);
    if distance <= motion.target.arrival_radius
        && motion.velocity.length() <= motion.max_speed * 0.22
    {
        motion.position = motion.target.position;
        motion.velocity = Vec2::ZERO;
        motion.acceleration = Vec2::ZERO;
        motion.arrived = true;
    }
    render_position(motion, transform);
    if motion.arrived {
        FlightStep::Arrived
    } else {
        FlightStep::Moving
    }
}

pub fn arrive_acceleration(
    position: Vec2,
    velocity: Vec2,
    target: FlightTarget,
    max_speed: f32,
    max_acceleration: f32,
) -> Vec2 {
    let offset = target.position - position;
    let distance = offset.length();
    if !distance.is_finite() || distance <= target.arrival_radius.max(0.0) {
        return clamp_length(-velocity, max_acceleration);
    }

    let direction = offset / distance;
    let slow_span = (target.slow_radius - target.arrival_radius).max(1.0);
    let distance_factor = ((distance - target.arrival_radius) / slow_span).clamp(0.0, 1.0);
    let cruise_speed = max_speed.max(0.0) * smoothstep(distance_factor);
    let braking_speed = (2.0 * max_acceleration.max(0.0) * (distance - target.arrival_radius))
        .max(0.0)
        .sqrt();
    let desired_speed = cruise_speed.min(braking_speed);
    let desired_velocity = direction * desired_speed;
    clamp_length(desired_velocity - velocity, max_acceleration)
}

pub fn clamp_length(value: Vec2, max_length: f32) -> Vec2 {
    if !value.is_finite() || !max_length.is_finite() || max_length <= 0.0 {
        return Vec2::ZERO;
    }
    let length = value.length();
    if !length.is_finite() || length <= max_length {
        value
    } else {
        value / length * max_length
    }
}

pub fn snap_to_grid(position: Vec2, grid: f32) -> Vec2 {
    if !position.is_finite() || !grid.is_finite() || grid <= 0.0 {
        return Vec2::ZERO;
    }
    Vec2::new(
        (position.x / grid).round() * grid,
        (position.y / grid).round() * grid,
    )
}

fn render_position(motion: &FlightMotion, transform: &mut Transform) {
    let snapped = snap_to_grid(motion.position, crate::rendering::DISPLAY_SCALE);
    transform.translation.x = snapped.x;
    transform.translation.y = snapped.y;
}

fn finite_vec2(value: Vec2, fallback: Vec2) -> Vec2 {
    if value.is_finite() {
        value
    } else if fallback.is_finite() {
        fallback
    } else {
        Vec2::ZERO
    }
}

fn finite_f32(value: f32) -> f32 {
    if value.is_finite() { value } else { 0.0 }
}

fn safe_delta(delta_secs: f32) -> f32 {
    if delta_secs.is_finite() {
        delta_secs.clamp(0.0, 0.25)
    } else {
        0.0
    }
}

fn smoothstep(value: f32) -> f32 {
    value * value * (3.0 - 2.0 * value)
}

pub fn log_flight_entered(debug: Res<DebugOptions>, state: Res<State<BatState>>) {
    if debug.enabled {
        info!("flight state={}", state.get().label());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arrival_steering_is_bounded_and_brakes_near_target() {
        let target = FlightTarget::new(Vec2::new(100.0, 0.0));
        let acceleration = arrive_acceleration(
            Vec2::new(90.0, 0.0),
            Vec2::new(150.0, 0.0),
            target,
            190.0,
            420.0,
        );
        assert!(acceleration.x < 0.0);
        assert!(acceleration.length() <= 420.0001);
    }

    #[test]
    fn integration_never_exceeds_speed_or_acceleration_limits() {
        let mut motion = FlightMotion::at(Vec2::ZERO);
        motion.max_speed = 80.0;
        motion.max_acceleration = 120.0;
        motion.set_target(FlightTarget::new(Vec2::new(500.0, 300.0)));
        let mut transform = Transform::default();
        for _ in 0..600 {
            advance_flight(&mut motion, &mut transform, 1.0 / 60.0);
            assert!(motion.position.is_finite());
            assert!(motion.velocity.is_finite());
            assert!(motion.acceleration.is_finite());
            assert!(motion.velocity.length() <= motion.max_speed + 0.001);
            assert!(motion.acceleration.length() <= motion.max_acceleration + 0.001);
        }
        assert!(
            motion.arrived,
            "position={:?} velocity={:?} distance={}",
            motion.position,
            motion.velocity,
            motion.position.distance(motion.target.position)
        );
        assert_eq!(motion.position, Vec2::new(500.0, 300.0));
    }

    #[test]
    fn arrival_converges_from_both_sides_without_nan() {
        for start in [Vec2::new(-120.0, -60.0), Vec2::new(180.0, 90.0)] {
            let mut motion = FlightMotion::at(start);
            motion.set_target(FlightTarget::new(Vec2::new(0.0, 0.0)));
            let mut transform = Transform::default();
            for _ in 0..600 {
                advance_flight(&mut motion, &mut transform, 1.0 / 60.0);
            }
            assert!(motion.arrived);
            assert!(motion.position.is_finite());
            assert!(motion.velocity.is_finite());
        }
    }

    #[test]
    fn snap_to_grid_preserves_pixel_perfect_rendering() {
        assert_eq!(
            snap_to_grid(Vec2::new(13.0, -19.0), 8.0),
            Vec2::new(16.0, -16.0)
        );
        assert_eq!(snap_to_grid(Vec2::new(f32::NAN, 2.0), 8.0), Vec2::ZERO);
    }

    #[test]
    fn perch_has_a_distinct_approach_target() {
        let perch = Perch::hanging(Vec2::ZERO);
        assert_ne!(perch.approach_target(), perch.anchor);
    }

    #[test]
    fn debug_loop_can_be_requested_without_renderer_state() {
        let debug = FlightDebug {
            loop_mode: true,
            delay: 0.0,
            ..Default::default()
        };
        assert!(debug.loop_mode);
        assert_eq!(debug.cycles, 0);
    }

    #[test]
    fn visual_frames_map_to_the_four_authored_wing_phases() {
        assert_eq!(FlightVisualFrame::Lift.sheet_index(), 0);
        assert_eq!(FlightVisualFrame::Spread.sheet_index(), 1);
        assert_eq!(FlightVisualFrame::Power.sheet_index(), 2);
        assert_eq!(FlightVisualFrame::Recover.sheet_index(), 3);
        assert!(FlightVisualFrame::Power.sheet_index() < 4);
    }

    #[test]
    fn airborne_pause_keeps_supporting_strokes_and_landing_waits_for_contact() {
        let mut motion = FlightMotion::at(Vec2::new(0.0, -24.0));
        motion.target = stage_target(Vec2::ZERO);
        motion.arrived = false;
        let mut intent = FlightVisualIntent::default();
        intent.advance(BatState::Landing, &motion, 0.05);
        assert_ne!(intent.frame, FlightVisualFrame::Reach);
        motion.arrived = true;
        intent.advance(BatState::Landing, &motion, 0.05);
        assert_eq!(intent.frame, FlightVisualFrame::Reach);
        intent.advance(BatState::HangingIdle, &motion, 0.05);
        assert_eq!(intent.body_offset, Vec2::ZERO);
        let mut frames = Vec::new();
        for _ in 0..60 {
            intent.advance(BatState::Flying, &motion, 1.0 / 60.0);
            if !frames.contains(&intent.frame) {
                frames.push(intent.frame);
            }
        }
        assert_eq!(
            frames.len(),
            4,
            "stationary flight must not freeze the wings"
        );
    }

    #[test]
    fn wing_phase_is_delta_time_based_and_survives_state_changes() {
        let motion = FlightMotion::at(Vec2::ZERO);
        let mut slow = FlightVisualIntent::default();
        let mut fast = slow;
        for _ in 0..30 {
            slow.advance(BatState::Flying, &motion, 1.0 / 30.0);
        }
        for _ in 0..120 {
            fast.advance(BatState::Returning, &motion, 1.0 / 120.0);
        }
        assert!((slow.phase - fast.phase).abs() < 0.0001);
        assert_eq!(slow.frame, fast.frame);
        let phase = fast.phase;
        fast.advance(
            BatState::Landing,
            &FlightMotion {
                arrived: false,
                ..motion
            },
            0.0,
        );
        assert_eq!(fast.phase, phase);
    }

    #[test]
    fn invalid_target_and_delta_are_sanitized() {
        let mut motion = FlightMotion::at(Vec2::new(f32::NAN, f32::INFINITY));
        motion.set_target(FlightTarget {
            position: Vec2::new(f32::INFINITY, f32::NAN),
            arrival_radius: f32::NAN,
            slow_radius: f32::INFINITY,
        });
        let mut transform = Transform::default();
        advance_flight(&mut motion, &mut transform, f32::NAN);

        assert!(motion.position.is_finite());
        assert!(motion.velocity.is_finite());
        assert!(motion.acceleration.is_finite());
        assert!(motion.target.position.is_finite());
        assert!(motion.target.arrival_radius.is_finite());
        assert!(motion.target.slow_radius.is_finite());
        assert!(motion.stage_elapsed.is_finite());
    }
}
