use super::visual::{AttentionIntent, BodyFrame, FaceFrame};
use crate::{
    debug::DebugOptions,
    pet::{Bat, BatState, ClickReaction, VisualPose},
};
use bevy::prelude::*;

pub const REACTION_DURATION: f32 = 1.45;

pub fn start_reaction(mut bats: Query<(&Transform, &mut ClickReaction), With<Bat>>) {
    for (transform, mut motion) in &mut bats {
        motion.elapsed = 0.0;
        motion.origin = transform.translation;
    }
}

/// A click is a small, anchored tuck-and-peek, with a readable quiet finish.
/// The former floating loop had neither wing strokes nor a return to rest.
pub fn animate_reaction(
    time: Res<Time>,
    mut bats: Query<(&mut VisualPose, &mut ClickReaction), With<Bat>>,
    mut next: ResMut<NextState<BatState>>,
) {
    for (mut pose, mut motion) in &mut bats {
        motion.elapsed += time.delta_secs();
        *pose = reaction_pose(motion.elapsed);
        if motion.elapsed >= REACTION_DURATION {
            next.set(BatState::HangingIdle);
        }
    }
}

fn reaction_pose(t: f32) -> VisualPose {
    let tucked = (0.12..0.52).contains(&t);
    VisualPose {
        body: if (0.52..0.76).contains(&t) {
            BodyFrame::EarTwitchRight
        } else {
            BodyFrame::Neutral
        },
        face: if (0.20..0.43).contains(&t) {
            FaceFrame::BlinkClosed
        } else if (0.14..0.49).contains(&t) {
            FaceFrame::BlinkHalf
        } else {
            FaceFrame::Open
        },
        attention: AttentionIntent {
            body_compression: if tucked { 0.012 } else { 0.0 },
            ear_alertness: if (0.52..0.96).contains(&t) { 0.8 } else { 0.0 },
            ..default()
        },
    }
}

pub fn log_reaction_started(debug: Res<DebugOptions>) {
    if debug.enabled {
        info!("bat state=Reacting trigger=click");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn click_has_anticipation_tuck_peek_and_rest() {
        assert_eq!(reaction_pose(0.0).face, FaceFrame::Open);
        assert_eq!(reaction_pose(0.3).face, FaceFrame::BlinkClosed);
        assert!(reaction_pose(0.3).attention.body_compression > 0.0);
        assert_eq!(reaction_pose(0.6).body, BodyFrame::EarTwitchRight);
        assert_eq!(reaction_pose(REACTION_DURATION), VisualPose::default());
    }
}
