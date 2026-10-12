use crate::layout;
use crate::thok::{Outcome, Thok};
use ratatui::{
    Frame,
    buffer::Buffer,
    layout::{Alignment, Constraint, Layout, Position, Rect},
    style::{Color, Modifier, Style},
    symbols::Marker,
    text::{Line, Span},
    widgets::{Axis, Chart, Dataset, GraphType, Paragraph, Widget},
};
use std::ops::Range;

const HORIZONTAL_MARGIN: u16 = 5;
const VERTICAL_MARGIN: u16 = 2;

const BOLD: Style = Style::new().add_modifier(Modifier::BOLD);
const CORRECT: Style = BOLD.fg(Color::Green);
const INCORRECT: Style = BOLD.fg(Color::Red);
const UNTYPED: Style = BOLD.add_modifier(Modifier::DIM);

/// Draws the test: the prompt while it is running, the results once it has
/// finished.
pub fn draw(frame: &mut Frame, thok: &Thok) {
    let area = frame.area();
    if let Some(cursor) = render(thok, area, frame.buffer_mut()) {
        frame.set_cursor_position(cursor);
    }
}

/// Returns where the cursor belongs; the results screen has none.
fn render(thok: &Thok, area: Rect, buf: &mut Buffer) -> Option<Position> {
    if thok.has_finished() {
        render_results(thok, area, buf);
        None
    } else {
        render_running(thok, area, buf)
    }
}

fn render_running(thok: &Thok, area: Rect, buf: &mut Buffer) -> Option<Position> {
    // the wrapped lines place both the text and the cursor (1:1 char to cell)
    let max_chars_per_line = area.width.saturating_sub(HORIZONTAL_MARGIN * 2).max(1);
    let lines = layout::wrap_chars(&thok.prompt_chars, max_chars_per_line);
    let prompt_lines = lines.len() as u16;
    let timer_lines = if thok.number_of_secs.is_some() { 2 } else { 0 };
    let padding = area.height.saturating_sub(prompt_lines) / 2;

    let [_, timer_area, prompt_area, _] = Layout::vertical([
        Constraint::Length(padding),
        Constraint::Length(timer_lines),
        Constraint::Length(prompt_lines),
        Constraint::Length(padding),
    ])
    .horizontal_margin(HORIZONTAL_MARGIN)
    .areas(area);

    let pace = thok.pace_caret_index();
    let text: Vec<Line> = lines
        .iter()
        .map(|line| prompt_line(thok, line.clone(), pace))
        .collect();

    // when the prompt is small enough to fit on one line
    // centering the text gives a nice zen feeling
    let x_offset = match text.as_slice() {
        [line] => (prompt_area.width / 2).saturating_sub(line.width() as u16 / 2),
        _ => 0,
    };
    let text_area = Rect {
        x: prompt_area.x + x_offset,
        width: prompt_area.width - x_offset,
        ..prompt_area
    };
    Paragraph::new(text).render(text_area, buf);

    if let Some(secs) = thok.seconds_remaining {
        Paragraph::new(Span::styled(format!("{secs:.1}"), UNTYPED))
            .alignment(Alignment::Center)
            .render(timer_area, buf);
    }

    let (line_no, col) = layout::char_cell(&lines, thok.input.len())?;
    Some(Position::new(
        prompt_area.x + x_offset + col,
        prompt_area.y + line_no as u16,
    ))
}

/// One wrapped line of the prompt. Typed chars are green or red, the rest
/// dim; the pace caret is its own char with the colours reversed. The cursor
/// cell is a plain char: the terminal's bar cursor is drawn over it.
/// Neighbouring chars of one style share a span.
fn prompt_line(thok: &Thok, line: Range<usize>, pace: Option<usize>) -> Line<'static> {
    let mut spans: Vec<Span> = Vec::new();
    for idx in line {
        let expected = thok.prompt_chars[idx];
        let (char, style) = match thok.input.get(idx).map(|input| input.outcome) {
            Some(Outcome::Correct) => (expected, CORRECT),
            Some(Outcome::Incorrect) if expected == ' ' => ('·', INCORRECT),
            Some(Outcome::Incorrect) => (expected, INCORRECT),
            None => (expected, UNTYPED),
        };
        let style = if pace == Some(idx) {
            style.add_modifier(Modifier::REVERSED)
        } else {
            style
        };
        match spans.last_mut() {
            Some(span) if span.style == style => span.content.to_mut().push(char),
            _ => spans.push(Span::styled(String::from(char), style)),
        }
    }
    Line::from(spans)
}

/// The bounds of the seconds axis and of the wpm axis. No point of the
/// results lies outside them: the chart cuts a line off at its edges, so a
/// point outside would leave its peak shaved or its line missing.
fn chart_bounds(thok: &Thok) -> ([f64; 2], [f64; 2]) {
    // rounded up, never to the nearest: a peak of 98.4 must fit under the top
    let highest_wpm = thok
        .wpm_coords
        .iter()
        .map(|&(_, wpm)| wpm)
        .fold(0.0, f64::max)
        .ceil();
    // the axis starts at second 1, or at 0 for a test over in under a second
    let first_second = match thok.wpm_coords.first() {
        Some(&(secs, _)) if secs < 1.0 => 0.0,
        _ => 1.0,
    };
    let overall_duration = thok
        .wpm_coords
        .last()
        .map_or(thok.seconds_remaining.unwrap_or(1.0), |&(secs, _)| secs)
        .max(1.0);
    ([first_second, overall_duration], [0.0, highest_wpm])
}

fn render_results(thok: &Thok, area: Rect, buf: &mut Buffer) {
    let [chart_area, stats_area, _, legend_area] = Layout::vertical([
        Constraint::Min(1),
        Constraint::Length(1),
        Constraint::Length(1), // for padding
        Constraint::Length(1),
    ])
    .horizontal_margin(HORIZONTAL_MARGIN)
    .vertical_margin(VERTICAL_MARGIN)
    .areas(area);

    let ([first_second, overall_duration], [_, highest_wpm]) = chart_bounds(thok);

    let wpm_over_time = Dataset::default()
        .marker(Marker::Braille)
        .style(Style::new().fg(Color::Magenta))
        .graph_type(GraphType::Line)
        .data(&thok.wpm_coords);

    Chart::new(vec![wpm_over_time])
        .x_axis(
            Axis::default()
                .title("seconds")
                .bounds([first_second, overall_duration])
                .labels([
                    Span::styled(format!("{first_second}"), BOLD),
                    Span::styled(format!("{overall_duration:.2}"), BOLD),
                ]),
        )
        .y_axis(
            Axis::default()
                .title("wpm")
                .bounds([0.0, highest_wpm])
                .labels([
                    Span::styled("0", BOLD),
                    Span::styled(format!("{highest_wpm}"), BOLD),
                ]),
        )
        .render(chart_area, buf);

    let stats = format!(
        "{} wpm   {}% acc   {:.2} sd",
        thok.wpm, thok.accuracy, thok.std_dev
    );
    Paragraph::new(Span::styled(stats, BOLD))
        .alignment(Alignment::Center)
        .render(stats_area, buf);

    Paragraph::new(Span::styled(
        "(r)etry / (n)ew / (esc)ape",
        Style::new().add_modifier(Modifier::ITALIC),
    ))
    .render(legend_area, buf);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::thok::Input;
    use ratatui::{Terminal, backend::TestBackend};
    use std::fmt::Write;
    use std::time::{Duration, Instant};

    const PROMPT: &str = "the quick brown fox jumps over the lazy dog";
    const LONG_PROMPT: &str = "it was the best of times it was the worst of times it was the age of wisdom it was the age of foolishness it was the epoch of belief it was the epoch of incredulity";

    /// Draws one frame and returns it as text: the characters, then one
    /// style letter per cell, then what each letter means, then where the
    /// cursor is.
    fn draw(thok: &Thok, width: u16, height: u16) -> String {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        let mut cursor = None;
        terminal
            .draw(|f| cursor = render(thok, f.area(), f.buffer_mut()))
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

    #[test]
    fn running_fresh() {
        let thok = Thok::new(PROMPT.to_string(), 9, None);
        insta::assert_snapshot!(draw(&thok, 80, 24));
    }

    #[test]
    fn running_with_correct_and_incorrect_input() {
        // "quick" typed as "quack", and the space after it typed as "x"
        let thok = Thok::typed(PROMPT, "the quackxbr", None, &[]);
        insta::assert_snapshot!(draw(&thok, 80, 24));
    }

    #[test]
    fn running_wraps_and_left_aligns_a_long_prompt() {
        let thok = Thok::typed(
            LONG_PROMPT,
            "it was the best of times it was the worst of times it was the age of wisdom it w",
            None,
            &[],
        );
        insta::assert_snapshot!(draw(&thok, 80, 24));
    }

    #[test]
    fn running_in_a_narrow_terminal() {
        let thok = Thok::typed(LONG_PROMPT, "it was the bust of times", None, &[]);
        insta::assert_snapshot!(draw(&thok, 30, 14));
    }

    #[test]
    fn running_in_a_terminal_too_small_for_the_prompt() {
        let thok = Thok::typed(LONG_PROMPT, "it was", None, &[]);
        insta::assert_snapshot!(draw(&thok, 20, 6));
    }

    #[test]
    fn pace_caret_ahead_of_the_typist() {
        // 1.2 wpm is one character every 10 seconds; 55 seconds in, the caret is on index 5
        let mut thok = Thok::typed(PROMPT, "the", None, &[]);
        thok.pace_wpm = Some(1.2);
        thok.started_at = Some(Instant::now() - Duration::from_secs(55));
        insta::assert_snapshot!(draw(&thok, 80, 24));
    }

    #[test]
    fn pace_caret_behind_the_typist() {
        let mut thok = Thok::typed(PROMPT, "the quack br", None, &[]);
        thok.pace_wpm = Some(1.2);
        thok.started_at = Some(Instant::now() - Duration::from_secs(65));
        insta::assert_snapshot!(draw(&thok, 80, 24));
    }

    #[test]
    fn timed_test_shows_the_countdown() {
        let mut thok = Thok::typed(PROMPT, "the qu", Some(30.0), &[]);
        for _ in 0..17 {
            thok.on_tick();
        }
        insta::assert_snapshot!(draw(&thok, 80, 24));
    }

    #[test]
    fn timed_test_in_a_narrow_terminal() {
        let mut thok = Thok::typed(LONG_PROMPT, "it was the", Some(15.0), &[]);
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
        let mut thok = Thok::typed(prompt, "the quack brown fox", None, &offsets);
        thok.calc_results();
        thok
    }

    #[test]
    fn results_screen() {
        insta::assert_snapshot!(draw(&finished(), 80, 24));
    }

    /// The shape of issue #22: a 10 second test whose highest point, at
    /// second 5, is 98.4 wpm, with lower points either side.
    fn peak_with_a_fraction_below_one_half() -> Thok {
        let mut thok = Thok::new(String::new(), 15, None);
        thok.wpm_coords = vec![
            (1.0, 36.0),
            (2.0, 60.0),
            (3.0, 78.0),
            (4.0, 92.0),
            (5.0, 98.4),
            (6.0, 93.0),
            (7.0, 91.0),
            (8.0, 95.0),
            (9.0, 97.9),
            (10.0, 94.0),
        ];
        thok.wpm = 94.0;
        thok.accuracy = 100.0;
        thok.std_dev = 2.6;
        thok
    }

    #[test]
    fn results_chart_draws_a_peak_with_a_fraction_below_one_half() {
        insta::assert_snapshot!(draw(&peak_with_a_fraction_below_one_half(), 100, 30));
    }

    #[test]
    fn results_chart_of_a_test_shorter_than_a_second() {
        // two keys, 0.2 s and 0.4 s after a start 0.5 s ago: one point, left of second 1
        let mut thok = Thok::new("hi".to_string(), 1, None);
        let started_at = Instant::now() - Duration::from_millis(500);
        thok.started_at = Some(started_at);
        for (c, ms) in [('h', 200), ('i', 400)] {
            thok.input.push(Input {
                char: c,
                outcome: Outcome::Correct,
                timestamp: started_at + Duration::from_millis(ms),
            });
        }
        thok.calc_results();

        let ([first_second, last_second], _) = chart_bounds(&thok);
        assert_eq!((first_second, last_second), (0.0, 1.0));
        assert!(thok.wpm_coords[0].0 < 1.0);
    }

    #[test]
    fn every_point_of_generated_results_lies_inside_both_axes() {
        // a small fixed-seed generator, so a failure can be run again
        let mut seed: u64 = 22;
        let mut next = |below: u64| {
            seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            (seed >> 33) % below
        };

        for case in 0..500 {
            // tests of 0.1 s to 12 s, 1 to 80 keys, about one in eight wrong
            let duration_ms = 100 + next(11_900);
            let keys = 1 + next(80) as usize;
            let mut offsets: Vec<u64> = (0..keys).map(|_| next(duration_ms)).collect();
            offsets.sort_unstable();

            let mut thok = Thok::new("a".repeat(keys), 1, None);
            let started_at = Instant::now() - Duration::from_millis(duration_ms);
            thok.started_at = Some(started_at);
            for ms in offsets {
                thok.input.push(Input {
                    char: 'a',
                    outcome: if next(8) == 0 {
                        Outcome::Incorrect
                    } else {
                        Outcome::Correct
                    },
                    timestamp: started_at + Duration::from_millis(ms),
                });
            }
            thok.calc_results();

            let ([left, right], [bottom, top]) = chart_bounds(&thok);
            for &(secs, wpm) in &thok.wpm_coords {
                assert!(
                    (left..=right).contains(&secs) && (bottom..=top).contains(&wpm),
                    "case {case}: ({secs}, {wpm}) is outside [{left}, {right}] x [{bottom}, {top}]"
                );
            }
        }
    }

    #[test]
    fn results_screen_in_a_narrow_terminal() {
        insta::assert_snapshot!(draw(&finished(), 30, 14));
    }

    #[test]
    fn results_screen_after_the_timer_ran_out() {
        let mut thok = Thok::typed(
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
