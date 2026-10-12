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
