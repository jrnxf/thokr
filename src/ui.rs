use crate::layout;
use crate::thok::{Outcome, Thok};
use ratatui::{
    Frame,
    buffer::Buffer,
    layout::{Alignment, Constraint, Layout, Position, Rect},
    style::{Color, Modifier, Style},
    symbols::Marker,
    text::{Line, Span},
    widgets::{
        Paragraph, Widget,
        canvas::{Canvas, Line as CanvasLine},
    },
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

const LINE: Color = Color::Magenta;
const FAINT: Style = Style::new().fg(Color::DarkGray).add_modifier(Modifier::DIM);
/// What the seconds axis is called, under the wpm labels.
const SEC: &str = "sec";

/// The smallest of 1, 2, 5 x 10^n (and at least 1) that splits `span` into
/// at most `max_intervals` steps.
fn nice_step(span: f64, max_intervals: usize) -> f64 {
    let raw = (span / max_intervals.max(1) as f64).max(1.0);
    let magnitude = 10f64.powf(raw.log10().floor());
    [1.0, 2.0, 5.0, 10.0]
        .into_iter()
        .map(|multiple| multiple * magnitude)
        .find(|&step| step >= raw - 1e-9)
        .unwrap_or(raw)
}

/// A wpm axis: `lo` and `hi` are whole multiples of `step`.
#[derive(Debug, PartialEq)]
struct Scale {
    lo: f64,
    hi: f64,
    step: f64,
}

impl Scale {
    fn ticks(&self) -> Vec<f64> {
        let count = ((self.hi - self.lo) / self.step).round() as usize;
        (0..=count)
            .map(|i| self.lo + i as f64 * self.step)
            .collect()
    }
}

/// A wpm axis zoomed to the data, on round numbers. No value lies outside
/// it, and it never spans less than 10 wpm, so a steady test does not look
/// wild.
fn wpm_scale(values: impl Iterator<Item = f64>, rows: u16) -> Scale {
    let (mut min, mut max) =
        values.fold((f64::MAX, f64::MIN), |(lo, hi), v| (lo.min(v), hi.max(v)));
    if min > max {
        (min, max) = (0.0, 10.0);
    }
    if max - min < 10.0 {
        min = ((min + max) / 2.0 - 5.0).max(0.0);
        max = min + 10.0;
    }
    // a rule every four rows or so, and a row to each tick
    let rows = rows as usize;
    let intervals = (rows / 4).clamp(2, 6).min(rows.max(2) - 1);
    let mut step = nice_step(max - min, intervals);
    loop {
        let lo = (min / step).floor() * step;
        let hi = (max / step).ceil() * step;
        if ((hi - lo) / step).round() as usize <= intervals {
            return Scale { lo, hi, step };
        }
        step = nice_step(step * 1.5, 1);
    }
}

/// The cell, of `cells` in a line, that the canvas draws a point in when the
/// point is `offset` into an axis `span` long: the canvas rounds to a dot, two
/// to a cell.
fn canvas_cell(offset: f64, span: f64, cells: u16) -> u16 {
    let dots = f64::from(cells) * 2.0;
    (offset * (dots - 1.0) / span).round() as u16 / 2
}

/// Where the plot goes once the wpm labels (left) and the seconds labels
/// (bottom row) are taken out of the chart area, and what its axes span.
/// No point of the results lies outside the axes: the canvas cuts a line off
/// at its edges, so a point outside would leave its peak shaved or its line
/// missing.
struct Plot {
    /// the plot itself
    area: Rect,
    /// the columns left of the plot, for the wpm labels
    gutter: u16,
    y: Scale,
    x0: f64,
    x1: f64,
}

impl Plot {
    fn new(coords: &[(f64, f64)], chart: Rect) -> Self {
        let y = wpm_scale(
            coords.iter().map(|&(_, wpm)| wpm),
            chart.height.saturating_sub(1),
        );
        // as wide as the longest wpm label, or as `sec` under them
        let gutter = y
            .ticks()
            .iter()
            .map(|v| format!("{v}").len())
            .fold(SEC.len(), usize::max) as u16
            + 1;
        let first = coords.first().map_or(0.0, |&(secs, _)| secs);
        let last = coords.last().map_or(1.0, |&(secs, _)| secs);
        // the axis starts at second 1, or earlier for a test that began
        // sooner; one point alone sits in the middle
        let (x0, x1) = if last > first {
            (first.min(1.0), last)
        } else {
            (0.0, (last * 2.0).max(1.0))
        };
        let gutter = gutter.min(chart.width);
        Self {
            area: Rect {
                x: chart.x + gutter,
                y: chart.y,
                width: chart.width - gutter,
                height: chart.height.saturating_sub(1),
            },
            gutter,
            y,
            x0,
            x1,
        }
    }

    /// The column of a second.
    fn col(&self, secs: f64) -> u16 {
        self.area.x + canvas_cell(secs - self.x0, self.x1 - self.x0, self.area.width)
    }

    /// The row of a wpm value.
    fn row(&self, wpm: f64) -> u16 {
        self.area.y + canvas_cell(self.y.hi - wpm, self.y.hi - self.y.lo, self.area.height)
    }

    /// The seconds labels, each as the column it starts in and its text:
    /// centred under its second, as many as fit.
    fn seconds_labels(&self) -> Vec<(u16, String)> {
        let span = self.x1 - self.x0;
        let intervals = (self.area.width as usize / 9).clamp(2, 10);
        // under two seconds whole steps leave one label, so go to halves
        let step = if span < 2.0 {
            0.5
        } else {
            // a step past the last second has no label: one more interval
            // brings it back under
            let step = nice_step(span, intervals);
            if step > self.x1 {
                nice_step(span, intervals + 1)
            } else {
                step
            }
        };
        let mut secs = (self.x0 / step).ceil() * step;
        if secs == 0.0 {
            secs = step;
        }
        let mut labels = Vec::new();
        let mut next_free = self.area.x;
        while secs <= self.x1 + 1e-9 {
            let label = format!("{secs}");
            let len = label.len() as u16;
            if len <= self.area.width {
                // pulled in at the right edge
                let start = self
                    .col(secs)
                    .saturating_sub(len / 2)
                    .min(self.area.right() - len);
                if start >= next_free {
                    labels.push((start, label));
                    next_free = start + len + 1;
                }
            }
            secs += step;
        }
        labels
    }

    /// wpm labels with a faint rule across each, seconds labels underneath.
    fn axes(&self, buf: &mut Buffer) {
        if self.area.height == 0 || self.area.width == 0 {
            return;
        }
        // from the top; one row has room for the top tick alone
        let ticks = self.y.ticks().into_iter().rev();
        for wpm in ticks.take(self.area.height as usize) {
            let row = self.row(wpm);
            let label = format!("{wpm}");
            buf.set_string(
                self.area.x - 1 - label.len() as u16,
                row,
                &label,
                Style::new(),
            );
            for x in self.area.x..self.area.right() {
                if buf[(x, row)].symbol() == " " {
                    buf[(x, row)].set_char('─').set_style(FAINT);
                }
            }
        }
        for (x, label) in self.seconds_labels() {
            buf.set_string(x, self.area.bottom(), &label, Style::new());
        }
        buf.set_string(self.area.x - self.gutter, self.area.bottom(), SEC, FAINT);
    }

    /// The line, one quadrant block char to a 2x2 grid of dots.
    fn draw_line(&self, coords: &[(f64, f64)], buf: &mut Buffer) {
        if self.area.height == 0 || self.area.width == 0 {
            return;
        }
        Canvas::default()
            .marker(Marker::Quadrant)
            .x_bounds([self.x0, self.x1])
            .y_bounds([self.y.lo, self.y.hi])
            .paint(|ctx| {
                for pair in coords.windows(2) {
                    ctx.draw(&CanvasLine::new(
                        pair[0].0, pair[0].1, pair[1].0, pair[1].1, LINE,
                    ));
                }
                if let &[(secs, wpm)] = coords {
                    // one point alone: a dash three cells wide, a dot is lost
                    let half = 1.5 * (self.x1 - self.x0) / self.area.width as f64;
                    ctx.draw(&CanvasLine::new(secs - half, wpm, secs + half, wpm, LINE));
                }
            })
            .render(self.area, buf);
    }
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

    let plot = Plot::new(&thok.wpm_coords, chart_area);
    plot.draw_line(&thok.wpm_coords, buf);
    plot.axes(buf);

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

    /// Draws one frame and returns its cells.
    fn buffer(thok: &Thok, width: u16, height: u16) -> Buffer {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal
            .draw(|f| {
                render(thok, f.area(), f.buffer_mut())
                    .map(drop)
                    .unwrap_or_default()
            })
            .unwrap();
        terminal.backend().buffer().clone()
    }

    /// The characters of one row of a buffer.
    fn row_text(buf: &Buffer, y: u16) -> String {
        (0..buf.area.width).map(|x| buf[(x, y)].symbol()).collect()
    }

    /// The characters of a whole buffer, a row to a line.
    fn screen_text(buf: &Buffer) -> String {
        (0..buf.area.height)
            .map(|y| row_text(buf, y))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// How many cells the pace caret is from the first char of the prompt
    /// (the caret is the one reversed cell), on a one line prompt.
    fn pace_caret_offset(thok: &Thok) -> usize {
        let buf = buffer(thok, 80, 24);
        let cells: Vec<(u16, u16)> = (0..buf.area.height)
            .flat_map(|y| (0..buf.area.width).map(move |x| (x, y)))
            .filter(|&pos| buf[pos].modifier.contains(Modifier::REVERSED))
            .collect();
        assert_eq!(cells.len(), 1, "one pace caret");
        let (x, y) = cells[0];
        let first = (0..buf.area.width)
            .find(|&x| buf[(x, y)].symbol() != " ")
            .unwrap();
        (x - first) as usize
    }

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
        assert_eq!(pace_caret_offset(&thok), 5);
    }

    #[test]
    fn pace_caret_behind_the_typist() {
        let mut thok = Thok::typed(PROMPT, "the quack br", None, &[]);
        thok.pace_wpm = Some(1.2);
        thok.started_at = Some(Instant::now() - Duration::from_secs(65));
        // the typist is on index 12; the caret is behind, on index 6
        assert_eq!(pace_caret_offset(&thok), 6);
    }

    #[test]
    fn timed_test_shows_the_countdown() {
        let mut thok = Thok::typed(PROMPT, "the qu", Some(30.0), &[]);
        for _ in 0..17 {
            thok.on_tick();
        }
        assert!(screen_text(&buffer(&thok, 80, 24)).contains("28.3"));
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

        assert_eq!(thok.wpm_coords.len(), 1);
        let secs = thok.wpm_coords[0].0;
        assert!(secs < 1.0);
        let plot = Plot::new(&thok.wpm_coords, Rect::new(0, 0, 90, 20));
        assert!(plot.x0 <= secs && secs <= plot.x1);
        // and it is drawn: the dash around the point
        assert!(line_columns(&thok, 100, 30) > 0);
    }

    /// How many columns of the results chart have a char of the line in them.
    fn line_columns(thok: &Thok, width: u16, height: u16) -> usize {
        let buf = buffer(thok, width, height);
        (0..width)
            .filter(|&x| {
                (0..height).any(|y| {
                    matches!(
                        buf[(x, y)].symbol().chars().next(),
                        Some('\u{2580}'..='\u{259F}')
                    )
                })
            })
            .count()
    }

    #[test]
    fn the_whole_line_of_a_peak_with_a_fraction_below_one_half_is_drawn() {
        let thok = peak_with_a_fraction_below_one_half();
        let chart = Rect::new(HORIZONTAL_MARGIN, VERTICAL_MARGIN, 90, 23);
        let plot = Plot::new(&thok.wpm_coords, chart);
        // the axis is above the peak of 98.4
        assert!(plot.y.hi >= 99.0);
        // and the line has a char in every column from the first point to the last
        let buf = buffer(&thok, 100, 30);
        for x in plot.col(1.0)..=plot.col(10.0) {
            assert!(
                (plot.area.y..plot.area.bottom()).any(|y| matches!(
                    buf[(x, y)].symbol().chars().next(),
                    Some('\u{2580}'..='\u{259F}')
                )),
                "column {x} has no line"
            );
        }
    }

    #[test]
    fn nice_step_is_a_one_two_or_five() {
        assert_eq!(nice_step(10.0, 5), 2.0);
        assert_eq!(nice_step(10.0, 2), 5.0);
        assert_eq!(nice_step(60.0, 6), 10.0);
        assert_eq!(nice_step(61.0, 6), 20.0);
        assert_eq!(nice_step(100.0, 4), 50.0);
        assert_eq!(nice_step(0.5, 4), 1.0);
    }

    #[test]
    fn wpm_scale_covers_the_data_on_round_numbers() {
        let scale = wpm_scale([36.0, 98.4, 94.0].into_iter(), 24);
        assert_eq!(
            scale,
            Scale {
                lo: 20.0,
                hi: 100.0,
                step: 20.0
            }
        );
        assert_eq!(scale.ticks(), [20.0, 40.0, 60.0, 80.0, 100.0]);
    }

    #[test]
    fn wpm_scale_never_spans_less_than_ten() {
        for values in [vec![84.0, 86.0], vec![50.0], vec![0.0], vec![3.0], vec![]] {
            let scale = wpm_scale(values.iter().copied(), 20);
            assert!(scale.hi - scale.lo >= 10.0, "{values:?}: {scale:?}");
            assert!(scale.lo >= 0.0);
            assert!(values.iter().all(|&v| scale.lo <= v && v <= scale.hi));
        }
    }

    #[test]
    fn wpm_scale_has_few_enough_ticks_for_the_height() {
        for rows in [1, 2, 3, 6, 12, 20, 40] {
            let scale = wpm_scale([13.7, 171.2].into_iter(), rows);
            let ticks = scale.ticks().len();
            assert!(
                ticks <= (rows as usize / 4).clamp(2, 6) + 1,
                "{rows} rows: {scale:?}"
            );
            // a row to each tick, short of the two ends every scale has
            assert!(ticks <= rows.max(2) as usize, "{rows} rows: {scale:?}");
            assert!(scale.lo <= 13.7 && 171.2 <= scale.hi);
        }
    }

    /// The wpm labels of a drawn results chart, from the top: each as the
    /// row it is in and its value.
    fn wpm_labels(buf: &Buffer, plot: &Plot) -> Vec<(u16, f64)> {
        (plot.area.y..plot.area.bottom())
            .filter_map(|y| {
                let left: String = (0..plot.area.x).map(|x| buf[(x, y)].symbol()).collect();
                Some((y, left.trim().parse::<f64>().ok()?))
            })
            .collect()
    }

    #[test]
    fn every_wpm_label_is_on_the_rule_of_its_own_value() {
        for thok in [finished(), peak_with_a_fraction_below_one_half()] {
            // a terminal 9 rows high leaves the plot one row, 10 high two
            for height in 9..=24 {
                let chart = Rect::new(HORIZONTAL_MARGIN, VERTICAL_MARGIN, 70, height - 7);
                let plot = Plot::new(&thok.wpm_coords, chart);
                let labels = wpm_labels(&buffer(&thok, 80, height), &plot);
                for &(y, wpm) in &labels {
                    assert_eq!(plot.row(wpm), y, "{height} high: {wpm}");
                }
                // top to bottom they are the whole scale; one row has room
                // for the top of it alone
                let mut ticks = plot.y.ticks();
                ticks.reverse();
                if plot.area.height == 1 {
                    ticks.truncate(1);
                }
                let drawn: Vec<f64> = labels.iter().map(|&(_, wpm)| wpm).collect();
                assert_eq!(drawn, ticks, "{height} high");
            }
        }
    }

    #[test]
    fn sec_starts_at_the_left_edge_of_the_chart() {
        // wpm labels of two digits, then of three
        for thok in [finished(), peak_with_a_fraction_below_one_half()] {
            let chart = Rect::new(HORIZONTAL_MARGIN, VERTICAL_MARGIN, 70, 17);
            let seconds = row_text(&buffer(&thok, 80, 24), chart.bottom() - 1);
            assert_eq!(seconds.find("sec"), Some(chart.x as usize), "{seconds:?}");
        }
    }

    #[test]
    fn seconds_labels_sit_under_their_second_and_do_not_touch() {
        let coords: Vec<(f64, f64)> = (1..=30).map(|s| (s as f64, 40.0)).collect();
        for width in [90, 60, 24, 12] {
            let plot = Plot::new(&coords, Rect::new(0, 0, width, 20));
            let labels = plot.seconds_labels();
            assert!(!labels.is_empty(), "width {width}");
            let mut end = 0;
            for (x, label) in &labels {
                let secs: f64 = label.parse().unwrap();
                let centre = *x + label.len() as u16 / 2;
                // under the second, give or take the rounding of a column
                // and a label pulled in at the right edge
                let col = plot.col(secs);
                if (*x + label.len() as u16) < plot.area.right() {
                    assert!(centre.abs_diff(col) <= 1, "{label} at {x}, second at {col}");
                }
                assert!(*x > end || end == 0, "{label} touches the label before it");
                assert!(*x + label.len() as u16 <= plot.area.right());
                end = *x + label.len() as u16;
            }
        }
    }

    #[test]
    fn the_seconds_axis_has_a_label_at_every_length() {
        // tests that end anywhere from 1.1 to 60 seconds, and some of 400 to
        // 500, as a line and as one point alone
        for tenths in (11..=600).chain([4015, 4500, 4999]) {
            let last = tenths as f64 / 10.0;
            for coords in [vec![(1.0, 40.0), (last, 40.0)], vec![(last, 40.0)]] {
                // the wpm labels take 4 columns of the chart: plots 3 to 30
                // columns wide, room for any of these labels
                for width in 7..=34 {
                    let plot = Plot::new(&coords, Rect::new(0, 0, width, 20));
                    assert!(
                        !plot.seconds_labels().is_empty(),
                        "{coords:?} in a chart {width} wide"
                    );
                }
            }
        }
    }

    /// The cell of a plot that the canvas draws a point in.
    fn drawn_cell(plot: &Plot, secs: f64, wpm: f64) -> (u16, u16) {
        let mut buf = Buffer::empty(plot.area);
        // a line from the point to itself is one dot
        plot.draw_line(&[(secs, wpm), (secs, wpm)], &mut buf);
        let cells: Vec<(u16, u16)> = (plot.area.y..plot.area.bottom())
            .flat_map(|y| (plot.area.x..plot.area.right()).map(move |x| (x, y)))
            .filter(|&pos| buf[pos].symbol() != " ")
            .collect();
        assert_eq!(cells.len(), 1, "one point");
        cells[0]
    }

    #[test]
    fn a_second_is_in_the_column_the_line_has_it_in() {
        // terminals 80 and 100 columns wide, tests of 3 to 60 seconds
        for width in [70, 90] {
            for last in 3..=60 {
                let coords = [(1.0, 40.0), (last as f64, 40.0)];
                let plot = Plot::new(&coords, Rect::new(0, 0, width, 17));
                for secs in 1..=last {
                    let secs = secs as f64;
                    let (x, _) = drawn_cell(&plot, secs, 40.0);
                    assert_eq!(plot.col(secs), x, "second {secs} of {last}, {width} wide");
                }
            }
        }
    }

    #[test]
    fn a_wpm_tick_is_in_the_row_the_line_has_it_in() {
        for thok in [finished(), peak_with_a_fraction_below_one_half()] {
            // terminals 9 to 40 rows high
            for height in 2..=33 {
                let plot = Plot::new(&thok.wpm_coords, Rect::new(0, 0, 70, height));
                for wpm in plot.y.ticks() {
                    let (_, y) = drawn_cell(&plot, 1.0, wpm);
                    assert_eq!(plot.row(wpm), y, "{wpm} wpm, {height} high");
                }
            }
        }
    }

    #[test]
    fn seconds_labels_of_a_test_under_two_seconds() {
        let plot = Plot::new(&[(1.0, 40.0), (1.6, 42.0)], Rect::new(0, 0, 80, 20));
        let labels: Vec<_> = plot.seconds_labels().into_iter().map(|(_, l)| l).collect();
        assert_eq!(labels, ["1", "1.5"]);
    }

    #[test]
    fn the_results_chart_is_readable_at_every_size_and_length() {
        let sixty: Vec<(f64, f64)> = (1..=60)
            .map(|s| (s as f64, 70.0 + (s % 7) as f64))
            .collect();
        let results: [Vec<(f64, f64)>; 6] = [
            vec![],
            vec![(0.9, 72.0)],
            vec![(1.0, 60.0), (1.6, 66.0)],
            vec![(1.0, 60.0), (2.0, 66.0), (2.71, 63.0)],
            peak_with_a_fraction_below_one_half().wpm_coords,
            sixty,
        ];
        for coords in results {
            for (width, height) in [(100, 30), (60, 20), (30, 14), (12, 5), (3, 2), (0, 0)] {
                let mut thok = Thok::new(String::new(), 0, None);
                thok.wpm_coords = coords.clone();
                // no panic is the point; the stats and legend stay
                let screen = screen_text(&buffer(&thok, width, height));
                if width >= 60 && height >= 20 {
                    assert!(screen.contains("(r)etry / (n)ew / (esc)ape"));
                    if !coords.is_empty() {
                        assert!(line_columns(&thok, width, height) > 0);
                    }
                }
            }
        }
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

            let plot = Plot::new(&thok.wpm_coords, Rect::new(0, 0, 90, 24));
            let (left, right, bottom, top) = (plot.x0, plot.x1, plot.y.lo, plot.y.hi);
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
        assert!(screen_text(&buffer(&thok, 80, 24)).contains("36 wpm   100% acc   0.50 sd"));
    }

    #[test]
    fn results_screen_for_an_empty_prompt() {
        let thok = Thok::new(String::new(), 0, None);
        insta::assert_snapshot!(draw(&thok, 80, 24));
    }
}
