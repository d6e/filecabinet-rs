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

    // let cocoon = Cocoon::new(b"password");
    // let mut file = File::create("foo.cocoon")?;
    // encrypt_file(&cocoon, &mut file, "data".as_bytes().to_vec());

    // let mut unencrypted_file = File::create("foo.txt")?;
    // decrypt_file(&cocoon, &mut unencrypted_file);

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
    env::set_current_dir(directory).unwrap();
    chain(
        glob("*.pdf").expect("Can't read directory."),
        glob("*.jpg").expect("Can't read directory."),
    )
    .map(|e| e.unwrap().into())
    .collect()
}

#[get("/")]
fn index() -> Template {
    let now: DateTime<Utc> = Utc::now();
    let mut context = HashMap::new();
    context.insert("filename".to_string(), "uboot.pdf".to_string());
    context.insert("date".to_string(),  now.format("%Y-%m-%d").to_string());
    Template::render("index", &context)
}

#[get("/files")]
fn files(config: State<cli::Config>) -> JsonValue {
    let files: Vec<String> = list_files(&PathBuf::from(&config.target_directory))
        .iter()
        .map(|x| x.to_str().unwrap().to_owned())
        .collect();
    JsonValue(serde_json::json!(files))
}
