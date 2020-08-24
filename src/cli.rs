use clap::{value_t, App, Arg, SubCommand};

#[derive(Clone)]
pub struct Config {
    pub verbose: bool,
    pub launch_web: bool,
    pub target_directory: String,
}

pub fn get_program_input() -> Config {
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
