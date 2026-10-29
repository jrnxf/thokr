use crate::layout;
use crate::ui::MAX_VISIBLE_LINES;
use crate::util::std_dev;
use rand::seq::SliceRandom;
use std::time::{Duration, Instant};

/// How often the countdown and the pace caret move while a test is running.
pub const TICK: Duration = Duration::from_millis(100);

#[derive(Clone, Debug, Copy, PartialEq)]
pub enum Outcome {
    Correct,
    Incorrect,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Input {
    pub char: char,
    pub outcome: Outcome,
    pub timestamp: Instant,
}

/// How many fresh words to append at a time when the prompt runs short.
const REFILL_WORD_COUNT: usize = 10;

/// Chars per line before the screen has reported its width: an 80 column
/// terminal less its margins.
const DEFAULT_LINE_WIDTH: u16 = 70;

/// represents a test being displayed to the user
#[derive(Debug)]
pub struct Thok {
    pub prompt: String,
    pub prompt_chars: Vec<char>,
    pub input: Vec<Input>,
    pub wpm_coords: Vec<(f64, f64)>,
    pub started_at: Option<Instant>,
    pub seconds_remaining: Option<f64>,
    pub number_of_secs: Option<f64>,
    pub number_of_words: usize,
    pub elapsed_secs: f64,
    pub wpm: f64,
    pub accuracy: f64,
    pub std_dev: f64,
    pub pace_wpm: Option<f64>,
    /// End the test the instant an incorrect character is typed.
    pub death_mode: bool,
    /// Word pool for continuous (timed) mode. When non-empty, the prompt is
    /// topped up as the cursor advances and the test ends only on the timer,
    /// never by reaching the end of the words.
    pub refill_words: Vec<String>,
    /// How many chars fit on a line of the screen, to size the refill by.
    line_width: u16,
    ticks: u32,
}

impl Thok {
    pub fn new(prompt: String, number_of_words: usize, number_of_secs: Option<f64>) -> Self {
        let prompt_chars = prompt.chars().collect();
        Self {
            prompt,
            prompt_chars,
            input: vec![],
            wpm_coords: vec![],
            started_at: None,
            number_of_secs,
            number_of_words,
            seconds_remaining: number_of_secs,
            elapsed_secs: 0.0,
            wpm: 0.0,
            accuracy: 0.0,
            std_dev: 0.0,
            pace_wpm: None,
            ticks: 0,
            death_mode: false,
            refill_words: vec![],
            line_width: DEFAULT_LINE_WIDTH,
        }
    }

    /// Index of the pace caret after `elapsed_secs`, or None if pacing is
    /// off or the pace caret has run past the end of the prompt.
    pub fn pace_caret_index_at(&self, elapsed_secs: f64) -> Option<usize> {
        let wpm = self.pace_wpm?;
        let idx = (elapsed_secs * wpm * 5.0 / 60.0).floor() as usize;
        (idx < self.char_count()).then_some(idx)
    }

    /// Pace caret position now; None before the test starts.
    pub fn pace_caret_index(&self) -> Option<usize> {
        self.pace_caret_index_at(self.started_at?.elapsed().as_secs_f64())
    }

    /// When the next tick is due. None unless a test with something to
    /// animate (a countdown or a pace caret) is running.
    pub fn next_tick(&self) -> Option<Instant> {
        let animated = self.number_of_secs.is_some() || self.pace_wpm.is_some();
        if !animated || self.has_finished() {
            return None;
        }
        Some(self.started_at? + TICK * (self.ticks + 1))
    }

    /// True when this test streams new words to fill a time limit rather than
    /// ending at a fixed prompt length.
    pub fn is_continuous(&self) -> bool {
        !self.refill_words.is_empty()
    }

    /// Tells the test how wide the screen's lines are, and tops the prompt up
    /// for that width.
    pub fn set_line_width(&mut self, line_width: u16) {
        if self.line_width != line_width {
            self.line_width = line_width;
            self.refill_if_needed();
        }
    }

    /// In continuous mode, append fresh words until the prompt wraps to at
    /// least a full window of lines below the cursor's line, so the view
    /// always shows the same number of lines and never grows mid-test. No-op
    /// otherwise.
    pub fn refill_if_needed(&mut self) {
        if self.refill_words.is_empty() {
            return;
        }

        while self.lines_short() {
            let rng = &mut rand::thread_rng();
            let mut appended = String::new();
            for _ in 0..REFILL_WORD_COUNT {
                if let Some(word) = self.refill_words.choose(rng) {
                    // words are space-separated, but the prompt must not
                    // start with a space
                    if !self.prompt_chars.is_empty() || !appended.is_empty() {
                        appended.push(' ');
                    }
                    appended.push_str(word);
                }
            }
            self.prompt.push_str(&appended);
            self.prompt_chars.extend(appended.chars());
        }
    }

    /// True when fewer lines than the window shows follow the cursor's line.
    fn lines_short(&self) -> bool {
        let lines = layout::wrap_chars(&self.prompt_chars, self.line_width);
        let cursor_line = layout::char_cell(&lines, self.input.len())
            .map_or(lines.len().saturating_sub(1), |(line_no, _)| line_no);
        lines.len() < cursor_line + MAX_VISIBLE_LINES
    }

    pub fn on_tick(&mut self) {
        self.ticks += 1;
        if let Some(number_of_secs) = self.number_of_secs {
            // derived from the tick count rather than subtracted tick by
            // tick, so the rounding error of 0.1 cannot build up
            let ticked_secs = (TICK * self.ticks).as_secs_f64();
            self.seconds_remaining = Some(number_of_secs - ticked_secs);
        }
    }

    pub fn char_count(&self) -> usize {
        self.prompt_chars.len()
    }

    pub fn get_expected_char(&self, idx: usize) -> char {
        self.prompt_chars[idx]
    }

    pub fn calc_results(&mut self) {
        let Some(started_at) = self.started_at else {
            return;
        };
        let elapsed_secs = started_at.elapsed().as_secs_f64();
        let whole_secs = elapsed_secs.floor();

        // Each correct key press counts towards the whole second it fell in
        // (0.0..=1.0 is second 1), or towards the unfinished last second,
        // which is plotted at the moment the test ended.
        let mut seconds: Vec<f64> = self
            .input
            .iter()
            .filter(|input| input.outcome == Outcome::Correct)
            .map(|input| {
                let secs = input.timestamp.duration_since(started_at).as_secs_f64();
                if secs == 0.0 {
                    1.0
                } else if secs.ceil() <= whole_secs {
                    secs.ceil()
                } else {
                    elapsed_secs
                }
            })
            .collect();
        seconds.sort_by(f64::total_cmp);
        let correct_chars = seconds.len();

        let correct_chars_per_sec: Vec<(f64, f64)> = seconds
            .chunk_by(|a, b| a == b)
            .map(|second| (second[0], second.len() as f64))
            .collect();

        // the last second is cut short by the end of the test, so it would
        // skew the deviation
        let whole_sec_counts: Vec<f64> = correct_chars_per_sec
            .split_last()
            .map(|(_, whole)| whole.iter().map(|&(_, count)| count).collect())
            .unwrap_or_default();

        let mut correct_chars_until_now = 0.0;
        self.wpm_coords = correct_chars_per_sec
            .iter()
            .map(|&(second, count)| {
                correct_chars_until_now += count;
                (second, ((60.0 / second) * correct_chars_until_now) / 5.0)
            })
            .collect();

        self.elapsed_secs = elapsed_secs;
        self.std_dev = std_dev(&whole_sec_counts).unwrap_or(0.0);
        self.wpm = self.wpm_coords.last().map_or(0.0, |&(_, wpm)| wpm.ceil());
        self.accuracy = if self.input.is_empty() {
            0.0
        } else {
            ((correct_chars as f64 / self.input.len() as f64) * 100.0).round()
        };
    }

    pub fn backspace(&mut self) {
        self.input.pop();
    }

    pub fn write(&mut self, c: char) {
        self.started_at.get_or_insert_with(Instant::now);

        let outcome = if c == self.get_expected_char(self.input.len()) {
            Outcome::Correct
        } else {
            Outcome::Incorrect
        };

        self.input.push(Input {
            char: c,
            outcome,
            timestamp: Instant::now(),
        });
        self.refill_if_needed();
    }

    pub fn has_finished(&self) -> bool {
        if self.died() {
            return true;
        }
        let timer_expired = self.seconds_remaining.is_some_and(|secs| secs <= 0.0);
        if self.is_continuous() {
            // words stream forever; only the clock ends a continuous test
            return timer_expired;
        }
        self.input.len() == self.char_count() || timer_expired
    }

    /// Death mode ends the test on the first mistake, ahead of the usual
    /// length and timer rules. A run that ends this way is not a result.
    pub fn died(&self) -> bool {
        self.death_mode && self.input.iter().any(|i| i.outcome == Outcome::Incorrect)
    }
}

#[cfg(test)]
impl Thok {
    /// A test that started 60 seconds ago, with `typed` typed against
    /// `prompt`: the i-th key `offsets_ms[i]` after the start, or right at
    /// the start if it has no offset.
    pub fn typed(prompt: &str, typed: &str, secs: Option<f64>, offsets_ms: &[u64]) -> Self {
        let mut thok = Self::new(prompt.to_string(), prompt.split(' ').count(), secs);
        let started_at = Instant::now() - Duration::from_secs(60);
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
        }
        thok
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    #[test]
    fn write_records_correct_and_incorrect() {
        let mut thok = Thok::new("hi".to_string(), 1, None);
        thok.write('h');
        thok.write('x');
        assert_eq!(thok.input[0].outcome, Outcome::Correct);
        assert_eq!(thok.input[1].outcome, Outcome::Incorrect);
        assert_eq!(thok.input.len(), 2);
        assert!(thok.started_at.is_some());
    }

    #[test]
    fn backspace_removes_last_input() {
        let mut thok = Thok::new("hi".to_string(), 1, None);
        thok.write('h');
        thok.write('x');
        thok.backspace();
        assert_eq!(thok.input.len(), 1);

        // backspace with no input must not panic and leaves the input empty
        let mut empty = Thok::new("hi".to_string(), 1, None);
        empty.backspace();
        assert!(empty.input.is_empty());
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
        // each tick takes TICK (100ms = 0.1s) off the countdown
        thok.on_tick();
        thok.on_tick();
        thok.on_tick();
        assert!(thok.has_finished());
    }

    #[test]
    fn death_mode_finishes_on_first_mistake() {
        let mut thok = Thok::new("hello world".to_string(), 2, None);
        thok.death_mode = true;
        thok.write('h');
        thok.write('e');
        assert!(!thok.has_finished(), "correct typing keeps the test alive");
        thok.write('x'); // expected 'l'
        assert!(thok.has_finished(), "a mistake ends a death-mode test");
    }

    #[test]
    fn died_only_after_a_mistake_in_death_mode() {
        let mut thok = Thok::new("hello".to_string(), 1, None);
        thok.death_mode = true;
        thok.write('h');
        assert!(!thok.died());
        thok.write('x');
        assert!(thok.died());

        let mut off = Thok::new("hello".to_string(), 1, None);
        off.write('x');
        assert!(!off.died(), "a mistake only kills a death-mode test");
    }

    #[test]
    fn death_mode_off_tolerates_mistakes() {
        let mut thok = Thok::new("hello".to_string(), 1, None);
        thok.write('x');
        assert!(!thok.has_finished());
    }

    #[test]
    fn death_mode_beats_a_running_timer() {
        let mut thok = Thok::new("hello".to_string(), 1, Some(10.0));
        thok.death_mode = true;
        thok.write('x');
        assert!(thok.has_finished());
    }

    /// How many lines the prompt wraps to at `width` chars a line.
    fn wrapped_lines(thok: &Thok, width: u16) -> usize {
        layout::wrap_chars(&thok.prompt_chars, width).len()
    }

    #[test]
    fn continuous_mode_keeps_a_full_window_of_lines_across_writes() {
        for width in [20, 40, 70, 150] {
            for prompt in ["", "a b", "word word word word word word word word"] {
                let mut thok = Thok::new(prompt.to_string(), 2, Some(30.0));
                thok.refill_words = vec!["word".to_string(), "longerword".to_string()];
                thok.set_line_width(width);
                thok.refill_if_needed();

                // the lines the screen shows: the window is a fixed size
                // as long as the prompt wraps to at least that many
                assert!(wrapped_lines(&thok, width) >= MAX_VISIBLE_LINES, "{width}");
                for _ in 0..300 {
                    thok.write('x');
                    let lines = wrapped_lines(&thok, width);
                    let cursor_line = layout::char_cell(
                        &layout::wrap_chars(&thok.prompt_chars, width),
                        thok.input.len(),
                    )
                    .unwrap()
                    .0;
                    assert!(
                        lines >= cursor_line + MAX_VISIBLE_LINES,
                        "window shrinks at width {width} after {} keys",
                        thok.input.len()
                    );
                }
            }
        }
    }

    #[test]
    fn continuous_mode_refills_when_the_screen_gets_narrower() {
        let mut thok = Thok::new(String::new(), 0, Some(30.0));
        thok.refill_words = vec!["word".to_string()];
        thok.set_line_width(200);
        thok.refill_if_needed();
        thok.set_line_width(20);
        assert!(wrapped_lines(&thok, 20) >= MAX_VISIBLE_LINES);
    }

    #[test]
    fn continuous_mode_finishes_only_on_the_timer() {
        let mut thok = Thok::new("ab".to_string(), 1, Some(0.2));
        thok.refill_words = vec!["word".to_string()];
        thok.write('a');
        thok.write('b');
        assert!(!thok.has_finished(), "length must not end a timed test");
        thok.on_tick();
        thok.on_tick();
        assert!(thok.has_finished());
    }

    #[test]
    fn continuous_mode_starts_an_empty_prompt_without_a_space() {
        let mut thok = Thok::new(String::new(), 0, Some(5.0));
        thok.refill_words = vec!["word".to_string()];
        thok.refill_if_needed();
        assert!(thok.char_count() > 0);
        assert!(!thok.prompt.starts_with(' '));
        // the first key must not index past the end of an empty prompt
        thok.write('w');
        assert_eq!(thok.input[0].outcome, Outcome::Correct);
    }

    #[test]
    fn death_mode_ends_a_continuous_test_on_a_mistake() {
        let mut thok = Thok::new("hello".to_string(), 1, Some(10.0));
        thok.refill_words = vec!["word".to_string()];
        thok.death_mode = true;
        thok.refill_if_needed();
        thok.write('h');
        assert!(!thok.has_finished());
        thok.write('x');
        assert!(thok.has_finished(), "a mistake still ends a -d -s run");
        assert!(thok.died());
    }

    #[test]
    fn calc_results_accuracy() {
        let mut thok = Thok::typed("hello", "hellx", None, &[100, 300, 500, 700, 900]);
        thok.calc_results();
        // 4 of 5 correct => 80%
        assert_eq!(thok.accuracy, 80.0);
    }

    #[test]
    fn calc_results_wpm_all_correct() {
        let mut thok = Thok::typed("hello", "hello", None, &[200, 400, 600, 800, 950]);
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
        let offsets = [100, 300, 500, 1200, 1400, 2350];
        let mut thok = Thok::typed("abcdef", "abcdef", None, &offsets);
        thok.calc_results();
        assert!(!thok.wpm_coords.is_empty());
        assert!(thok.wpm_coords.iter().all(|(x, _)| x.is_finite()));
        let last_x = thok.wpm_coords.last().unwrap().0;
        assert!(last_x >= 2.0);
        assert!(last_x < 60.0);
        assert!(thok.wpm > 0.0);
    }

    #[test]
    fn pace_index_advances_linearly() {
        let mut thok = Thok::new("abcdefghijklmnopqrstuvwxyz".to_string(), 1, None);
        thok.pace_wpm = Some(60.0);
        assert_eq!(thok.pace_caret_index_at(0.0), Some(0));
        assert_eq!(thok.pace_caret_index_at(1.0), Some(5));
        assert_eq!(thok.pace_caret_index_at(2.5), Some(12));
    }

    #[test]
    fn pace_index_none_when_finished() {
        let mut thok = Thok::new("abcdefghijklmnopqrstuvwxyz".to_string(), 1, None);
        thok.pace_wpm = Some(60.0);
        assert_eq!(thok.pace_caret_index_at(1000.0), None);
    }

    #[test]
    fn pace_index_none_when_off() {
        let thok = Thok::new("abcdefghijklmnopqrstuvwxyz".to_string(), 1, None);
        assert_eq!(thok.pace_caret_index_at(1.0), None);
    }

    #[test]
    fn pace_index_none_before_start() {
        let mut thok = Thok::new("abcdefghijklmnopqrstuvwxyz".to_string(), 1, None);
        thok.pace_wpm = Some(60.0);
        thok.started_at = None;
        assert_eq!(thok.pace_caret_index(), None);
    }

    #[test]
    fn tick_without_timer_is_noop() {
        let mut thok = Thok::new("abc".to_string(), 1, None);
        assert_eq!(thok.number_of_secs, None);
        thok.on_tick();
        assert_eq!(thok.seconds_remaining, None);
        assert!(!thok.has_finished());
    }

    #[test]
    fn calc_results_empty_input_does_not_panic() {
        let mut thok = Thok::new("hello".to_string(), 1, None);
        thok.started_at = Some(Instant::now() - Duration::from_secs(60));
        thok.calc_results();
        assert_eq!(thok.wpm, 0.0);
        assert_eq!(thok.std_dev, 0.0);
        assert_eq!(thok.accuracy, 0.0);
    }
}
