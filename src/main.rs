#![feature(proc_macro_hygiene, decl_macro)]
#[macro_use] extern crate rocket;
#[macro_use] extern crate lazy_static;

use rocket::request::Form;
use rocket::State;
use rocket_contrib::json::JsonValue;
use rocket_contrib::templates::Template;
use std::error::Error;
#[allow(dead_code)]
use std::path::{Path, PathBuf};

use rocket_contrib::serve::StaticFiles;
use chrono::{DateTime, Utc};
use serde::Serialize;
use std::ffi::OsStr;
use rocket::response::Redirect;
use regex::Regex;
use indicatif::ProgressBar;
use rayon::iter::{ParallelIterator, IntoParallelRefIterator};

mod crypto;
mod cli;
mod checksum;

#[derive(FromForm, Clone)]
struct Document {
    filename: String,
    date: String,
    institution: String,
    name: String,
    page: String,
    encrypt_it: bool,
}

struct OptDoc {
    date: Option<String>,
    institution: Option<String>,
    name: Option<String>,
    page: Option<String>,
}

fn main() -> Result<(), Box<dyn Error>> {
    let config = cli::get_program_input();
    if config.verify {
        let files: Vec<PathBuf> = Path::new(&config.target_directory)
            .read_dir()
            .expect("read_dir call failed")
            .map(|x| x.unwrap().path())
            .filter(|x| Path::new(x).is_file())
            .collect();

        let not_checksum_files: Vec<&PathBuf> = files.iter()
            .filter(|x| {
                    x.extension().unwrap_or(OsStr::new("")) != "sha256"
            })
            .collect();

        let checksum_files: Vec<&PathBuf> = files.iter()
            .filter(|x| {
                    x.extension().unwrap_or(OsStr::new("")) == "sha256"
            })
            .collect();

        let missing_checksum: Vec<String> = not_checksum_files.iter()
            .filter(|path| {
                // Check if corresponding checksum is available.
                let mut p: PathBuf = (**path).to_owned();
                let mut ext = p.extension().unwrap_or(OsStr::new("")).to_owned();
                ext.push(".sha256");
                p.set_extension(ext);
                ! checksum_files.contains(&&p)
            })
            .map(|p| p.to_str().unwrap().to_owned())
            .collect();

        let results: Vec<bool> = checksum_files.par_iter()
            .map( |c| {
                let is_valid = checksum::validate_sha256(c).unwrap();
                println!("Validating \"{}\"... {}", c.to_str().unwrap(), if is_valid {"OK"} else {"FAILED"});
                is_valid
            })
            .collect();

        let successes = results.iter()
            .filter(|is_valid| **is_valid)
            .count();

        let failures = results.iter()
            .filter(|is_valid| !*is_valid)
            .count();

        println!("--------------------");
        println!("Successes: {}", successes);
        println!("Failures: {}", failures);
        println!("Missing checksums: {}", missing_checksum.join("\n    "));
        std::process::exit(0);
    }

    // If there's a specific file we should decrypt, do that.
    if let Some(paths) = &config.file_to_decrypt {
        let pass = &config.password.clone().unwrap();
        let included: Vec<String> = paths.iter().filter(|p| p.ends_with(crypto::ENCRYPTION_FILE_EXT)).map(String::to_string).collect();
        let excluded: Vec<String> = paths.iter().filter(|p| {! p.ends_with(crypto::ENCRYPTION_FILE_EXT)}).map(String::to_string).collect();
        let pb = ProgressBar::new(included.len() as u64);
        pb.set_position(0); // Start drawing progress bar
        included.par_iter().map(|path| {
            match crypto::decrypt_file(path, pass) {
                Err(s) => pb.println(format!("ERROR {}", s)),
                Ok(_) => pb.println(format!("Decrypted {}", path))
            }
            pb.inc(1);
        }).collect::<Vec<_>>();
        pb.finish();
        let bullet_pt = "\n    ";
        println!("Successfully decrypted files:{}{}", bullet_pt, included.join(bullet_pt));
        println!("Ignored:{}{}", bullet_pt, excluded.join(bullet_pt));
        std::process::exit(0);
    }

    // If there's a specific file we should encrypt, do that too.
    if let Some(paths) = &config.file_to_encrypt {
        let pass = &config.password.clone().unwrap();
        let exclude_files = |p: &str| { p.ends_with(crypto::ENCRYPTION_FILE_EXT) || p.ends_with(".sha256")};
        let included: Vec<String> = paths.iter().filter(|p| {! exclude_files(p)} ).map(String::to_string).collect();
        let excluded: Vec<String> = paths.iter().filter(|p| exclude_files(p)).map(String::to_string).collect();
        let pb = ProgressBar::new(included.len() as u64);
        pb.set_position(0); // Start drawing progress bar
        included.par_iter().map(|path| {
            let target = crypto::get_encrypted_name(path);
            match crypto::encrypt_file(path, target, pass) {
                Err(s) => pb.println(format!("ERROR {}", s)),
                Ok(_) => {
                    pb.println(format!("Encrypted {}", path));
                    checksum::generate_sha256(Path::new(path).to_path_buf()).unwrap();
                }
            }
            pb.inc(1);
        }).collect::<Vec<_>>();
        pb.finish();
        let bullet_pt = "\n    ";
        println!("Successfully encrypted files:{}{}", bullet_pt, included.join(bullet_pt));
        println!("Ignored:{}{}", bullet_pt, excluded.join(bullet_pt));
        std::process::exit(0);
    }

    if let Some(paths) = &config.normalize {
        paths.iter()
            .map(Path::new)
            .filter(|p| p.is_file())
            .map(|source| {
                let doc: OptDoc = to_document(source);
                let extension: String = source.extension()
                    .and_then(std::ffi::OsStr::to_str)
                    .map(|s| s.to_ascii_lowercase())
                    .unwrap_or(String::new());
                println!("\tParsing {:?}", source);
                let target = Path::new(&config.target_directory)
                    .join(format!("{}_{}_{}_{}.{}",
                        doc.date.expect("date error"),
                        doc.institution.expect("institution error"),
                        doc.name.expect("name error"),
                        doc.page.unwrap_or("1".to_owned()),
                        extension));
                println!("Renaming {:?} to {:?}", source, target);
                std::fs::rename(source, target).unwrap();
            }).for_each(drop);
        std::process::exit(0);
    }

    if config.launch_web {
        rocket::ignite()
            .mount("/node_modules", StaticFiles::from("node_modules"))
            .mount("/static", StaticFiles::from("static"))
            .mount("/documents", StaticFiles::from("documents"))
            .mount("/", routes![index, get_docs, get_doc, new])
            .manage(config)
            .attach(Template::fairing())
            .launch();
    }
    Ok(())
}

#[derive(Serialize, Debug)]
struct Context {
  filename: String,
  name: String,
  date: String,
  institution: String,
  files: Vec<String>,
  target_directory: String,
  page: String
}

#[get("/")]
fn index(config: State<cli::Config>) -> Template {
    get_doc(config, String::new())
}

#[get("/doc")]
fn get_docs(config: State<cli::Config>) -> JsonValue {
    println!("GET /doc -- listing files in '{}'", &config.target_directory);
    let mut docs = list_files(&PathBuf::from(&config.target_directory));
    docs.sort();
    JsonValue(serde_json::json!(docs))
}

#[get("/doc/<filename>")]
fn get_doc(config: State<cli::Config>, filename: String) -> Template {
    let now: DateTime<Utc> = Utc::now();
    let mut files: Vec<String> = list_files(&PathBuf::from(&config.target_directory));
    files.sort();
    if filename.ends_with(crypto::ENCRYPTION_FILE_EXT) {
        // If file is encrypted, decrypt to temporary dir and return new file
        let path = Path::new(&config.target_directory).join(filename.clone());
        let pass = config.password.clone().unwrap();
        if let Err(e) = crypto::decrypt_file(path.to_str().unwrap(), &pass) {
            // TODO: Return appropriate HTTP error in response.
        }
        let unencrypted_path: PathBuf = crypto::get_decrypted_name(path);
        let unencrypted_name = unencrypted_path.file_name().unwrap().to_str().unwrap();
        let date = parse_date(&unencrypted_name).unwrap();
        let doc = to_document(&unencrypted_path.to_str().unwrap());

        let target_directory = unencrypted_path.parent().unwrap().to_str().unwrap().to_owned().replace("/", "\\u{002F}");
        let new_context = Context {
            filename: unencrypted_name.to_string(),
            name: doc.name.unwrap_or(String::new()),
            date: doc.date.unwrap_or(date),
            institution: doc.institution.unwrap_or(String::new()),
            page: doc.page.unwrap_or(String::from("1")),
            files: files,
            target_directory: target_directory
        };
        return Template::render("index", &new_context);
    } else {
        let date = parse_date(&filename.as_ref()).unwrap_or(now.format("%Y-%m-%d").to_string());
        let doc = to_document(&filename);
        let context = Context {
            filename: filename,
            name: doc.name.unwrap_or(String::new()),
            date: doc.date.unwrap_or(date),
            institution: doc.institution.unwrap_or(String::new()),
            page: doc.page.unwrap_or(String::from("1")),
            files: files,
            target_directory: config.target_directory.clone()
        };
        return Template::render("index", &context);
    }
}

#[post("/doc", data = "<doc>")]
fn new(config: State<cli::Config>, doc: Form<Document>) -> Result<Redirect, Box<dyn Error>> {
    if doc.encrypt_it {
        let filename = Path::new(&config.target_directory)
            .join(format!("{}_{}_{}_{}.cocoon", doc.date, doc.institution, doc.name, doc.page));
        let checksum_name = filename.clone();
        let password = config.password.clone().unwrap();
        let source = Path::new(&config.target_directory).join(&doc.filename);
        crypto::encrypt_file(source, filename, &password)?;
        checksum::generate_sha256(checksum_name).unwrap();
    } else {
        // Normalize the filename
        let source = Path::new(&config.target_directory).join(&doc.filename);
        let extension: String = source.extension()
            .and_then(std::ffi::OsStr::to_str)
            .map(|s| s.to_ascii_lowercase())
            .unwrap_or(String::new());
        let target = Path::new(&config.target_directory)
            .join(format!("{}_{}_{}_{}.{}", doc.date, doc.institution, doc.name, doc.page, extension));
        std::fs::rename(source, target)?;
    }
    Ok(Redirect::to("/"))
}

fn list_files(path: &PathBuf) -> Vec<String> {
    if !path.exists() {
        return Vec::new();
    }
    path.read_dir()
        .expect("read_dir call failed")
        .map(|x| x.unwrap().path())
        .filter(|x| Path::new(x).is_file())
        .filter(|x| {
                let ext: String = x.extension()
                    .and_then(std::ffi::OsStr::to_str)
                    .map(|s| s.to_ascii_lowercase())
                    .unwrap_or(String::new());
                ext == "pdf" ||
                ext == "jpg" ||
                ext == "png" ||
                ext == "cocoon"
            })
        .map(|x| x.file_name().unwrap().to_str().unwrap().to_owned())
        .collect()
}

lazy_static! {
    static ref RE_WITH_HYPHENS: Regex = Regex::new(r"^(?P<year>\d{4})-(?P<month>\d{2})-(?P<day>\d{2})").unwrap();
    static ref RE_NO_HYPHENS: Regex = Regex::new(r"^(?P<year>\d{4})(?P<month>\d{2})(?P<day>\d{2})").unwrap();
    static ref RE_YEAR_ONLY: Regex = Regex::new(r"^(?P<year>\d{4})").unwrap();
}

fn parse_date(text: &&str) -> Option<String> {
    // Returns the parsed date in ISO8601 format
    RE_WITH_HYPHENS.captures(text).map(|x|
        format!("{}-{}-{}",
            x.name("year").unwrap().as_str(),
            x.name("month").unwrap().as_str(),
            x.name("day").unwrap().as_str(),
        )
    ).or(
        RE_NO_HYPHENS.captures(text).map(|x|
            format!("{}-{}-{}",
                x.name("year").unwrap().as_str(),
                x.name("month").unwrap().as_str(),
                x.name("day").unwrap().as_str(),
            )
        )
    ).or(
        RE_YEAR_ONLY.captures(text).map(|x|
            format!("{}-{}-{}",
                x.name("year").unwrap().as_str(),
                x.name("month").map(|m|m.as_str()).unwrap_or("01"),
                x.name("day").map(|m|m.as_str()).unwrap_or("01"),
            )
        )
    )
}

lazy_static! {
    static ref RE_PARSE_PAGE: Regex = Regex::new(r"(\d+)").unwrap();
}

fn parse_page(text: &&str) -> Option<String> {
    RE_PARSE_PAGE.captures(text).and_then(|c| c.get(1)).map(|m| m.as_str().to_owned())
}

#[test]
fn test_parse_page() {
    assert_eq!(parse_page(&""), None);
    assert_eq!(parse_page(&"pg"), None);
    assert_eq!(parse_page(&"01"), Some("01".to_owned()));
    assert_eq!(parse_page(&"20"), Some("20".to_owned()));
    assert_eq!(parse_page(&"pg20"), Some("20".to_owned()));
}

fn to_document<T: AsRef<Path>>(filename: T) -> OptDoc {
    let filename = filename.as_ref();
    let filestem: &str = filename.file_stem()
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


#[test]
fn test_parse_date_hyphens() {
    assert_eq!(parse_date(&"2020-04-03_boop_loop"), Some("2020-04-03".to_string()))
}

#[test]
fn test_parse_date_no_hyphens() {
    assert_eq!(parse_date(&"20180530_boop_loop"), Some("2018-05-30".to_string()))
}
#[test]
fn test_parse_date_year_only() {
    assert_eq!(parse_date(&"2018_boop_loop"), Some("2018-01-01".to_string()))
}
