#[allow(unused_imports)]
#[allow(unused_macros)]

#[macro_use]
extern crate log;

extern crate env_logger;
extern crate clap;

extern crate tokio;
extern crate rayon;

extern crate rand;

extern crate notify;

use std::env;

use clap::{App, Arg};

static LOG_KEY: &str = "RUST_LOG";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let matches = App::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .arg(
            Arg::with_name("config")
                .short("c")
                .long("config")
                .value_name("FILE")
                .help("Sets a custom config file")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("verbose")
                .short("v")
                .multiple(true)
                .help("Increase verbosity, pass multiple times to change level"),
        )
        .get_matches();

    if let Some(c) = matches.value_of("config") {
        eprintln!("Value for config: {}", c);
    }

    // You can see how many times a particular flag or argument occurred
    // Note, only flags can have multiple occurrences
    match matches.occurrences_of("verbose") {
        0 => {
            eprintln!("Logging level set to 0 (error)");
            std::env::set_var(LOG_KEY, "ERROR");
        },
        1 => {
            eprintln!("Logging level is 1 (info, error)");
            std::env::set_var(LOG_KEY, "WARN");
        },
        2 => { 
            eprintln!("Logging level is 2 (info, warn, error)");
            std::env::set_var(LOG_KEY, "INFO");
        },
        3  => {
            eprintln!("Logging level is 3 (info, warn, error, debug)");
            std::env::set_var(LOG_KEY, "DEBUG");
        },
        4 | _ => {
            eprintln!("Logging level is 4 (info, warn, error, debug)");
            std::env::set_var(LOG_KEY, "TRACE");
        },
    }
    env_logger::init();

    webserv::run()
}
