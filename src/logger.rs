use log::{LevelFilter, Metadata, Record, SetLoggerError};

use crate::println;

struct Logger;

impl log::Log for Logger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        false
    }

    fn log(&self, record: &Record) {
        let level = match record.level() {
            log::Level::Error => "\x1b[31merror\x1b[0m",
            log::Level::Warn => "\x1b[93mwarn \x1b[0m",
            log::Level::Info => "\x1b[34minfo \x1b[0m",
            log::Level::Debug => "\x1b[35mdebug\x1b[0m",
            log::Level::Trace => "\x1b[96mtrace\x1b[0m",
        };
        println!("{} {}", level, record.args());
    }

    fn flush(&self) {}
}

static LOGGER: Logger = Logger;

pub fn init(level: LevelFilter) -> Result<(), SetLoggerError> {
    log::set_logger(&LOGGER).map(|()| log::set_max_level(level))
}
