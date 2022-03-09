use std::{fmt::Error, time::Instant};
use tui::{
    backend::Backend,
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{Paragraph, Wrap},
    Frame,
};

#[derive(Clone)]
pub struct Prompt {
    pub prompt: String,
    pub input: Vec<char>,
    pub cursor_pos: usize,
    pub duration: Option<Instant>,
}

impl Prompt {
    pub fn new(prompt: String) -> Self {
        Self {
            prompt,
            input: vec![],
            cursor_pos: 0,
            duration: None,
        }
    }

    pub fn get_expected_char(&self, idx: usize) -> String {
        self.prompt.chars().nth(idx).unwrap().to_string()
    }

    pub fn get_is_input_char_correct(&self, idx: usize) -> bool {
        idx < self.input.len() && self.input[idx].to_string() == self.get_expected_char(idx)
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

    pub fn get_cpm(self) -> usize {
        let elapsed = self.duration.unwrap().elapsed();
        (60000 / elapsed.as_millis() as usize) * self.prompt.len()
    }

    pub fn get_wpm(self) -> usize {
        self.get_cpm() / 5
    }

    pub fn backspace(&mut self) {
        if self.cursor_pos > 0 {
            self.input.remove(self.cursor_pos - 1);
            self.decrement_cursor();
        }
    }

    pub fn write(&mut self, c: char) {
        if self.input.len() == 0 {
            self.duration = Some(Instant::now());
        }
        self.input.insert(self.cursor_pos, c);
        self.increment_cursor();
    }

    // pub fn go_to_start(&mut self) {
    //     self.cursor_pos = 0;
    // }

    // pub fn go_to_end(&mut self) {
    //     self.cursor_pos = self.input.len();
    // }
    //

    pub fn draw_prompt<B: Backend>(&self, f: &mut Frame<B>, chunk: Rect) -> Result<(), Error> {
        let mut spans = vec![];

        let mut idx = 0;
        loop {
            let expected_char = self.prompt.chars().nth(idx).unwrap().to_string();
            let (span, style);

            let correct_input = self.get_is_input_char_correct(idx);

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
                        // correct char
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

    pub fn draw_results<B: Backend>(&self, f: &mut Frame<B>, chunk: Rect) -> Result<(), Error> {
        let style = Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD);

        let wpm = self.clone().get_wpm();
        let span = Span::styled(String::from(format!("{} WPM", wpm)), style);

        f.render_widget(
            Paragraph::new(span)
                .alignment(Alignment::Center)
                .wrap(Wrap { trim: true }),
            chunk,
        );
        Ok(())
    }
}
