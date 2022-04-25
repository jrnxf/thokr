use tui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{Axis, Chart, Dataset, GraphType, Paragraph, Widget, Wrap},
};
use unicode_width::UnicodeWidthStr;

use crate::thok::{Outcome, Thok};

const HORIZONTAL_MARGIN: u16 = 10;

impl Widget for &Thok {
    fn render(self, area: Rect, buf: &mut Buffer) {
        match !self.has_finished() {
            true => {
                let max_chars_per_line = area.width - (HORIZONTAL_MARGIN * 2);
                let mut prompt_occupied_lines =
                    ((self.prompt.width() as f64 / max_chars_per_line as f64).ceil() + 1.0) as u16;
                let time_left_lines = 2;

                if self.prompt.width() <= max_chars_per_line as usize {
                    prompt_occupied_lines = 1;
                }

                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .horizontal_margin(HORIZONTAL_MARGIN)
                    .constraints(
                        [
                            Constraint::Length(
                                ((area.height as f64 - prompt_occupied_lines as f64) / 2.0) as u16,
                            ),
                            Constraint::Length(time_left_lines),
                            Constraint::Length(prompt_occupied_lines),
                            Constraint::Length(
                                ((area.height as f64 - prompt_occupied_lines as f64) / 2.0) as u16,
                            ),
                        ]
                        .as_ref(),
                    )
                    .split(area);

                let mut spans = self
                    .input
                    .iter()
                    .enumerate()
                    .map(|(idx, input)| {
                        Span::styled(
                            self.get_expected_char(idx).to_string(),
                            Style::default()
                                .fg(match input.outcome {
                                    Outcome::Correct => Color::Green,
                                    Outcome::Incorrect => Color::Red,
                                })
                                .add_modifier(Modifier::BOLD),
                        )
                    })
                    .collect::<Vec<Span>>();

                spans.push(Span::styled(
                    self.prompt[self.cursor_pos..self.prompt.len()].to_string(),
                    Style::default()
                        .add_modifier(Modifier::DIM)
                        .add_modifier(Modifier::BOLD),
                ));

                let widget = match prompt_occupied_lines {
                    1 => Paragraph::new(Spans::from(spans))
                        .alignment(Alignment::Center)
                        .wrap(Wrap { trim: true }),
                    _ => Paragraph::new(Spans::from(spans)).wrap(Wrap { trim: true }),
                };

                widget.render(chunks[2], buf);

                if self.duration.is_some() {
                    let timer = Paragraph::new(Span::styled(
                        format!("{:.1}", self.duration.unwrap()),
                        Style::default()
                            .add_modifier(Modifier::DIM)
                            .add_modifier(Modifier::BOLD),
                    ))
                    .alignment(Alignment::Center);

                    timer.render(chunks[1], buf);
                }
            }
            false => {
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .horizontal_margin(10)
                    .vertical_margin(5)
                    .constraints(
                        [
                            Constraint::Percentage(90),
                            Constraint::Length(1),
                            Constraint::Length(1), // for spacing
                            Constraint::Length(1),
                        ]
                        .as_ref(),
                    )
                    .split(area);

                let mut highest_wpm = 0.0;

                for ts in &self.wpm_coords {
                    if ts.1 > highest_wpm {
                        highest_wpm = ts.1 as f64;
                    }
                }

                let datasets = vec![Dataset::default()
                    .marker(tui::symbols::Marker::Braille)
                    .style(Style::default().fg(Color::Magenta))
                    .graph_type(GraphType::Line)
                    .data(&self.wpm_coords)];

                let mut overall_duration = match self.wpm_coords.last() {
                    Some(x) => x.0,
                    _ => self.duration.unwrap_or(1.0),
                };

                overall_duration = if overall_duration < 1.0 {
                    1.0
                } else {
                    overall_duration
                };

                let chart = Chart::new(datasets)
                    .x_axis(
                        Axis::default()
                            .title("seconds")
                            .style(Style::default().fg(Color::Gray))
                            .bounds([1.0, overall_duration])
                            .labels(vec![
                                Span::styled("1", Style::default().add_modifier(Modifier::BOLD)),
                                Span::styled(
                                    format!("{:.2}", overall_duration),
                                    Style::default().add_modifier(Modifier::BOLD),
                                ),
                            ]),
                    )
                    .y_axis(
                        Axis::default()
                            .title("wpm")
                            .style(Style::default().fg(Color::Gray))
                            .bounds([0.0, highest_wpm.round()])
                            .labels(vec![
                                Span::styled("0", Style::default().add_modifier(Modifier::BOLD)),
                                Span::styled(
                                    format!("{}", highest_wpm.round()),
                                    Style::default().add_modifier(Modifier::BOLD),
                                ),
                            ]),
                    );

                chart.render(chunks[0], buf);

                let stats = Paragraph::new(Span::styled(
                    format!(
                        "{} wpm   {}% acc   {:.2} sd",
                        self.wpm, self.accuracy, self.std_dev
                    ),
                    Style::default().add_modifier(Modifier::BOLD),
                ))
                .alignment(Alignment::Center);

                stats.render(chunks[1], buf);

                let legend = Paragraph::new(Span::styled(
                    String::from("(r)etry / (n)ew / (t)weet / (esc)ape"),
                    Style::default().add_modifier(Modifier::ITALIC),
                ));

                legend.render(chunks[3], buf);
            }
        }
    }
}
