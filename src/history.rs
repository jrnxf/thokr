//! The results history: one CSV row per finished test, appended to `log.csv`
//! in the platform's config folder. People keep these files across versions,
//! so the columns and their formats must not change.

use crate::thok::Thok;
use chrono::{Local, NaiveDateTime};
use directories::ProjectDirs;
use std::{
    fs::{self, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
};

const HEADER: &str = "date,num_words,num_secs,elapsed_secs,wpm,accuracy,std_dev";

/// Where the log lives; `None` when the system has no home directory.
pub fn path() -> Option<PathBuf> {
    ProjectDirs::from("", "", "thokr").map(|dirs| dirs.config_dir().join("log.csv"))
}

/// Appends the results of a finished test to the log.
pub fn append(thok: &Thok) -> io::Result<()> {
    match path() {
        Some(path) => append_row(&path, &row(Local::now().naive_local(), thok)),
        None => Ok(()),
    }
}

fn row(finished_at: NaiveDateTime, thok: &Thok) -> String {
    format!(
        "{},{},{},{:.2},{},{},{:.2}",
        finished_at.format("%c"),
        thok.number_of_words,
        thok.number_of_secs
            .map_or_else(String::new, |secs| format!("{secs:.2}")),
        thok.elapsed_secs,
        thok.wpm,      // already a whole number
        thok.accuracy, // already a whole number
        thok.std_dev,
    )
}

/// Appends `row` to the file at `path`. A file that does not exist yet is
/// created, along with its folder, and starts with the header.
fn append_row(path: &Path, row: &str) -> io::Result<()> {
    if let Some(folder) = path.parent() {
        fs::create_dir_all(folder)?;
    }
    let is_new = !path.exists();
    let mut file = OpenOptions::new().append(true).create(true).open(path)?;
    if is_new {
        writeln!(file, "{HEADER}")?;
    }
    writeln!(file, "{row}")
}
