//! ratatui front-end: terminal lifecycle and the main event loop.

pub mod app;
pub mod boss_mode;
pub mod keys;
pub mod matrix_rain;
pub mod ui;

use std::io::{self, Stdout};
use std::time::Duration;

use chrono::Utc;
use crossterm::event::{self, Event, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use crate::app::state::Shared;
use crate::cli::Args;
use crate::engine::{EngineCommand, EngineEvent, EngineHandle};
use crate::runtime::Runtime;

type Tui = Terminal<CrosstermBackend<Stdout>>;

fn setup() -> io::Result<Tui> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    Terminal::new(CrosstermBackend::new(stdout))
}

fn teardown(mut terminal: Tui) -> io::Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

/// Run the TUI until the user quits or the engine stops.
pub fn run(shared: Shared, handle: EngineHandle, mut runtime: Runtime, args: &Args) -> io::Result<()> {
    let mut terminal = setup()?;
    let mut app = app::TuiApp::default();
    let mut engine_alive = true;

    let result = (|| -> io::Result<()> {
        loop {
            // Fold any pending engine events into the shared state.
            while let Ok(ev) = handle.events.try_recv() {
                if matches!(ev, EngineEvent::Stopped) {
                    engine_alive = false;
                }
                runtime.apply(&shared, ev);
            }

            // Draw.
            {
                let st = shared.read();
                terminal.draw(|f| ui::draw(f, &st, &mut app))?;
            }

            // Input.
            if event::poll(Duration::from_millis(120))? {
                if let Event::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press
                        && app.handle_key(key, &shared, &handle, args)
                    {
                        break;
                    }
                }
            }

            // Periodic housekeeping: expire stale kill streaks.
            {
                let mut st = shared.write();
                st.streak.expire_if_idle(Utc::now());
                // little nod to Mr. Robot once you've been running an hour
                if !app.mr_robot_shown && st.stats.elapsed_secs() >= 3600 {
                    app.mr_robot_shown = true;
                    st.log(
                        crate::app::state::ActivityKind::Info,
                        crate::fun::easter_eggs::mr_robot_line(),
                    );
                }
            }

            if !engine_alive {
                break;
            }
        }
        Ok(())
    })();

    teardown(terminal)?;
    result?;

    // Stop the engine and export.
    let _ = handle.commands.send(EngineCommand::Stop);
    crate::ui::finalize(&shared, args);
    let _ = handle.join.join();
    Ok(())
}
