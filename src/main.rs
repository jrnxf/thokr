mod lang;
mod session;

use crate::lang::Language;
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use itertools::Itertools;
use session::Session;
use std::{env, error::Error, io};
use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    Frame, Terminal,
};
use unicode_width::UnicodeWidthStr;

#[derive(PartialEq, Debug, Clone)]
enum Screen {
    Prompt,
    Results,
}

#[derive(Debug, Clone)]
struct App {
    session: Session,
    lang: Language,
    screen: Screen,
}

impl App {
    fn new() -> Self {
        let l = Language::new("src/lang/english.json");
        let args: Vec<String> = env::args().collect();
        Self {
            // session: Session::new(l.get_random(100).join(" ")[0..106].to_string()),
            session: Session::new(l.get_random(args[1].parse().unwrap()).join(" ")),
            lang: l,
            screen: Screen::Prompt,
        }
    }

    fn reset(self: &mut Self) {
        let l = Language::new("src/lang/english.json");
        self.screen = Screen::Prompt;
        self.session = Session::new(l.get_random(15).join(" "));
        self.lang = l;
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();
    let result = run_app(&mut terminal, &mut app);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen,)?;
    terminal.show_cursor()?;

    if let Err(err) = result {
        println!("{:?}", err)
    }

    println!("{:?}", app.session.logs);

    Ok(())
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut app: &mut App) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui(f, &mut app))?;

        let a = &mut app;

        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Esc => {
                    return Ok(());
                }
                KeyCode::Backspace => {
                    if a.screen == Screen::Prompt {
                        a.session.backspace();
                    }
                }
                KeyCode::Char(c) => match a.screen {
                    Screen::Prompt => {
                        a.session.write(c);
                        if a.session.is_finished() {
                            app.session.calc_results();
                            app.screen = Screen::Results;
                        }
                    }
                    Screen::Results => {
                        if let KeyCode::Char('r') = key.code {
                            a.session.prompt = a.lang.get_random(10).join(" ");
                            a.reset()
                        }
                    }
                },
                _ => {}
            }
        }
    }
}

fn ui<B: Backend>(f: &mut Frame<B>, app: &mut App) {
    match app.screen {
        Screen::Prompt => {
            app.session.draw_prompt(f).unwrap();
        }
        Screen::Results => {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(4)
                .constraints(
                    [
                        // Constraint::Percentage(50),
                        Constraint::Min(15),
                        Constraint::Min(1),
                        // Constraint::Percentage(50),
                    ]
                    .as_ref(),
                )
                .split(f.size());

            app.session.draw_results(f, chunks).unwrap();
        }
    }
}
