#![warn(clippy::all, clippy::pedantic)]
#![allow(clippy::enum_glob_use)]
#![allow(clippy::similar_names)] // allow usage of `req` and `res`
#![allow(dead_code)]

use std::{io, net::TcpListener, process::exit};

use handlebars::Handlebars;
use log::warn;
use pico_args::Arguments as PicoArgs;

mod config;
mod macros;
mod router;
mod stream_handler;

use config::{Config, OptionalConfigValues};
use router::Router;
use stream_handler::StreamHandler;

const VERSION: &str = "http-server, version 0.0.0";

const HELP: &str = "\
USAGE:
    http-server [OPTIONS]

OPTIONS:
    -a --address <ADDRESS>      Address to use
    -p --port <PORT>            Port to use
       --host <HOST>            Expected Host header value (if it is not an IP address)
    -v --verbose                Increase the level of verbosity; can be repeated up to 4 times
       --version                Show version and exit
       --help                   Show this message and exit
";

fn parse_arguments() -> Config {
    let mut args = PicoArgs::from_env();

    if args.contains("--help") {
        print!("{VERSION}\n\n{HELP}");
        exit(0);
    }

    if args.contains("--version") {
        println!("{VERSION}");
        exit(0);
    }

    let partial = match OptionalConfigValues::from_pico_args(&mut args) {
        Ok(partial) => partial,
        Err(err) => {
            eprintln!("{err}");
            exit(1);
        }
    };

    let extra_args = args.finish();
    if !extra_args.is_empty() {
        let plural = if extra_args.len() == 1 { "" } else { "s" };
        let arg_list = extra_args.join(std::ffi::OsStr::new(", "));
        let arg_list = arg_list.to_string_lossy();
        eprintln!("Unknown argument{plural}: {arg_list}");
        exit(1);
    }

    partial.into()
}

fn init_logger(config: &Config) {
    use simplelog::{ColorChoice, ConfigBuilder, TermLogger, TerminalMode};

    let log_config = ConfigBuilder::new()
        .add_filter_allow_str("http_server")
        .build();

    TermLogger::init(
        config.verbosity,
        log_config,
        TerminalMode::Mixed,
        ColorChoice::Always,
    )
    .expect("unable to initialize simplelog");
}

fn init_handlebars_registry() -> Handlebars<'static> {
    let mut handlebars = Handlebars::new();

    handlebars
        .register_template_string("dir", include_str!("dir.hbs"))
        .unwrap();

    handlebars
}

fn main() -> io::Result<()> {
    let config = parse_arguments();

    init_logger(&config);

    let listener = TcpListener::bind((config.address, config.port))?;
    let mut handler = StreamHandler::new(Router::new(init_handlebars_registry(), &config));
    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => handler.dispatch(&mut stream),
            Err(err) => warn!("Connection failed {err}"),
        }
    }

    Ok(())
}
