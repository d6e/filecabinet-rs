use clap::{value_t,values_t, App, Arg};
use std::fs::File;
use std::io::prelude::*;

pub struct Config {
    pub verbose: bool,
    pub launch_web: bool,
    pub target_directory: String,
    pub password: Option<String>,
    pub file_to_decrypt: Option<Vec<String>>,
    pub file_to_encrypt: Option<Vec<String>>,
    pub verify: bool,
}

pub fn get_program_input() -> Config {
    let name_verbose = "verbose";
    let name_launch_web = "web";
    let name_target_directory = "target-directory";
    let name_password_file = "password-file";
    let name_decrypt_file = "decrypt-file";
    let name_encrypt_file = "encrypt-file";
    let name_verify = "verify";
    let default_target_directory = String::from(".");
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
                .help("Target directory for archival."),
        ).arg(
            Arg::with_name(name_password_file)
                .short("p")
                .long(name_password_file)
                .takes_value(true)
                .value_name("FILE")
                .help("File containing password for encryption."),
        ).arg(
            Arg::with_name(name_decrypt_file)
                .long(name_decrypt_file)
                .takes_value(true)
                .multiple(true)
                .value_name("FILE")
                .help("The file to decrypt."),
        ).arg(
            Arg::with_name(name_encrypt_file)
                .long(name_encrypt_file)
                .takes_value(true)
                .multiple(true)
                .value_name("FILE")
                .help("The file to encrypt."),
        )
        .arg(
            Arg::with_name(name_verify)
                .long(name_verify)
                .help("Verify the file integrity of all files."),
        )
        .get_matches();

    // Read password file
    let mut password: Option<String> = None;
    if matches.is_present(name_password_file) {
        let password_file: String = value_t!(matches, name_password_file, String).unwrap();
        let mut file = File::open(&password_file).expect(&format!("Couldn't open '{}'", &password_file));
        let mut buffer = String::new();
        file.read_to_string(&mut buffer).unwrap();
        if buffer.trim().len() > 0 {
            password = Some(buffer.trim().to_string());
        } else {
            eprintln!("ERROR: password file {} is empty!", password_file);
        }
    }

    let config = Config {
        verbose: matches.is_present(name_verbose),
        launch_web: matches.is_present(name_launch_web),
        target_directory: value_t!(matches, name_target_directory, String)
            .unwrap_or(default_target_directory),
        password: password,
        file_to_decrypt: values_t!(matches.values_of(name_decrypt_file), String).ok(),
        file_to_encrypt: values_t!(matches.values_of(name_encrypt_file), String).ok(),
        verify: matches.is_present(name_verify),
    };

    // Validate times the password must be specified
    if config.password.is_none() && (config.file_to_decrypt.is_some() || config.file_to_encrypt.is_some() || config.launch_web) {
        eprintln!("ERROR The password was not specified.");
        std::process::exit(1);
    }
    return config;
}