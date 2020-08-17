#![feature(proc_macro_hygiene, decl_macro)]
#[macro_use] extern crate rocket;
#[allow(dead_code)]
use std::fs::{File, read_to_string};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};
use std::error::Error;
use std::io::prelude::*;
use clap::{Arg, App, SubCommand, value_t};
use cocoon::{Cocoon, Creation};
use rand::rngs::ThreadRng;
use glob::glob;
use std::path::Path;
use rocket::request::Form;
use std::fs;
use rocket::response::content;
use error_chain::error_chain;
use std::path::PathBuf;
use serde_json::Value;

#[derive(FromForm, Clone)]
struct Document {
    original_file: String,
    time: String,
    institution: String,
    document_name: String,
    page: String
}

struct Config {
    verbose: bool,
    launch_web: bool,
}

fn get_program_input() -> Config {
    let name_verbose = "verbose";
    let name_launch_web = "web";
    let matches = App::new("filecabinet")
        .version("1.0")
        .author("Danielle <filecabinet@d6e.io>")
        .about("Filecabinet - A secure solution to managing scanned files.")
        .arg(Arg::with_name(name_verbose)
            .short("v")
            .long(name_verbose)
            .multiple(true)
            .help("Sets the level of verbosity"))
        .arg(Arg::with_name(name_launch_web)
            .short("w")
            .long(name_launch_web)
            .help("Launches the web server."))
        .get_matches();
    Config {
        verbose: matches.is_present(name_verbose),
        launch_web: matches.is_present(name_launch_web),
    }
}


fn main() -> Result<(), Box<dyn Error>> {
    let config = get_program_input();

    // let cocoon = Cocoon::new(b"password");
    // let mut file = File::create("foo.cocoon")?;
    // encrypt_file(&cocoon, &mut file, "data".as_bytes().to_vec());

    // let mut unencrypted_file = File::create("foo.txt")?;
    // decrypt_file(&cocoon, &mut unencrypted_file);

    if config.launch_web {
        rocket::ignite().mount("/", routes![index, files, new]).launch();
    }
    Ok(())
}

#[post("/document", data = "<doc>")]
fn new(doc: Form<Document>) -> Result<(), Box<dyn Error>> {
    let cocoon = Cocoon::new(b"password");
    let mut file = File::create(format!("{}_{}_{}_{}.cocoon", doc.time, doc.institution, doc.document_name, doc.page))?;
    let data: String = fs::read_to_string(&doc.original_file)?;
    encrypt_file(&cocoon, &mut file, data.as_bytes().to_vec())?;
    Ok(())
}

fn encrypt_file(cocoon: &Cocoon<ThreadRng, Creation>, file: &mut File, data: Vec<u8>) -> Result<(), Box<dyn Error>> {
    cocoon.dump(data, file).unwrap();
    Ok(())
}

fn decrypt_file(cocoon: &Cocoon<ThreadRng, Creation>, file: &mut File) -> Result<(), Box<dyn Error>>  {
    let data = cocoon.parse(file).unwrap();
    Ok(())
}

fn list_files() -> Vec<PathBuf> {
    glob("./*").expect("Can't read directory.").map(|e| e.unwrap().into()).collect()
}

#[get("/")]
fn index() -> &'static str {
    "Hello, world!"
}

#[get("/files")]
fn files() -> content::Json<String> {
    let files: Vec<String> = list_files().iter()
        .map(|x| x.to_str()
            .unwrap()
            .to_owned()
        ).collect();
    let value: Value = serde_json::json!(files);
    let s: String = serde_json::from_value(value).unwrap();
    content::Json(s)
}
