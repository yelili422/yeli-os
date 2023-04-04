use log::{Level, LevelFilter, Log, Metadata, Record, SetLoggerError};

use crate::println;

struct Logger;

impl Log for Logger {
    fn enabled(&self, _metadata: &Metadata) -> bool {
        unimplemented!()
    }

    fn log(&self, record: &Record) {
        let level = match record.level() {
            Level::Error => "\x1b[31merror\x1b[0m",
            Level::Warn => "\x1b[93mwarn \x1b[0m",
            Level::Info => "\x1b[34minfo \x1b[0m",
            Level::Debug => "\x1b[35mdebug\x1b[0m",
            Level::Trace => "\x1b[96mtrace\x1b[0m",
        };
        println!("{} {}", level, record.args());
    }

    fn flush(&self) {}
}

static LOGGER: Logger = Logger;

pub fn init(level: LevelFilter) -> Result<(), SetLoggerError> {
    log::set_logger(&LOGGER).map(|()| log::set_max_level(level))
}
