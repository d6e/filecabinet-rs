use regex::Regex;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

pub struct OptDoc {
    pub(crate) date: Option<String>,
    pub(crate) institution: Option<String>,
    pub(crate) name: Option<String>,
    pub(crate) page: Option<String>,
}

// TODO: use async paths
pub fn list_files(path: &PathBuf) -> Vec<String> {
    if !path.exists() {
        return Vec::new();
    }
    path.read_dir()
        .expect("read_dir call failed")
        .map(|x| x.unwrap().path())
        .filter(|x| Path::new(x).is_file())
        .filter(|x| {
            let ext: String = x
                .extension()
                .and_then(std::ffi::OsStr::to_str)
                .map(|s| s.to_ascii_lowercase())
                .unwrap_or(String::new());
            ext == "pdf" || ext == "jpg" || ext == "png" || ext == "cocoon"
        })
        .map(|x| x.file_name().unwrap().to_str().unwrap().to_owned())
        .collect()
}

pub fn to_document<T: AsRef<Path>>(filename: T) -> OptDoc {
    let filename = filename.as_ref();
    let filestem: &str = filename
        .file_stem()
        .and_then(OsStr::to_str)
        .unwrap_or(filename.to_str().unwrap());
    let v: Vec<&str> = filestem.split('_').collect();
    OptDoc {
        date: v.get(0).and_then(parse_date),
        institution: v.get(1).map(|x| x.to_string()),
        name: v.get(2).map(|x| x.to_string()),
        page: v.get(3).and_then(parse_page),
    }
}

lazy_static! {
    static ref RE_PARSE_PAGE: Regex = Regex::new(r"(\d+)").unwrap();
}

fn parse_page(text: &&str) -> Option<String> {
    RE_PARSE_PAGE
        .captures(text)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_owned())
}

#[test]
fn test_parse_page() {
    assert_eq!(parse_page(&""), None);
    assert_eq!(parse_page(&"pg"), None);
    assert_eq!(parse_page(&"01"), Some("01".to_owned()));
    assert_eq!(parse_page(&"20"), Some("20".to_owned()));
    assert_eq!(parse_page(&"pg20"), Some("20".to_owned()));
}

lazy_static! {
    static ref RE_WITH_HYPHENS: Regex =
        Regex::new(r"^(?P<year>\d{4})-(?P<month>\d{2})-(?P<day>\d{2})").unwrap();
    static ref RE_NO_HYPHENS: Regex =
        Regex::new(r"^(?P<year>\d{4})(?P<month>\d{2})(?P<day>\d{2})").unwrap();
    static ref RE_YEAR_ONLY: Regex = Regex::new(r"^(?P<year>\d{4})").unwrap();
}

pub fn parse_date(text: &&str) -> Option<String> {
    // Returns the parsed date in ISO8601 format
    RE_WITH_HYPHENS
        .captures(text)
        .map(|x| {
            format!(
                "{}-{}-{}",
                x.name("year").unwrap().as_str(),
                x.name("month").unwrap().as_str(),
                x.name("day").unwrap().as_str(),
            )
        })
        .or(RE_NO_HYPHENS.captures(text).map(|x| {
            format!(
                "{}-{}-{}",
                x.name("year").unwrap().as_str(),
                x.name("month").unwrap().as_str(),
                x.name("day").unwrap().as_str(),
            )
        }))
        .or(RE_YEAR_ONLY.captures(text).map(|x| {
            format!(
                "{}-{}-{}",
                x.name("year").unwrap().as_str(),
                x.name("month").map(|m| m.as_str()).unwrap_or("01"),
                x.name("day").map(|m| m.as_str()).unwrap_or("01"),
            )
        }))
}

#[test]
fn test_parse_date_hyphens() {
    assert_eq!(
        parse_date(&"2020-04-03_boop_loop"),
        Some("2020-04-03".to_string())
    )
}

#[test]
fn test_parse_date_no_hyphens() {
    assert_eq!(
        parse_date(&"20180530_boop_loop"),
        Some("2018-05-30".to_string())
    )
}
#[test]
fn test_parse_date_year_only() {
    assert_eq!(
        parse_date(&"2018_boop_loop"),
        Some("2018-01-01".to_string())
    )
}
