use cgisf_lib::cgisf;
use clap::ValueEnum;
use rand::{Rng, seq::SliceRandom};

// A word list is compiled into the binary as text, one word per line.
#[derive(Debug, Copy, Clone, ValueEnum)]
pub enum Language {
    English,
    English1k,
    English10k,
}

impl Language {
    fn words(self) -> &'static str {
        match self {
            Self::English => include_str!("lang/english.txt"),
            Self::English1k => include_str!("lang/english1k.txt"),
            Self::English10k => include_str!("lang/english10k.txt"),
        }
    }

    /// `count` different words in random order, joined by spaces. A list
    /// shorter than `count` gives every word it has.
    pub fn random_words(self, count: usize) -> String {
        let words: Vec<&str> = self.words().lines().collect();
        let picked: Vec<&str> = words
            .choose_multiple(&mut rand::thread_rng(), count)
            .copied()
            .collect();
        picked.join(" ")
    }
}

/// `count` random sentences joined by spaces, and how many words they hold.
pub fn random_sentences(count: usize) -> (String, usize) {
    let mut rng = rand::thread_rng();
    let mut text = String::new();
    for _ in 0..count {
        text.push_str(&cgisf(
            rng.gen_range(1..3),
            rng.gen_range(1..3),
            rng.gen_range(1..5),
            rng.gen_bool(0.5),
            rng.gen_range(1..3),
            rng.gen_bool(0.5),
        ));
    }
    // every sentence ends with a space, so the spaces count the words
    let word_count = text.matches(' ').count();
    text.pop();
    (text, word_count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn word_lists_hold_one_word_per_line() {
        for (language, size) in [
            (Language::English, 200),
            (Language::English1k, 999),
            (Language::English10k, 9954),
        ] {
            let words: Vec<&str> = language.words().lines().collect();
            assert_eq!(words.len(), size, "{language:?}");
            assert!(
                words.iter().all(|w| !w.is_empty() && !w.contains(' ')),
                "{language:?} has a blank line or a space"
            );
        }
    }

    #[test]
    fn random_words_are_distinct_words_of_the_list() {
        let list: HashSet<&str> = Language::English.words().lines().collect();
        let prompt = Language::English.random_words(15);
        let words: Vec<&str> = prompt.split(' ').collect();
        assert_eq!(words.len(), 15);
        assert_eq!(words.iter().collect::<HashSet<_>>().len(), 15);
        assert!(words.iter().all(|w| list.contains(w)));
    }

    #[test]
    fn random_words_stop_at_the_size_of_the_list() {
        let prompt = Language::English.random_words(500);
        assert_eq!(prompt.split(' ').count(), 200);
        assert_eq!(Language::English.random_words(0), "");
    }

    #[test]
    fn random_sentences_count_their_words() {
        let (text, word_count) = random_sentences(3);
        assert_eq!(text.split(' ').count(), word_count);
        assert_eq!(text.matches('.').count(), 3);
        assert!(text.ends_with('.'));
        assert_eq!(random_sentences(0), (String::new(), 0));
    }
}
