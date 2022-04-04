mod lang;
mod thok;
mod util;

use crate::{lang::Language, thok::Thok};
use clap::{ArgEnum, Parser};
use log::info;
use std::{error::Error, io, sync::mpsc, thread, time::Duration};
use termion::{
    event::Key,
    input::{MouseTerminal, TermRead},
    raw::IntoRawMode,
    screen::AlternateScreen,
};
use tui::{
    backend::{Backend, TermionBackend},
    Frame, Terminal,
};

/// a typing tui written in rust
#[derive(Parser, Debug, Clone)]
#[clap(version, about, long_about= None)]
pub struct Args {
    /// Length of password
    #[clap(short = 'w', long, default_value_t = 15)]
    number_of_words: usize,

    /// Path of file to use
    #[clap(short = 's', long)]
    number_of_secs: Option<usize>,

    /// Path of file to use
    #[clap(short = 'f', long)]
    file: Option<String>,

    /// Language to pull words from
    #[clap(short = 'l', long, arg_enum, default_value_t = SupportedLanguage::English)]
    supported_language: SupportedLanguage,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ArgEnum, strum_macros::Display)]
enum SupportedLanguage {
    English,
    English1k,
    English10k,
    Spanish,
}

impl SupportedLanguage {
    fn as_lang(&self) -> Language {
        Language::new(self.to_string().to_lowercase())
    }
}

#[derive(PartialEq, Debug, Clone)]
enum Screen {
    Prompt,
    Results,
}

#[derive(Debug, Clone)]
struct App {
    args: Option<Args>,
    thok: Thok,
    screen: Screen,
}

impl App {
    fn new(args: Args) -> Self {
        let prompt;

        let language = args.supported_language.as_lang();

        info!("Language selected:  {:?}", language);

        prompt = language.get_random(args.number_of_words).join(" ");

        Self {
            thok: Thok::new(prompt, args.number_of_secs),
            screen: Screen::Prompt,
            args: Some(args),
        }
    }

    fn reset(self: &mut Self, new_prompt: Option<String>) {
        let prompt;
        let args = self.args.clone().unwrap();
        match new_prompt {
            Some(_) => {
                prompt = new_prompt.unwrap();
            }
            _ => {
                let language = args.supported_language.as_lang();
                prompt = language.get_random(args.number_of_words).join(" ");
            }
        }

        self.thok = Thok::new(prompt, args.number_of_secs);
        self.screen = Screen::Prompt;
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    simple_logging::log_to_file("out.log", log::LevelFilter::Info).unwrap();
    // check for input on stdin here, if it exists, store it,
    // otherwise continue
    let args = Args::parse();

    let stdout = io::stdout().into_raw_mode()?;
    let stdout = MouseTerminal::from(stdout);
    let stdout = AlternateScreen::from(stdout);
    let backend = TermionBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(args);
    let result = run_app(&mut terminal, &mut app);

    if let Err(err) = result {
        println!("{:?}", err)
    }

    Ok(())
}

enum ExitType {
    Restart,
    New,
    Quit,
}
fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    mut app: &mut App,
) -> Result<(), Box<dyn Error>> {
    let args = app.args.clone();
    let events = get_events(args.unwrap().number_of_secs.unwrap_or(0) > 0);

    loop {
        let mut exit_type: ExitType = ExitType::Quit;
        terminal.draw(|f| ui(f, &mut app))?;
        loop {
            let app = &mut app;

            match events.recv()? {
                Events::Tick => {
                    if app.thok.has_started() && !app.thok.has_finished() {
                        app.thok.on_tick();
                        terminal.draw(|f| ui(f, app))?;
                    } else if app.thok.has_finished() && app.screen == Screen::Prompt {
                        app.thok.calc_results();
                        app.screen = Screen::Results;
                    }
                }
                Events::Input(key) => {
                    match key {
                        Key::Esc => {
                            break;
                        }
                        Key::Backspace => {
                            if app.screen == Screen::Prompt {
                                app.thok.backspace();
                            }
                        }
                        Key::Left => {
                            exit_type = ExitType::Restart;
                            break;
                        }
                        Key::Right => {
                            exit_type = ExitType::New;
                            break;
                        }
                        Key::Char(c) => match app.screen {
                            Screen::Prompt => {
                                app.thok.write(c);
                                if app.thok.has_finished() {
                                    app.thok.calc_results();
                                    app.screen = Screen::Results;
                                } else {
                                    info!("not finished yet");
                                }
                            }
                            Screen::Results => match key {
                                Key::Char('t') => {
                                    webbrowser::open(&format!("https://twitter.com/intent/tweet?text={}%20wpm%20%2F%20{}%25%20acc%20%2F%20{:.2}%20sd%0A%0Ahttps%3A%2F%2Fgithub.com%2Fdevdeadly%2Fthokr", app.thok.wpm, app.thok.accuracy, app.thok.std_dev))
                                .unwrap();
                                }
                                Key::Char('r') => {
                                    exit_type = ExitType::Restart;
                                    break;
                                }
                                Key::Char('n') => {
                                    exit_type = ExitType::New;
                                    break;
                                }
                                _ => {}
                            },
                        },
                        _ => {}
                    }
                    terminal.draw(|f| ui(f, app))?;
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
enum Events {
    Input(Key),
    Tick,
}

fn get_events(should_tick: bool) -> mpsc::Receiver<Events> {
    let (tx, rx) = mpsc::channel();

    if should_tick {
        let tick_x = tx.clone();
        thread::spawn(move || loop {
            tick_x.send(Events::Tick).unwrap();
            thread::sleep(Duration::from_millis(100))
        });
    }

    info!("should_tick {}", should_tick);

    thread::spawn(move || {
        let stdin = io::stdin();
        for key in stdin.keys().flatten() {
            tx.send(Events::Input(key)).unwrap();
        }
    });

    rx
}

fn ui<B: Backend>(f: &mut Frame<B>, app: &mut App) {
    match app.screen {
        Screen::Prompt => {
            app.thok.draw_prompt(f).unwrap();
        }
        Screen::Results => {
            app.thok.draw_results(f).unwrap();
        }
    }
}
