//! A small cut-paper rig using the original texels. All output edges stay on
//! the eight-pixel art grid; no rotated or fractionally scaled sprites.
use super::DISPLAY_SCALE as S;
use super::{FLIGHT_FRAME_COUNT, FLIGHT_FRAME_HEIGHT, FLIGHT_SPRITE_WIDTH};
use crate::pet::visual::{BodyFrame, FaceFrame};
use crate::pet::{
    Bat, BatState, BreathingMotion, FlightMotion, FlightVisualFrame, FlightVisualIntent, Perch,
    VisualPose,
};
use bevy::{prelude::*, sprite::Anchor};

#[derive(Component)]
pub struct Piece {
    origin: Vec2,
    kind: Kind,
}

#[derive(Component)]
pub(crate) struct FlightSprite;
#[derive(Clone, Copy)]
enum Kind {
    Support,
    BodyUpper,
    BodyMiddle,
    ChestBody(u8),
    WingLeft,
    WingRight,
    WingLeftLower,
    WingRightLower,
    BreathFill,
    Ear(bool),
    Eye,
    Lid,
    Crease,
}

pub fn spawn_with_flight(
    commands: &mut Commands,
    bat: Entity,
    image: Handle<Image>,
    flight_image: Handle<Image>,
) {
    let mut pieces = Vec::new();
    // The ear root overlaps one row so a tip can settle without a seam.
    let mut cuts = vec![
        (0., 0., 32., 3., Kind::Support),
        (9., 3., 14., 9., Kind::BodyUpper),
        (0., 3., 13., 9., Kind::WingLeft),
        (19., 3., 13., 9., Kind::WingRight),
        (9., 19., 14., 8., Kind::BodyMiddle),
        (0., 19., 13., 8., Kind::WingLeftLower),
        (19., 19., 13., 8., Kind::WingRightLower),
        (0., 26., 11., 6., Kind::Ear(true)),
        (21., 26., 11., 6., Kind::Ear(false)),
        (0., 16., 32., 1., Kind::BreathFill),
    ];
    for row in 12..19 {
        cuts.push((9., row as f32, 14., 1., Kind::ChestBody(row)));
        cuts.push((0., row as f32, 13., 1., Kind::WingLeftLower));
        cuts.push((19., row as f32, 13., 1., Kind::WingRightLower));
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
    commands.entity(bat).with_children(|parent| {
        parent.spawn((
            FlightSprite,
            Sprite {
                image: flight_image.clone(),
                custom_size: Some(Vec2::new(
                    FLIGHT_SPRITE_WIDTH as f32 * S,
                    FLIGHT_FRAME_HEIGHT as f32 * S,
                )),
                ..default()
            },
            Anchor::TOP_LEFT,
            Transform::from_xyz(-(FLIGHT_SPRITE_WIDTH as f32 * 0.5) * S, 0., 0.),
            Visibility::Hidden,
        ));
    });
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
    state: Res<State<BatState>>,
    flight: Query<(&FlightMotion, &Perch), With<Bat>>,
    flight_intent: Query<&FlightVisualIntent, With<Bat>>,
    mut flight_sprites: Query<(&mut Sprite, &mut Transform, &mut Visibility), With<FlightSprite>>,
    mut pieces: Query<
        (&Piece, &mut Transform, &mut Sprite, &mut Visibility),
        Without<FlightSprite>,
    >,
) {
    let Some((pose, breathing)) = bats.iter().next() else {
        return;
    };
    let head = head_offset(pose, breathing);
    let flight_motion = flight.iter().next();
    let flight_intent = flight_intent.iter().next().copied().unwrap_or_default();
    let flight_active = matches!(
        state.get(),
        BatState::Takeoff | BatState::Flying | BatState::Returning | BatState::Landing
    );
    let body_offset = if flight_active {
        flight_intent.body_offset
    } else {
        Vec2::ZERO
    };
    let flight_pose_active = flight_active
        && !matches!(
            flight_intent.frame,
            FlightVisualFrame::Coil | FlightVisualFrame::Reach
        );
    for (mut sprite, mut transform, mut visibility) in &mut flight_sprites {
        *visibility = if flight_pose_active {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        sprite.rect = Some(Rect::new(
            0.,
            flight_intent
                .frame
                .sheet_index()
                .min(FLIGHT_FRAME_COUNT - 1) as f32
                * FLIGHT_FRAME_HEIGHT as f32,
            FLIGHT_SPRITE_WIDTH as f32,
            (flight_intent
                .frame
                .sheet_index()
                .min(FLIGHT_FRAME_COUNT - 1)
                + 1) as f32
                * FLIGHT_FRAME_HEIGHT as f32,
        ));
        sprite.flip_x = flight_pose_active && flight_intent.facing < 0;
        transform.translation.y = body_offset.y;
    }
    for (piece, mut transform, mut sprite, mut visibility) in &mut pieces {
        *visibility = Visibility::Visible;
        let offset = match piece.kind {
            Kind::Support => {
                let show_support = match state.get() {
                    BatState::HangingIdle | BatState::Reacting => true,
                    BatState::Takeoff => flight_motion
                        .map(|(motion, _)| motion.stage_elapsed < crate::pet::TAKEOFF_SUPPORT_HOLD)
                        .unwrap_or(false),
                    BatState::Landing => flight_motion
                        .map(|(motion, perch)| {
                            motion.arrived || motion.position.distance(perch.anchor) <= 24.0
                        })
                        .unwrap_or(false),
                    BatState::Flying | BatState::Returning => false,
                };
                if !show_support {
                    *visibility = Visibility::Hidden;
                }
                if flight_pose_active {
                    *visibility = Visibility::Hidden;
                }
                Vec2::ZERO
            }
            Kind::BodyUpper | Kind::BodyMiddle => {
                if flight_pose_active {
                    *visibility = Visibility::Hidden;
                }
                body_offset
            }
            Kind::ChestBody(row) => {
                if flight_pose_active {
                    *visibility = Visibility::Hidden;
                }
                // Insert/remove one authored scanline rather than stretching pixels.
                if !flight_active && row == 15 && head.y > 0. {
                    *visibility = Visibility::Hidden;
                }
                if flight_active {
                    body_offset
                } else if row >= 16 {
                    Vec2::new(0., head.y)
                } else {
                    Vec2::ZERO
                }
            }
            Kind::BreathFill => {
                *visibility = if flight_pose_active {
                    Visibility::Hidden
                } else if head.y < 0. {
                    Visibility::Visible
                } else {
                    Visibility::Hidden
                };
                Vec2::ZERO
            }
            Kind::Ear(left) => {
                if flight_pose_active {
                    *visibility = Visibility::Hidden;
                }
                let twitch = matches!(
                    (left, pose.body),
                    (true, BodyFrame::EarTwitchLeft) | (false, BodyFrame::EarTwitchRight)
                );
                // The ear facing the stimulus listens first; both rise only
                // for a genuinely close encounter, then settle independently.
                let facing = left == (pose.attention.body_offset.x < 0.0);
                let alert = pose.attention.ear_alertness > if facing { 0.42 } else { 0.92 };
                head + body_offset
                    + wing_offset(&flight_intent, left, flight_active)
                    + Vec2::new(
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
                head + body_offset
            }
            Kind::Crease => {
                *visibility = if pose.face == FaceFrame::BlinkClosed {
                    Visibility::Visible
                } else {
                    Visibility::Hidden
                };
                head + body_offset
            }
            Kind::WingLeft => wing_offset(&flight_intent, true, flight_active),
            Kind::WingRight => wing_offset(&flight_intent, false, flight_active),
            Kind::WingLeftLower => wing_offset(&flight_intent, true, flight_active),
            Kind::WingRightLower => wing_offset(&flight_intent, false, flight_active),
            Kind::Eye => head + body_offset,
        };
        if flight_pose_active
            && matches!(
                piece.kind,
                Kind::WingLeft | Kind::WingRight | Kind::WingLeftLower | Kind::WingRightLower
            )
        {
            *visibility = Visibility::Hidden;
        }
        transform.translation.x = piece.origin.x + offset.x;
        transform.translation.y = piece.origin.y + offset.y;
    }
}

fn wing_offset(intent: &FlightVisualIntent, left: bool, active: bool) -> Vec2 {
    if !active {
        return Vec2::ZERO;
    }

    let (spread, lift) = match intent.frame {
        FlightVisualFrame::Coil => (0.0, 0.0),
        FlightVisualFrame::Lift => (3.0, 4.0),
        FlightVisualFrame::Spread => (4.0, 1.0),
        FlightVisualFrame::Power => (4.0, -3.0),
        FlightVisualFrame::Recover => (2.0, 0.0),
        FlightVisualFrame::Reach => (0.0, -1.0),
        FlightVisualFrame::Rest => (0.0, 0.0),
    };
    let side = if left { -1.0 } else { 1.0 };
    let leading = (left && intent.facing < 0) || (!left && intent.facing > 0);
    Vec2::new(
        side * spread * S,
        (lift + if leading { 1.0 } else { 0.0 }) * S,
    )
}
