use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Direction, Layout, Position, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Axis, Chart, Dataset, GraphType, Paragraph, Widget},
};
use webbrowser::Browser;

use crate::layout;
use crate::thok::{Outcome, Thok};

const HORIZONTAL_MARGIN: u16 = 5;
const VERTICAL_MARGIN: u16 = 2;

/// Shared geometry for the running view, so the renderer and the hardware
/// cursor math cannot drift. Returns the per-line max width, the wrapped
/// line ranges (1:1 char↔cell), and the 4-chunk vertical layout.
struct RunningGeometry {
    max_chars_per_line: u16,
    lines: Vec<std::ops::Range<usize>>,
    chunks: std::rc::Rc<[Rect]>,
}

fn running_geometry(thok: &Thok, area: Rect) -> RunningGeometry {
    let max_chars_per_line = area.width.saturating_sub(HORIZONTAL_MARGIN * 2).max(1);
    let lines = layout::wrap_chars(&thok.prompt_chars, max_chars_per_line);
    let prompt_occupied_lines = lines.len() as u16;

    let time_left_lines = if thok.number_of_secs.is_some() { 2 } else { 0 };

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

    RunningGeometry {
        max_chars_per_line,
        lines,
        chunks,
    }
}

/// Screen cell for the hardware cursor while a test is running.
/// `None` when the test has finished (the results screen has no cursor).
pub fn cursor_screen_position(thok: &Thok, area: Rect) -> Option<Position> {
    if thok.has_finished() {
        return None;
    }

    let geo = running_geometry(thok, area);
    let prompt_chunk = geo.chunks[2];

    let (line_no, col) =
        layout::char_cell(&thok.prompt_chars, geo.max_chars_per_line, thok.cursor_pos)?;

    let line_len = geo.lines.get(line_no).map(|r| r.end - r.start).unwrap_or(0) as u16;

    // alignment matches the renderer: center only when the prompt is one line
    let x_offset = if geo.lines.len() == 1 {
        (prompt_chunk.width.saturating_sub(line_len)) / 2
    } else {
        0
    };

    let x = prompt_chunk.x + x_offset + col;
    let y = prompt_chunk.y + line_no as u16;
    Some(Position::new(x, y))
}

impl Widget for &Thok {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // styles
        let bold_style = Style::default().add_modifier(Modifier::BOLD);

        let green_bold_style = Style::default().patch(bold_style).fg(Color::Green);
        let red_bold_style = Style::default().patch(bold_style).fg(Color::Red);

        let dim_bold_style = Style::default()
            .patch(bold_style)
            .add_modifier(Modifier::DIM);

        let italic_style = Style::default().add_modifier(Modifier::ITALIC);

        let magenta_style = Style::default().fg(Color::Magenta);

        match !self.has_finished() {
            true => {
                let geo = running_geometry(self, area);
                let chunks = geo.chunks;
                let pace = self.pace_caret_index();

                // one span per prompt char (1:1 with cells). The pace cell
                // keeps its real character and gets a REVERSED block patched
                // onto whatever style it already has (demo variant). The
                // cursor cell is a plain dim-bold char — the hardware bar
                // cursor overlays it (set in main::ui).
                let spans = self
                    .prompt_chars
                    .iter()
                    .enumerate()
                    .map(|(idx, &expected)| {
                        let mut span = if idx < self.input.len() {
                            match self.input[idx].outcome {
                                Outcome::Incorrect => Span::styled(
                                    if expected == ' ' {
                                        "·".to_owned()
                                    } else {
                                        expected.to_string()
                                    },
                                    red_bold_style,
                                ),
                                Outcome::Correct => {
                                    Span::styled(expected.to_string(), green_bold_style)
                                }
                            }
                        } else {
                            Span::styled(expected.to_string(), dim_bold_style)
                        };

                        if Some(idx) == pace {
                            span.style = span.style.add_modifier(Modifier::REVERSED);
                        }
                        span
                    })
                    .collect::<Vec<Span>>();

                // chunk the flat span list into lines using the wrap ranges
                let text_lines = geo
                    .lines
                    .iter()
                    .map(|r| Line::from(spans[r.clone()].to_vec()))
                    .collect::<Vec<Line>>();

                let widget = Paragraph::new(text_lines).alignment(if geo.lines.len() == 1 {
                    // when the prompt is small enough to fit on one line
                    // centering the text gives a nice zen feeling
                    Alignment::Center
                } else {
                    Alignment::Left
                });

                widget.render(chunks[2], buf);

                if let Some(sr) = self.seconds_remaining {
                    let timer = Paragraph::new(Span::styled(format!("{:.1}", sr), dim_bold_style))
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
                        highest_wpm = ts.1;
                    }
                }

                let datasets = vec![Dataset::default()
                    .marker(ratatui::symbols::Marker::Braille)
                    .style(magenta_style)
                    .graph_type(GraphType::Line)
                    .data(&self.wpm_coords)];

                let mut overall_duration = match self.wpm_coords.last() {
                    Some(x) => x.0,
                    _ => self.seconds_remaining.unwrap_or(1.0),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::thok::Input;
    use ratatui::{backend::TestBackend, Terminal};
    use std::fmt::Write;
    use std::time::{Duration, SystemTime};

    const PROMPT: &str = "the quick brown fox jumps over the lazy dog";
    const LONG_PROMPT: &str = "it was the best of times it was the worst of times it was the age of wisdom it was the age of foolishness it was the epoch of belief it was the epoch of incredulity";

    /// Draws one frame the way `main::ui` does and returns it as text: the
    /// characters, then one style letter per cell, then what each letter
    /// means, then where the cursor is.
    fn draw(thok: &Thok, width: u16, height: u16) -> String {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        let mut cursor = None;
        terminal
            .draw(|f| {
                f.render_widget(thok, f.area());
                cursor = cursor_screen_position(thok, f.area());
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let mut styles: Vec<Style> = vec![ratatui::buffer::Cell::EMPTY.style()];
        let mut text = String::new();
        let mut style_map = String::new();
        for y in 0..height {
            for x in 0..width {
                let cell = &buffer[(x, y)];
                text.push_str(cell.symbol());
                let style = cell.style();
                let id = styles.iter().position(|s| *s == style).unwrap_or_else(|| {
                    styles.push(style);
                    styles.len() - 1
                });
                style_map.push(if id == 0 {
                    '.'
                } else {
                    (b'a' + id as u8 - 1) as char
                });
            }
            text.push('\n');
            style_map.push('\n');
        }

        let mut out = format!("{text}\n{style_map}\n");
        for (id, style) in styles.iter().enumerate().skip(1) {
            let letter = (b'a' + id as u8 - 1) as char;
            writeln!(
                out,
                "{letter} = fg {:?}, {:?}",
                style.fg.unwrap(),
                style.add_modifier
            )
            .unwrap();
        }
        writeln!(out, "cursor = {cursor:?}").unwrap();
        out
    }

    /// Types `typed` against the prompt, the i-th key `offsets_ms[i]` after a
    /// start that lies 60 seconds in the past.
    fn typed(prompt: &str, typed: &str, secs: Option<f64>, offsets_ms: &[u64]) -> Thok {
        let mut thok = Thok::new(prompt.to_string(), prompt.split(' ').count(), secs);
        let started_at = SystemTime::now() - Duration::from_secs(60);
        thok.started_at = Some(started_at);
        for (i, c) in typed.chars().enumerate() {
            let outcome = if c == thok.get_expected_char(i) {
                Outcome::Correct
            } else {
                Outcome::Incorrect
            };
            thok.input.push(Input {
                char: c,
                outcome,
                timestamp: started_at
                    + Duration::from_millis(offsets_ms.get(i).copied().unwrap_or(0)),
            });
            thok.cursor_pos += 1;
        }
        thok
    }

    #[test]
    fn running_fresh() {
        let thok = Thok::new(PROMPT.to_string(), 9, None);
        insta::assert_snapshot!(draw(&thok, 80, 24));
    }

    #[test]
    fn running_with_correct_and_incorrect_input() {
        // "quick" typed as "quack", and the space after it typed as "x"
        let thok = typed(PROMPT, "the quackxbr", None, &[]);
        insta::assert_snapshot!(draw(&thok, 80, 24));
    }

    #[test]
    fn running_wraps_and_left_aligns_a_long_prompt() {
        let thok = typed(
            LONG_PROMPT,
            "it was the best of times it was the worst of times it was the age of wisdom it w",
            None,
            &[],
        );
        insta::assert_snapshot!(draw(&thok, 80, 24));
    }

    #[test]
    fn running_in_a_narrow_terminal() {
        let thok = typed(LONG_PROMPT, "it was the bust of times", None, &[]);
        insta::assert_snapshot!(draw(&thok, 30, 14));
    }

    #[test]
    fn running_in_a_terminal_too_small_for_the_prompt() {
        let thok = typed(LONG_PROMPT, "it was", None, &[]);
        insta::assert_snapshot!(draw(&thok, 20, 6));
    }

    #[test]
    fn pace_caret_ahead_of_the_typist() {
        // 12 wpm is one character a second; 5.5 seconds in, the caret is on index 5
        let mut thok = typed(PROMPT, "the", None, &[]);
        thok.pace_wpm = Some(12.0);
        thok.started_at = Some(SystemTime::now() - Duration::from_millis(5500));
        insta::assert_snapshot!(draw(&thok, 80, 24));
    }

    #[test]
    fn pace_caret_behind_the_typist() {
        let mut thok = typed(PROMPT, "the quack br", None, &[]);
        thok.pace_wpm = Some(12.0);
        thok.started_at = Some(SystemTime::now() - Duration::from_millis(6500));
        insta::assert_snapshot!(draw(&thok, 80, 24));
    }

    #[test]
    fn timed_test_shows_the_countdown() {
        let mut thok = typed(PROMPT, "the qu", Some(30.0), &[]);
        for _ in 0..17 {
            thok.on_tick();
        }
        insta::assert_snapshot!(draw(&thok, 80, 24));
    }

    #[test]
    fn timed_test_in_a_narrow_terminal() {
        let mut thok = typed(LONG_PROMPT, "it was the", Some(15.0), &[]);
        for _ in 0..42 {
            thok.on_tick();
        }
        insta::assert_snapshot!(draw(&thok, 30, 14));
    }

    fn finished() -> Thok {
        let prompt = "the quick brown fox";
        let offsets: Vec<u64> = [
            150, 300, 420, 700, 900, 1100, 1350, 1500, 1800, 2400, 2550, 2700, 3100, 3300, 3450,
            3900, 4200, 4600, 5250,
        ]
        .to_vec();
        let mut thok = typed(prompt, "the quack brown fox", None, &offsets);
        thok.calc_results();
        thok
    }

    #[test]
    fn results_screen() {
        insta::assert_snapshot!(draw(&finished(), 80, 24));
    }

    #[test]
    fn results_screen_in_a_narrow_terminal() {
        insta::assert_snapshot!(draw(&finished(), 30, 14));
    }

    #[test]
    fn results_screen_after_the_timer_ran_out() {
        let mut thok = typed(
            PROMPT,
            "the quick brown",
            Some(0.3),
            &[
                200, 400, 600, 900, 1200, 1500, 1800, 2100, 2400, 2700, 3000, 3300, 3600, 3900,
                4200,
            ],
        );
        for _ in 0..3 {
            thok.on_tick();
        }
        assert!(thok.has_finished());
        thok.calc_results();
        insta::assert_snapshot!(draw(&thok, 80, 24));
    }

    #[test]
    fn results_screen_for_an_empty_prompt() {
        let thok = Thok::new(String::new(), 0, None);
        insta::assert_snapshot!(draw(&thok, 80, 24));
    }
}
