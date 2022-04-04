use log::info;
use rand::seq::SliceRandom;
use serde::Deserialize;
use serde_json::from_str;

use include_dir::{include_dir, Dir};
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

    pub fn get_random(&self, num: usize) -> Vec<String> {
        let mut rng = &mut rand::thread_rng();

        self.words.choose_multiple(&mut rng, num).cloned().collect()
    }
}

fn read_language_from_file(file_name: String) -> Result<Language, Box<dyn Error>> {
    info!("file_name {}", file_name);
    info!("LANG_DIR {:?}", LANG_DIR);
    let file = LANG_DIR
        .get_file(file_name)
        .expect("Language file not found");

    let file_as_str = file
        .contents_utf8()
        .expect("Unable to interpret file as a string");

    let lang = from_str(file_as_str).expect("Unable to deserialize language json");

    Ok(lang)
}
