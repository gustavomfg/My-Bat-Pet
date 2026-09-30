use bevy::prelude::Vec2;

/// Convert Bevy's top-left, logical window input coordinates into the
/// camera-local simulation space used by `Transform` and `FlightMotion`.
pub fn window_cursor_to_simulation(cursor: Vec2, logical_window_size: Vec2) -> Vec2 {
    let center = logical_window_size * 0.5;
    Vec2::new(cursor.x - center.x, center.y - cursor.y)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn window_center_maps_to_simulation_origin_and_flips_y() {
        let size = Vec2::splat(320.0);
        assert_eq!(
            window_cursor_to_simulation(Vec2::splat(160.0), size),
            Vec2::ZERO
        );
        assert_eq!(
            window_cursor_to_simulation(Vec2::new(200.0, 100.0), size),
            Vec2::new(40.0, 60.0)
        );
    }
}
