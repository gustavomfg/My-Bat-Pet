use bevy::prelude::{Component, Query, Vec2, With};

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

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AttentionIntent {
    /// Perception stays continuous; the rig quantizes only the final pose.
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

pub fn resolve_visual_pose(mut bats: Query<(&AnimationIntent, &mut VisualPose), With<Bat>>) {
    for (intent, mut pose) in &mut bats {
        pose.body = intent.effective_body();
        pose.face = intent.face;
        pose.attention = intent.attention;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pose_resolution_keeps_the_continuous_acting_channels() {
        use bevy::prelude::*;
        let mut app = App::new();
        app.add_systems(Update, resolve_visual_pose);
        let intent = AnimationIntent {
            attention: AttentionIntent {
                head_offset: Vec2::new(0.8, 0.2),
                ear_alertness: 0.7,
                body_compression: 0.01,
                ..default()
            },
            ..default()
        };
        let entity = app
            .world_mut()
            .spawn((Bat, intent, VisualPose::default()))
            .id();
        app.update();
        assert_eq!(
            app.world().get::<VisualPose>(entity).unwrap().attention,
            intent.attention
        );
    }

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
}
