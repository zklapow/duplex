#[macro_use]
extern crate log;

extern crate clap;
extern crate bytes;
extern crate futures;
extern crate tokio_io;
extern crate tokio_core;
extern crate tokio_proto;
extern crate tokio_service;
extern crate env_logger;
extern crate chrono;

mod proxy;

use std::io;
use std::io::{Error, ErrorKind};
use std::env;

use log::{LogRecord, LogLevelFilter};
use env_logger::LogBuilder;
use clap::{Arg, App, SubCommand, AppSettings};
use chrono::prelude::*;

fn main() -> Result<(), Error> {
    init_logging();

    info!("Hello, world!");
    let matches = App::new("duplex")
        .version("0.1")
        .author("Ze'ev Klapow")
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .setting(AppSettings::ColoredHelp)
        .subcommand(SubCommand::with_name("proxy")
            .arg(Arg::with_name("from")
                .short("f")
                .default_value("localhost:8080"))
            .arg(Arg::with_name("to")
                .short("t")
                .default_value("localhost:8081")))
        .get_matches();

    match matches.subcommand() {
        ("proxy", Some(m)) => {
            proxy::proxy(m.value_of("from").unwrap(), m.value_of("to").unwrap());
            Ok(())
        }
        _ => Err(Error::new(ErrorKind::InvalidInput, "unknown command"))
    }
}

fn init_logging() {
    let format = |record: &LogRecord| {
        let now = UTC::now();
        format!("[{}] {}:{}: {}", now.format("%H:%M:%S"),
                record.level(),
                record.location().module_path(),
                record.args())
    };


    let mut builder = LogBuilder::new();
    builder.format(format).filter(None, LogLevelFilter::Info);

    if env::var("RUST_LOG").is_ok() {
        builder.parse(&env::var("RUST_LOG").unwrap());
    }

    builder.init().unwrap();
}
