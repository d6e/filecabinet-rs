#![feature(proc_macro_hygiene, decl_macro)]
#[macro_use]

use clap::{value_t, App, Arg, SubCommand};
use cocoon::{Cocoon, Creation};
use error_chain::error_chain;
use glob::glob;
use itertools::chain;
use rand::rngs::ThreadRng;
use tera::Context;
use serde;
use serde_json::Value;
use std::env;
use std::error::Error;
use std::fs;
use std::fs::{read_to_string, File};
use std::io::prelude::*;
use std::path::Path;
use std::path::PathBuf;
use std::thread;
use std::convert::Infallible;
use std::net::SocketAddr;
use std::time::{SystemTime, UNIX_EPOCH};
use warp::{Filter, http::StatusCode, reject, Reply, Rejection, reply::json};
use askama::Template;

#[derive(Clone)]
struct Document {
    original_file: String,
    time: String,
    institution: String,
    document_name: String,
    page: String,
}

#[derive(Clone)]
struct Config {
    verbose: bool,
    launch_web: bool,
    target_directory: String,
}

fn get_program_input() -> Config {
    let name_verbose = "verbose";
    let name_launch_web = "web";
    let name_target_directory = "target-directory";
    let default_target_directory: String = String::from("./");
    let matches = App::new("filecabinet")
        .version("1.0")
        .author("Danielle <filecabinet@d6e.io>")
        .about("Filecabinet - A relatively secure solution to managing scanned files.")
        .arg(
            Arg::with_name(name_verbose)
                .short("v")
                .long(name_verbose)
                .multiple(true)
                .help("Sets the level of verbosity"),
        )
        .arg(
            Arg::with_name(name_launch_web)
                .short("w")
                .long(name_launch_web)
                .help("Launches the web server."),
        )
        .arg(
            Arg::with_name(name_target_directory)
                .short("d")
                .long(name_target_directory)
                .takes_value(true)
                .value_name("DIR")
                .help("Target directory for archival.")
        ).get_matches();
    Config {
        verbose: matches.is_present(name_verbose),
        launch_web: matches.is_present(name_launch_web),
        target_directory: value_t!(matches, name_target_directory, String).unwrap_or(default_target_directory),
    }
}

fn with_config(config: Config) -> impl Filter<Extract = (Config,), Error = Infallible> + Clone {
    warp::any().map(move || config.clone())
}

#[tokio::main]
async fn main() {
    let config = get_program_input();

    let index_route = warp::path::end()
        .and_then(index)
        .map(|body| {
            warp::reply::html(body)
        });

    let health_route = warp::path!("health")
        .map(|| StatusCode::OK);

    let files_route = warp::path("files")
        .and(with_config(config))
        .and_then(endpoint_files);

    let routes = health_route
        .or(index_route)
        .or(files_route)
        .with(warp::cors().allow_any_origin());


    warp::serve(routes).run(([127, 0, 0, 1], 8000)).await;
}

// fn main() -> Result<(), Box<dyn Error>> {
    // let cocoon = Cocoon::new(b"password");
    // let mut file = File::create("foo.cocoon")?;
    // encrypt_file(&cocoon, &mut file, "data".as_bytes().to_vec());

    // let mut unencrypted_file = File::create("foo.txt")?;
    // decrypt_file(&cocoon, &mut unencrypted_file);

//     Ok(())
// }

// #[post("/document", data = "<doc>")]
// fn new(doc: Form<Document>) -> Result<(), Box<dyn Error>> {
//     let cocoon = Cocoon::new(b"password");
//     let mut file = File::create(format!(
//         "{}_{}_{}_{}.cocoon",
//         doc.time, doc.institution, doc.document_name, doc.page
//     ))?;
//     let data: String = fs::read_to_string(&doc.original_file)?;
//     encrypt_file(&cocoon, &mut file, data.as_bytes().to_vec())?;
//     Ok(())
// }

fn encrypt_file(
    cocoon: &Cocoon<ThreadRng, Creation>,
    file: &mut File,
    data: Vec<u8>,
) -> Result<(), Box<dyn Error>> {
    cocoon.dump(data, file).unwrap();
    Ok(())
}

fn decrypt_file(
    cocoon: &Cocoon<ThreadRng, Creation>,
    file: &mut File,
) -> Result<(), Box<dyn Error>> {
    let data = cocoon.parse(file).unwrap();
    Ok(())
}


#[derive(Template, Clone)]
#[template(path = "index.html")]
struct IndexTemplate<'a> {
    name: &'a str,
}

// impl Clone for IndexTemplate {
//     fn clone(&self) -> IndexTemplate {
//         IndexTemplate { name: self.name }
//     }
// }

async fn index() -> Result<String, Rejection>  {
    let hello = IndexTemplate { name: "world" };
    Ok(hello.render().unwrap())
}

fn list_files(directory: &PathBuf) -> Vec<PathBuf> {
    println!("directory={:?}", directory);
    env::set_current_dir(directory).unwrap();
    chain(
        glob("*.pdf").expect("Can't read directory."),
        glob("*.jpg").expect("Can't read directory."),
    )
    .map(|e| e.unwrap().into())
    .collect()
}

async fn endpoint_files(config: Config) -> Result<impl Reply, Rejection> {
    let files: Vec<String> = list_files(&PathBuf::from(&config.target_directory))
        .iter()
        .map(|x| x.to_str().unwrap().to_owned())
        .collect();
    Ok(warp::reply::json(&files))
}
