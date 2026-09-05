//! Opt-in, reproducible visual rehearsal through the actual Bevy renderer.
//! `--review-dir /tmp/batpet-review/final` writes 12 seconds at 12 fps.
use crate::pet::{BatState, CursorState};
use bevy::{
    prelude::*,
    render::view::screenshot::{Screenshot, save_to_disk},
};
use std::io::Write;

#[derive(Resource, Default)]
pub struct Review {
    directory: Option<String>,
    frame: u32,
    clicked: bool,
    presence: bool,
    os_cursor: bool,
    trace: Option<std::fs::File>,
    external_cursor: bool,
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
        let trace = directory.as_ref().map(|path| {
            let mut file = std::fs::File::create(std::path::Path::new(path).join("cursor.csv"))
                .expect("create cursor trace");
            writeln!(file, "frame,time,cursor_x,cursor_y,target_x,target_y")
                .expect("write trace header");
            file
        });
        Self {
            directory,
            trace,
            external_cursor: std::env::args().any(|arg| arg == "--review-external-cursor"),
            presence: std::env::args().any(|arg| arg == "--review-presence"),
            os_cursor: std::env::args().any(|arg| arg == "--review-os-cursor"),
            ..default()
        }
    }
}

pub fn drive(
    time: Res<Time>,
    mut review: ResMut<Review>,
    mut cursor: ResMut<CursorState>,
    mut state: ResMut<NextState<BatState>>,
    mut windows: Query<&mut Window>,
) {
    if review.directory.is_none() {
        return;
    }
    let t = time.elapsed_secs();
    if review.external_cursor {
        return;
    }
    if review.presence {
        let position = presence_cursor(t);
        if review.os_cursor {
            if let (Some(position), Some(mut window)) = (position, windows.iter_mut().next()) {
                window.set_cursor_position(Some(position));
            }
        } else {
            cursor.position = position;
        }
        return;
    }
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
    cursor: Res<CursorState>,
) {
    let Some(directory) = review.directory.clone() else {
        return;
    };
    let t = time.elapsed_secs();
    let end = if review.presence { 26.5 } else { 12.5 };
    if t > end + 0.5 {
        assert_eq!(
            *state.get(),
            BatState::HangingIdle,
            "click must return to rest"
        );
        exit.write(AppExit::Success);
        return;
    }
    if t < 0.5 || t > end {
        return;
    }
    let frame = ((t - 0.5) * 12.) as u32;
    if frame >= review.frame {
        review.frame = frame + 1;
        if let Some(trace) = review.trace.as_mut() {
            let p = cursor.position.unwrap_or(Vec2::splat(f32::NAN));
            let target = presence_cursor(t).unwrap_or(Vec2::splat(f32::NAN));
            writeln!(
                trace,
                "{frame},{t:.4},{:.2},{:.2},{:.2},{:.2}",
                p.x, p.y, target.x, target.y
            )
            .expect("write cursor trace");
        }
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(format!("{directory}/{frame:03}.png")));
    }
}

/// Approach, follow, stay still, withdraw, absence, then a brief return.
pub fn presence_cursor(t: f32) -> Option<Vec2> {
    let slide = |a: Vec2, b: Vec2, start: f32, duration: f32| {
        let p = ((t - start) / duration).clamp(0., 1.);
        a.lerp(b, p * p * (3. - 2. * p))
    };
    match t {
        t if (2.0..4.0).contains(&t) => {
            Some(slide(Vec2::new(312., 180.), Vec2::new(246., 180.), 2., 2.))
        }
        t if (4.0..6.0).contains(&t) => {
            Some(slide(Vec2::new(246., 180.), Vec2::new(74., 210.), 4., 2.))
        }
        t if (6.0..8.0).contains(&t) => {
            Some(slide(Vec2::new(74., 210.), Vec2::new(178., 168.), 6., 2.))
        }
        t if (8.0..13.0).contains(&t) => Some(Vec2::new(178., 168.)),
        t if (13.0..15.0).contains(&t) => {
            Some(slide(Vec2::new(178., 168.), Vec2::new(310., 220.), 13., 2.))
        }
        t if (22.0..23.0).contains(&t) => Some(Vec2::new(60., 185.)),
        _ => None,
    }
}
