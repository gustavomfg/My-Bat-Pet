use bevy::{
    asset::AssetServer,
    prelude::{Camera2d, Commands, Res, Sprite, Transform, Vec2},
    sprite::Anchor,
};

use crate::pet::Bat;

pub const BAT_SPRITE_PATH: &str = "bat/idle/bat_idle.png";
pub const SPRITE_WIDTH: u32 = 20;
pub const SPRITE_HEIGHT: u32 = 20;
pub const DISPLAY_SCALE: f32 = 12.0;
pub const DISPLAY_WIDTH: f32 = SPRITE_WIDTH as f32 * DISPLAY_SCALE;
pub const DISPLAY_HEIGHT: f32 = SPRITE_HEIGHT as f32 * DISPLAY_SCALE;
pub const TOP_MARGIN: f32 = 8.0;

pub fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2d);

    let image = asset_server.load(BAT_SPRITE_PATH);
    commands.spawn((
        Bat,
        Sprite {
            image,
            custom_size: Some(Vec2::new(DISPLAY_WIDTH, DISPLAY_HEIGHT)),
            ..Default::default()
        },
        Anchor::TOP_CENTER,
        Transform::from_xyz(0.0, 128.0 - TOP_MARGIN, 0.0),
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uses_the_expected_idle_sprite_asset() {
        assert_eq!(BAT_SPRITE_PATH, "bat/idle/bat_idle.png");
        assert_eq!(SPRITE_WIDTH, 20);
        assert_eq!(SPRITE_HEIGHT, 20);
    }

    #[test]
    fn display_scale_is_an_integer_multiple_of_the_logical_art() {
        assert_eq!(DISPLAY_WIDTH as u32 % SPRITE_WIDTH, 0);
        assert_eq!(DISPLAY_HEIGHT as u32 % SPRITE_HEIGHT, 0);
    }
}
