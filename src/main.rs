mod lang;
mod math;
mod session;

use crate::lang::Language;
use clap::Parser;
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use session::Session;
use std::{error::Error, io};
use tui::{
    backend::{Backend, CrosstermBackend},
    Frame, Terminal,
};

/// a typing tui written in rust
#[derive(Parser, Debug, Clone)]
#[clap(version, about, long_about= None)]
pub struct Args {
    /// Length of password
    #[clap(short = 'w', long, default_value_t = 20)]
    words: usize,

    /// Source to pull words from
    #[clap(short = 's', long, default_value_t = String::from("english"))]
    source: String,
}

#[derive(PartialEq, Debug, Clone)]
enum Screen {
    Prompt,
    Results,
}

#[derive(Debug, Clone)]
struct App {
    args: Option<Args>,
    session: Session,
    lang: Language,
    screen: Screen,
}

impl App {
    fn new(args: Args) -> Self {
        let l = Language::new(format!("src/lang/{}.json", args.source));
        Self {
            // session: Session::new(l.get_random(100).join(" ")[0..106].to_string()),
            session: Session::new(l.get_random(args.words).join(" ")),
            lang: l,
            screen: Screen::Prompt,
            args: Some(args),
        }
    }

    fn reset(self: &mut Self) {
        let a = self.args.clone().unwrap();
        let l = Language::new(format!("src/lang/{}.json", a.source));
        self.screen = Screen::Prompt;
        self.session = Session::new(l.get_random(a.words).join(" "));
        self.lang = l;
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(args);
    let result = run_app(&mut terminal, &mut app);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen,)?;
    terminal.show_cursor()?;

    if let Err(err) = result {
        println!("{:?}", err)
    }

    for x in app.session.logs {
        println!("{:?}", x);
    }

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
            app.session.draw_results(f).unwrap();
        }
    }
}
