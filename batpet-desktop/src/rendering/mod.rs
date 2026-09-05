pub mod rig;

use bevy::{
    asset::AssetServer,
    prelude::{Camera2d, Commands, Res, Sprite, Transform, Vec2, Visibility},
};

use crate::{
    pet::{
        AnimationIntent, AttentionMotion, Bat, BlinkState, BreathingMotion, ClickReaction,
        EarTwitchMotion, EyePupil, FlightMotion, IdleAdjustmentMotion, IdleGazeMotion, IdleMotion,
        Perch, VisualPose,
    },
    window::WINDOW_HEIGHT,
};

pub const BAT_SPRITE_PATH: &str = "bat/idle/bat_idle.png";
pub const SPRITE_WIDTH: u32 = 32;
pub const SPRITE_HEIGHT: u32 = 32;
pub const DISPLAY_SCALE: f32 = 8.0;
pub const DISPLAY_WIDTH: f32 = SPRITE_WIDTH as f32 * DISPLAY_SCALE;
pub const DISPLAY_HEIGHT: f32 = SPRITE_HEIGHT as f32 * DISPLAY_SCALE;
pub const EYE_CENTER_LOCAL: Vec2 = Vec2::new(0.0, -21.5 * DISPLAY_SCALE);
pub const TOP_MARGIN: f32 = 8.0;
pub const PUPIL_PIXEL_SIZE: f32 = 3.0;
pub const PUPIL_DISPLAY_SIZE: f32 = PUPIL_PIXEL_SIZE * DISPLAY_SCALE;
// The redesigned idle sprite places the eye sockets lower on its hanging face.
pub const LEFT_PUPIL_TOP_LEFT: Vec2 = Vec2::new(10.0, 20.0);
pub const RIGHT_PUPIL_TOP_LEFT: Vec2 = Vec2::new(19.0, 20.0);

pub fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2d);
    let base = Transform::from_xyz(0.0, WINDOW_HEIGHT as f32 * 0.5 - TOP_MARGIN, 0.0);
    let bat = commands
        .spawn((
            Bat,
            ClickReaction::default(),
            Perch::hanging(base.translation.truncate()),
            FlightMotion::at(base.translation.truncate()),
            IdleMotion::new(base.translation),
            BreathingMotion::default(),
            AttentionMotion::default(),
            IdleGazeMotion::default(),
            EarTwitchMotion::default(),
            IdleAdjustmentMotion::default(),
            AnimationIntent::default(),
            VisualPose::default(),
            BlinkState::default(),
            base,
            Visibility::Visible,
        ))
        .id();
    rig::spawn(&mut commands, bat, asset_server.load(BAT_SPRITE_PATH));
    commands.entity(bat).with_children(|parent| {
        for base_position in [
            pixel_top_left_to_local(LEFT_PUPIL_TOP_LEFT),
            pixel_top_left_to_local(RIGHT_PUPIL_TOP_LEFT),
        ] {
            parent
                .spawn((
                    EyePupil { base_position },
                    Sprite {
                        color: bevy::prelude::Color::srgb_u8(46, 17, 80),
                        custom_size: Some(Vec2::splat(PUPIL_DISPLAY_SIZE)),
                        ..Default::default()
                    },
                    Transform::from_xyz(base_position.x, base_position.y, 1.0),
                ))
                .with_children(|eye| {
                    eye.spawn((
                        Sprite::from_color(
                            bevy::prelude::Color::srgb_u8(251, 240, 216),
                            Vec2::splat(DISPLAY_SCALE),
                        ),
                        Transform::from_xyz(-DISPLAY_SCALE, DISPLAY_SCALE, 0.1),
                    ));
                });
        }
    });
}

pub fn pixel_top_left_to_local(top_left: Vec2) -> Vec2 {
    Vec2::new(
        (top_left.x + PUPIL_PIXEL_SIZE * 0.5 - SPRITE_WIDTH as f32 * 0.5) * DISPLAY_SCALE,
        -(top_left.y + PUPIL_PIXEL_SIZE * 0.5) * DISPLAY_SCALE,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uses_the_expected_idle_sprite_asset() {
        assert_eq!(BAT_SPRITE_PATH, "bat/idle/bat_idle.png");
        assert_eq!(SPRITE_WIDTH, 32);
        assert_eq!(SPRITE_HEIGHT, 32);
    }

    #[test]
    fn display_scale_is_an_integer_multiple_of_the_logical_art() {
        assert_eq!(DISPLAY_WIDTH as u32 % SPRITE_WIDTH, 0);
        assert_eq!(DISPLAY_HEIGHT as u32 % SPRITE_HEIGHT, 0);
    }

    #[test]
    fn pupil_base_positions_are_symmetric() {
        let left = pixel_top_left_to_local(LEFT_PUPIL_TOP_LEFT);
        let right = pixel_top_left_to_local(RIGHT_PUPIL_TOP_LEFT);

        assert_eq!(left.y, right.y);
        assert_eq!(left.x, -right.x);
    }
}
