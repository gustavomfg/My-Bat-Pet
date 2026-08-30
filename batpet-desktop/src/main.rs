mod debug;
mod pet;
mod rendering;
mod window;

use bevy::{
    asset::AssetPlugin,
    log::{Level, LogPlugin},
    prelude::*,
};

use debug::DebugOptions;
use pet::{BatState, CursorState};

fn main() {
    let debug = DebugOptions::from_args();
    let log_filter = if debug.enabled {
        "error,batpet_desktop=info"
    } else {
        "error"
    };

    App::new()
        .insert_resource(debug)
        .insert_resource(ClearColor(Color::NONE))
        .add_plugins(
            DefaultPlugins
                .set(AssetPlugin {
                    file_path: format!("{}/../assets", env!("CARGO_MANIFEST_DIR")),
                    ..Default::default()
                })
                .set(LogPlugin {
                    filter: log_filter.to_owned(),
                    level: Level::INFO,
                    ..Default::default()
                })
                .set(ImagePlugin::default_nearest())
                .set(window::plugin_with_window(window::primary_window())),
        )
        .init_state::<BatState>()
        .init_resource::<CursorState>()
        .add_systems(Startup, (rendering::setup, window::log_startup))
        .add_systems(Update, pet::capture_cursor)
        .run();
}
