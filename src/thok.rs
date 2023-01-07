use crate::util::std_dev;
use crate::TICK_RATE_MS;
use chrono::prelude::*;
use directories::ProjectDirs;
use itertools::Itertools;
use std::collections::HashSet;
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
    pub seconds_remaining: Option<f64>,
    pub number_of_secs: Option<f64>,
    pub number_of_words: usize,
    pub wpm: f64,
    pub accuracy: f64,
    pub std_dev: f64,
    pub line_lengths: Vec<usize>,
    pub total_line_length: usize,
    pub skip_curr: usize,
    pub current_line: usize,
    pub endpoints: HashSet<usize>,
}

impl Thok {
    pub fn new(prompt: String, number_of_words: usize, number_of_secs: Option<f64>) -> Self {
        Self {
            prompt,
            input: vec![],
            raw_coords: vec![],
            wpm_coords: vec![],
            cursor_pos: 0,
            started_at: None,
            number_of_secs,
            number_of_words,
            seconds_remaining: number_of_secs,
            wpm: 0.0,
            accuracy: 0.0,
            std_dev: 0.0,
            line_lengths: vec![],
            total_line_length: 0,
            skip_curr: 0,
            current_line: 0,
            endpoints: HashSet::new(),
        }
    }

    pub fn on_tick(&mut self) {
        self.seconds_remaining =
            Some(self.seconds_remaining.unwrap() - (TICK_RATE_MS as f64 / 1000_f64));
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

    pub fn calc_results(&mut self) {
        let correct_chars = self
            .input
            .clone()
            .into_iter()
            .filter(|i| i.outcome == Outcome::Correct)
            .collect::<Vec<Input>>();

        let elapsed_secs = self.started_at.unwrap().elapsed().unwrap().as_millis() as f64;

        let whole_second_limit = elapsed_secs.floor();

        let correct_chars_per_sec: Vec<(f64, f64)> = correct_chars
            .clone()
            .into_iter()
            .fold(HashMap::new(), |mut map, i| {
                let mut num_secs = i
                    .timestamp
                    .duration_since(self.started_at.unwrap())
                    .unwrap()
                    .as_secs_f64();

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
                    num_secs = elapsed_secs;
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
            // if the user is on an endpoint
            if self.endpoints.contains(&self.cursor_pos) {
                // and we have already skipped a line
                if self.skip_curr > 0 {
                    // we need to add another line to the view
                    // so we "unskip" it
                    self.skip_curr -= self.line_lengths[self.current_line - 2];
                }
                // line changed so decrease the total line length and the current line
                self.total_line_length -= self.line_lengths.pop().unwrap();
                self.current_line -= 1;

                // and finally remove it from the set
                self.endpoints.remove(&self.cursor_pos);
            }
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
            || (self.seconds_remaining.is_some() && self.seconds_remaining.unwrap() <= 0.0)
    }

    pub fn save_results(&self) -> io::Result<()> {
        if let Some(proj_dirs) = ProjectDirs::from("", "", "thokr") {
            let config_dir = proj_dirs.config_dir();
            let log_path = config_dir.join("log.csv");

            std::fs::create_dir_all(config_dir)?;

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
                    "date,num_words,num_secs,elapsed_secs,wpm,accuracy,std_dev"
                )?;
            }

            let elapsed_secs = self.started_at.unwrap().elapsed().unwrap().as_secs_f64();

            writeln!(
                log_file,
                "{},{},{},{:.2},{},{},{:.2}",
                Local::now().format("%c"),
                self.number_of_words,
                self.number_of_secs
                    .map_or(String::from(""), |ns| format!("{:.2}", ns)),
                elapsed_secs,
                self.wpm,      // already rounded, no need to round to two decimal places
                self.accuracy, // already rounded, no need to round to two decimal places
                self.std_dev,
            )?;
        }

        Ok(())
    }

    // this is a helper function which decides if we should scroll to the next line
    // it is called on every keystroke and it calculates the length of the next word
    // to determine if it can fit on the same line. If it can't, it means we have moved onto the
    // next line and so we attempt to scroll - that is achieved by the scroll_line function
    pub fn scroll_if_line_exhausted(&mut self, max_width: usize) {
        let count = self.cursor_pos - self.total_line_length;

        // this is a special case when there's a single word which spans multiple lines
        // since there is no space, the check at line 283 fails.
        if count == max_width {
            // user is now on a new line
            // so scroll
            self.scroll_line(count);
            return;
        }

        // this is the case when the current position is <= the max width
        // but the next word might not fit and may be on the next line
        // first, we remove everything up to the current position
        let rest = &self.prompt[self.cursor_pos..];

        // then we find the next space
        // which helps us in finding the length of the next word
        let index = rest.find(' ');
        if let Some(index) = index {
            let next_word = &rest[..index];
            let next_word_len = next_word.len();
            // if the next word can't fit on the current line
            if count + next_word_len > max_width {
                // user is now on a new line
                // so scroll
                self.scroll_line(count);
            }
        }
    }

    // this function is responsible for scrolling to the next line
    // if the number of typed lines is less than 2, then this does nothing
    // if its greater, its updates the skip_count so that the 2nd last typed line
    // is no longer visible
    fn scroll_line(&mut self, line_length: usize) {
        // line_length is the actual number of characters in this line
        // we push it on the Vector
        self.line_lengths.push(line_length);
        self.total_line_length += line_length;

        // this stores the endpoint of each line
        // when the user hits backspace, its used to check if we should go back to a previous line
        self.endpoints.insert(self.total_line_length);
        self.current_line += 1;

        // we start skipping lines when the number of lines completed is >= 2
        if self.line_lengths.len() >= 2 {
            // we always have a delay of 2 - if you're on line 3, then we must skip line
            // 3 - 2, that is line 1
            self.skip_curr += self.line_lengths[self.current_line - 2];
        }
    }
}
