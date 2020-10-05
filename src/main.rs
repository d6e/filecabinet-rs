#![feature(proc_macro_hygiene, decl_macro)]
#[macro_use]
extern crate rocket;
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
mod crypto;
mod cli;

#[derive(FromForm, Clone)]
struct Document {
    orig_name: String,
    date: String,
    institution: String,
    name: String,
    page: String,
}

fn main() -> Result<(), Box<dyn Error>> {
    let config = cli::get_program_input();
    if config.launch_web {
        rocket::ignite()
             .mount("/node_modules", StaticFiles::from("node_modules"))
            .mount("/static", StaticFiles::from("static"))
            .mount("/", routes![index, files, new])
            .manage(config)
            .attach(Template::fairing())
            .launch();
    }
    Ok(())
}

#[derive(Serialize)]
struct Context {
  filename: String,
  date: String,
  files: Vec<String>
}

#[get("/")]
fn index(config: State<cli::Config>) -> Template {
    let now: DateTime<Utc> = Utc::now();
    let files: Vec<String> = list_files(&PathBuf::from(&config.target_directory)).iter().map(|x|x.to_str().unwrap().to_owned()).collect();
    let context = Context {
        filename: "uboot.pdf".to_string(),
        date: now.format("%Y-%m-%d").to_string(),
        files: files,
    };
    Template::render("index", &context)
}

#[get("/files")]
fn files(config: State<cli::Config>) -> JsonValue {
    println!("target_dir={}", &config.target_directory);
    let files: Vec<String> = list_files(&PathBuf::from(&config.target_directory))
        .iter()
        .map(|x| x.to_str().unwrap().to_owned())
        .collect();
    JsonValue(serde_json::json!(files))
}

#[post("/document", data = "<doc>")]
fn new(doc: Form<Document>) -> Result<(), Box<dyn Error>> {
    let mut file = File::create(format!(
        "static/{}_{}_{}_{}.cocoon",
        doc.date, doc.institution, doc.name, doc.page
    ))?;

    let mut f = File::open(format!("static/{}",&doc.orig_name))?;
    let mut buffer = Vec::new();
    f.read_to_end(&mut buffer)?;
    crypto::encrypt_file(&mut file, buffer)?;
    Ok(())
}

fn list_files(directory: &PathBuf) -> Vec<PathBuf> {
    if !directory.exists() {
        return Vec::new(); // TODO: turn this into an optional
    }
    chain(
        glob("static/*.pdf").expect("Can't read directory."),
        glob("static/*.jpg").expect("Can't read directory."),
    )
    // .map(|e| e.unwrap().into())
    .map(|x| x.unwrap().strip_prefix("static/").unwrap().into())
    .collect()
}
