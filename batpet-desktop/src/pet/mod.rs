mod components;
mod state;

pub use components::{Bat, CursorState};
pub use state::BatState;

use bevy::{
    ecs::message::MessageReader,
    log::info,
    prelude::{Res, ResMut},
    window::{CursorLeft, CursorMoved},
};

use crate::debug::DebugOptions;

pub fn capture_cursor(
    mut moved_events: MessageReader<CursorMoved>,
    mut left_events: MessageReader<CursorLeft>,
    mut cursor: ResMut<CursorState>,
    debug: Res<DebugOptions>,
) {
    for event in moved_events.read() {
        if cursor.position == Some(event.position) {
            continue;
        }

        cursor.position = Some(event.position);
        if debug.enabled {
            info!(
                "cursor position=({:.0},{:.0})",
                event.position.x, event.position.y
            );
        }
    }

    for _ in left_events.read() {
        if cursor.position.take().is_some() && debug.enabled {
            info!("cursor position=outside");
        }
    }
}
