//! A timing harness, not a correctness test:
//! `cargo test --release -- --ignored --nocapture bench`
//!
//! Frames go through the real crossterm backend into a sink, so the numbers
//! cover handling the key, rendering, diffing against the last frame and
//! encoding the escape codes, but not the terminal emulator itself.

use crate::{App, Cli};
use clap::Parser;
use ratatui::{Terminal, TerminalOptions, Viewport, backend::CrosstermBackend, layout::Rect};
use std::{io, time::Instant};

const PROMPT: &str = "the quick brown fox jumps over the lazy dog while seven wizards quietly judge a boxing match near the old stone bridge";
const RUNS: usize = 300;

fn median(mut samples: Vec<f64>) -> f64 {
    samples.sort_by(f64::total_cmp);
    samples[samples.len() / 2]
}

#[test]
#[ignore = "timing harness"]
fn bench_key_and_frame() {
    for (width, height) in [(100, 30), (40, 20)] {
        let mut terminal = Terminal::with_options(
            CrosstermBackend::new(io::sink()),
            TerminalOptions {
                viewport: Viewport::Fixed(Rect::new(0, 0, width, height)),
            },
        )
        .unwrap();
        let mut app = App::new(Cli::parse_from(["thokr", "-p", PROMPT]));
        let keys = PROMPT.chars().count() - 1;

        // type all but the last char, drawing a frame after every key
        let mut samples = Vec::new();
        for _ in 0..RUNS {
            app.restart();
            terminal.draw(|frame| app.draw(frame)).unwrap();
            let start = Instant::now();
            for c in PROMPT.chars().take(keys) {
                app.thok.write(c);
                terminal.draw(|frame| app.draw(frame)).unwrap();
            }
            samples.push(start.elapsed().as_nanos() as f64 / keys as f64);
        }
        println!(
            "key + frame   {width}x{height}: {:.2} us",
            median(samples) / 1000.0
        );

        // finish the test, then redraw the results screen
        app.thok.write('e');
        app.thok.calc_results();
        let mut samples = Vec::new();
        for _ in 0..RUNS {
            let start = Instant::now();
            for _ in 0..20 {
                terminal.draw(|frame| app.draw(frame)).unwrap();
            }
            samples.push(start.elapsed().as_nanos() as f64 / 20.0);
        }
        println!(
            "results frame {width}x{height}: {:.2} us",
            median(samples) / 1000.0
        );
    }
}
