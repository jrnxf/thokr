use std::ops::Range;

/// Greedy word-wrap over `chars` into lines of at most `width` cells.
/// Invariants: every char occupies exactly one cell on exactly one line
/// (1:1 index↔cell mapping — nothing dropped, nothing inserted); no line
/// exceeds `width`. A break is inserted before a maximal non-space run
/// ("word") that doesn't fit on the remainder of the current line but does
/// fit on an empty line; words longer than `width` hard-break at the width
/// boundary. Spaces are placed like any char (a line may end with spaces;
/// a continuation line starts with the word, since the break occurs before it).
///
/// Note: chars are assumed to be exactly one cell wide (no double-width/CJK
/// support), matching the renderer's assumption today.
pub fn wrap_chars(chars: &[char], width: u16) -> Vec<Range<usize>> {
    let width = width.max(1) as usize;
    let n = chars.len();
    let mut lines: Vec<Range<usize>> = Vec::new();
    if n == 0 {
        // an empty prompt still occupies a single (empty) line
        lines.push(0..0);
        return lines;
    }

    let mut line_start = 0usize; // index of first char on the current line
    let mut i = 0usize; // scan cursor

    while i < n {
        let col = i - line_start; // current column on the line (0-based)

        if chars[i] == ' ' {
            // spaces are placed like any char; if the line is already full,
            // wrap before placing this space.
            if col >= width {
                lines.push(line_start..i);
                line_start = i;
            }
            i += 1;
            continue;
        }

        // start of a (non-space) word: find its end
        let word_start = i;
        let mut word_end = i;
        while word_end < n && chars[word_end] != ' ' {
            word_end += 1;
        }
        let word_len = word_end - word_start;

        if word_len > width {
            // word longer than a whole line: hard-break at width boundaries.
            // first, if there's content already on this line, break before it.
            if col > 0 {
                lines.push(line_start..word_start);
            }
            // emit full-width chunks until the remaining word fits a line
            let mut chunk_start = word_start;
            while word_end - chunk_start > width {
                let chunk_end = chunk_start + width;
                lines.push(chunk_start..chunk_end);
                chunk_start = chunk_end;
            }
            line_start = chunk_start;
            i = word_end;
            continue;
        }

        // word fits on an empty line; does it fit on the remainder here?
        if col + word_len > width {
            // break before the word
            lines.push(line_start..word_start);
            line_start = word_start;
        }
        i = word_end;
    }

    // flush the final line
    lines.push(line_start..n);
    lines
}

/// (line, col) of char `idx` under the wrapping above. None if idx >= chars.len().
pub fn char_cell(chars: &[char], width: u16, idx: usize) -> Option<(usize, u16)> {
    if idx >= chars.len() {
        return None;
    }
    let lines = wrap_chars(chars, width);
    for (line_no, range) in lines.iter().enumerate() {
        if idx >= range.start && idx < range.end {
            return Some((line_no, (idx - range.start) as u16));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chars(s: &str) -> Vec<char> {
        s.chars().collect()
    }

    #[test]
    fn no_wrap_when_fits() {
        let c = chars("ab cd");
        let lines = wrap_chars(&c, 10);
        assert_eq!(lines, vec![0..5]);
        assert_eq!(char_cell(&c, 10, 4), Some((0, 4)));
    }

    #[test]
    fn breaks_before_word() {
        let c = chars("aaa bb");
        let lines = wrap_chars(&c, 4);
        assert_eq!(lines, vec![0..4, 4..6]);
        assert_eq!(char_cell(&c, 4, 3), Some((0, 3)));
        assert_eq!(char_cell(&c, 4, 4), Some((1, 0)));
    }

    #[test]
    fn hard_breaks_long_word() {
        let c = chars("abcdefgh");
        let lines = wrap_chars(&c, 3);
        assert_eq!(lines, vec![0..3, 3..6, 6..8]);
    }

    #[test]
    fn multiple_spaces_kept() {
        let c = chars("a  b");
        let lines = wrap_chars(&c, 10);
        assert_eq!(lines, vec![0..4]);
        assert_eq!(char_cell(&c, 10, 2), Some((0, 2)));
    }

    #[test]
    fn every_index_has_exactly_one_cell() {
        let cases = [
            ("ab cd", 10u16),
            ("aaa bb", 4),
            ("abcdefgh", 3),
            ("a  b", 10),
            ("the quick brown fox jumps", 7),
            ("supercalifragilistic word", 5),
        ];
        for (s, width) in cases {
            let c = chars(s);
            let lines = wrap_chars(&c, width);
            // contiguous, disjoint, cover 0..len
            assert_eq!(lines.first().unwrap().start, 0, "{s:?}@{width} starts at 0");
            assert_eq!(
                lines.last().unwrap().end,
                c.len(),
                "{s:?}@{width} covers end"
            );
            for w in lines.windows(2) {
                assert_eq!(w[0].end, w[1].start, "{s:?}@{width} contiguous/disjoint");
            }
            for r in &lines {
                assert!(
                    r.end - r.start <= width as usize,
                    "{s:?}@{width} line {r:?} exceeds width"
                );
            }
            // each in-range index maps to a cell within its line
            for idx in 0..c.len() {
                let (line, col) = char_cell(&c, width, idx).expect("in-range idx has a cell");
                assert!(col < width, "{s:?}@{width} col in bounds");
                assert!(line < lines.len(), "{s:?}@{width} line in bounds");
            }
            assert_eq!(
                char_cell(&c, width, c.len()),
                None,
                "{s:?}@{width} len = None"
            );
        }
    }
}
