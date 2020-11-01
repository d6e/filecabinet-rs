#![feature(proc_macro_hygiene, decl_macro)]
#[macro_use] extern crate rocket;
#[macro_use] extern crate lazy_static;
use error_chain::error_chain;
use glob::glob;
use itertools::chain;
use rocket::request::Form;
use rocket::response::content;
use rocket::State;
use rocket_contrib::json::{Json, JsonValue};
use rocket_contrib::templates::Template;
use std::env;
use std::error::Error;
#[allow(dead_code)]
use std::path::PathBuf;
use std::fs::File;
use std::io::prelude::*;
use std::collections::HashMap;
use rocket_contrib::serve::StaticFiles;
use chrono::{DateTime, Utc};
use serde::Serialize;
use std::path::Path;
use std::ffi::OsStr;
use rocket::response::Redirect;
use regex::Regex;
use std::fs;
mod crypto;
mod cli;

#[derive(FromForm, Clone)]
struct Document {
    orig_name: String,
    date: String,
    institution: String,
    name: String,
    page: String,
    extension: String
}

fn main() -> Result<(), Box<dyn Error>> {
    let config = cli::get_program_input();
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
  date: String,
  files: Vec<String>,
  target_directory: String
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

#[get("/doc/<name>")]
fn get_doc(config: State<cli::Config>, name: String) -> Template {
    let now: DateTime<Utc> = Utc::now();
    let files: Vec<String> = list_files(&PathBuf::from(&config.target_directory));
    if name.ends_with(".cocoon") {
        // If file is encrypted, decrypt to temporary dir and return new file
        let mut encrypted: File = File::open(Path::new(&config.target_directory).join(name.clone())).unwrap();
        let data = crypto::decrypt_file(&mut encrypted).unwrap();
        // let mut unencrypted_path = env::temp_dir(); // TODO: maybe put temp dir under target_driectory
        let mut unencrypted_path = Path::new(&config.target_directory).join("tmp");
        if !unencrypted_path.exists() {
            fs::create_dir(unencrypted_path.clone()).unwrap();
        }
        let unencrypted_name = name.replace("cocoon", "pdf");
        unencrypted_path.push(unencrypted_name.clone());
        let mut unecrypted: File = File::create(unencrypted_path.clone()).unwrap();
        unecrypted.write(&data).unwrap();

        let date = parse_date(&unencrypted_name).unwrap();
        let new_context = Context {
            filename: unencrypted_name,
            date: date,
            files: files,
            target_directory: unencrypted_path.parent().unwrap().to_str().unwrap().to_owned().replace("/", "\\u{002F}")
        };
        return Template::render("index", &new_context);
    } else {
        let date = parse_date(&name).unwrap_or(now.format("%Y-%m-%d").to_string());
        let context = Context {
            filename: name,
            date: date,
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
    let mut unencrypted = File::open(Path::new(&config.target_directory).join(&doc.orig_name))?;
    let mut buffer = Vec::new();
    unencrypted.read_to_end(&mut buffer)?;
    crypto::encrypt_file(&mut File::create(file_to_write)?, buffer)?;
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
    assert_eq!(parse_date("20200530_boop_loop"), Some("2020-05-30".to_string()))
}


fn get_filestem_from_filename(filename: &str) -> Option<&str> {
    Path::new(filename)
        .file_stem()
        .and_then(OsStr::to_str)
}

fn get_extension_from_filename(filename: &str) -> Option<&str> {
    Path::new(filename)
        .extension()
        .and_then(OsStr::to_str)
}

fn parse_institution(filename: &str) -> Option<String> {
    let filestem = get_filestem_from_filename(filename).unwrap_or(filename);
    let v: Vec<&str> = filestem.split('_').collect();
    let institution = v.get(1);
    let doc_name: &str = v.get(2).unwrap();
    let page_num: &str = v.get(3).unwrap();
    return institution.map(|x| x.to_string());
}

fn to_document(filename: &str) -> Document {
    let filestem = get_filestem_from_filename(filename).unwrap_or(filename);
    let v: Vec<&str> = filestem.split('_').collect();
    Document {
        orig_name: filename.to_string(),
        date: parse_date(&v.get(0).unwrap().to_string()).unwrap(),
        institution: v.get(1).unwrap().to_string(),
        name: v.get(2).unwrap().to_string(),
        page: v.get(3).map(|x| x.to_string()).unwrap_or(String::new()).to_string(),
        extension: get_extension_from_filename(filename).unwrap_or("").to_string()
    }
}