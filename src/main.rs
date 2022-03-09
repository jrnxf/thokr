mod lang;
mod ui;

use crate::lang::Language;
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::{error::Error, io};
use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    Frame, Terminal,
};
use ui::prompt::Prompt;

struct App {
    prompt: Prompt,
}

impl App {
    fn new(prompt_str: String) -> Self {
        Self {
            prompt: Prompt::new(prompt_str),
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let l = Language::new("src/lang/english.json");

    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // create app and run it
    let app = App::new(l.get_random(10).join(" "));
    let result = run_app(&mut terminal, app);

    // restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen,)?;
    terminal.show_cursor()?;

    if let Err(err) = result {
        println!("{:?}", err)
    }

    Ok(())
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut app: App) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui(f, &mut app))?;

        let p = &mut app.prompt;

        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Esc => {
                    return Ok(());
                }
                // KeyCode::Left => {
                //     p.decrement_cursor();
                // }

                // KeyCode::Right => {
                //     p.increment_cursor();
                // }
                KeyCode::Backspace => {
                    p.backspace();
                }
                // KeyCode::Home => {
                //     p.go_to_start();
                // }
                // KeyCode::End => {
                //     p.go_to_end();
                // }
                KeyCode::Char(c) => {
                    p.write(c);
                }
                _ => {}
            }
        }
    }
}

fn ui<B: Backend>(f: &mut Frame<B>, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(10)
        .constraints(
            [
                Constraint::Percentage(50),
                Constraint::Min(5),
                Constraint::Percentage(50),
            ]
            .as_ref(),
        )
        .split(f.size());

    let p = app.prompt.clone();
    if p.input.len() == p.prompt.len() {
        p.draw_results(f, chunks[1]).unwrap();
    } else {
        p.draw_prompt(f, chunks[1]).unwrap();
    }
}
