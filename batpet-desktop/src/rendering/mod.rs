use bevy::{
    asset::AssetServer,
    prelude::{Camera2d, Commands, Res, Sprite, Transform, Vec2},
    sprite::Anchor,
};

use crate::{
    pet::{Bat, EyePupil, FlightMotion},
    window::WINDOW_HEIGHT,
};

pub const BAT_SPRITE_PATH: &str = "bat/idle/bat_idle.png";
pub const PUPIL_SPRITE_PATH: &str = "bat/eyes/pupil.png";
pub const SPRITE_WIDTH: u32 = 32;
pub const SPRITE_HEIGHT: u32 = 32;
pub const DISPLAY_SCALE: f32 = 8.0;
pub const DISPLAY_WIDTH: f32 = SPRITE_WIDTH as f32 * DISPLAY_SCALE;
pub const DISPLAY_HEIGHT: f32 = SPRITE_HEIGHT as f32 * DISPLAY_SCALE;
pub const TOP_MARGIN: f32 = 8.0;
pub const PUPIL_PIXEL_SIZE: f32 = 2.0;
pub const PUPIL_DISPLAY_SIZE: f32 = PUPIL_PIXEL_SIZE * DISPLAY_SCALE;
// The redesigned idle sprite places the eye sockets lower on its hanging face.
pub const LEFT_PUPIL_TOP_LEFT: Vec2 = Vec2::new(10.5, 20.5);
pub const RIGHT_PUPIL_TOP_LEFT: Vec2 = Vec2::new(19.5, 20.5);

pub fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2d);

    let image = asset_server.load(BAT_SPRITE_PATH);
    let pupil_image = asset_server.load(PUPIL_SPRITE_PATH);
    let bat = commands
        .spawn((
            Bat,
            FlightMotion::default(),
            Sprite {
                image,
                custom_size: Some(Vec2::new(DISPLAY_WIDTH, DISPLAY_HEIGHT)),
                ..Default::default()
            },
            Anchor::TOP_CENTER,
            Transform::from_xyz(0.0, WINDOW_HEIGHT as f32 * 0.5 - TOP_MARGIN, 0.0),
        ))
        .id();

    commands.entity(bat).with_children(|parent| {
        for base_position in [
            pixel_top_left_to_local(LEFT_PUPIL_TOP_LEFT),
            pixel_top_left_to_local(RIGHT_PUPIL_TOP_LEFT),
        ] {
            parent.spawn((
                EyePupil { base_position },
                Sprite {
                    image: pupil_image.clone(),
                    custom_size: Some(Vec2::splat(PUPIL_DISPLAY_SIZE)),
                    ..Default::default()
                },
                Transform::from_xyz(base_position.x, base_position.y, 1.0),
            ));
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
        assert_eq!(PUPIL_SPRITE_PATH, "bat/eyes/pupil.png");
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
