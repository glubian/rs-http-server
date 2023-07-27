use pico_args::{Arguments as PicoArgs, Error as PicoError};
use std::{
    fmt,
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
};

use crate::apply_if_some;

#[derive(Debug)]
pub enum ParsingError {
    VerbosityOutOfBounds,
    Pico(PicoError),
}

impl From<PicoError> for ParsingError {
    fn from(err: PicoError) -> Self {
        Self::Pico(err)
    }
}

impl fmt::Display for ParsingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pico(err) => fmt::Display::fmt(err, f),
            Self::VerbosityOutOfBounds => write!(f, "verbosity specified more than four times"),
        }
    }
}

impl std::error::Error for ParsingError {}

fn log_filter_from_int(verbosity: i32) -> log::LevelFilter {
    use log::LevelFilter::*;
    match verbosity.clamp(0, 5) {
        0 => Off,
        1 => Error,
        2 => Warn,
        3 => Info,
        4 => Debug,
        5 => Trace,
        _ => unreachable!(),
    }
}

fn parse_verbosity(args: &mut PicoArgs) -> Result<log::LevelFilter, ParsingError> {
    let mut verbosity = 1;
    for _ in 0..4 {
        if args.contains(["-v", "--verbose"]) {
            verbosity += 1;
        } else {
            break;
        }
    }

    if args.contains(["-v", "--verbose"]) {
        Err(ParsingError::VerbosityOutOfBounds)
    } else {
        Ok(log_filter_from_int(verbosity))
    }
}

fn is_localhost(addr: IpAddr) -> bool {
    addr == Ipv4Addr::LOCALHOST || addr == Ipv6Addr::LOCALHOST
}

pub struct OptionalConfigValues {
    pub address: Option<IpAddr>,
    pub port: Option<u16>,
    pub host: String,
    pub verbosity: log::LevelFilter,
}

impl OptionalConfigValues {
    pub fn from_pico_args(args: &mut PicoArgs) -> Result<Self, ParsingError> {
        Ok(OptionalConfigValues {
            address: args.opt_value_from_str(["-a", "--address"])?,
            port: args.opt_value_from_str(["-p", "--port"])?,
            host: args
                .opt_value_from_str("--host")
                .map(Option::unwrap_or_default)?,
            verbosity: parse_verbosity(args)?,
        })
    }
}

pub struct Config {
    pub address: IpAddr,
    pub port: u16,
    pub host: String,
    pub verbosity: log::LevelFilter,
}

impl Config {
    pub fn apply_optional(&mut self, partial: OptionalConfigValues) {
        apply_if_some!(self.address, partial.address);
        apply_if_some!(self.port, partial.port);

        if !partial.host.is_empty() {
            self.host = partial.host;
        } else if is_localhost(self.address) {
            self.host = "localhost".to_string();
        }
        self.verbosity = partial.verbosity;
    }
}

impl From<OptionalConfigValues> for Config {
    fn from(partial: OptionalConfigValues) -> Self {
        let mut config = Self::default();
        config.apply_optional(partial);
        config
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            address: Ipv4Addr::LOCALHOST.into(),
            port: 8000,
            host: String::new(),
            verbosity: log::LevelFilter::Error,
        }
    }
}
