use std::io::{self, Write};

use crossterm::{
    QueueableCommand,
    cursor::MoveTo,
    style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor},
};

use crate::{
    bat::{Bat, BatState, Position},
    eyes::{
        EYE_BASE_SIZE, EyeOffset, LEFT_EYE_BASE, MAX_PUPIL_OFFSET_X, MAX_PUPIL_OFFSET_Y,
        RIGHT_EYE_BASE,
    },
};

use super::pixel_art::{BatPixel, BatPixelArt, HANGING_IDLE, pixel_art_for_state};
use super::{AnimationFrame, RenderDebugInfo, Renderer, TerminalSize};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DrawState {
    position: Position,
    state: BatState,
    pupil_offset: (i32, i32),
}

pub struct PixelRenderer {
    size: TerminalSize,
    previous: Option<DrawState>,
}

impl PixelRenderer {
    pub fn new(size: TerminalSize) -> Self {
        Self {
            size,
            previous: None,
        }
    }

    pub fn centered_position(size: TerminalSize) -> Position {
        let sprite_width = HANGING_IDLE.width() as u16;
        Position {
            x: size.0.saturating_sub(sprite_width) / 2,
            y: 0,
        }
    }

    fn position(&self) -> Position {
        Self::centered_position(self.size)
    }

    fn drawable_height(&self) -> u16 {
        // Reserve the last row for the in-place debug status line.
        self.size.1.saturating_sub(1)
    }

    fn render_if_needed(
        &mut self,
        writer: &mut dyn Write,
        bat: &Bat,
        eye_offset: EyeOffset,
    ) -> io::Result<()> {
        let next = DrawState {
            position: self.position(),
            state: bat.state,
            pupil_offset: quantized_eye_offset(eye_offset),
        };

        if self.previous == Some(next) {
            return Ok(());
        }

        self.clear_previous(writer)?;

        if let Some(art) = pixel_art_for_state(next.state) {
            self.draw_art(writer, art, next.position, next.pupil_offset)?;
        }

        self.previous = Some(next);
        Ok(())
    }

    fn draw_art(
        &self,
        writer: &mut dyn Write,
        art: &BatPixelArt,
        position: Position,
        pupil_offset: (i32, i32),
    ) -> io::Result<()> {
        let drawable_height = self.drawable_height();

        for cell_y in 0..art.terminal_height() {
            let terminal_y = position.y.saturating_add(cell_y as u16);
            if terminal_y >= drawable_height {
                break;
            }

            for cell_x in 0..art.width() {
                let terminal_x = position.x.saturating_add(cell_x as u16);
                if terminal_x >= self.size.0 {
                    continue;
                }

                let top = composed_pixel(art, cell_x, cell_y * 2, pupil_offset);
                let bottom = composed_pixel(art, cell_x, cell_y * 2 + 1, pupil_offset);
                draw_cell(writer, terminal_x, terminal_y, top, bottom)?;
            }
        }

        writer.queue(ResetColor).map(|_| ())
    }

    fn clear_previous(&self, writer: &mut dyn Write) -> io::Result<()> {
        let Some(previous) = self.previous else {
            return Ok(());
        };

        let drawable_height = self.drawable_height();
        for cell_y in 0..HANGING_IDLE.terminal_height() {
            let terminal_y = previous.position.y.saturating_add(cell_y as u16);
            if terminal_y >= drawable_height {
                break;
            }

            if previous.position.x >= self.size.0 {
                continue;
            }

            let width = self.size.0 - previous.position.x;
            let width = width.min(HANGING_IDLE.width() as u16) as usize;
            writer.queue(MoveTo(previous.position.x, terminal_y))?;
            writer.queue(ResetColor)?;
            writer.queue(Print(" ".repeat(width)))?;
        }

        Ok(())
    }
}

impl Renderer for PixelRenderer {
    fn render(
        &mut self,
        writer: &mut dyn Write,
        bat: &Bat,
        eye_offset: EyeOffset,
    ) -> io::Result<()> {
        self.render_if_needed(writer, bat, eye_offset)
    }

    fn update_eyes(
        &mut self,
        writer: &mut dyn Write,
        bat: &Bat,
        eye_offset: EyeOffset,
    ) -> io::Result<()> {
        self.render_if_needed(writer, bat, eye_offset)
    }

    fn resize(&mut self, writer: &mut dyn Write, size: TerminalSize) -> io::Result<()> {
        self.clear_previous(writer)?;
        self.size = size;
        self.previous = None;
        Ok(())
    }

    fn clear(&mut self, writer: &mut dyn Write) -> io::Result<()> {
        self.clear_previous(writer)?;
        self.previous = None;
        writer.queue(ResetColor).map(|_| ())
    }

    fn debug_info(
        &self,
        bat: &Bat,
        frame: AnimationFrame,
        eye_target: EyeOffset,
        eye_offset: EyeOffset,
    ) -> RenderDebugInfo {
        RenderDebugInfo {
            renderer: "pixel",
            state: bat.state,
            frame,
            bat_position: self.position(),
            mouse_position: bat.mouse_position,
            eye_direction: bat.eye_direction,
            sprite_size: (HANGING_IDLE.width(), HANGING_IDLE.height()),
            eye_target,
            eye_offset,
        }
    }
}

fn quantized_eye_offset(offset: EyeOffset) -> (i32, i32) {
    let (x, y) = offset.rounded();
    (
        x.clamp(-MAX_PUPIL_OFFSET_X, MAX_PUPIL_OFFSET_X),
        y.clamp(-MAX_PUPIL_OFFSET_Y, MAX_PUPIL_OFFSET_Y),
    )
}

fn composed_pixel(art: &BatPixelArt, x: usize, y: usize, pupil_offset: (i32, i32)) -> BatPixel {
    let base = art.pixel(x, y).unwrap_or(BatPixel::Transparent);
    if base != BatPixel::Eye {
        return base;
    }

    let left_pupil = pupil_position(LEFT_EYE_BASE, pupil_offset);
    let right_pupil = pupil_position(RIGHT_EYE_BASE, pupil_offset);
    if (x, y) == left_pupil || (x, y) == right_pupil {
        BatPixel::Pupil
    } else {
        base
    }
}

fn pupil_position(eye_base: (usize, usize), offset: (i32, i32)) -> (usize, usize) {
    let max_offset = EYE_BASE_SIZE.saturating_sub(1) as i32;
    let center = (EYE_BASE_SIZE / 2) as i32;
    let x = (center + offset.0).clamp(0, max_offset) as usize;
    let y = (center + offset.1).clamp(0, max_offset) as usize;
    (eye_base.0 + x, eye_base.1 + y)
}

fn draw_cell(
    writer: &mut dyn Write,
    x: u16,
    y: u16,
    top: BatPixel,
    bottom: BatPixel,
) -> io::Result<()> {
    let top_color = color_for(top);
    let bottom_color = color_for(bottom);

    writer.queue(MoveTo(x, y))?;
    match (top_color, bottom_color) {
        (None, None) => {
            writer.queue(ResetColor)?;
            writer.queue(Print(' '))?;
        }
        (Some(top), None) => {
            writer.queue(SetForegroundColor(top))?;
            writer.queue(SetBackgroundColor(Color::Reset))?;
            writer.queue(Print('▀'))?;
        }
        (None, Some(bottom)) => {
            writer.queue(SetForegroundColor(bottom))?;
            writer.queue(SetBackgroundColor(Color::Reset))?;
            writer.queue(Print('▄'))?;
        }
        (Some(top), Some(bottom)) => {
            writer.queue(SetForegroundColor(top))?;
            writer.queue(SetBackgroundColor(bottom))?;
            writer.queue(Print('▀'))?;
        }
    }

    Ok(())
}

fn color_for(pixel: BatPixel) -> Option<Color> {
    match pixel {
        BatPixel::Transparent => None,
        BatPixel::Body => Some(Color::Rgb {
            r: 61,
            g: 35,
            b: 94,
        }),
        BatPixel::Wing => Some(Color::Rgb {
            r: 108,
            g: 57,
            b: 151,
        }),
        BatPixel::InnerEar => Some(Color::Rgb {
            r: 193,
            g: 126,
            b: 221,
        }),
        BatPixel::Eye => Some(Color::Rgb {
            r: 255,
            g: 218,
            b: 83,
        }),
        BatPixel::Pupil => Some(Color::Rgb {
            r: 25,
            g: 16,
            b: 34,
        }),
        BatPixel::Fang => Some(Color::Rgb {
            r: 248,
            g: 239,
            b: 255,
        }),
        BatPixel::Claw => Some(Color::Rgb {
            r: 224,
            g: 185,
            b: 240,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bat::WingFrame;

    #[test]
    fn centers_the_sprite_at_the_top_of_the_terminal() {
        assert_eq!(
            PixelRenderer::centered_position((80, 24)),
            Position { x: 30, y: 0 }
        );
        assert_eq!(
            PixelRenderer::centered_position((19, 24)),
            Position { x: 0, y: 0 }
        );
    }

    #[test]
    fn resize_recalculates_the_centered_position() {
        let mut renderer = PixelRenderer::new((80, 24));
        let mut output = Vec::new();
        let bat = Bat::new(40, 9);

        renderer
            .render(&mut output, &bat, EyeOffset::CENTER)
            .unwrap();
        assert_eq!(
            renderer.previous.unwrap().position,
            Position { x: 30, y: 0 }
        );

        renderer.resize(&mut output, (100, 30)).unwrap();
        renderer
            .render(&mut output, &bat, EyeOffset::CENTER)
            .unwrap();
        assert_eq!(
            renderer.previous.unwrap().position,
            Position { x: 40, y: 0 }
        );
    }

    #[test]
    fn skips_writes_when_only_the_background_animation_tick_changes() {
        let mut renderer = PixelRenderer::new((80, 24));
        let mut output = Vec::new();
        let mut bat = Bat::new(40, 9);

        renderer
            .render(&mut output, &bat, EyeOffset::CENTER)
            .unwrap();
        output.clear();

        bat.wing_frame = WingFrame::Closed;
        renderer
            .render(&mut output, &bat, EyeOffset::CENTER)
            .unwrap();

        assert!(output.is_empty());
    }

    #[test]
    fn pupil_offset_is_clamped_inside_each_eye() {
        assert_eq!(pupil_position(LEFT_EYE_BASE, (-9, 9)), (5, 10));
        assert_eq!(pupil_position(RIGHT_EYE_BASE, (9, -9)), (14, 8));
    }
}
