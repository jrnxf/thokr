use cgisf_lib::cgisf;
use rand::seq::SliceRandom;
use serde::Deserialize;
use serde_json::from_str;

use include_dir::{include_dir, Dir};
use rand::Rng;
use std::error::Error;

static LANG_DIR: Dir = include_dir!("src/lang");

#[allow(dead_code)]
#[derive(Deserialize, Clone, Debug)]
pub struct Language {
    name: String,
    size: u32,
    words: Vec<String>,
}

impl Language {
    pub fn new(file_name: String) -> Self {
        read_language_from_file(format!("{}.json", file_name)).unwrap()
    }

    pub fn get_random_sentence(&self, num: usize) -> (Vec<String>, usize) {
        let rng = &mut rand::thread_rng();
        let mut vec = Vec::new();
        let mut word_count = 0;
        for _ in 0..num {
            let s = cgisf(
                rng.gen_range(1..3),
                rng.gen_range(1..3),
                rng.gen_range(1..5),
                rng.gen_bool(0.5),
                rng.gen_range(1..3),
                rng.gen_bool(0.5),
            );
            word_count += &s.matches(' ').count();
            // gets the word count of the sentence.
            vec.push(s);
        }
        (vec, word_count)
    }

    pub fn get_random(&self, num: usize) -> Vec<String> {
        let mut rng = &mut rand::thread_rng();

        self.words.choose_multiple(&mut rng, num).cloned().collect()
    }
}

fn read_language_from_file(file_name: String) -> Result<Language, Box<dyn Error>> {
    let file = LANG_DIR
        .get_file(file_name)
        .expect("Language file not found");

    let file_as_str = file
        .contents_utf8()
        .expect("Unable to interpret file as a string");

    let lang = from_str(file_as_str).expect("Unable to deserialize language json");

    Ok(lang)
}
