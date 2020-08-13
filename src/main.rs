#[allow(dead_code)]
use std::fs::{File, read_to_string};
use std::{thread, time};
use std::error::Error;
use std::io::prelude::*;
use clap::{Arg, App, value_t};

struct Config {
    verbose: bool,
    p_value: f32,
}

fn get_program_input() -> Config {
    let name_verbose = "verbose";
    let name_p_value = "pvalue";
    let default_p_value = 1.0;
    let matches = App::new("filecabinet")
        .version("1.0")
        .author("Danielle <filecabinet@d6e.io>")
        .about("Filecabinet - A scanned file solution.")
        .arg(Arg::with_name(name_verbose)
            .short("v")
            .long(name_verbose)
            .multiple(true)
            .help("Sets the level of verbosity"))
        .arg(Arg::with_name(name_p_value)
            .short("p")
            .long("pvalue")
            .value_name("P")
            .help("Sets the P gain value of the PID controller.")
            .takes_value(true))
        .get_matches();
    Config {
        verbose: matches.is_present(name_verbose),
        p_value: value_t!(matches, name_p_value, f32).unwrap_or(default_p_value),
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let config = get_program_input();

    println!("Starting filecabinet...");
    Ok(())
}
