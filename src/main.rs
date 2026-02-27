mod app;
mod input;
#[cfg(target_os = "macos")]
mod macos_helper;
mod theme;
mod ui;
mod widgets;

use std::io;
use std::time::Duration;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use crate::app::App;

fn main() -> io::Result<()> {
    // Terminal setup
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let app = App::new();
    let res = run_app(&mut terminal, app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(e) = res {
        eprintln!("Error: {e:?}");
    }

    Ok(())
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    mut app: App,
) -> io::Result<()> {
    loop {
        // Periodic refresh
        if app.last_update.elapsed() >= app.tick_rate {
            app.refresh();
        }

        terminal.draw(|f| ui::draw(f, &mut app))?;

        // Input handling
        if event::poll(Duration::from_millis(10))? {
            if input::handle_event(event::read()?, &mut app) {
                return Ok(());
            }
        }
    }
}
