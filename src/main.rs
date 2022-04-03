mod lang;
mod thok;
mod util;

use crate::{lang::Language, thok::Thok};
use clap::Parser;
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
    #[clap(short = 'w', long, default_value_t = 20)]
    number_of_words: usize,

    /// The prompt to use for the test
    #[clap(short = 'p', long, default_value_t = String::from(""))]
    prompt: String,

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
    thok: Thok,
    lang: Language,
    screen: Screen,
}

impl App {
    fn new(args: Args) -> Self {
        let l = Language::new(format!("src/lang/{}.json", args.source));
        let p;

        if args.prompt != String::from("") {
            p = args.prompt.clone();
        } else {
            p = l.get_random(args.number_of_words).join(" ");
        }

        Self {
            thok: Thok::new(p),
            lang: l,
            screen: Screen::Prompt,
            args: Some(args),
        }
    }

    fn reset(self: &mut Self, new_prompt: String) {
        let a = self.args.clone().unwrap();
        let l = Language::new(format!("src/lang/{}.json", a.source));
        self.screen = Screen::Prompt;
        self.thok = Thok::new(new_prompt);
        self.lang = l;
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
    let events = get_events();
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
                app.reset(app.thok.prompt.clone());
            }
            ExitType::New => {
                let foo = app.args.clone();
                match foo {
                    Some(x) => {
                        app.reset(app.lang.get_random(x.number_of_words).join(" "));
                    }
                    _ => {
                        app.reset(app.lang.get_random(10).join(" "));
                    }
                }
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

fn get_events() -> mpsc::Receiver<Events> {
    let (tx, rx) = mpsc::channel();
    let tick_x = tx.clone();

    thread::spawn(move || loop {
        tick_x.send(Events::Tick).unwrap();
        thread::sleep(Duration::from_millis(100))
    });

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
