use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers};

use crate::animation::{AnimationFrame, WingAnimation};
use crate::bat::{Bat, Position};
use crate::eyes::{EyeOffset, Eyes};
use crate::render::HANGING_IDLE_EYE_CENTER;

pub struct App {
    width: u16,
    height: u16,
    should_quit: bool,
    bat: Bat,
    wing_animation: WingAnimation,
    eyes: Eyes,
}

impl App {
    pub fn new(width: u16, height: u16) -> Self {
        Self {
            width,
            height,
            should_quit: false,
            bat: Bat::new(width / 2, HANGING_IDLE_EYE_CENTER.1 as u16),
            wing_animation: WingAnimation::new(),
            eyes: Eyes::new(),
        }
    }

    pub fn should_quit(&self) -> bool {
        self.should_quit
    }

    pub fn bat(&self) -> &Bat {
        &self.bat
    }

    pub fn terminal_size(&self) -> (u16, u16) {
        (self.width, self.height)
    }

    pub fn animation_frame(&self) -> AnimationFrame {
        self.wing_animation.current_animation_frame()
    }

    pub fn eye_target(&self) -> EyeOffset {
        self.eyes.target_offset()
    }

    pub fn eye_offset(&self) -> EyeOffset {
        self.eyes.current_offset()
    }

    pub fn update(&mut self) -> bool {
        let eyes_changed = self.eyes.update(&mut self.bat);
        let previous_frame = self.wing_animation.current_animation_frame();
        let wings_changed = self.wing_animation.update(&mut self.bat);
        let frame_changed = previous_frame != self.wing_animation.current_animation_frame();
        eyes_changed || wings_changed || frame_changed
    }

    pub fn handle_event(&mut self, event: Event) {
        match event {
            Event::Key(key)
                if key.kind == KeyEventKind::Press
                    && (matches!(key.code, KeyCode::Char('q' | 'Q'))
                        || (matches!(key.code, KeyCode::Char('c' | 'C'))
                            && key.modifiers.contains(KeyModifiers::CONTROL))) =>
            {
                self.should_quit = true;
            }
            Event::Mouse(mouse_event) => {
                self.bat.set_mouse_position(Position {
                    x: mouse_event.column,
                    y: mouse_event.row,
                });
                self.eyes.update(&mut self.bat);
            }
            Event::Resize(width, height) => {
                self.width = width;
                self.height = height;
                self.bat.set_position(Position {
                    x: width / 2,
                    y: HANGING_IDLE_EYE_CENTER.1 as u16,
                });
            }
            _ => {}
        }
    }
}
