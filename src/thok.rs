use crate::util::std_dev;
use chrono::prelude::*;
use itertools::Itertools;
use std::fs::OpenOptions;
use std::io::{self, Write};
use std::{char, collections::HashMap, time::SystemTime};

#[derive(Clone, Debug, Copy, PartialEq)]
pub enum Outcome {
    Correct,
    Incorrect,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Input {
    pub char: char,
    pub outcome: Outcome,
    pub timestamp: SystemTime,
}

/// represents a test being displayed to the user
#[derive(Debug)]
pub struct Thok {
    pub prompt: String,
    pub input: Vec<Input>,
    pub raw_coords: Vec<(f64, f64)>,
    pub wpm_coords: Vec<(f64, f64)>,
    pub cursor_pos: usize,
    pub started_at: Option<SystemTime>,
    // How much time is left
    pub time_remaining: Option<f64>,
    // The duration of the test
    pub test_duration: Option<f64>,
    pub wpm: f64,
    pub accuracy: f64,
    pub std_dev: f64,
}

impl Thok {
    pub fn new(prompt_string: String, duration: Option<usize>) -> Self {
        let duration = duration.map(|d| d as f64);

        Self {
            prompt: prompt_string,
            input: vec![],
            raw_coords: vec![],
            wpm_coords: vec![],
            cursor_pos: 0,
            started_at: None,
            time_remaining: duration,
            test_duration: duration,
            wpm: 0.0,
            accuracy: 0.0,
            std_dev: 0.0,
        }
    }

    pub fn on_tick(&mut self) {
        self.time_remaining = Some(self.time_remaining.unwrap() - 0.1);
    }

    pub fn get_expected_char(&self, idx: usize) -> char {
        self.prompt.chars().nth(idx).unwrap()
    }

    pub fn increment_cursor(&mut self) {
        if self.cursor_pos < self.input.len() {
            self.cursor_pos += 1;
        }
    }

    pub fn decrement_cursor(&mut self) {
        if self.cursor_pos > 0 {
            self.cursor_pos -= 1;
        }
    }

    pub fn save_results(&self) -> io::Result<()> {
        let log_path = dirs::data_dir().unwrap().join("thokr.log");
        // If the config file doesn't exist, we need to emit a header
        let needs_header = !log_path.exists();

        let mut log_file = OpenOptions::new()
            .write(true)
            .append(true)
            .create(true)
            .open(log_path)?;

        if needs_header {
            writeln!(
                log_file,
                "time, wpm, accuracy, standard deviation, words, duration"
            )?;
        }

        writeln!(
            log_file,
            "{},{},{},{},{},{}",
            Local::now(),
            self.wpm,
            self.accuracy,
            self.std_dev,
            // The number of words in the prompt
            // TODO: it may be best to pre-calculate this, but it's not super important'
            //       as the prompt will likely be replaced on the next test
            self.prompt.split_whitespace().count(),
            // Don't log anything if duration is None. Log the float otherwise
            self.test_duration.map_or(String::new(), |f| f.to_string())
        )?;

        Ok(())
    }

    pub fn calc_results(&mut self) {
        let elapsed = self.started_at.unwrap().elapsed();

        let correct_chars = self
            .input
            .clone()
            .into_iter()
            .filter(|i| i.outcome == Outcome::Correct)
            .collect::<Vec<Input>>();

        let total_time = elapsed.unwrap().as_millis() as f64 / 1000.0;

        let whole_second_limit = total_time.floor();

        let correct_chars_per_sec: Vec<(f64, f64)> = correct_chars
            .clone()
            .into_iter()
            .fold(HashMap::new(), |mut map, i| {
                let mut num_secs = i
                    .timestamp
                    .duration_since(self.started_at.unwrap())
                    .unwrap()
                    .as_millis() as f64
                    / 1000.0;

                if num_secs == 0.0 {
                    num_secs = 1.;
                } else if num_secs.ceil() <= whole_second_limit {
                    if num_secs > 0. && num_secs < 1. {
                        // this accounts for the initiated keypress at 0.000
                        num_secs = 1.;
                    } else {
                        num_secs = num_secs.ceil()
                    }
                } else {
                    num_secs = total_time;
                }

                *map.entry(num_secs.to_string()).or_insert(0) += 1;
                map
            })
            .into_iter()
            .map(|(k, v)| (k.parse::<f64>().unwrap(), v as f64))
            .sorted_by(|a, b| a.partial_cmp(b).unwrap())
            .collect();

        let correct_chars_at_whole_sec_intervals = correct_chars_per_sec
            .iter()
            .enumerate()
            .filter(|&(i, _)| i < correct_chars_per_sec.len() - 1)
            .map(|(_, x)| x.1)
            .collect::<Vec<f64>>();

        if !correct_chars_at_whole_sec_intervals.is_empty() {
            self.std_dev = std_dev(&correct_chars_at_whole_sec_intervals).unwrap();
        } else {
            self.std_dev = 0.0;
        }

        let mut correct_chars_pressed_until_now = 0.0;

        for x in correct_chars_per_sec {
            correct_chars_pressed_until_now += x.1;
            self.wpm_coords
                .push((x.0, ((60.00 / x.0) * correct_chars_pressed_until_now) / 5.0))
        }

        if !self.wpm_coords.is_empty() {
            self.wpm = self.wpm_coords.last().unwrap().1.ceil();
        } else {
            self.wpm = 0.0;
        }
        self.accuracy = ((correct_chars.len() as f64 / self.input.len() as f64) * 100.0).round();

        let _ = self.save_results();
    }

    pub fn backspace(&mut self) {
        if self.cursor_pos > 0 {
            self.input.remove(self.cursor_pos - 1);
            self.decrement_cursor();
        }
    }

    pub fn start(&mut self) {
        self.started_at = Some(SystemTime::now());
    }

    pub fn write(&mut self, c: char) {
        let idx = self.input.len();
        if idx == 0 && self.started_at.is_none() {
            self.start();
        }

        let outcome = if c == self.get_expected_char(idx) {
            Outcome::Correct
        } else {
            Outcome::Incorrect
        };

        self.input.insert(
            self.cursor_pos,
            Input {
                char: c,
                outcome,
                timestamp: SystemTime::now(),
            },
        );
        self.increment_cursor();
    }

    pub fn has_started(&self) -> bool {
        self.started_at.is_some()
    }

    pub fn has_finished(&self) -> bool {
        (self.input.len() == self.prompt.len())
            || (self.time_remaining.is_some() && self.time_remaining.unwrap() <= 0.0)
    }
}
