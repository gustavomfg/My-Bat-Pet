use bevy::prelude::{Component, Query, Resource, Vec2, With};

use crate::pet::Bat;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BaseBodyFrame {
    #[default]
    Neutral,
    BreatheIn,
    BreatheOut,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BodyAction {
    WingAdjust,
    EarTwitchLeft,
    EarTwitchRight,
}

impl BodyAction {
    pub const fn label(self) -> &'static str {
        match self {
            Self::WingAdjust => "wing-adjust",
            Self::EarTwitchLeft => "left",
            Self::EarTwitchRight => "right",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FaceFrame {
    #[default]
    Open,
    BlinkHalf,
    BlinkClosed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AttentionBodyFrame {
    #[default]
    Relaxed,
    Attentive,
    Left,
    Right,
}

impl AttentionBodyFrame {
    pub const fn body_frame(self) -> Option<BodyFrame> {
        match self {
            Self::Relaxed => None,
            Self::Attentive => Some(BodyFrame::Attentive),
            Self::Left => Some(BodyFrame::AttentionLeft),
            Self::Right => Some(BodyFrame::AttentionRight),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AttentionReaction {
    #[default]
    None,
    VeryNear,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BodyFrame {
    #[default]
    Neutral,
    BreatheIn,
    BreatheOut,
    WingAdjust,
    EarTwitchLeft,
    EarTwitchRight,
    AttentionLeft,
    AttentionRight,
    Attentive,
    VeryNear,
}

impl BaseBodyFrame {
    pub const fn body_frame(self) -> BodyFrame {
        match self {
            Self::Neutral => BodyFrame::Neutral,
            Self::BreatheIn => BodyFrame::BreatheIn,
            Self::BreatheOut => BodyFrame::BreatheOut,
        }
    }
}

impl BodyAction {
    pub const fn body_frame(self) -> BodyFrame {
        match self {
            Self::WingAdjust => BodyFrame::WingAdjust,
            Self::EarTwitchLeft => BodyFrame::EarTwitchLeft,
            Self::EarTwitchRight => BodyFrame::EarTwitchRight,
        }
    }
}

impl BodyFrame {
    /// Index order for the future ten-cell body atlas described in the asset spec.
    pub const fn atlas_index(self) -> usize {
        match self {
            Self::Neutral => 0,
            Self::BreatheIn => 1,
            Self::BreatheOut => 2,
            Self::WingAdjust => 3,
            Self::EarTwitchLeft => 4,
            Self::EarTwitchRight => 5,
            Self::AttentionLeft => 6,
            Self::AttentionRight => 7,
            Self::Attentive => 8,
            Self::VeryNear => 9,
        }
    }
}

impl FaceFrame {
    /// The open face has no overlay cell; blink cells are half-closed then closed.
    pub const fn atlas_index(self) -> Option<usize> {
        match self {
            Self::Open => None,
            Self::BlinkHalf => Some(0),
            Self::BlinkClosed => Some(1),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AttentionIntent {
    /// Continuous values keep perception separate from the authored pose.
    /// The current monolithic sprite consumes `body_offset` as a safe fallback;
    /// a future layered head/ear renderer can consume the other channels.
    pub level: f32,
    pub head_offset: Vec2,
    pub body_offset: Vec2,
    pub ear_alertness: f32,
    pub body_compression: f32,
    pub body_frame: AttentionBodyFrame,
    pub reaction: AttentionReaction,
}

impl Default for AttentionIntent {
    fn default() -> Self {
        Self {
            level: 0.0,
            head_offset: Vec2::ZERO,
            body_offset: Vec2::ZERO,
            ear_alertness: 0.0,
            body_compression: 0.0,
            body_frame: AttentionBodyFrame::Relaxed,
            reaction: AttentionReaction::None,
        }
    }
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Default)]
pub struct AnimationIntent {
    pub base_body: BaseBodyFrame,
    pub body_action: Option<BodyAction>,
    pub face: FaceFrame,
    pub attention: AttentionIntent,
}

impl AnimationIntent {
    pub const fn effective_body(self) -> BodyFrame {
        if matches!(self.attention.reaction, AttentionReaction::VeryNear) {
            return BodyFrame::VeryNear;
        }

        if let Some(attention_frame) = self.attention.body_frame.body_frame() {
            return attention_frame;
        }

        match self.body_action {
            Some(action) => action.body_frame(),
            None => self.base_body.body_frame(),
        }
    }
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Default)]
pub struct VisualPose {
    pub body: BodyFrame,
    pub face: FaceFrame,
    pub attention: AttentionIntent,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Resource)]
pub struct VisualAssetAvailability {
    pub body_atlas: bool,
    pub face_atlas: bool,
}

pub fn resolve_visual_pose(mut bats: Query<(&AnimationIntent, &mut VisualPose), With<Bat>>) {
    for (intent, mut pose) in &mut bats {
        pose.body = intent.effective_body();
        pose.face = intent.face;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transient_body_action_has_priority_over_breathing() {
        let intent = AnimationIntent {
            base_body: BaseBodyFrame::BreatheIn,
            body_action: Some(BodyAction::EarTwitchRight),
            face: FaceFrame::Open,
            attention: AttentionIntent::default(),
        };

        assert_eq!(intent.effective_body(), BodyFrame::EarTwitchRight);
    }

    #[test]
    fn attention_has_priority_over_background_actions() {
        let intent = AnimationIntent {
            base_body: BaseBodyFrame::BreatheIn,
            body_action: Some(BodyAction::WingAdjust),
            face: FaceFrame::Open,
            attention: AttentionIntent {
                body_frame: AttentionBodyFrame::Right,
                ..Default::default()
            },
        };

        assert_eq!(intent.effective_body(), BodyFrame::AttentionRight);
    }

    #[test]
    fn very_near_reaction_has_highest_body_priority() {
        let intent = AnimationIntent {
            body_action: Some(BodyAction::EarTwitchLeft),
            attention: AttentionIntent {
                body_frame: AttentionBodyFrame::Right,
                reaction: AttentionReaction::VeryNear,
                ..Default::default()
            },
            ..Default::default()
        };

        assert_eq!(intent.effective_body(), BodyFrame::VeryNear);
    }

    #[test]
    fn body_atlas_indices_are_stable_and_contiguous() {
        assert_eq!(BodyFrame::Neutral.atlas_index(), 0);
        assert_eq!(BodyFrame::BreatheIn.atlas_index(), 1);
        assert_eq!(BodyFrame::BreatheOut.atlas_index(), 2);
        assert_eq!(BodyFrame::WingAdjust.atlas_index(), 3);
        assert_eq!(BodyFrame::EarTwitchLeft.atlas_index(), 4);
        assert_eq!(BodyFrame::EarTwitchRight.atlas_index(), 5);
        assert_eq!(BodyFrame::AttentionLeft.atlas_index(), 6);
        assert_eq!(BodyFrame::AttentionRight.atlas_index(), 7);
        assert_eq!(BodyFrame::Attentive.atlas_index(), 8);
        assert_eq!(BodyFrame::VeryNear.atlas_index(), 9);
    }

    #[test]
    fn open_face_has_no_overlay_and_blink_frames_are_ordered() {
        assert_eq!(FaceFrame::Open.atlas_index(), None);
        assert_eq!(FaceFrame::BlinkHalf.atlas_index(), Some(0));
        assert_eq!(FaceFrame::BlinkClosed.atlas_index(), Some(1));
    }
}
