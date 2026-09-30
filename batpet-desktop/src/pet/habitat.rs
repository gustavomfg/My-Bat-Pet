use bevy::{
    input::ButtonInput,
    log::info,
    prelude::{KeyCode, NextState, Res, ResMut, Resource, State, Time, Vec2},
};

use crate::{
    debug::DebugOptions,
    rendering::{DISPLAY_HEIGHT, TOP_MARGIN},
    window::WINDOW_HEIGHT,
};

use super::BatState;

const APPROACH_DISTANCE: f32 = 16.0;
const DEVELOPMENT_BOTTOM_MARGIN: f32 = 8.0;
const TAKEOFF_FRACTION: f32 = 0.30;
const FLIGHT_FRACTION: f32 = 0.70;
const HABITAT_LOOP_DELAY: f32 = 1.6;

#[derive(Debug, Clone, Copy, PartialEq, Resource)]
pub struct HabitatDebug {
    pub loop_mode: bool,
    pub once: bool,
    pub delay: f32,
    pub cycles: u32,
}

impl Default for HabitatDebug {
    fn default() -> Self {
        Self {
            loop_mode: false,
            once: false,
            delay: 0.0,
            cycles: 0,
        }
    }
}

impl HabitatDebug {
    pub fn from_args() -> Self {
        let loop_mode =
            std::env::args().any(|arg| arg == "--flight-loop" || arg == "--habitat-loop");
        let once = std::env::args().any(|arg| arg == "--flight-once");
        Self {
            loop_mode,
            once,
            delay: if loop_mode || once { 1.0 } else { 0.0 },
            ..Default::default()
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PerchId(pub u8);

impl PerchId {
    pub const A: Self = Self(0);
    pub const B: Self = Self(1);

    pub const fn label(self) -> &'static str {
        match self.0 {
            0 => "A",
            1 => "B",
            _ => "?",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PerchSurface {
    Overhead,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PerchFacing {
    Left,
    Right,
}

impl PerchFacing {
    pub const fn root_scale_x(self) -> f32 {
        match self {
            Self::Left => -1.0,
            Self::Right => 1.0,
        }
    }
}

/// A semantic hanging place in the current simulation space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Perch {
    pub id: PerchId,
    /// Camera-local simulation position: origin at the window center, +Y up.
    pub anchor: Vec2,
    /// A point the bat reaches before moving the final distance to `anchor`.
    pub approach: Vec2,
    pub surface: PerchSurface,
    pub facing: PerchFacing,
    pub available: bool,
}

impl Perch {
    pub const fn overhead(id: PerchId, anchor: Vec2, approach: Vec2) -> Self {
        Self {
            id,
            anchor,
            approach,
            surface: PerchSurface::Overhead,
            facing: PerchFacing::Right,
            available: true,
        }
    }

    pub fn is_valid(self) -> bool {
        let approach_distance = self.anchor.distance(self.approach);
        self.anchor.is_finite()
            && self.approach.is_finite()
            && approach_distance.is_finite()
            && approach_distance > 8.0
    }
}

/// A position-only handoff from habitat behavior to the shared Flight system.
/// Flight consumes these waypoints without looking up perch identities.
#[derive(Debug, Clone, Copy, PartialEq, Resource)]
pub struct FlightPlan {
    pub origin: Vec2,
    pub takeoff: Vec2,
    pub flight: Vec2,
    pub approach: Vec2,
    pub landing: Vec2,
}

impl FlightPlan {
    pub const fn parked_at(position: Vec2) -> Self {
        Self {
            origin: position,
            takeoff: position,
            flight: position,
            approach: position,
            landing: position,
        }
    }

    pub fn between(source: Perch, destination: Perch) -> Self {
        Self {
            origin: source.anchor,
            takeoff: source.anchor.lerp(destination.approach, TAKEOFF_FRACTION),
            flight: source.anchor.lerp(destination.approach, FLIGHT_FRACTION),
            approach: destination.approach,
            landing: destination.anchor,
        }
    }

    pub fn is_finite(self) -> bool {
        self.origin.is_finite()
            && self.takeoff.is_finite()
            && self.flight.is_finite()
            && self.approach.is_finite()
            && self.landing.is_finite()
    }
}

impl Default for FlightPlan {
    fn default() -> Self {
        Self::parked_at(development_start_anchor())
    }
}

#[derive(Debug, Clone, Resource)]
pub struct Habitat {
    perches: Vec<Perch>,
    current: PerchId,
    destination: Option<PerchId>,
    pub completed_trips: u32,
}

impl Habitat {
    pub fn new(perches: Vec<Perch>, current: PerchId) -> Option<Self> {
        if perches.is_empty()
            || perches.iter().any(|perch| !perch.is_valid())
            || perches.iter().enumerate().any(|(index, perch)| {
                perches[..index]
                    .iter()
                    .any(|other| other.id == perch.id || other.anchor.distance(perch.anchor) <= 8.0)
            })
        {
            return None;
        }

        let initial = perches.iter().find(|perch| perch.id == current)?;
        if !initial.available {
            return None;
        }

        Some(Self {
            perches,
            current,
            destination: None,
            completed_trips: 0,
        })
    }

    pub fn development() -> Self {
        let window_height = WINDOW_HEIGHT as f32;
        let top_anchor = development_start_anchor();
        let lower_anchor = Vec2::new(
            0.0,
            -window_height * 0.5 + DISPLAY_HEIGHT + DEVELOPMENT_BOTTOM_MARGIN,
        );
        let upper = Perch::overhead(
            PerchId::A,
            top_anchor,
            top_anchor - Vec2::Y * APPROACH_DISTANCE,
        );
        let mut lower = Perch::overhead(
            PerchId::B,
            lower_anchor,
            lower_anchor + Vec2::Y * APPROACH_DISTANCE,
        );
        lower.facing = PerchFacing::Left;
        let perches = vec![upper, lower];
        Self::new(perches, PerchId::A).expect("development habitat must be valid")
    }

    pub fn perches(&self) -> &[Perch] {
        &self.perches
    }

    pub fn perch(&self, id: PerchId) -> Option<Perch> {
        self.perches.iter().copied().find(|perch| perch.id == id)
    }

    pub const fn current_id(&self) -> PerchId {
        self.current
    }

    pub const fn destination_id(&self) -> Option<PerchId> {
        self.destination
    }

    pub fn current_perch(&self) -> Option<Perch> {
        self.perch(self.current)
    }

    /// Select the next available place in catalog order, excluding the current perch.
    pub fn prepare_next_trip(&mut self) -> Option<FlightPlan> {
        let current_index = self
            .perches
            .iter()
            .position(|perch| perch.id == self.current)?;
        let source = self.perches[current_index];

        for distance in 1..=self.perches.len() {
            let index = (current_index + distance) % self.perches.len();
            let destination = self.perches[index];
            if destination.available && destination.id != self.current {
                let plan = FlightPlan::between(source, destination);
                if !plan.is_finite() {
                    return None;
                }
                self.destination = Some(destination.id);
                return Some(plan);
            }
        }

        None
    }

    pub fn prepare_trip_to(&mut self, id: PerchId) -> Option<FlightPlan> {
        if id == self.current {
            return None;
        }
        let source = self.current_perch()?;
        let destination = self.perch(id).filter(|perch| perch.available)?;
        let plan = FlightPlan::between(source, destination);
        if !plan.is_finite() {
            return None;
        }
        self.destination = Some(destination.id);
        Some(plan)
    }

    /// Commit the selected place only after Landing has completed.
    pub fn complete_trip(&mut self) -> Option<Perch> {
        let destination = self
            .perch(self.destination?)
            .filter(|perch| perch.available)?;
        self.current = destination.id;
        self.destination = None;
        self.completed_trips = self.completed_trips.saturating_add(1);
        Some(destination)
    }
}

impl Default for Habitat {
    fn default() -> Self {
        Self::development()
    }
}

pub fn trigger_habitat_flight(
    time: Res<Time>,
    keyboard: Res<ButtonInput<KeyCode>>,
    state: Res<State<BatState>>,
    mut debug: ResMut<HabitatDebug>,
    mut habitat: ResMut<Habitat>,
    mut plan: ResMut<FlightPlan>,
    mut next: ResMut<NextState<BatState>>,
    options: Res<DebugOptions>,
) {
    if *state.get() != BatState::HangingIdle {
        return;
    }

    debug.delay = (debug.delay - safe_delta(time.delta_secs())).max(0.0);
    let selected_perch = if keyboard.just_pressed(KeyCode::Digit1) {
        Some(PerchId::A)
    } else if keyboard.just_pressed(KeyCode::Digit2) {
        Some(PerchId::B)
    } else {
        None
    };
    let keyboard_request = keyboard.just_pressed(KeyCode::KeyF) || selected_perch.is_some();
    let scheduled_request = (debug.loop_mode || debug.once) && debug.delay <= 0.0;
    if !keyboard_request && !scheduled_request {
        return;
    }

    let source = habitat.current_id();
    let selected = match selected_perch {
        Some(id) => habitat.prepare_trip_to(id),
        None => habitat.prepare_next_trip(),
    };
    let Some(selected_plan) = selected else {
        return;
    };
    if !selected_plan.is_finite() {
        return;
    }

    debug.cycles = debug.cycles.saturating_add(1);
    debug.delay = if debug.loop_mode {
        HABITAT_LOOP_DELAY
    } else {
        f32::INFINITY
    };
    *plan = selected_plan;
    next.set(BatState::Takeoff);
    if options.enabled {
        let cycle = debug.cycles;
        let destination = habitat.destination_id().unwrap_or(PerchId(u8::MAX));
        info!(
            "habitat trip requested cycle={} from={} to={} perches={} trigger={}",
            cycle,
            source.label(),
            destination.label(),
            habitat.perches().len(),
            if selected_perch.is_some() {
                "manual-perch"
            } else if keyboard_request {
                "key-f"
            } else {
                "debug"
            }
        );
    }
}

pub fn complete_habitat_landing(mut habitat: ResMut<Habitat>, debug: Res<DebugOptions>) {
    let previous = habitat.current_id();
    if habitat.complete_trip().is_some() && debug.enabled {
        let current = habitat.current_id();
        info!(
            "habitat landing from={} current={} completed={}",
            previous.label(),
            current.label(),
            habitat.completed_trips
        );
    }
}

pub const fn development_start_anchor() -> Vec2 {
    Vec2::new(0.0, WINDOW_HEIGHT as f32 * 0.5 - TOP_MARGIN)
}

fn safe_delta(delta_secs: f32) -> f32 {
    if delta_secs.is_finite() {
        delta_secs.clamp(0.0, 0.25)
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn development_habitat_has_two_valid_places_inside_the_window() {
        let habitat = Habitat::development();
        assert_eq!(habitat.perches().len(), 2);
        assert!(habitat.perches().iter().all(|perch| perch.is_valid()));
        assert_eq!(habitat.current_id(), PerchId::A);
        assert_ne!(habitat.perch(PerchId::A), habitat.perch(PerchId::B));
        assert_eq!(
            habitat.perch(PerchId::A).unwrap().surface,
            PerchSurface::Overhead
        );
        assert_eq!(
            habitat.perch(PerchId::B).unwrap().surface,
            PerchSurface::Overhead
        );
        assert_eq!(
            habitat.perch(PerchId::A).unwrap().facing.root_scale_x(),
            1.0
        );
        assert_eq!(
            habitat.perch(PerchId::B).unwrap().facing.root_scale_x(),
            -1.0
        );
    }

    #[test]
    fn habitat_loop_debug_starts_deterministically() {
        let debug = HabitatDebug {
            loop_mode: true,
            delay: 0.0,
            ..Default::default()
        };
        assert!(debug.loop_mode);
        assert_eq!(debug.cycles, 0);
    }

    #[test]
    fn habitat_rejects_duplicate_ids_missing_current_and_invalid_points() {
        let a = Perch::overhead(PerchId::A, Vec2::ZERO, Vec2::Y * 16.0);
        let b = Perch::overhead(PerchId::A, Vec2::X * 32.0, Vec2::X * 48.0);
        assert!(Habitat::new(vec![a, b], PerchId::A).is_none());
        assert!(Habitat::new(vec![a], PerchId::B).is_none());
        let same_place = Perch::overhead(PerchId::B, Vec2::ZERO, Vec2::NEG_Y * 16.0);
        assert!(Habitat::new(vec![a, same_place], PerchId::A).is_none());

        let invalid = Perch::overhead(PerchId::B, Vec2::NAN, Vec2::ZERO);
        assert!(Habitat::new(vec![a, invalid], PerchId::A).is_none());
    }

    #[test]
    fn selection_is_valid_available_and_different_from_the_current_perch() {
        let mut habitat = Habitat::development();
        let plan = habitat.prepare_next_trip().unwrap();
        let destination = habitat.destination_id().unwrap();

        assert_ne!(destination, habitat.current_id());
        assert!(habitat.perch(destination).unwrap().available);
        assert!(plan.is_finite());
        assert_eq!(plan.approach, habitat.perch(destination).unwrap().approach);
        assert_eq!(plan.landing, habitat.perch(destination).unwrap().anchor);
    }

    #[test]
    fn unavailable_destinations_are_skipped_and_manual_selection_is_checked() {
        let mut habitat = Habitat::development();
        habitat.perches[1].available = false;
        assert!(habitat.prepare_next_trip().is_none());
        assert!(habitat.prepare_trip_to(PerchId::B).is_none());
        assert!(habitat.prepare_trip_to(PerchId::A).is_none());
        assert_eq!(habitat.destination_id(), None);
    }

    #[test]
    fn landing_commits_the_destination_as_the_new_current_perch() {
        let mut habitat = Habitat::development();
        let plan = habitat.prepare_next_trip().unwrap();
        let destination = habitat.destination_id().unwrap();

        let landed = habitat.complete_trip().unwrap();
        assert_eq!(landed.id, destination);
        assert_eq!(habitat.current_id(), destination);
        assert_eq!(habitat.destination_id(), None);
        assert_eq!(habitat.completed_trips, 1);
        assert_eq!(plan.landing, landed.anchor);
    }

    #[test]
    fn deterministic_selection_alternates_a_and_b() {
        let mut habitat = Habitat::development();
        for expected in [PerchId::B, PerchId::A, PerchId::B, PerchId::A] {
            habitat.prepare_next_trip().unwrap();
            assert_eq!(habitat.destination_id(), Some(expected));
            habitat.complete_trip().unwrap();
            assert_eq!(habitat.current_id(), expected);
        }
    }
}
