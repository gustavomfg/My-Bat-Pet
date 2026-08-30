use std::io::{self, Stderr, Stdout, Write, stderr, stdout};

use crossterm::{
    cursor::{Hide, Show},
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{
        EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode, size,
    },
};

use crate::app::App;
use crate::render::{self, RenderDebugInfo, Renderer};

pub struct Terminal {
    stdout: Stdout,
    size: (u16, u16),
    active: bool,
    renderer: Box<dyn Renderer>,
}

impl Terminal {
    pub fn enter() -> io::Result<Self> {
        let initial_size = size()?;
        let renderer = render::create(initial_size)?;
        enable_raw_mode()?;

        let mut terminal = Self {
            stdout: stdout(),
            size: initial_size,
            active: true,
            renderer,
        };

        let setup_result = execute!(
            terminal.stdout,
            EnterAlternateScreen,
            EnableMouseCapture,
            Hide
        );

        if let Err(error) = setup_result {
            let _ = terminal.restore();
            return Err(error);
        }

        Ok(terminal)
    }

    pub fn size(&self) -> io::Result<(u16, u16)> {
        size()
    }

    pub fn draw(&mut self, app: &App) -> io::Result<()> {
        self.renderer
            .render(&mut self.stdout, app.bat(), app.eye_offset())?;
        self.stdout.flush()
    }

    pub fn update_eyes(&mut self, app: &App) -> io::Result<()> {
        self.renderer
            .update_eyes(&mut self.stdout, app.bat(), app.eye_offset())?;
        self.stdout.flush()
    }

    pub fn resize(&mut self, size: (u16, u16)) -> io::Result<()> {
        self.renderer.resize(&mut self.stdout, size)?;
        self.size = size;
        self.stdout.flush()
    }

    pub fn debug_render(&self, app: &App) -> io::Result<()> {
        let info = self.renderer.debug_info(
            app.bat(),
            app.animation_frame(),
            app.eye_target(),
            app.eye_offset(),
        );
        let mut stderr = stderr();
        write_debug_line(&mut stderr, self.size, &info)?;
        stderr.flush()
    }

    pub fn restore(&mut self) -> io::Result<()> {
        if !self.active {
            return Ok(());
        }

        let mut first_error = None;

        if let Err(error) = self.renderer.clear(&mut self.stdout) {
            first_error = Some(error);
        }

        if let Err(error) = execute!(self.stdout, DisableMouseCapture) {
            if first_error.is_none() {
                first_error = Some(error);
            }
        }

        if let Err(error) = execute!(self.stdout, LeaveAlternateScreen) {
            if first_error.is_none() {
                first_error = Some(error);
            }
        }

        if let Err(error) = execute!(self.stdout, Show) {
            if first_error.is_none() {
                first_error = Some(error);
            }
        }

        if let Err(error) = disable_raw_mode() {
            if first_error.is_none() {
                first_error = Some(error);
            }
        }

        if let Err(error) = self.stdout.flush() {
            if first_error.is_none() {
                first_error = Some(error);
            }
        }

        self.active = false;
        first_error.map_or(Ok(()), Err)
    }
}

fn write_debug_line(
    stderr: &mut Stderr,
    size: (u16, u16),
    info: &RenderDebugInfo,
) -> io::Result<()> {
    let width = size.0 as usize;
    let status = info.status_line(width);
    let row = size.1.max(1);

    write!(stderr, "\x1b7\x1b[{};1H\x1b[2K{}\x1b8", row, status)
}

impl Drop for Terminal {
    fn drop(&mut self) {
        let _ = self.restore();
    }
}
