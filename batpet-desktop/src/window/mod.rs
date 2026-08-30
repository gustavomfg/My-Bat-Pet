use bevy::{
    log::info,
    prelude::{Query, Res, State, With},
    window::{
        CompositeAlphaMode, MonitorSelection, PrimaryWindow, Window, WindowLevel, WindowPlugin,
        WindowPosition, WindowResolution,
    },
};

use crate::{debug::DebugOptions, pet::BatState};

pub const WINDOW_WIDTH: u32 = 256;
pub const WINDOW_HEIGHT: u32 = 256;

pub fn primary_window() -> Window {
    Window {
        title: "BatPet Desktop".to_owned(),
        resolution: WindowResolution::new(WINDOW_WIDTH, WINDOW_HEIGHT)
            .with_scale_factor_override(1.0),
        transparent: true,
        decorations: false,
        resizable: false,
        // When the compositor knows the current monitor this centers the window;
        // during creation winit safely falls back to the window manager's default.
        position: WindowPosition::Centered(MonitorSelection::Current),
        window_level: WindowLevel::AlwaysOnTop,
        #[cfg(target_os = "linux")]
        composite_alpha_mode: CompositeAlphaMode::PreMultiplied,
        ..Default::default()
    }
}

pub fn log_startup(
    debug: Res<DebugOptions>,
    windows: Query<&Window, With<PrimaryWindow>>,
    state: Res<State<BatState>>,
) {
    if !debug.enabled {
        return;
    }

    let Some(window) = windows.iter().next() else {
        return;
    };

    info!("renderer iniciado: bevy2d window backend=winit");
    info!(
        "window compositor={} size={:.0}x{:.0} transparent={} decorations={} resizable={} always_on_top=requested",
        compositor_hint(),
        window.resolution.width(),
        window.resolution.height(),
        window.transparent,
        window.decorations,
        window.resizable,
    );
    info!("bat state={}", state.get().label());
}

fn compositor_hint() -> &'static str {
    if std::env::var_os("WAYLAND_DISPLAY").is_some() {
        "Wayland"
    } else if std::env::var_os("DISPLAY").is_some() {
        "X11"
    } else {
        "unknown"
    }
}

pub fn plugin_with_window(window: Window) -> WindowPlugin {
    WindowPlugin {
        primary_window: Some(window),
        ..Default::default()
    }
}
