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
use std::fs::File;
use std::io::prelude::*;
use rocket_contrib::serve::StaticFiles;
use chrono::{DateTime, Utc};
use serde::Serialize;
use std::convert::AsRef;
use std::ffi::OsStr;
use rocket::response::Redirect;
use regex::Regex;
use std::fs;

mod crypto;
mod cli;
mod checksum;

const ENCRYPTION_FILE_EXT: &str = ".cocoon";

#[derive(FromForm, Clone)]
struct Document {
    filename: String,
    date: String,
    institution: String,
    name: String,
    page: String,
}

struct OptDoc {
    date: Option<String>,
    institution: Option<String>,
    name: Option<String>,
    page: Option<String>,
}

fn main() -> Result<(), Box<dyn Error>> {
    let config = cli::get_program_input();
    let pass = &config.password.clone().unwrap();

    // If there's a specific file we should decrypt, do that.
    if let Some(path) = &config.file_to_decrypt {
        if path.ends_with(ENCRYPTION_FILE_EXT) {
            let decrypted_path = get_decrypted_name(path);
            let mut encrypted_file = File::open(path).unwrap();
            let data = crypto::decrypt_file(&pass, &mut encrypted_file).unwrap();
            let mut unecrypted: File = File::create(decrypted_path).unwrap();
            unecrypted.write(&data).unwrap();
        } else {
            println!("Error: '{}' is not encrypted.", path);
        }
    }

    // If there's a specific file we should encrypt, do that too.
    if let Some(path) = &config.file_to_encrypt {
        if ! path.ends_with(ENCRYPTION_FILE_EXT) {
            let encrypted_path = get_encrypted_name(path);
            let mut unencrypted = File::open(path)?;
            let mut buffer = Vec::new();
            unencrypted.read_to_end(&mut buffer)?;
            crypto::encrypt_file(&pass, &mut File::create(encrypted_path.clone())?, buffer)?;
            checksum::generate_sha256(encrypted_path).unwrap();
        } else {
            println!("Error: '{}' is already encrypted.", path);
        }
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

fn get_decrypted_name<T: AsRef<Path>>(path: T) -> PathBuf {
    let path = path.as_ref();
    let mut filename = path.file_name().unwrap().to_str().unwrap().to_string();
    let basepath = path.parent().unwrap();
    // if it ends with the encryption extension, remove it.
    if filename.ends_with(ENCRYPTION_FILE_EXT) {
        filename = filename.replace(ENCRYPTION_FILE_EXT, "");
    }
    return basepath.join(filename);
}

fn get_encrypted_name<T: AsRef<Path>>(path: T) -> PathBuf {
    let path = path.as_ref();
    let mut filename = path.file_name().unwrap().to_str().unwrap().to_string();
    let basepath = path.parent().unwrap();
    // if it doesn't end with the encryption extension, add it.
    if ! filename.ends_with(ENCRYPTION_FILE_EXT) {
        filename.push_str(ENCRYPTION_FILE_EXT);
    }
    return basepath.join(filename);
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
    let docs = list_files(&PathBuf::from(&config.target_directory));
    println!("docs={:?}", docs);
    JsonValue(serde_json::json!(docs))
}

#[get("/doc/<filename>")]
fn get_doc(config: State<cli::Config>, filename: String) -> Template {
    let now: DateTime<Utc> = Utc::now();
    let files: Vec<String> = list_files(&PathBuf::from(&config.target_directory));
    if filename.ends_with(".cocoon") {
        // If file is encrypted, decrypt to temporary dir and return new file
        let mut encrypted: File = File::open(Path::new(&config.target_directory).join(filename.clone())).unwrap();
        let pass = config.password.clone().unwrap();
        let data = crypto::decrypt_file(&pass, &mut encrypted).unwrap();
        let mut unencrypted_path = Path::new(&config.target_directory).join("tmp");
        if !unencrypted_path.exists() {
            fs::create_dir(unencrypted_path.clone()).unwrap();
        }
        let unencrypted_name = filename.replace(".cocoon", ".pdf");
        unencrypted_path.push(unencrypted_name.clone());
        let mut unecrypted: File = File::create(unencrypted_path.clone()).unwrap();
        unecrypted.write(&data).unwrap();

        let date = parse_date(&unencrypted_name).unwrap();
        let doc = to_document(&unencrypted_path.to_str().unwrap());
        let new_context = Context {
            filename: unencrypted_name,
            name: doc.name.unwrap_or(String::new()),
            date: doc.date.unwrap_or(date),
            institution: doc.institution.unwrap_or(String::new()),
            page: doc.page.unwrap_or(String::from("1")),
            files: files,
            target_directory: unencrypted_path.parent().unwrap().to_str().unwrap().to_owned().replace("/", "\\u{002F}")
        };
        return Template::render("index", &new_context);
    } else {
        let date = parse_date(&filename).unwrap_or(now.format("%Y-%m-%d").to_string());
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
    let file_to_write = Path::new(&config.target_directory)
        .join(format!("{}_{}_{}_{}.cocoon", doc.date, doc.institution, doc.name, doc.page));
    let mut unencrypted = File::open(Path::new(&config.target_directory).join(&doc.filename))?;
    let mut buffer = Vec::new();
    unencrypted.read_to_end(&mut buffer)?;
    let checksum_name = file_to_write.clone();
    let password = config.password.clone().unwrap();
    crypto::encrypt_file(&password, &mut File::create(file_to_write)?, buffer)?;
    checksum::generate_sha256(checksum_name).unwrap();
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
                let ext = x.extension().unwrap();
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
}

fn parse_date(text: &str) -> Option<String> {
    // Returns the parsed date in ISO8601 format
    RE_WITH_HYPHENS.captures(text).map(|x|
        format!("{}-{}-{}",
            x.name("year").unwrap().as_str().to_string(),
            x.name("month").unwrap().as_str().to_string(),
            x.name("day").unwrap().as_str().to_string(),
        )
    ).or(
        RE_NO_HYPHENS.captures(text).map(|x|
            format!("{}-{}-{}",
                x.name("year").unwrap().as_str().to_string(),
                x.name("month").unwrap().as_str().to_string(),
                x.name("day").unwrap().as_str().to_string(),
            )
        )
    )
}

#[test]
fn test_parse_date_hyphens() {
    assert_eq!(parse_date("2020-04-03_boop_loop"), Some("2020-04-03".to_string()))
}

#[test]
fn test_parse_date_no_hyphens() {
    assert_eq!(parse_date("20180530_boop_loop"), Some("2018-05-30".to_string()))
}

fn get_filestem_from_filename(filename: &str) -> Option<&str> {
    Path::new(filename)
        .file_stem()
        .and_then(OsStr::to_str)
}

fn to_document(filename: &str) -> OptDoc {
    let filestem = get_filestem_from_filename(filename).unwrap_or(filename);
    let v: Vec<&str> = filestem.split('_').collect();
    OptDoc {
        date: parse_date(&v.get(0).unwrap().to_string()),
        institution: v.get(1).map(|x| x.to_string()),
        name: v.get(2).map(|x| x.to_string()),
        page: v.get(3).map(|x| x.to_string()),
    }
}