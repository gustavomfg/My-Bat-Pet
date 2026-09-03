use std::path::Path;

use bevy::{
    asset::AssetServer,
    prelude::{
        Assets, Camera2d, Children, Commands, Query, Res, ResMut, Sprite, TextureAtlas,
        TextureAtlasLayout, Transform, UVec2, Vec2, Visibility, With, Without,
    },
    sprite::Anchor,
};

use crate::{
    pet::{
        AnimationIntent, AttentionMotion, Bat, BlinkState, BreathingMotion, EarTwitchMotion,
        EyeLid, EyePupil, FlightMotion, IdleAdjustmentMotion, IdleGazeMotion, IdleMotion,
        VisualAssetAvailability, VisualPose,
    },
    window::WINDOW_HEIGHT,
};

pub const BAT_SPRITE_PATH: &str = "bat/idle/bat_idle.png";
pub const BODY_SPRITE_SHEET_PATH: &str = "bat/idle/bat_hanging_body.png";
pub const FACE_SPRITE_SHEET_PATH: &str = "bat/idle/bat_hanging_face.png";
pub const PUPIL_SPRITE_PATH: &str = "bat/eyes/pupil.png";
pub const SPRITE_WIDTH: u32 = 32;
pub const SPRITE_HEIGHT: u32 = 32;
pub const BODY_ATLAS_FRAME_COUNT: u32 = 10;
pub const FACE_ATLAS_FRAME_COUNT: u32 = 2;
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

pub fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
    mut visual_assets: ResMut<VisualAssetAvailability>,
) {
    commands.spawn(Camera2d);

    // The reviewed atlas is opt-in: until the art exists, keep the known-good
    // single-frame asset and make the missing visual work explicit in the
    // runtime resource.
    visual_assets.body_atlas = asset_exists(BODY_SPRITE_SHEET_PATH);
    visual_assets.face_atlas = visual_assets.body_atlas && asset_exists(FACE_SPRITE_SHEET_PATH);

    let (image, texture_atlas) = if visual_assets.body_atlas {
        let image = asset_server.load(BODY_SPRITE_SHEET_PATH);
        let layout = texture_atlas_layouts.add(TextureAtlasLayout::from_grid(
            UVec2::new(SPRITE_WIDTH, SPRITE_HEIGHT),
            BODY_ATLAS_FRAME_COUNT,
            1,
            None,
            None,
        ));

        (image, Some(TextureAtlas { layout, index: 0 }))
    } else {
        (asset_server.load(BAT_SPRITE_PATH), None)
    };

    let face_layer = if visual_assets.face_atlas {
        let image = asset_server.load(FACE_SPRITE_SHEET_PATH);
        let layout = texture_atlas_layouts.add(TextureAtlasLayout::from_grid(
            UVec2::new(SPRITE_WIDTH, SPRITE_HEIGHT),
            FACE_ATLAS_FRAME_COUNT,
            1,
            None,
            None,
        ));

        Some((image, layout))
    } else {
        None
    };

    let pupil_image = asset_server.load(PUPIL_SPRITE_PATH);
    let base_translation = Transform::from_xyz(0.0, WINDOW_HEIGHT as f32 * 0.5 - TOP_MARGIN, 0.0);
    let bat = commands
        .spawn((
            Bat,
            FlightMotion::default(),
            IdleMotion::new(base_translation.translation),
            BreathingMotion::default(),
            AttentionMotion::default(),
            IdleGazeMotion::default(),
            EarTwitchMotion::default(),
            IdleAdjustmentMotion::default(),
            AnimationIntent::default(),
            VisualPose::default(),
            BlinkState::default(),
            Sprite {
                image,
                texture_atlas,
                custom_size: Some(Vec2::new(DISPLAY_WIDTH, DISPLAY_HEIGHT)),
                ..Default::default()
            },
            Anchor::TOP_CENTER,
            base_translation,
        ))
        .id();

    commands.entity(bat).with_children(|parent| {
        if let Some((image, layout)) = face_layer {
            parent.spawn((
                EyeLid,
                Sprite {
                    image,
                    texture_atlas: Some(TextureAtlas { layout, index: 0 }),
                    custom_size: Some(Vec2::new(DISPLAY_WIDTH, DISPLAY_HEIGHT)),
                    ..Default::default()
                },
                Anchor::TOP_CENTER,
                Transform::from_xyz(0.0, 0.0, 2.0),
                Visibility::Hidden,
            ));
        }

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

fn asset_exists(relative_path: &str) -> bool {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../assets")
        .join(relative_path)
        .is_file()
}

pub fn apply_visual_pose(
    mut bats: Query<(&VisualPose, &mut Sprite, &Children), (With<Bat>, Without<EyeLid>)>,
    mut face_layers: Query<(&mut Sprite, &mut Visibility), (With<EyeLid>, Without<Bat>)>,
) {
    for (pose, mut sprite, children) in &mut bats {
        if let Some(atlas) = sprite.texture_atlas.as_mut() {
            atlas.index = pose.body.atlas_index();
        }
        // Without an atlas the current asset is intentionally left as the
        // one-frame fallback until the reviewed body art is added.

        for child in children.iter() {
            let Ok((mut face_sprite, mut visibility)) = face_layers.get_mut(*child) else {
                continue;
            };

            let Some(index) = pose.face.atlas_index() else {
                *visibility = Visibility::Hidden;
                continue;
            };

            let Some(atlas) = face_sprite.texture_atlas.as_mut() else {
                *visibility = Visibility::Hidden;
                continue;
            };

            atlas.index = index;
            *visibility = Visibility::Visible;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uses_the_expected_idle_sprite_asset() {
        assert_eq!(BAT_SPRITE_PATH, "bat/idle/bat_idle.png");
        assert_eq!(BODY_SPRITE_SHEET_PATH, "bat/idle/bat_hanging_body.png");
        assert_eq!(FACE_SPRITE_SHEET_PATH, "bat/idle/bat_hanging_face.png");
        assert_eq!(PUPIL_SPRITE_PATH, "bat/eyes/pupil.png");
        assert_eq!(SPRITE_WIDTH, 32);
        assert_eq!(SPRITE_HEIGHT, 32);
        assert_eq!(BODY_ATLAS_FRAME_COUNT, 10);
        assert_eq!(FACE_ATLAS_FRAME_COUNT, 2);
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
