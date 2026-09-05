//! A small cut-paper rig using the original texels. All output edges stay on
//! the eight-pixel art grid; no rotated or fractionally scaled sprites.
use super::DISPLAY_SCALE as S;
use crate::pet::visual::{BodyFrame, FaceFrame};
use crate::pet::{Bat, BreathingMotion, VisualPose};
use bevy::{prelude::*, sprite::Anchor};

#[derive(Component)]
pub struct Piece {
    origin: Vec2,
    kind: Kind,
}
#[derive(Clone, Copy)]
enum Kind {
    Fixed,
    ChestRow(u8),
    BreathFill,
    Head,
    Ear(bool),
    Eye,
    Lid,
    Crease,
}

pub fn spawn(commands: &mut Commands, bat: Entity, image: Handle<Image>) {
    let mut pieces = Vec::new();
    // The ear root overlaps one row so a tip can settle without a seam.
    let mut cuts = vec![
        (0., 0., 32., 12., Kind::Fixed),
        (0., 19., 32., 8., Kind::Head),
        (11., 27., 10., 5., Kind::Head),
        (0., 26., 11., 6., Kind::Ear(true)),
        (21., 26., 11., 6., Kind::Ear(false)),
        (0., 16., 32., 1., Kind::BreathFill),
    ];
    for row in 12..19 {
        cuts.push((0., row as f32, 32., 1., Kind::ChestRow(row)));
    }
    for (x, y, w, h, kind) in cuts {
        let origin = Vec2::new((x - 16.) * S, -y * S);
        pieces.push(
            commands
                .spawn((
                    Piece { origin, kind },
                    Sprite {
                        image: image.clone(),
                        rect: Some(Rect::new(x, y, x + w, y + h)),
                        custom_size: Some(Vec2::new(w * S, h * S)),
                        ..default()
                    },
                    Anchor::TOP_LEFT,
                    Transform::from_xyz(origin.x, origin.y, 0.),
                ))
                .id(),
        );
    }
    let fur = Color::srgb_u8(90, 57, 141);
    let cream = Color::srgb_u8(251, 240, 216);
    let ink = Color::srgb_u8(46, 17, 80);
    for x in [9., 18.] {
        // Quiet, rounded eye shapes replace the mottled socket interior.
        for (dx, dy, w, h, color, z, kind) in [
            (0., 0., 5., 5., fur, 0.2, Kind::Eye),
            (1., 0., 3., 5., cream, 0.3, Kind::Eye),
            (0., 1., 5., 3., cream, 0.3, Kind::Eye),
            (0., 0., 5., 5., fur, 2., Kind::Lid),
            (1., 3., 3., 1., ink, 2.1, Kind::Crease),
        ] {
            let origin = Vec2::new((x + dx - 16.) * S, -(19. + dy) * S);
            let entity = commands
                .spawn((
                    Piece { origin, kind },
                    Sprite::from_color(color, Vec2::new(w * S, h * S)),
                    Anchor::TOP_LEFT,
                    Transform::from_xyz(origin.x, origin.y, z),
                ))
                .id();
            pieces.push(entity);
        }
    }
    commands.entity(bat).add_children(&pieces);
}

pub fn head_offset(pose: &VisualPose, breathing: &BreathingMotion) -> Vec2 {
    let inhale = (breathing.elapsed / breathing.duration).rem_euclid(1.);
    // A long rest, a soft inhale, then release. One logical pixel is enough.
    let breath = if (0.20..0.48).contains(&inhale) {
        -S
    } else {
        0.
    };
    let x = if pose.body == BodyFrame::WingAdjust {
        -S
    } else {
        pose.attention.head_offset.x.round().clamp(-1., 1.) * S
    };
    let shy = if pose.attention.body_compression > 0.006 {
        S
    } else {
        0.
    };
    Vec2::new(x, breath + shy)
}

pub fn animate(
    bats: Query<(&VisualPose, &BreathingMotion), With<Bat>>,
    mut pieces: Query<(&Piece, &mut Transform, &mut Sprite, &mut Visibility)>,
) {
    let Some((pose, breathing)) = bats.iter().next() else {
        return;
    };
    let head = head_offset(pose, breathing);
    for (piece, mut transform, mut sprite, mut visibility) in &mut pieces {
        *visibility = Visibility::Visible;
        let offset = match piece.kind {
            Kind::Fixed => Vec2::ZERO,
            Kind::ChestRow(row) => {
                // Insert/remove one authored scanline rather than stretching pixels.
                if row == 15 && head.y > 0. {
                    *visibility = Visibility::Hidden;
                }
                if row >= 16 {
                    Vec2::new(0., head.y)
                } else {
                    Vec2::ZERO
                }
            }
            Kind::BreathFill => {
                *visibility = if head.y < 0. {
                    Visibility::Visible
                } else {
                    Visibility::Hidden
                };
                Vec2::ZERO
            }
            Kind::Ear(left) => {
                let twitch = matches!(
                    (left, pose.body),
                    (true, BodyFrame::EarTwitchLeft) | (false, BodyFrame::EarTwitchRight)
                );
                // The ear facing the stimulus listens first; both rise only
                // for a genuinely close encounter, then settle independently.
                let facing = left == (pose.attention.body_offset.x < 0.0);
                let alert = pose.attention.ear_alertness > if facing { 0.42 } else { 0.92 };
                head + Vec2::new(
                    if twitch {
                        if left { S } else { -S }
                    } else {
                        0.
                    },
                    if twitch || alert { -S } else { 0. },
                )
            }
            Kind::Lid => {
                *visibility = if pose.face == FaceFrame::Open {
                    Visibility::Hidden
                } else {
                    Visibility::Visible
                };
                sprite.custom_size = Some(Vec2::new(
                    5. * S,
                    if pose.face == FaceFrame::BlinkHalf {
                        2. * S
                    } else {
                        5. * S
                    },
                ));
                head
            }
            Kind::Crease => {
                *visibility = if pose.face == FaceFrame::BlinkClosed {
                    Visibility::Visible
                } else {
                    Visibility::Hidden
                };
                head
            }
            _ => head,
        };
        transform.translation.x = piece.origin.x + offset.x;
        transform.translation.y = piece.origin.y + offset.y;
    }
}
