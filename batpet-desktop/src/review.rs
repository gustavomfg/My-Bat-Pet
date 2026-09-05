//! Opt-in, reproducible visual rehearsal through the actual Bevy renderer.
//! `--review-dir /tmp/batpet-review/final` writes 12 seconds at 12 fps.
use crate::pet::{BatState, CursorState};
use bevy::{
    prelude::*,
    render::view::screenshot::{Screenshot, save_to_disk},
};

#[derive(Resource, Default)]
pub struct Review {
    directory: Option<String>,
    frame: u32,
    clicked: bool,
}
impl Review {
    pub fn from_args() -> Self {
        let mut args = std::env::args();
        let directory = args
            .find(|arg| arg == "--review-dir")
            .and_then(|_| args.next());
        if let Some(path) = &directory {
            std::fs::create_dir_all(path).expect("create review directory");
        }
        Self {
            directory,
            ..default()
        }
    }
}

pub fn drive(
    time: Res<Time>,
    mut review: ResMut<Review>,
    mut cursor: ResMut<CursorState>,
    mut state: ResMut<NextState<BatState>>,
) {
    if review.directory.is_none() {
        return;
    }
    let t = time.elapsed_secs();
    cursor.position = match t {
        t if (2.0..3.5).contains(&t) => Some(Vec2::new(40., 180.)),
        t if (3.5..5.0).contains(&t) => Some(Vec2::new(280., 180.)),
        t if (5.0..6.0).contains(&t) => Some(Vec2::new(166., 178.)),
        _ => None,
    };
    if t >= 6.5 && !review.clicked {
        state.set(BatState::Reacting);
        review.clicked = true;
    }
}

pub fn capture(
    time: Res<Time>,
    mut review: ResMut<Review>,
    mut commands: Commands,
    mut exit: MessageWriter<AppExit>,
    state: Res<State<BatState>>,
) {
    let Some(directory) = review.directory.clone() else {
        return;
    };
    let t = time.elapsed_secs();
    if t > 13. {
        assert_eq!(
            *state.get(),
            BatState::HangingIdle,
            "click must return to rest"
        );
        exit.write(AppExit::Success);
        return;
    }
    if t < 0.5 || t > 12.5 {
        return;
    }
    let frame = ((t - 0.5) * 12.) as u32;
    if frame >= review.frame {
        review.frame = frame + 1;
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(format!("{directory}/{frame:03}.png")));
    }
}
