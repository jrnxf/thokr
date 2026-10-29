#[cfg(test)]
mod bench;
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

/// sleek typing tui with visualized results and historical logging
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    /// number of words to use in test (initial buffer only when timed)
    #[arg(short = 'w', long, default_value_t = 15)]
    number_of_words: usize,

    /// number of sentences to use in test
    #[arg(short = 'f', long = "full-sentences")]
    number_of_sentences: Option<usize>,

    /// run a timed test for this many seconds; words stream continuously until
    /// the clock stops (unless -p or -f fixes the text)
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

    /// end the test the moment you mistype a character
    #[arg(short = 'd', long = "death-mode")]
    death_mode: bool,
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

    /// A timed test on the default words streams fresh ones until the clock
    /// stops. A custom prompt or sentences are fixed text, so they end at
    /// their own length.
    fn is_continuous(cli: &Cli) -> bool {
        cli.number_of_secs.is_some() && cli.prompt.is_none() && cli.number_of_sentences.is_none()
    }

    fn new_thok(cli: &Cli, prompt: String, word_count: usize) -> Thok {
        let number_of_secs = cli.number_of_secs.map(|secs| secs as f64);
        let mut thok = Thok::new(prompt, word_count, number_of_secs);
        thok.pace_wpm = cli.pace.map(f64::from);
        thok.death_mode = cli.death_mode;
        if Self::is_continuous(cli) {
            thok.refill_words = cli.supported_language.word_pool();
            // so the prompt has its full length from the first frame
            thok.refill_if_needed();
        }
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

    fn draw(&mut self, frame: &mut ratatui::Frame) {
        // a timed test streams words to fill the lines the screen shows
        self.thok.set_line_width(ui::line_width(frame.area()));
        ui::draw(frame, &self.thok, Instant::now());
    }

    fn on_key(&mut self, key: KeyEvent) -> ControlFlow<()> {
        let finished = self.thok.has_finished();
        match key.code {
            KeyCode::Esc => return ControlFlow::Break(()),
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                return ControlFlow::Break(());
            }
            // a test that just ended ignores these, dropped rather than held
            KeyCode::Left | KeyCode::Right | KeyCode::Char('r' | 'n')
                if !self.thok.restart_keys_live_at(Instant::now()) => {}
            KeyCode::Left => self.restart(),
            KeyCode::Right => self.new_test(),
            KeyCode::Backspace if !finished => self.thok.backspace(),
            KeyCode::Char(c) if !finished => {
                if self.thok.write(c, Instant::now()) {
                    self.save_results();
                }
            }
            KeyCode::Char('r') => self.restart(),
            KeyCode::Char('n') => self.new_test(),
            _ => {}
        }
        ControlFlow::Continue(())
    }

    fn on_tick(&mut self) {
        if self.thok.on_tick(Instant::now()) {
            self.save_results();
        }
    }

    /// Call once, when a key press or a tick has ended the test.
    fn save_results(&mut self) {
        self.thok.calc_results();
        if self.thok.died() {
            return;
        }
        if let Err(error) = history::append(&self.thok) {
            self.save_error.get_or_insert(error);
        }
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

        match next_input(app.thok.next_tick(Instant::now()))? {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::thok::END_LOCKOUT;
    use std::time::Duration;

    /// An app on a finished test that ended `ended_ago` ago.
    fn finished_app(ended_ago: Duration) -> App {
        let mut app = App::new(Cli::parse_from(["thokr", "-p", "ab"]));
        app.thok = Thok::typed("ab", "ab", None, &[]);
        app.thok.ended_at = Some(Instant::now() - ended_ago);
        app
    }

    fn press(app: &mut App, code: KeyCode) -> ControlFlow<()> {
        app.on_key(KeyEvent::new(code, KeyModifiers::NONE))
    }

    #[test]
    fn retry_and_new_are_ignored_just_after_the_end() {
        for code in [
            KeyCode::Char('r'),
            KeyCode::Char('n'),
            KeyCode::Left,
            KeyCode::Right,
        ] {
            let mut app = finished_app(Duration::ZERO);
            assert!(press(&mut app, code).is_continue());
            assert_eq!(app.thok.input.len(), 2, "{code:?} left the end screen");
        }
    }

    #[test]
    fn a_dropped_key_is_not_replayed_when_the_lockout_ends() {
        let mut app = finished_app(Duration::ZERO);
        let _ = press(&mut app, KeyCode::Char('r'));
        app.thok.ended_at = Some(Instant::now() - END_LOCKOUT);
        assert_eq!(app.thok.input.len(), 2, "nothing was buffered");
    }

    #[test]
    fn retry_and_new_work_once_the_lockout_has_passed() {
        for code in [
            KeyCode::Char('r'),
            KeyCode::Char('n'),
            KeyCode::Left,
            KeyCode::Right,
        ] {
            let mut app = finished_app(END_LOCKOUT);
            assert!(press(&mut app, code).is_continue());
            assert!(app.thok.input.is_empty(), "{code:?} started a new test");
            assert!(app.thok.ended_at.is_none());
        }
    }

    #[test]
    fn escape_and_ctrl_c_quit_inside_the_lockout() {
        let mut app = finished_app(Duration::ZERO);
        assert!(press(&mut app, KeyCode::Esc).is_break());
        let ctrl_c = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        assert!(app.on_key(ctrl_c).is_break());
    }

    #[test]
    fn a_running_test_is_never_locked_out() {
        let mut app = App::new(Cli::parse_from(["thokr", "-p", "ab"]));
        let _ = press(&mut app, KeyCode::Char('a'));
        assert!(press(&mut app, KeyCode::Left).is_continue());
        assert!(app.thok.input.is_empty(), "left restarted the running test");
    }
}
