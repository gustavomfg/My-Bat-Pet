use std::{
    fmt,
    io::{self, Write},
};

use crate::{
    animation::WING_FRAME_DURATION,
    bat::{Bat, BatState, EyeDirection, Position},
    eyes::EyeOffset,
};

mod pixel;
mod pixel_art;

pub use crate::animation::AnimationFrame;
pub use pixel::PixelRenderer;
pub use pixel_art::{
    BAT_HEIGHT, BAT_WIDTH, BatPixel, BatPixelArt, HANGING_IDLE, HANGING_IDLE_EYE_CENTER, PixelArt,
};

pub type TerminalSize = (u16, u16);

pub struct RenderDebugInfo {
    pub renderer: &'static str,
    pub state: BatState,
    pub frame: AnimationFrame,
    pub bat_position: Position,
    pub mouse_position: Option<Position>,
    pub eye_direction: EyeDirection,
    pub sprite_size: (usize, usize),
    pub eye_target: EyeOffset,
    pub eye_offset: EyeOffset,
}

impl fmt::Display for RenderDebugInfo {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (width, height) = self.sprite_size;
        write!(
            formatter,
            "renderer={} state={} frame={} bat_position=({},{}) mouse={} eye_direction={} sprite_size={}x{} interval={}ms eye_target=({:.1},{:.1}) eye_offset=({:.1},{:.1})",
            self.renderer,
            self.state.label(),
            self.frame.label(),
            self.bat_position.x,
            self.bat_position.y,
            self.mouse_label(),
            self.eye_direction.label(),
            width,
            height,
            WING_FRAME_DURATION.as_millis(),
            self.eye_target.x,
            self.eye_target.y,
            self.eye_offset.x,
            self.eye_offset.y,
        )
    }
}

impl RenderDebugInfo {
    pub fn status_line(&self, width: usize) -> String {
        if width == 0 {
            return String::new();
        }

        let full = self.to_string();
        if full.chars().count() <= width {
            return full;
        }

        let compact = format!(
            "renderer={} state={} frame={} bat_position=({},{}) mouse={} eye_direction={} sprite_size={}x{}",
            self.renderer,
            self.state.label(),
            self.frame.label(),
            self.bat_position.x,
            self.bat_position.y,
            self.mouse_label(),
            self.eye_direction.label(),
            self.sprite_size.0,
            self.sprite_size.1,
        );
        compact.chars().take(width).collect()
    }

    fn mouse_label(&self) -> String {
        self.mouse_position.map_or_else(
            || "(-,-)".to_owned(),
            |position| format!("({},{})", position.x, position.y),
        )
    }
}

pub trait Renderer {
    fn render(
        &mut self,
        writer: &mut dyn Write,
        bat: &Bat,
        eye_offset: EyeOffset,
    ) -> io::Result<()>;

    fn update_eyes(
        &mut self,
        writer: &mut dyn Write,
        bat: &Bat,
        eye_offset: EyeOffset,
    ) -> io::Result<()>;

    fn resize(&mut self, writer: &mut dyn Write, size: TerminalSize) -> io::Result<()>;

    fn clear(&mut self, writer: &mut dyn Write) -> io::Result<()>;

    fn debug_info(
        &self,
        bat: &Bat,
        frame: AnimationFrame,
        eye_target: EyeOffset,
        eye_offset: EyeOffset,
    ) -> RenderDebugInfo;
}

pub fn create(size: TerminalSize) -> io::Result<Box<dyn Renderer>> {
    Ok(Box::new(PixelRenderer::new(size)))
}
