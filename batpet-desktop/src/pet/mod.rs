mod acting;
mod components;
mod eyes;
mod flight;
mod idle;
mod motion;
mod state;
pub(crate) mod visual;

pub use acting::{AttentionMotion, IdleGazeMotion, update_attention, update_idle_gaze};
pub use components::{Bat, ClickReaction, CursorState, EyePupil};
pub use eyes::{EyeState, update_eyes};
#[allow(unused_imports)]
pub use flight::{
    FlightDebug, FlightMotion, FlightTarget, Perch, TAKEOFF_SUPPORT_HOLD, finish_landing,
    log_flight_entered, start_flying, start_landing, start_returning, start_takeoff,
    trigger_flight, update_flying, update_landing, update_returning, update_takeoff,
};
pub use idle::{
    BlinkState, BreathingMotion, EarTwitchMotion, IdleAdjustmentMotion, IdleMotion, IdleScheduler,
    advance_idle_scheduler, apply_idle_motion, update_blink, update_breathing, update_ear_twitch,
    update_idle_adjustment,
};
pub use motion::{animate_reaction, log_reaction_started, start_reaction};
pub use state::BatState;
pub use visual::{AnimationIntent, VisualPose, resolve_visual_pose};

use bevy::{
    ecs::message::MessageReader,
    input::ButtonState,
    input::mouse::{MouseButton, MouseButtonInput},
    log::info,
    prelude::{Query, Res, ResMut, State, Transform, With},
    window::{CursorEntered, CursorLeft, CursorMoved, PrimaryWindow, Window},
};

use crate::debug::DebugOptions;
use crate::rendering::{DISPLAY_HEIGHT, DISPLAY_WIDTH};

pub fn capture_cursor(
    mut moved_events: MessageReader<CursorMoved>,
    mut entered_events: MessageReader<CursorEntered>,
    mut left_events: MessageReader<CursorLeft>,
    mut cursor: ResMut<CursorState>,
    debug: Res<DebugOptions>,
    windows: Query<&Window, With<PrimaryWindow>>,
) {
    let entered = entered_events.read().next().is_some();

    for event in moved_events.read() {
        // X11 can report stale/outside coordinates while a window is placed.
        // Treat those as absence, not as an unseen stimulus to keep tracking.
        cursor.position = windows.iter().next().and_then(|window| {
            cursor_inside_window(event.position, window).then_some(event.position)
        });
    }

    for _ in left_events.read() {
        if cursor.position.take().is_some() && debug.enabled {
            info!("cursor left window");
        }
    }

    if entered && debug.enabled {
        info!("cursor entered window");
    }
}

fn cursor_inside_window(position: bevy::prelude::Vec2, window: &Window) -> bool {
    position.is_finite()
        && position.x >= 0.0
        && position.y >= 0.0
        && position.x < window.resolution.width()
        && position.y < window.resolution.height()
}

pub fn trigger_reaction(
    mut button_events: MessageReader<MouseButtonInput>,
    cursor: Res<CursorState>,
    windows: Query<&Window, With<PrimaryWindow>>,
    bats: Query<&Transform, With<Bat>>,
    state: Res<State<BatState>>,
    mut next_state: ResMut<bevy::prelude::NextState<BatState>>,
) {
    let clicked = button_events
        .read()
        .any(|event| event.button == MouseButton::Left && event.state == ButtonState::Pressed);

    if *state.get() != BatState::HangingIdle {
        return;
    }

    let Some(window) = windows.iter().next() else {
        return;
    };
    let Some(bat_transform) = bats.iter().next() else {
        return;
    };
    let Some(cursor_position) = cursor.position.or_else(|| window.cursor_position()) else {
        return;
    };

    let over_bat = cursor_over_bat(cursor_position, window, bat_transform);
    let clicked_on_bat = clicked && over_bat;

    // Hover is part of the acting/perception layer now. Reacting remains an
    // explicit interaction so a very-near cursor can produce the shy reaction
    // without immediately taking the pet out of HangingIdle.
    if clicked_on_bat {
        next_state.set(BatState::Reacting);
    }
}

pub fn cursor_over_bat(cursor: bevy::prelude::Vec2, window: &Window, bat: &Transform) -> bool {
    let window_center = bevy::prelude::Vec2::new(
        window.resolution.width() * 0.5,
        window.resolution.height() * 0.5,
    );
    let cursor_world =
        bevy::prelude::Vec2::new(cursor.x - window_center.x, window_center.y - cursor.y);

    let half_width = DISPLAY_WIDTH * 0.5;
    let top = bat.translation.y;
    let bottom = top - DISPLAY_HEIGHT;

    cursor_world.x >= bat.translation.x - half_width
        && cursor_world.x <= bat.translation.x + half_width
        && cursor_world.y >= bottom
        && cursor_world.y <= top
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::prelude::Vec2;

    #[test]
    fn startup_and_outside_coordinates_are_not_presence() {
        let window = Window {
            resolution: bevy::window::WindowResolution::new(320, 320),
            ..Default::default()
        };
        assert!(cursor_inside_window(Vec2::new(160., 180.), &window));
        for position in [
            Vec2::new(960., 572.),
            Vec2::new(-1., 180.),
            Vec2::new(320., 180.),
            Vec2::splat(f32::NAN),
        ] {
            assert!(!cursor_inside_window(position, &window));
        }
    }

    #[test]
    fn detects_cursor_inside_and_outside_the_bat_hitbox() {
        let window = Window::default();
        let bat = Transform::from_xyz(0.0, 100.0, 0.0);
        let center = Vec2::new(
            window.resolution.width() * 0.5,
            window.resolution.height() * 0.5,
        );

        assert!(cursor_over_bat(center, &window, &bat));
        assert!(!cursor_over_bat(Vec2::new(0.0, center.y), &window, &bat));
    }
}
