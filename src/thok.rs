use crate::util::std_dev;
use crate::TICK_RATE_MS;
use chrono::prelude::*;
use directories::ProjectDirs;
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
    pub prompt_chars: Vec<char>,
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
}

impl Thok {
    pub fn new(prompt: String, number_of_words: usize, number_of_secs: Option<f64>) -> Self {
        let prompt_chars = prompt.chars().collect();
        Self {
            prompt,
            prompt_chars,
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
        }
    }

    pub fn on_tick(&mut self) {
        self.seconds_remaining =
            Some(self.seconds_remaining.unwrap() - (TICK_RATE_MS as f64 / 1000_f64));
    }

    pub fn char_count(&self) -> usize {
        self.prompt_chars.len()
    }

    pub fn get_expected_char(&self, idx: usize) -> char {
        self.prompt_chars[idx]
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

        let elapsed_secs = self.started_at.unwrap().elapsed().unwrap().as_secs_f64();

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
        self.accuracy = if self.input.is_empty() {
            0.0
        } else {
            ((correct_chars.len() as f64 / self.input.len() as f64) * 100.0).round()
        };
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
        (self.input.len() == self.char_count())
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, SystemTime};

    /// Builds a finished-by-length thok: every char of `typed` written against
    /// `prompt`, with the i-th keystroke stamped at started_at + offsets[i].
    fn thok_with_input(prompt: &str, typed: &str, offsets_ms: &[u64]) -> Thok {
        let mut thok = Thok::new(prompt.to_string(), prompt.split(' ').count(), None);
        let started_at = SystemTime::now() - Duration::from_secs(60);
        thok.started_at = Some(started_at);
        for (i, c) in typed.chars().enumerate() {
            let outcome = if c == prompt.chars().nth(i).unwrap() {
                Outcome::Correct
            } else {
                Outcome::Incorrect
            };
            thok.input.push(Input {
                char: c,
                outcome,
                timestamp: started_at + Duration::from_millis(offsets_ms[i]),
            });
            thok.cursor_pos += 1;
        }
        thok
    }

    #[test]
    fn write_records_correct_and_incorrect() {
        let mut thok = Thok::new("hi".to_string(), 1, None);
        thok.write('h');
        thok.write('x');
        assert_eq!(thok.input[0].outcome, Outcome::Correct);
        assert_eq!(thok.input[1].outcome, Outcome::Incorrect);
        assert_eq!(thok.cursor_pos, 2);
        assert!(thok.has_started());
    }

    #[test]
    fn backspace_removes_last_input() {
        let mut thok = Thok::new("hi".to_string(), 1, None);
        thok.write('h');
        thok.write('x');
        thok.backspace();
        assert_eq!(thok.input.len(), 1);
        assert_eq!(thok.cursor_pos, 1);

        // backspace with no input must not panic and leaves cursor at 0
        let mut empty = Thok::new("hi".to_string(), 1, None);
        empty.backspace();
        assert_eq!(empty.cursor_pos, 0);
    }

    #[test]
    fn has_finished_by_length() {
        let mut thok = Thok::new("ab".to_string(), 1, None);
        thok.write('a');
        assert!(!thok.has_finished());
        thok.write('b');
        assert!(thok.has_finished());
    }

    #[test]
    fn has_finished_by_timer() {
        let mut thok = Thok::new("abc".to_string(), 1, Some(0.3));
        thok.write('a');
        assert!(!thok.has_finished());
        // each tick subtracts TICK_RATE_MS (100ms = 0.1s)
        thok.on_tick();
        thok.on_tick();
        thok.on_tick();
        assert!(thok.has_finished());
    }

    #[test]
    fn calc_results_accuracy() {
        let mut thok = thok_with_input("hello", "hellx", &[100, 300, 500, 700, 900]);
        thok.calc_results();
        // 4 of 5 correct => 80%
        assert_eq!(thok.accuracy, 80.0);
    }

    #[test]
    fn calc_results_wpm_all_correct() {
        let mut thok = thok_with_input("hello", "hello", &[200, 400, 600, 800, 950]);
        thok.calc_results();
        // all 5 offsets are under 1s so they clamp to bucket 1.0:
        // cumulative 5 chars => (60/1)*5/5 = 60, ceiled
        assert_eq!(thok.wpm, 60.0);
        assert_eq!(thok.wpm_coords.len(), 1);
    }

    #[test]
    fn unicode_prompt_finishes_by_length() {
        let mut thok = Thok::new("épée".to_string(), 1, None);
        for c in ['é', 'p', 'é', 'e'] {
            thok.write(c);
        }
        assert!(thok.has_finished());
        assert!(thok.input.iter().all(|i| i.outcome == Outcome::Correct));
    }

    #[test]
    fn unicode_wrong_char_marked_incorrect() {
        let mut thok = Thok::new("épée".to_string(), 1, None);
        thok.write('e');
        assert_eq!(thok.input[0].outcome, Outcome::Incorrect);
    }

    #[test]
    fn get_expected_char_multibyte() {
        let thok = Thok::new("héllo".to_string(), 1, None);
        assert_eq!(thok.get_expected_char(1), 'é');
        assert_eq!(thok.get_expected_char(4), 'o');
    }

    #[test]
    fn calc_results_buckets_have_sane_x_axis() {
        let mut thok = thok_with_input("abcdef", "abcdef", &[100, 300, 500, 1200, 1400, 2350]);
        thok.calc_results();
        assert!(!thok.wpm_coords.is_empty());
        assert!(thok.wpm_coords.iter().all(|(x, _)| x.is_finite()));
        let last_x = thok.wpm_coords.last().unwrap().0;
        assert!(last_x >= 2.0);
        assert!(last_x < 60.0);
        assert!(thok.wpm > 0.0);
    }

    #[test]
    fn calc_results_empty_input_does_not_panic() {
        let mut thok = Thok::new("hello".to_string(), 1, None);
        thok.started_at = Some(SystemTime::now() - Duration::from_secs(60));
        thok.calc_results();
        assert_eq!(thok.wpm, 0.0);
        assert_eq!(thok.std_dev, 0.0);
        assert_eq!(thok.accuracy, 0.0);
    }
}
