use itertools::Itertools;
use std::{collections::HashMap, fmt::Error, time::SystemTime};
use tui::{
    backend::Backend,
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{Axis, Chart, Dataset, GraphType, Paragraph, Wrap},
    Frame,
};

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

#[derive(Clone, Debug)]
pub struct Session {
    pub prompt: String,
    pub input: Vec<Input>,
    pub timestamps: Vec<(f64, f64)>,
    pub cursor_pos: usize,
    pub started_at: Option<SystemTime>,
    pub wpm: usize,
    pub accuracy: f64,
}

impl Session {
    pub fn new(prompt_string: String) -> Self {
        Self {
            prompt: prompt_string,
            input: vec![],
            timestamps: vec![],
            cursor_pos: 0,
            started_at: None,
            wpm: 0,
            accuracy: 0.0,
        }
    }

    pub fn get_expected_char(&self, idx: usize) -> char {
        self.prompt.chars().nth(idx).unwrap()
    }

    // pub fn get_is_input_char_correct(&self, idx: usize) -> bool {
    //     idx < self.input.len() && self.input[idx].to_string() == self.get_expected_char(idx)
    // }

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
        let elapsed = self.started_at.unwrap().elapsed();
        let to_minute_ratio = 60000 / elapsed.unwrap().as_millis() as usize;

        let correct_chars = self
            .input
            .clone()
            .into_iter()
            .filter(|i| i.outcome == Outcome::Correct)
            .collect::<Vec<Input>>();

        let chars_per_sec = correct_chars
            .clone()
            .into_iter()
            .fold(HashMap::new(), |mut map, i| {
                let num_secs = i
                    .timestamp
                    .duration_since(self.started_at.unwrap())
                    .unwrap()
                    .as_secs();

                *map.entry(num_secs).or_insert(0) += 1;
                map
            })
            .into_iter()
            .sorted_by_key(|k| k.0)
            .map(|(k, v)| ((k + 1) as f64, ((v * 60) / 5) as f64))
            .collect();

        self.timestamps = chars_per_sec;
        self.accuracy = ((correct_chars.len() as f64 / self.input.len() as f64) * 100.0).round();
        let cpm = to_minute_ratio * correct_chars.len();
        self.wpm = cpm / 5;
    }

    pub fn backspace(&mut self) {
        if self.cursor_pos > 0 {
            self.input.remove(self.cursor_pos - 1);
            self.decrement_cursor();
        }
    }

    pub fn write(&mut self, c: char) {
        let idx = self.input.len();
        if idx == 0 {
            self.started_at = Some(SystemTime::now());
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

    pub fn is_finished(&self) -> bool {
        self.input.len() == self.prompt.len()
    }

    pub fn draw_prompt<B: Backend>(&self, f: &mut Frame<B>, chunk: Rect) -> Result<(), Error> {
        let mut spans = vec![];

        let mut idx = 0;
        loop {
            let expected_char = self.prompt.chars().nth(idx).unwrap().to_string();
            let (span, style);

            let correct_input =
                idx < self.input.len() && self.input[idx].outcome == Outcome::Correct;

            if idx == self.cursor_pos {
                if idx >= self.input.len() {
                    style = Style::default()
                        .add_modifier(Modifier::BOLD)
                        .add_modifier(Modifier::DIM)
                        .add_modifier(Modifier::UNDERLINED);
                } else {
                    if correct_input {
                        style = Style::default()
                            .fg(Color::Green)
                            .add_modifier(Modifier::BOLD)
                            .add_modifier(Modifier::UNDERLINED);
                    } else {
                        style = Style::default()
                            .fg(Color::Red)
                            .add_modifier(Modifier::BOLD)
                            .add_modifier(Modifier::UNDERLINED);
                    }
                }
            } else {
                if idx > self.input.len() {
                    style = Style::default()
                        .add_modifier(Modifier::DIM)
                        .add_modifier(Modifier::BOLD);
                } else {
                    if correct_input {
                        style = Style::default()
                            .fg(Color::Green)
                            .add_modifier(Modifier::BOLD);
                    } else {
                        style = Style::default().fg(Color::Red).add_modifier(Modifier::BOLD);
                    }
                }
            }
            span = Span::styled(expected_char, style);
            spans.push(span);

            idx += 1;

            if idx == self.prompt.len() {
                break;
            }
        }

        f.render_widget(
            Paragraph::new(Spans::from(spans))
                .alignment(Alignment::Center)
                .wrap(Wrap { trim: true }),
            chunk,
        );
        Ok(())
    }

    pub fn draw_results<B: Backend>(
        &mut self,
        f: &mut Frame<B>,
        chunks: Vec<Rect>,
    ) -> Result<(), Error> {
        let mut highest_wpm = 0.0;

        self.timestamps.pop();

        for ts in &self.timestamps {
            if ts.1 > highest_wpm {
                highest_wpm = ts.1 as f64;
            }
        }
        let datasets = vec![Dataset::default()
            .marker(tui::symbols::Marker::Braille)
            .style(Style::default().fg(Color::Cyan))
            .graph_type(GraphType::Line)
            .data(&self.timestamps)];

        let chart = Chart::new(datasets)
            .x_axis(
                Axis::default()
                    .title("seconds")
                    .style(Style::default().fg(Color::Gray))
                    .bounds([1.0, self.timestamps.len() as f64])
                    .labels(vec![
                        Span::styled("1", Style::default().add_modifier(Modifier::BOLD)),
                        Span::styled(
                            format!("{}", self.timestamps.len()),
                            Style::default().add_modifier(Modifier::BOLD),
                        ),
                    ]),
            )
            .y_axis(
                Axis::default()
                    .title("wpm")
                    .style(Style::default().fg(Color::Gray))
                    .bounds([0.0, highest_wpm as f64])
                    .labels(vec![
                        Span::styled("0", Style::default().add_modifier(Modifier::BOLD)),
                        Span::styled(
                            format!("{}", highest_wpm),
                            Style::default().add_modifier(Modifier::BOLD),
                        ),
                    ]),
            );
        f.render_widget(chart, chunks[0]);

        let style = Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD);

        let mut spans = vec![];
        spans.push(Span::styled(
            String::from(format!(" {} WPM ", self.wpm)),
            style,
        ));
        spans.push(Span::styled(
            String::from(format!(" {}% ACCURACY ", self.accuracy)),
            style,
        ));

        f.render_widget(
            Paragraph::new(Spans::from(spans))
                .alignment(Alignment::Center)
                .wrap(Wrap { trim: true }),
            chunks[1],
        );
        Ok(())
    }
}
