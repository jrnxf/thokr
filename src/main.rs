mod lang;
mod thok;
mod ui;
mod util;

use crate::{lang::Language, thok::Thok};
use clap::error::ErrorKind;
use clap::{CommandFactory, Parser, ValueEnum};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    crossterm::{
        event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
        tty::IsTty,
    },
    Frame, Terminal,
};
use std::{
    error::Error,
    io::{self, stdin},
    sync::mpsc,
    thread,
    time::Duration,
};
use webbrowser::Browser;

const TICK_RATE_MS: u64 = 100;

/// sleek typing tui with visualized results and historical logging
#[derive(Parser, Debug, Clone)]
#[command(version, about, long_about = None)]
pub struct Cli {
    /// number of words to use in test
    #[arg(short = 'w', long, default_value_t = 15)]
    number_of_words: usize,

    /// number of sentences to use in test
    #[arg(short = 'f', long = "full-sentences")]
    number_of_sentences: Option<usize>,

    /// number of seconds to run test
    #[arg(short = 's', long)]
    number_of_secs: Option<usize>,

    /// custom prompt to use
    #[arg(short = 'p', long)]
    prompt: Option<String>,

    /// language to pull words from
    #[arg(short = 'l', long, value_enum, default_value_t = SupportedLanguage::English)]
    supported_language: SupportedLanguage,

    /// ghost caret pacing at this WPM to race against
    #[arg(long)]
    pace: Option<u16>,
}

#[derive(Debug, Copy, Clone, ValueEnum, strum_macros::Display)]
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
    cli: Cli,
    thok: Thok,
}

impl App {
    /// (prompt, word_count) per the CLI flags.
    fn generate_prompt(cli: &Cli) -> (String, usize) {
        if let Some(p) = &cli.prompt {
            (p.clone(), cli.number_of_words)
        } else if let Some(n) = cli.number_of_sentences {
            let language = cli.supported_language.as_lang();
            let (s, count) = language.get_random_sentence(n);
            (s.join(""), count)
        } else {
            let language = cli.supported_language.as_lang();
            (
                language.get_random(cli.number_of_words).join(" "),
                cli.number_of_words,
            )
        }
    }

    fn new(cli: Cli) -> Self {
        let (prompt, count) = Self::generate_prompt(&cli);
        let mut thok = Thok::new(prompt, count, cli.number_of_secs.map(|ns| ns as f64));
        thok.pace_wpm = cli.pace.map(f64::from);
        Self { thok, cli }
    }

    fn reset(&mut self, new_prompt: Option<String>) {
        let (prompt, count) = match new_prompt {
            Some(p) => (p, self.thok.number_of_words),
            None => Self::generate_prompt(&self.cli),
        };
        self.thok = Thok::new(prompt, count, self.cli.number_of_secs.map(|ns| ns as f64));
        self.thok.pace_wpm = self.cli.pace.map(f64::from);
    }
}

/// Best-effort terminal restore; used on panic and on exit.
fn restore_terminal() {
    let _ = disable_raw_mode();
    let _ = execute!(io::stdout(), LeaveAlternateScreen);
}

fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();

    if !stdin().is_tty() {
        let mut cmd = Cli::command();
        cmd.error(ErrorKind::Io, "stdin must be a tty").exit();
    }

    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        restore_terminal();
        default_hook(info);
    }));

    enable_raw_mode()?;

    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(cli);
    let res = start_tui(&mut terminal, &mut app);

    restore_terminal();
    terminal.show_cursor()?;

    res
}

enum ExitType {
    Restart,
    New,
    Quit,
}
fn start_tui<B: Backend>(
    terminal: &mut Terminal<B>,
    mut app: &mut App,
) -> Result<(), Box<dyn Error>>
where
    <B as Backend>::Error: 'static,
{
    let should_tick = app.cli.number_of_secs.unwrap_or(0) > 0 || app.cli.pace.is_some();

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
                            let _ = app.thok.save_results();
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
                        KeyCode::Backspace if !app.thok.has_finished() => {
                            app.thok.backspace();
                        }
                        KeyCode::Left => {
                            exit_type = ExitType::Restart;
                            break;
                        }
                        KeyCode::Right => {
                            exit_type = ExitType::New;
                            break;
                        }
                        KeyCode::Char(c) => {
                            if key.modifiers.contains(KeyModifiers::CONTROL)
                                && key.code == KeyCode::Char('c')
                            // ctrl+c to quit
                            {
                                break;
                            }

                            match app.thok.has_finished() {
                                false => {
                                    app.thok.write(c);
                                    if app.thok.has_finished() {
                                        app.thok.calc_results();
                                        let _ = app.thok.save_results();
                                    }
                                }
                                true => match key.code {
                                    KeyCode::Char('t') if Browser::is_available() => {
                                        webbrowser::open(&format!("https://twitter.com/intent/tweet?text={}%20wpm%20%2F%20{}%25%20acc%20%2F%20{:.2}%20sd%0A%0Ahttps%3A%2F%2Fgithub.com%2Fthatvegandev%2Fthokr", app.thok.wpm, app.thok.accuracy, app.thok.std_dev))
                                    .unwrap_or_default();
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
                            }
                        }
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

            thread::sleep(Duration::from_millis(TICK_RATE_MS))
        });
    }

    thread::spawn(move || loop {
        let evt = match event::read().unwrap() {
            Event::Key(key) if key.kind == KeyEventKind::Press => Some(ThokEvent::Key(key)),
            Event::Key(_) => None,
            Event::Resize(_, _) => Some(ThokEvent::Resize),
            _ => None,
        };

        if evt.is_some() && tx.send(evt.unwrap()).is_err() {
            break;
        }
    });

    rx
}

fn ui(app: &mut App, f: &mut Frame) {
    f.render_widget(&app.thok, f.area());
}
