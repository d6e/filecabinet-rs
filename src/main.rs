#![feature(proc_macro_hygiene, decl_macro)]
#[macro_use]

use error_chain::error_chain;
use glob::glob;
use itertools::chain;
use std::env;
use std::error::Error;
use std::fs;
use std::fs::{read_to_string, File};
use std::io::prelude::*;
use std::path::Path;
use std::path::PathBuf;
use std::convert::Infallible;
use std::time::{SystemTime, UNIX_EPOCH};
use warp::{Filter, http::StatusCode, Reply, Rejection};
use askama::Template;
mod cli;

#[derive(Clone)]
struct Document {
    original_file: String,
    time: String,
    institution: String,
    document_name: String,
    page: String,
}

fn with_config(config: cli::Config) -> impl Filter<Extract = (cli::Config,), Error = Infallible> + Clone {
    warp::any().map(move || config.clone())
}

#[tokio::main]
async fn main() {
    let config = cli::get_program_input();

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

#[derive(Template, Clone)]
#[template(path = "index.html")]
struct IndexTemplate<'a> {
    name: &'a str,
}

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

async fn endpoint_files(config: cli::Config) -> Result<impl Reply, Rejection> {
    let files: Vec<String> = list_files(&PathBuf::from(&config.target_directory))
        .iter()
        .map(|x| x.to_str().unwrap().to_owned())
        .collect();
    Ok(warp::reply::json(&files))
}
