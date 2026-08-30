mod animation;
mod app;
pub mod bat;
mod eyes;
pub mod render;
mod terminal;

use std::{io, time::Duration};

use crossterm::event;

use app::App;
use terminal::Terminal;

fn main() {
    if let Err(error) = run() {
        eprintln!("batpet: {error}");
        std::process::exit(1);
    }
}

fn run() -> io::Result<()> {
    let debug_render = std::env::args().any(|argument| argument == "--debug-render");
    let mut terminal = Terminal::enter()?;
    let result = run_app(&mut terminal, debug_render);
    let cleanup_result = terminal.restore();

    match result {
        Err(error) => Err(error),
        Ok(()) => cleanup_result,
    }
}

fn run_app(terminal: &mut Terminal, debug_render: bool) -> io::Result<()> {
    let (width, height) = terminal.size()?;
    let mut app = App::new(width, height);

    terminal.draw(&app)?;
    if debug_render {
        terminal.debug_render(&app)?;
    }

    while !app.should_quit() {
        if app.update() {
            terminal.draw(&app)?;
            if debug_render {
                terminal.debug_render(&app)?;
            }
        }

        if event::poll(Duration::from_millis(16))? {
            let event = event::read()?;
            let is_mouse_event = matches!(&event, event::Event::Mouse(_));
            let is_resize_event = matches!(&event, event::Event::Resize(_, _));
            app.handle_event(event);

            if !app.should_quit() {
                if is_resize_event {
                    terminal.resize(app.terminal_size())?;
                    terminal.draw(&app)?;
                    if debug_render {
                        terminal.debug_render(&app)?;
                    }
                } else if is_mouse_event {
                    terminal.update_eyes(&app)?;
                    if debug_render {
                        terminal.debug_render(&app)?;
                    }
                }
            }
        }
    }

    Ok(())
}
