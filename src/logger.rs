use std::fmt::Display;

const DATE_FORMAT: &str = "%Y-%m-%d %H:%M:%S";
const VERBOSE: i8 = 2;
const DEBUG: i8 = 1;
const INFO: i8 = 0;
const WARNING: i8 = -1;
const ERROR: i8 = -2;
const FATAL: i8 = -3;

#[derive(Debug, Copy, Clone)]
pub struct Logger {
    level: i8,
}

impl Logger {
    pub fn new(level: i8) -> Logger {
        Logger { level }
    }
    fn get_timestamp(&self) -> String {
        chrono::offset::Local::now().format(DATE_FORMAT).to_string()
    }
    fn is_enabled_for(&self, level: i8) -> bool {
        self.level >= level
    }
    fn print_message<S: Display>(&self, level: i8, message: S) {
        if self.is_enabled_for(level) {
            println!(
                "{} {}: {}",
                self.get_timestamp(),
                match level {
                    VERBOSE => "VERBOSE",
                    DEBUG => "DEBUG",
                    INFO => "INFO",
                    WARNING => "WARNING",
                    ERROR => "ERROR",
                    FATAL => "FATAL",
                    _ => "UNK",
                },
                message
            );
        }
    }
    pub fn verbose<S: Display>(&self, message: S) {
        self.print_message(VERBOSE, message)
    }
    pub fn debug<S: Display>(&self, message: S) {
        self.print_message(DEBUG, message)
    }
    pub fn info<S: Display>(&self, message: S) {
        self.print_message(INFO, message)
    }
    pub fn warning<S: Display>(&self, message: S) {
        self.print_message(WARNING, message)
    }
    pub fn error<S: Display>(&self, message: S) {
        self.print_message(ERROR, message)
    }
    pub fn fatal<S: Display>(&self, message: S) {
        self.print_message(FATAL, message)
    }
}
