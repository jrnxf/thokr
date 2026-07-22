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

/// Most prompt lines shown at once in the running view. Continuous (timed)
/// tests grow the prompt without bound, so the view scrolls a fixed window
/// instead of expanding to fit — keeping the layout centered and stable.
const MAX_VISIBLE_LINES: usize = 3;

/// Once scrolling, keep this many completed lines above the cursor line so it
/// sits comfortably in the window rather than pinned to the top edge.
const WINDOW_LEAD: usize = 1;

/// Shared geometry for the running view, so the renderer and the hardware
/// cursor math cannot drift. Returns the per-line max width, the wrapped
/// line ranges (1:1 char↔cell), the scrolled window into those lines, and
/// the 4-chunk vertical layout.
struct RunningGeometry {
    max_chars_per_line: u16,
    lines: Vec<std::ops::Range<usize>>,
    /// index of the first visible line within `lines`
    window_start: usize,
    /// number of lines actually shown (== visible range length)
    visible_count: usize,
    chunks: std::rc::Rc<[Rect]>,
}

fn running_geometry(thok: &Thok, area: Rect) -> RunningGeometry {
    let max_chars_per_line = area.width.saturating_sub(HORIZONTAL_MARGIN * 2).max(1);
    let lines = layout::wrap_chars(&thok.prompt_chars, max_chars_per_line);

    // line the cursor currently sits on; falls back to the last line when the
    // cursor is at the very end of the prompt (idx == len has no cell).
    let cursor_line = lines
        .iter()
        .position(|r| thok.cursor_pos >= r.start && thok.cursor_pos < r.end)
        .unwrap_or_else(|| lines.len().saturating_sub(1));

    let visible_count = lines.len().min(MAX_VISIBLE_LINES);
    let max_start = lines.len() - visible_count;
    let window_start = cursor_line.saturating_sub(WINDOW_LEAD).min(max_start);

    let prompt_occupied_lines = visible_count as u16;

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
        window_start,
        visible_count,
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

    // cursor scrolled above the window (shouldn't happen: the window is
    // anchored to the cursor line) — nothing to draw.
    if line_no < geo.window_start || line_no >= geo.window_start + geo.visible_count {
        return None;
    }

    let line_len = geo.lines.get(line_no).map(|r| r.end - r.start).unwrap_or(0) as u16;

    // alignment matches the renderer: center only when the prompt is one line
    let x_offset = if geo.lines.len() == 1 {
        (prompt_chunk.width.saturating_sub(line_len)) / 2
    } else {
        0
    };

    let x = prompt_chunk.x + x_offset + col;
    let y = prompt_chunk.y + (line_no - geo.window_start) as u16;
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

                // chunk the flat span list into lines using the wrap ranges,
                // showing only the scrolled window (all lines when the prompt
                // already fits within MAX_VISIBLE_LINES).
                let visible = geo.window_start..(geo.window_start + geo.visible_count);
                let text_lines = geo.lines[visible]
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
