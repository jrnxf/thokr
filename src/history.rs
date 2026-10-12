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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    /// A log as thokr 0.5.0 wrote it: a word test, then a timed test.
    const OLD_LOG: &str = "\
date,num_words,num_secs,elapsed_secs,wpm,accuracy,std_dev
Mon Oct 12 10:19:27 2026,15,,3.42,64,95,1.70
Mon Oct 12 10:19:44 2026,15,3.00,2.99,72,100,0.00
";

    fn finished_thok(number_of_secs: Option<f64>) -> Thok {
        let mut thok = Thok::new("the quick brown fox".to_string(), 4, number_of_secs);
        thok.elapsed_secs = 5.2549;
        thok.wpm = 36.0;
        thok.accuracy = 95.0;
        thok.std_dev = 1.0198;
        thok
    }

    fn date() -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2026, 10, 5)
            .unwrap()
            .and_hms_opt(9, 7, 3)
            .unwrap()
    }

    /// Reads a row back column by column, the way a spreadsheet or a script would.
    fn parse(row: &str) -> (NaiveDateTime, usize, Option<f64>, f64, u32, u32, f64) {
        let columns: Vec<&str> = row.split(',').collect();
        assert_eq!(columns.len(), HEADER.split(',').count(), "{row}");
        (
            NaiveDateTime::parse_from_str(columns[0], "%c").unwrap(),
            columns[1].parse().unwrap(),
            (!columns[2].is_empty()).then(|| columns[2].parse().unwrap()),
            columns[3].parse().unwrap(),
            columns[4].parse().unwrap(),
            columns[5].parse().unwrap(),
            columns[6].parse().unwrap(),
        )
    }

    #[test]
    fn row_keeps_the_format_of_existing_logs() {
        assert_eq!(
            row(date(), &finished_thok(None)),
            "Mon Oct  5 09:07:03 2026,4,,5.25,36,95,1.02"
        );
        assert_eq!(
            row(date(), &finished_thok(Some(30.0))),
            "Mon Oct  5 09:07:03 2026,4,30.00,5.25,36,95,1.02"
        );
    }

    #[test]
    fn old_and_new_rows_parse_the_same_way() {
        let mut rows = OLD_LOG.lines();
        assert_eq!(rows.next(), Some(HEADER));
        let word_test = parse(rows.next().unwrap());
        assert_eq!((word_test.1, word_test.2, word_test.4), (15, None, 64));
        let timed_test = parse(rows.next().unwrap());
        assert_eq!((timed_test.2, timed_test.3), (Some(3.0), 2.99));

        let new = parse(&row(date(), &finished_thok(Some(30.0))));
        assert_eq!(new, (date(), 4, Some(30.0), 5.25, 36, 95, 1.02));
    }

    #[test]
    fn append_adds_to_an_existing_log_and_starts_a_new_one_with_the_header() {
        let folder = std::env::temp_dir().join(format!("thokr-test-{}", std::process::id()));
        let new_row = row(date(), &finished_thok(None));

        // a log from an older version is appended to, not rewritten
        let existing = folder.join("existing").join("log.csv");
        fs::create_dir_all(existing.parent().unwrap()).unwrap();
        fs::write(&existing, OLD_LOG).unwrap();
        append_row(&existing, &new_row).unwrap();
        assert_eq!(
            fs::read_to_string(&existing).unwrap(),
            format!("{OLD_LOG}{new_row}\n")
        );

        // a first run creates the folder and the file, header first
        let fresh = folder.join("fresh").join("thokr").join("log.csv");
        append_row(&fresh, &new_row).unwrap();
        append_row(&fresh, &new_row).unwrap();
        assert_eq!(
            fs::read_to_string(&fresh).unwrap(),
            format!("{HEADER}\n{new_row}\n{new_row}\n")
        );

        fs::remove_dir_all(&folder).unwrap();
    }

    #[test]
    fn append_reports_a_log_that_cannot_be_written() {
        // a folder cannot be created underneath a regular file
        let blocker = std::env::temp_dir().join(format!("thokr-test-file-{}", std::process::id()));
        fs::write(&blocker, "").unwrap();
        assert!(append_row(&blocker.join("thokr").join("log.csv"), "row").is_err());
        fs::remove_file(&blocker).unwrap();
    }
}
