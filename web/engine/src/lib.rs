//! A C ABI over thokr's real modules for the landing page. The page sends
//! keys and ticks; thokr's own code runs the test and draws the screen into
//! a ratatui TestBackend, which the page paints.
#![allow(dead_code)]

#[path = "../../../src/lang.rs"]
mod lang;
#[path = "../../../src/layout.rs"]
mod layout;
#[path = "../../../src/thok.rs"]
mod thok;
#[path = "../../../src/ui.rs"]
mod ui;
#[path = "../../../src/util.rs"]
mod util;

use lang::Language;
use ratatui::{
    Terminal,
    backend::{Backend, TestBackend},
    style::{Color, Modifier},
};
use std::cell::RefCell;
use std::fmt::Write;
use std::time::Instant;
use thok::{Outcome, Thok};

/// The CLI flags that shape a test, as main.rs's `Cli` holds them.
struct Cli {
    number_of_words: usize,
    number_of_sentences: Option<usize>,
    number_of_secs: Option<usize>,
    prompt: Option<String>,
    supported_language: Language,
    pace: Option<u16>,
}

struct App {
    cli: Cli,
    thok: Thok,
    out: String,
}

// generate_prompt, new_thok, restart, new_test and on_key follow main.rs.
impl App {
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
    fn restart(&mut self) {
        let prompt = std::mem::take(&mut self.thok.prompt);
        self.thok = Self::new_thok(&self.cli, prompt, self.thok.number_of_words);
    }
    fn new_test(&mut self) {
        let (prompt, word_count) = Self::generate_prompt(&self.cli);
        self.thok = Self::new_thok(&self.cli, prompt, word_count);
    }
}

thread_local! {
    static APP: RefCell<Option<App>> = const { RefCell::new(None) };
    static INBOX: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}

fn with<R>(f: impl FnOnce(&mut App) -> R) -> R {
    APP.with(|a| f(a.borrow_mut().as_mut().expect("tk_config first")))
}

/// Space for the page to write a prompt into before `tk_config`.
#[unsafe(no_mangle)]
pub extern "C" fn tk_inbox(len: usize) -> *mut u8 {
    INBOX.with(|b| {
        let mut b = b.borrow_mut();
        b.resize(len, 0);
        b.as_mut_ptr()
    })
}

/// Sets the flags (0 means "not given") and starts a new test. `lang` is
/// 0 english, 1 english1k, 2 english10k. A prompt is read from the inbox
/// when `prompt_len` is not 0.
#[unsafe(no_mangle)]
pub extern "C" fn tk_config(
    words: u32,
    sentences: u32,
    secs: u32,
    pace: u32,
    lang: u32,
    prompt_len: usize,
) {
    let prompt = (prompt_len > 0)
        .then(|| INBOX.with(|b| String::from_utf8_lossy(&b.borrow()[..prompt_len]).into_owned()));
    let cli = Cli {
        number_of_words: words as usize,
        number_of_sentences: (sentences > 0).then_some(sentences as usize),
        number_of_secs: (secs > 0).then_some(secs as usize),
        prompt,
        supported_language: match lang {
            1 => Language::English1k,
            2 => Language::English10k,
            _ => Language::English,
        },
        pace: (pace > 0).then_some(pace as u16),
    };
    let (prompt, word_count) = App::generate_prompt(&cli);
    let thok = App::new_thok(&cli, prompt, word_count);
    APP.with(|a| {
        *a.borrow_mut() = Some(App {
            cli,
            thok,
            out: String::new(),
        })
    });
}

pub const KEY_BACKSPACE: u32 = 0x11_0001;
pub const KEY_LEFT: u32 = 0x11_0002;
pub const KEY_RIGHT: u32 = 0x11_0003;

/// One key press, as main.rs's on_key handles it. A char is its code point.
#[unsafe(no_mangle)]
pub extern "C" fn tk_key(key: u32) {
    with(|app| {
        let finished = app.thok.has_finished();
        let restart_key =
            matches!(key, KEY_LEFT | KEY_RIGHT) || matches!(char::from_u32(key), Some('r' | 'n'));
        // a test that just ended ignores these, dropped rather than held
        if restart_key && !app.thok.restart_keys_live_at(Instant::now()) {
            return;
        }
        match key {
            KEY_LEFT => app.restart(),
            KEY_RIGHT => app.new_test(),
            KEY_BACKSPACE if !finished => app.thok.backspace(),
            _ => {
                let Some(c) = char::from_u32(key) else { return };
                if !finished {
                    if app.thok.write(c) {
                        app.thok.calc_results();
                    }
                } else if c == 'r' {
                    app.restart();
                } else if c == 'n' {
                    app.new_test();
                }
            }
        }
    })
}

/// Milliseconds until thokr's next tick is due, or -1 when none is.
#[unsafe(no_mangle)]
pub extern "C" fn tk_next_tick_ms() -> f64 {
    with(|app| {
        let now = Instant::now();
        match app.thok.next_tick(now) {
            Some(at) => at.saturating_duration_since(now).as_secs_f64() * 1000.0,
            None => -1.0,
        }
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn tk_tick() {
    with(|app| {
        if app.thok.on_tick(Instant::now()) {
            app.thok.calc_results();
        }
    })
}

fn color(c: Color) -> String {
    match c {
        Color::Green => "g".into(),
        Color::Red => "r".into(),
        Color::Magenta => "m".into(),
        Color::DarkGray => "k".into(),
        _ => String::new(),
    }
}

/// Draws thokr's screen at `w` x `h` cells with ui::draw and returns the
/// length of a JSON frame; `tk_out` points at it. Each cell is
/// [symbol, fg, modifier bits]; runs of blank cells are 0.
#[unsafe(no_mangle)]
pub extern "C" fn tk_render(w: u16, h: u16) -> usize {
    with(|app| {
        let mut terminal = Terminal::new(TestBackend::new(w.max(1), h.max(1))).unwrap();
        terminal.draw(|f| ui::draw(f, &app.thok, Instant::now())).unwrap();
        let typing = !app.thok.has_finished();
        let cursor = terminal.backend_mut().get_cursor_position().unwrap();
        let buf = terminal.backend().buffer();
        let out = &mut app.out;
        out.clear();
        let _ = write!(
            out,
            "{{\"w\":{},\"h\":{},\"cursor\":",
            buf.area.width, buf.area.height
        );
        if typing {
            let _ = write!(out, "[{},{}]", cursor.x, cursor.y);
        } else {
            out.push_str("null");
        }
        out.push_str(",\"rows\":[");
        for y in 0..buf.area.height {
            if y > 0 {
                out.push(',');
            }
            out.push('[');
            for x in 0..buf.area.width {
                if x > 0 {
                    out.push(',');
                }
                let cell = &buf[(x, y)];
                let sym = cell.symbol();
                if sym == " " && cell.modifier.is_empty() {
                    out.push('0');
                    continue;
                }
                out.push_str("[\"");
                for ch in sym.chars() {
                    match ch {
                        '"' => out.push_str("\\\""),
                        '\\' => out.push_str("\\\\"),
                        c => out.push(c),
                    }
                }
                let _ = write!(
                    out,
                    "\",\"{}\",{}]",
                    color(cell.fg),
                    (cell.modifier
                        & (Modifier::BOLD | Modifier::DIM | Modifier::ITALIC | Modifier::REVERSED))
                        .bits()
                );
            }
            out.push(']');
        }
        out.push_str("]}");
        out.len()
    })
}

/// The test's state for pages that draw the prompt themselves: the prompt,
/// one char per typed key (c correct, x incorrect), the pace caret, the
/// countdown, and once finished the chart points and results.
#[unsafe(no_mangle)]
pub extern "C" fn tk_state() -> usize {
    with(|app| {
        let t = &app.thok;
        let out = &mut app.out;
        out.clear();
        out.push_str("{\"prompt\":\"");
        for ch in t.prompt.chars() {
            match ch {
                '"' => out.push_str("\\\""),
                '\\' => out.push_str("\\\\"),
                c => out.push(c),
            }
        }
        out.push_str("\",\"input\":\"");
        for i in &t.input {
            out.push(if i.outcome == Outcome::Correct {
                'c'
            } else {
                'x'
            });
        }
        let _ = write!(
            out,
            "\",\"started\":{},\"finished\":{},\"pace\":{},\"remaining\":{},\"words\":{},\"secs\":{},\"wpm\":{},\"accuracy\":{},\"sd\":{},\"elapsed\":{},\"coords\":[",
            t.started_at.is_some(),
            t.has_finished(),
            t.pace_caret_index()
                .map_or("null".into(), |i| i.to_string()),
            t.seconds_remaining
                .map_or("null".into(), |s| format!("{s}")),
            t.number_of_words,
            t.number_of_secs.map_or("null".into(), |s| format!("{s}")),
            t.wpm,
            t.accuracy,
            t.std_dev,
            t.elapsed_secs,
        );
        for (i, (x, y)) in t.wpm_coords.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            let _ = write!(out, "[{x},{y}]");
        }
        out.push_str("]}");
        out.len()
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn tk_out() -> *const u8 {
    with(|app| app.out.as_ptr())
}
