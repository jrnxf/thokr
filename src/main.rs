mod lang;
mod thok;
mod ui;
mod util;

use crate::{lang::Language, thok::Thok};
use clap::{ArgEnum, ErrorKind, IntoApp, Parser};
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    tty::IsTty,
};
use std::{
    error::Error,
    io::{self, stdin},
    sync::mpsc,
    thread,
    time::Duration,
};
use tui::{
    backend::{Backend, CrosstermBackend},
    Frame, Terminal,
};
use webbrowser::Browser;

const TICK_RATE: u64 = 100;

/// a sleek typing tui written in rust
#[derive(Parser, Debug, Clone)]
#[clap(version, about, long_about= None)]
pub struct Cli {
    /// number of words to use in test
    #[clap(short = 'w', long, default_value_t = 15)]
    number_of_words: usize,

    /// number of seconds to run test
    #[clap(short = 's', long)]
    number_of_secs: Option<usize>,

    /// custom prompt to use
    #[clap(short = 'p', long)]
    prompt: Option<String>,

    /// language to pull words from
    #[clap(short = 'l', long, arg_enum, default_value_t = SupportedLanguage::English)]
    supported_language: SupportedLanguage,
}

#[derive(Debug, Copy, Clone, ArgEnum, strum_macros::Display)]
enum SupportedLanguage {
    English,
    English1k,
    English10k,
}

impl SupportedLanguage {
    fn as_lang(&self) -> Language {
        Language::new(self.to_string().to_lowercase())
    }
}

#[derive(Debug)]
struct App {
    cli: Option<Cli>,
    thok: Thok,
}

impl App {
    fn new(cli: Cli) -> Self {
        let prompt = if cli.prompt.is_some() {
            cli.prompt.clone().unwrap()
        } else {
            let language = cli.supported_language.as_lang();

            language.get_random(cli.number_of_words).join(" ")
        };

        Self {
            thok: Thok::new(prompt, cli.number_of_secs),
            cli: Some(cli),
        }
    }

    fn reset(&mut self, new_prompt: Option<String>) {
        let cli = self.cli.clone().unwrap();

        let prompt = match new_prompt {
            Some(_) => new_prompt.unwrap(),
            _ => {
                let language = cli.supported_language.as_lang();
                language.get_random(cli.number_of_words).join(" ")
            }
        };

        self.thok = Thok::new(prompt, cli.number_of_secs);
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();

    if !stdin().is_tty() {
        let mut cmd = Cli::command();
        cmd.error(ErrorKind::Io, "stdin must be a tty").exit();
    }

    enable_raw_mode()?;

    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(cli);
    start_tui(&mut terminal, &mut app)?;

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen,)?;
    terminal.show_cursor()?;

    Ok(())
}

enum ExitType {
    Restart,
    New,
    Quit,
}
fn start_tui<B: Backend>(
    terminal: &mut Terminal<B>,
    mut app: &mut App,
) -> Result<(), Box<dyn Error>> {
    let cli = app.cli.clone();

    let should_tick = cli.unwrap().number_of_secs.unwrap_or(0) > 0;

    let thok_events = get_thok_events(should_tick);

    loop {
        let mut exit_type: ExitType = ExitType::Quit;
        terminal.draw(|f| ui(app, f))?;

        loop {
            let app = &mut app;

            match thok_events.recv()? {
                ThokEvent::Tick => {
                    if app.thok.has_started() && !app.thok.has_finished() {
                        app.thok.on_tick();

                        if app.thok.has_finished() {
                            app.thok.calc_results();
                        }
                        terminal.draw(|f| ui(app, f))?;
                    }
                }
                ThokEvent::Resize => {
                    terminal.draw(|f| ui(app, f))?;
                }
                ThokEvent::Key(key) => {
                    match key.code {
                        KeyCode::Esc => {
                            break;
                        }
                        KeyCode::Backspace => {
                            if !app.thok.has_finished() {
                                app.thok.backspace();
                            }
                        }
                        KeyCode::Left => {
                            exit_type = ExitType::Restart;
                            break;
                        }
                        KeyCode::Right => {
                            exit_type = ExitType::New;
                            break;
                        }
                        KeyCode::Char(c) => match app.thok.has_finished() {
                            false => {
                                app.thok.write(c);
                                if app.thok.has_finished() {
                                    app.thok.calc_results();
                                }
                            }
                            true => match key.code {
                                KeyCode::Char('t') => {
                                    if Browser::is_available() {
                                        webbrowser::open(&format!("https://twitter.com/intent/tweet?text={}%20wpm%20%2F%20{}%25%20acc%20%2F%20{:.2}%20sd%0A%0Ahttps%3A%2F%2Fgithub.com%2Fcoloradocolby%2Fthokr", app.thok.wpm, app.thok.accuracy, app.thok.std_dev))
                                    .unwrap_or_default();
                                    }
                                }
                                KeyCode::Char('r') => {
                                    exit_type = ExitType::Restart;
                                    break;
                                }
                                KeyCode::Char('n') => {
                                    exit_type = ExitType::New;
                                    break;
                                }
                                _ => {}
                            },
                        },
                        _ => {}
                    }
                    terminal.draw(|f| ui(app, f))?;
                }
            }
        }

        match exit_type {
            ExitType::Restart => {
                app.reset(Some(app.thok.prompt.clone()));
            }
            ExitType::New => {
                app.reset(None);
            }
            ExitType::Quit => {
                break;
            }
        }
    }

    Ok(())
}

#[derive(Clone)]
enum ThokEvent {
    Key(KeyEvent),
    Resize,
    Tick,
}

fn get_thok_events(should_tick: bool) -> mpsc::Receiver<ThokEvent> {
    let (tx, rx) = mpsc::channel();

    if should_tick {
        let tick_x = tx.clone();
        thread::spawn(move || loop {
            if tick_x.send(ThokEvent::Tick).is_err() {
                break;
            }

            thread::sleep(Duration::from_millis(TICK_RATE))
        });
    }

    thread::spawn(move || loop {
        let evt = match event::read().unwrap() {
            Event::Key(key) => Some(ThokEvent::Key(key)),
            Event::Resize(_, _) => Some(ThokEvent::Resize),
            _ => None,
        };

        if evt.is_some() && tx.send(evt.unwrap()).is_err() {
            break;
        }
    });

    rx
}

fn ui<B: Backend>(app: &mut App, f: &mut Frame<B>) {
    f.render_widget(&app.thok, f.size());
}
