use bevy::prelude::{Query, Res, Time, Transform, Vec3, With, info};

use crate::{
    debug::DebugOptions,
    pet::{Bat, FlightMotion},
};

pub const FLIGHT_X_RADIUS: f32 = 28.0;
pub const FLIGHT_Y_RADIUS: f32 = 14.0;
pub const FLIGHT_LAUNCH_DROP: f32 = 22.0;
pub const FLIGHT_X_SPEED: f32 = 1.35;
pub const FLIGHT_Y_SPEED: f32 = 2.2;
pub const FLIGHT_ROTATION: f32 = 0.045;

pub fn start_flight(mut bats: Query<(&mut Transform, &mut FlightMotion), With<Bat>>) {
    for (mut transform, mut motion) in &mut bats {
        motion.elapsed = 0.0;
        motion.origin = transform.translation;
        transform.scale = Vec3::ONE;
    }
}

pub fn animate_flight(
    time: Res<Time>,
    mut bats: Query<(&mut Transform, &mut FlightMotion), With<Bat>>,
) {
    for (mut transform, mut motion) in &mut bats {
        motion.elapsed += time.delta_secs();
        let elapsed = motion.elapsed;

        transform.translation = motion.origin
            + Vec3::new(
                FLIGHT_X_RADIUS * (elapsed * FLIGHT_X_SPEED).sin(),
                -FLIGHT_LAUNCH_DROP + FLIGHT_Y_RADIUS * (elapsed * FLIGHT_Y_SPEED).sin(),
                0.0,
            );
        transform.rotation = bevy::prelude::Quat::from_rotation_z(
            FLIGHT_ROTATION * (elapsed * FLIGHT_X_SPEED).sin(),
        );
    }
}

pub fn log_flight_started(debug: Res<DebugOptions>) {
    if debug.enabled {
        info!("bat state=Flying trigger=hover_or_click");
    }
}
