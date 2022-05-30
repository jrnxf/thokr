use serde::{Deserialize, Serialize};
use serde_json::Result;
use std::{collections::HashMap, io::Read};

const API: &str = "https://en.wikipedia.org/w/api.php?action=query&format=json";

#[derive(Serialize, Deserialize)]
struct Response {
    batchcomplete: String,

    #[serde(rename = "continue", skip)]
    _cont: Option<Continue>,

    query: Query,
}

#[derive(Serialize, Deserialize)]
struct Continue {
    rncontinue: String,

    #[serde(rename = "continue")]
    continue_: String,
}

#[derive(Serialize, Deserialize)]
struct Query {
    random: Option<Vec<Page>>,
    pages: Option<HashMap<String, Page>>,
}

#[derive(Serialize, Deserialize)]
pub struct Page {
    #[serde(alias = "pageid")]
    pub id: u64,

    #[serde(skip)]
    _ns: i32,

    pub title: String,
    pub extract: Option<String>,
}

/// Gets one random page from Wikipedia, using the API, and returns that page's ID
fn random_id() -> Option<u64> {
    // Get one random page
    let mut res =
        match reqwest::blocking::get(API.to_string() + "&list=random&rnnamespace=0&rnlimit=1") {
            Ok(r) => r,
            Err(_e) => {
                println!("There was an error fetching Wikipedia's API! Do you have a connection?");
                return None;
            }
        };

    // Get the body of the response
    let mut body = String::new();
    res.read_to_string(&mut body).unwrap();

    // Try to parse and return the ID of the page
    let result: Result<Response> = serde_json::from_str(body.as_str());
    match result {
        Ok(x) => Some(match x.query.random {
            Some(pages) => pages[0].id,
            None => 0,
        }),
        Err(_e) => {
            println!("There was an error when parsing Wikipedia's API response for getting a random page.");
            println!("Please file a bug report saying that.");
            None
        }
    }
}

/// Returns a random article from Wikipedia, in the format of the Page struct
pub fn random_article() -> Option<Page> {
    // Get the page ID
    let page_id = match random_id() {
        Some(x) => x,
        None => return None,
    };

    // Request the content of the page
    let mut res = match reqwest::blocking::get(
        API.to_string() + "&prop=extracts&exintro&explaintext&pageids=" + &page_id.to_string(),
    ) {
        Ok(r) => r,
        Err(_e) => {
            println!("There was an error fetching Wikipedia's API! Do you have a connection?");
            return None;
        }
    };

    // Get the body of the response
    let mut body = String::new();
    res.read_to_string(&mut body).unwrap();

    // Parse the result
    let resp: Response = match serde_json::from_str(body.as_str()) {
        Ok(r) => r,
        Err(_e) => {
            println!("There was an error when parsing Wikipedia's API response for getting a page from an ID.");
            println!("Please file a bug report saying that.");
            return None;
        }
    };

    // Return the page if available
    match resp.query.pages {
        Some(p) => match p.into_iter().next() {
            Some(page) => Some(page.1),
            None => {
                println!("Wikipedia's API returned empty content for a page. Please try again.");
                println!(
                    "If the problem persists, please file a bug report describing the problem."
                );
                None
            }
        },
        None => {
            println!("Wikipedia's API returned no pages. Please try again.");
            println!("Please file a bug report saying that.");
            None
        }
    }
}
