mod history;
mod lang;
mod layout;
mod thok;
mod ui;
mod util;

use crate::{lang::Language, thok::Thok};
use clap::{CommandFactory, Parser, error::ErrorKind};
use ratatui::{
    DefaultTerminal, Terminal,
    backend::CrosstermBackend,
    crossterm::{
        cursor::{SetCursorStyle, Show},
        event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
        execute,
        terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
        tty::IsTty,
    },
};
use std::{
    io::{self, stdin},
    mem,
    ops::ControlFlow,
    panic, thread,
    time::Instant,
};
use webbrowser::Browser;

/// sleek typing tui with visualized results and historical logging
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
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
    #[arg(short = 'l', long, value_enum, default_value_t = Language::English)]
    supported_language: Language,

    /// ghost caret pacing at this WPM to race against
    #[arg(long)]
    pace: Option<u16>,
}

struct App {
    cli: Cli,
    thok: Thok,
    /// The first failure to write the history log, reported on exit.
    save_error: Option<io::Error>,
}

impl App {
    fn new(cli: Cli) -> Self {
        let (prompt, word_count) = Self::generate_prompt(&cli);
        Self {
            thok: Self::new_thok(&cli, prompt, word_count),
            cli,
            save_error: None,
        }
    }

    /// (prompt, word_count) per the CLI flags.
    fn generate_prompt(cli: &Cli) -> (String, usize) {
        if let Some(prompt) = &cli.prompt {
            (prompt.clone(), cli.number_of_words)
        } else if let Some(count) = cli.number_of_sentences {
            lang::random_sentences(count)
        } else {
            let words = cli.supported_language.random_words(cli.number_of_words);
            (words, cli.number_of_words)
        }
    }

    fn new_thok(cli: &Cli, prompt: String, word_count: usize) -> Thok {
        let number_of_secs = cli.number_of_secs.map(|secs| secs as f64);
        let mut thok = Thok::new(prompt, word_count, number_of_secs);
        thok.pace_wpm = cli.pace.map(f64::from);
        thok
    }

    /// Starts over with the same prompt.
    fn restart(&mut self) {
        let prompt = mem::take(&mut self.thok.prompt);
        self.thok = Self::new_thok(&self.cli, prompt, self.thok.number_of_words);
    }

    /// Starts over with a new prompt.
    fn new_test(&mut self) {
        let (prompt, word_count) = Self::generate_prompt(&self.cli);
        self.thok = Self::new_thok(&self.cli, prompt, word_count);
    }

    fn draw(&self, frame: &mut ratatui::Frame) {
        let can_tweet = self.thok.has_finished() && Browser::is_available();
        ui::draw(frame, &self.thok, can_tweet);
    }

    fn on_key(&mut self, key: KeyEvent) -> ControlFlow<()> {
        let finished = self.thok.has_finished();
        match key.code {
            KeyCode::Esc => return ControlFlow::Break(()),
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                return ControlFlow::Break(());
            }
            KeyCode::Left => self.restart(),
            KeyCode::Right => self.new_test(),
            KeyCode::Backspace if !finished => self.thok.backspace(),
            KeyCode::Char(c) if !finished => {
                self.thok.write(c);
                self.save_if_finished();
            }
            KeyCode::Char('r') => self.restart(),
            KeyCode::Char('n') => self.new_test(),
            KeyCode::Char('t') if Browser::is_available() => self.tweet(),
            _ => {}
        }
        ControlFlow::Continue(())
    }

    fn on_tick(&mut self) {
        self.thok.on_tick();
        self.save_if_finished();
    }

    /// Call after anything that can end the test: a key press or a tick.
    fn save_if_finished(&mut self) {
        if self.thok.has_finished() {
            self.thok.calc_results();
            if let Err(error) = history::append(&self.thok) {
                self.save_error.get_or_insert(error);
            }
        }
    }

    fn tweet(&self) {
        let url = format!(
            "https://twitter.com/intent/tweet?text={}%20wpm%20%2F%20{}%25%20acc%20%2F%20{:.2}%20sd%0A%0Ahttps%3A%2F%2Fgithub.com%2Fthatvegandev%2Fthokr",
            self.thok.wpm, self.thok.accuracy, self.thok.std_dev
        );
        // nothing useful can be done on the results screen if the browser fails
        let _ = webbrowser::open(&url);
    }
}

fn main() -> io::Result<()> {
    let cli = Cli::parse();

    if !stdin().is_tty() {
        let mut cmd = Cli::command();
        cmd.error(ErrorKind::Io, "stdin must be a tty").exit();
    }

    let mut app = App::new(cli);
    let result = run_in_terminal(&mut app);

    // only now is the terminal back in a state where this can be read
    if let Some(error) = app.save_error {
        let log = history::path().unwrap_or_default();
        eprintln!(
            "thokr: could not save results to {}: {error}",
            log.display()
        );
    }
    result
}

/// Best-effort terminal restore; used on panic and on exit.
fn restore_terminal() {
    let _ = disable_raw_mode();
    let _ = execute!(
        io::stdout(),
        SetCursorStyle::DefaultUserShape,
        LeaveAlternateScreen,
        Show
    );
}

/// Restores the terminal when dropped, so every way out of
/// `run_in_terminal` (return or `?`) leaves it usable.
struct TerminalGuard;

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        // After a panic the hook has restored the terminal already. Leaving
        // the alternate screen a second time would move the cursor back up,
        // and the shell prompt would overwrite the panic message.
        if !thread::panicking() {
            restore_terminal();
        }
    }
}

fn run_in_terminal(app: &mut App) -> io::Result<()> {
    // a panic message is only readable once the alternate screen is left
    let default_hook = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        restore_terminal();
        default_hook(info);
    }));

    enable_raw_mode()?;
    let _guard = TerminalGuard;

    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, SetCursorStyle::SteadyBar)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout))?;

    run(&mut terminal, app)
}

fn run(terminal: &mut DefaultTerminal, app: &mut App) -> io::Result<()> {
    loop {
        terminal.draw(|frame| app.draw(frame))?;

        match next_input(app.thok.next_tick())? {
            Input::Key(key) => {
                if app.on_key(key).is_break() {
                    return Ok(());
                }
            }
            Input::Tick => app.on_tick(),
            Input::Resize => {}
        }
    }
}

/// What the loop reacts to; each of them is followed by a redraw.
enum Input {
    Key(KeyEvent),
    Resize,
    Tick,
}

/// Waits for the next key press or resize, or for `tick_at` if that comes first.
fn next_input(tick_at: Option<Instant>) -> io::Result<Input> {
    loop {
        if let Some(tick_at) = tick_at {
            let wait = tick_at.saturating_duration_since(Instant::now());
            if !event::poll(wait)? {
                return Ok(Input::Tick);
            }
        }
        match event::read()? {
            Event::Key(key) if key.kind == KeyEventKind::Press => return Ok(Input::Key(key)),
            Event::Resize(_, _) => return Ok(Input::Resize),
            _ => {}
        }
    }
}
