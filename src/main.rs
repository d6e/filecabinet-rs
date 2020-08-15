#![feature(proc_macro_hygiene, decl_macro)]
#[macro_use] extern crate rocket;
#[allow(dead_code)]
use std::fs::{File, read_to_string};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};
use std::error::Error;
use std::io::prelude::*;
use clap::{Arg, App, value_t};
use cocoon::Cocoon;

struct Config {
    verbose: bool,
    launch_web: bool,
    p_value: f32,
}

fn get_program_input() -> Config {
    let name_verbose = "verbose";
    let name_launch_web = "web";
    let name_p_value = "pvalue";
    let default_p_value = 1.0;
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
        .arg(Arg::with_name(name_p_value)
            .short("p")
            .long("pvalue")
            .value_name("P")
            .help("Sets the P gain value of the PID controller.")
            .takes_value(true))
        .get_matches();
    Config {
        verbose: matches.is_present(name_verbose),
        launch_web: matches.is_present(name_launch_web),
        p_value: value_t!(matches, name_p_value, f32).unwrap_or(default_p_value),
    }
}

// fn encrypt_file(file: &mut File, data: Vec<u8>) -> Result<(), Box<dyn Error>> {
//     cocoon.dump(data, &mut file)?;
//     Ok(())
// }

// fn decrypt_file(file: &mut File) -> Result<(), Box<dyn Error>>  {
//     let data = cocoon::parse(&mut file)?;
//     Ok(())
// }

#[get("/")]
fn index() -> &'static str {
    "Hello, world!"
}

fn main() -> Result<(), Box<dyn Error>> {
    let config = get_program_input();

    let cocoon = Cocoon::new(b"password");
    let mut file = File::create("foo.cocoon")?;
    cocoon.dump("myplaintext".as_bytes().to_vec(), &mut file).unwrap();

    let data = cocoon.parse(&mut file).unwrap();
    let mut unencrypted_file = File::create("foo.txt")?;
    let mut contents = vec![];
    file.read_to_end(&mut contents)?;
    unencrypted_file.write_all(&contents);

    if config.launch_web {
        rocket::ignite().mount("/", routes![index]).launch();
    }
    Ok(())
}
