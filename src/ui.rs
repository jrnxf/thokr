use tui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{Axis, Chart, Dataset, GraphType, Paragraph, Widget, Wrap},
};
use unicode_width::UnicodeWidthStr;
use webbrowser::Browser;

use crate::thok::{Outcome, Thok};

const HORIZONTAL_MARGIN: u16 = 5;
const VERTICAL_MARGIN: u16 = 2;

impl Widget for &Thok {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // styles
        let bold_style = Style::default().add_modifier(Modifier::BOLD);

        let green_bold_style = Style::default().patch(bold_style).fg(Color::Green);
        let red_bold_style = Style::default().patch(bold_style).fg(Color::Red);

        let dim_bold_style = Style::default()
            .patch(bold_style)
            .add_modifier(Modifier::DIM);

        let underlined_dim_bold_style = Style::default()
            .patch(dim_bold_style)
            .add_modifier(Modifier::UNDERLINED);

        let italic_style = Style::default().add_modifier(Modifier::ITALIC);

        let magenta_style = Style::default().fg(Color::Magenta);

        match !self.has_finished() {
            true => {
                let max_chars_per_line = area.width - (HORIZONTAL_MARGIN * 2);
                let mut prompt_occupied_lines =
                    ((self.prompt.width() as f64 / max_chars_per_line as f64).ceil() + 1.0) as u16;

                let time_left_lines = if self.duration.is_some() { 2 } else { 0 };

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
                            match input.outcome {
                                Outcome::Correct => green_bold_style,
                                Outcome::Incorrect => red_bold_style,
                            },
                        )
                    })
                    .collect::<Vec<Span>>();

                spans.push(Span::styled(
                    self.get_expected_char(self.cursor_pos).to_string(),
                    underlined_dim_bold_style,
                ));

                spans.push(Span::styled(
                    self.prompt[(self.cursor_pos + 1)..self.prompt.len()].to_string(),
                    dim_bold_style,
                ));

                let widget = Paragraph::new(Spans::from(spans))
                    .alignment(if prompt_occupied_lines == 1 {
                        // when the prompt is small enough to fit on one line
                        // centering the text gives a nice zen feeling
                        Alignment::Center
                    } else {
                        Alignment::Left
                    })
                    .wrap(Wrap { trim: true });

                widget.render(chunks[2], buf);

                if self.duration.is_some() {
                    let timer = Paragraph::new(Span::styled(
                        format!("{:.1}", self.duration.unwrap()),
                        dim_bold_style,
                    ))
                    .alignment(Alignment::Center);

                    timer.render(chunks[1], buf);
                }
            }
            false => {
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .horizontal_margin(HORIZONTAL_MARGIN)
                    .vertical_margin(VERTICAL_MARGIN)
                    .constraints(
                        [
                            Constraint::Min(1),
                            Constraint::Length(1),
                            Constraint::Length(1), // for padding
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
                    .style(magenta_style)
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
                            .bounds([1.0, overall_duration])
                            .labels(vec![
                                Span::styled("1", bold_style),
                                Span::styled(format!("{:.2}", overall_duration), bold_style),
                            ]),
                    )
                    .y_axis(
                        Axis::default()
                            .title("wpm")
                            .bounds([0.0, highest_wpm.round()])
                            .labels(vec![
                                Span::styled("0", bold_style),
                                Span::styled(format!("{}", highest_wpm.round()), bold_style),
                            ]),
                    );

                chart.render(chunks[0], buf);

                let stats = Paragraph::new(Span::styled(
                    format!(
                        "{} wpm   {}% acc   {:.2} sd",
                        self.wpm, self.accuracy, self.std_dev
                    ),
                    bold_style,
                ))
                .alignment(Alignment::Center);

                stats.render(chunks[1], buf);

                let legend = Paragraph::new(Span::styled(
                    String::from(if Browser::is_available() {
                        "(r)etry / (n)ew / (t)weet / (esc)ape"
                    } else {
                        "(r)etry / (n)ew / (esc)ape"
                    }),
                    italic_style,
                ));

                legend.render(chunks[3], buf);
            }
        }
    }
}
