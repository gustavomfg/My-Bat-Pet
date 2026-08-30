use crate::bat::BatState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BatPixel {
    Transparent,
    Body,
    Wing,
    InnerEar,
    Eye,
    Pupil,
    Fang,
    Claw,
}

impl BatPixel {
    pub const fn is_transparent(self) -> bool {
        matches!(self, Self::Transparent)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PixelArt<const WIDTH: usize, const HEIGHT: usize> {
    pixels: [[BatPixel; WIDTH]; HEIGHT],
}

impl<const WIDTH: usize, const HEIGHT: usize> PixelArt<WIDTH, HEIGHT> {
    pub const fn new(pixels: [[BatPixel; WIDTH]; HEIGHT]) -> Self {
        Self { pixels }
    }

    pub const fn width(&self) -> usize {
        WIDTH
    }

    pub const fn height(&self) -> usize {
        HEIGHT
    }

    pub const fn terminal_height(&self) -> usize {
        HEIGHT.div_ceil(2)
    }

    pub fn pixel(&self, x: usize, y: usize) -> Option<BatPixel> {
        self.pixels.get(y).and_then(|row| row.get(x)).copied()
    }

    pub fn is_symmetric(&self) -> bool {
        self.pixels.iter().all(|row| {
            row.iter()
                .zip(row.iter().rev())
                .all(|(left, right)| left == right)
        })
    }
}

pub const BAT_WIDTH: usize = 20;
pub const BAT_HEIGHT: usize = 14;

// The eye center is also the model anchor used by the mouse direction logic.
pub const HANGING_IDLE_EYE_CENTER: (usize, usize) = (10, 9);

pub type BatPixelArt = PixelArt<BAT_WIDTH, BAT_HEIGHT>;

const T: BatPixel = BatPixel::Transparent;
const B: BatPixel = BatPixel::Body;
const W: BatPixel = BatPixel::Wing;
const I: BatPixel = BatPixel::InnerEar;
const E: BatPixel = BatPixel::Eye;
const F: BatPixel = BatPixel::Fang;
const C: BatPixel = BatPixel::Claw;

pub const HANGING_IDLE: BatPixelArt = PixelArt::new([
    // Ceiling claws.
    [T, T, T, T, T, T, T, C, T, T, T, T, C, T, T, T, T, T, T, T],
    [T, T, T, T, T, T, C, C, T, T, T, T, C, C, T, T, T, T, T, T],
    // Closed wings and the body hanging below the claws.
    [T, T, T, T, T, W, W, B, B, B, B, B, B, W, W, T, T, T, T, T],
    [T, T, T, T, W, W, W, B, B, B, B, B, B, W, W, W, T, T, T, T],
    [T, T, T, W, W, W, B, B, B, B, B, B, B, B, W, W, W, T, T, T],
    [T, T, W, W, W, W, B, B, B, B, B, B, B, B, W, W, W, W, T, T],
    [T, W, W, W, W, B, B, B, B, B, B, B, B, B, B, W, W, W, W, T],
    [T, W, W, W, B, B, B, B, B, B, B, B, B, B, B, B, W, W, W, T],
    [T, T, W, B, B, E, E, E, B, B, B, B, E, E, E, B, B, W, T, T],
    [T, T, W, B, B, E, E, E, B, B, B, B, E, E, E, B, B, W, T, T],
    [T, T, T, W, B, E, E, E, B, B, B, B, E, E, E, B, W, T, T, T],
    [T, T, T, T, W, B, B, B, F, B, B, F, B, B, B, W, T, T, T, T],
    [T, T, T, T, I, I, W, B, B, B, B, B, B, W, I, I, T, T, T, T],
    [T, T, T, T, T, I, I, T, B, B, B, B, T, I, I, T, T, T, T, T],
]);

pub fn pixel_art_for_state(state: BatState) -> Option<&'static BatPixelArt> {
    match state {
        BatState::HangingIdle => Some(&HANGING_IDLE),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hanging_idle_has_consistent_dimensions() {
        assert_eq!(HANGING_IDLE.width(), 20);
        assert_eq!(HANGING_IDLE.height(), 14);
        assert_eq!(HANGING_IDLE.terminal_height(), 7);
    }

    #[test]
    fn pixel_lookup_rejects_coordinates_outside_the_art() {
        assert_eq!(HANGING_IDLE.pixel(BAT_WIDTH, 0), None);
        assert_eq!(HANGING_IDLE.pixel(0, BAT_HEIGHT), None);
        assert_eq!(HANGING_IDLE.pixel(BAT_WIDTH + 1, BAT_HEIGHT + 1), None);
    }

    #[test]
    fn hanging_idle_is_symmetric() {
        assert!(HANGING_IDLE.is_symmetric());
    }

    #[test]
    fn only_hanging_idle_has_a_pixel_art_definition_for_now() {
        assert_eq!(
            pixel_art_for_state(BatState::HangingIdle),
            Some(&HANGING_IDLE)
        );
        assert_eq!(pixel_art_for_state(BatState::Flying), None);
    }
}
