#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Level {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl Level {
    fn label(self) -> &'static str {
        match self {
            Level::Error => "ERROR",
            Level::Warn => "WARN",
            Level::Info => "INFO",
            Level::Debug => "DEBUG",
            Level::Trace => "TRACE",
        }
    }
}

pub fn print_log(
    level: Level,
    file: &str,
    line: u32,
    reason: Option<&str>,
    message: std::fmt::Arguments,
) {
    match reason {
        Some(reason) => println!(
            "[{}] {}:{} ({}) - {}",
            level.label(),
            file,
            line,
            reason,
            message
        ),
        None => println!("[{}] {}:{} - {}", level.label(), file, line, message),
    }
}

#[macro_export]
macro_rules! log {
    ($level:expr, reason: $reason:expr, $($arg:tt)*) => {
        $crate::logging::print_log($level, file!(), line!(), Some($reason), format_args!($($arg)*))
    };
    ($level:expr, $($arg:tt)*) => {
        $crate::logging::print_log($level, file!(), line!(), None, format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_error {
    ($($arg:tt)*) => { $crate::log!($crate::logging::Level::Error, $($arg)*) };
}

#[macro_export]
macro_rules! log_warn {
    ($($arg:tt)*) => { $crate::log!($crate::logging::Level::Warn, $($arg)*) };
}

#[macro_export]
macro_rules! log_info {
    ($($arg:tt)*) => { $crate::log!($crate::logging::Level::Info, $($arg)*) };
}

#[macro_export]
macro_rules! log_debug {
    ($($arg:tt)*) => { $crate::log!($crate::logging::Level::Debug, $($arg)*) };
}

#[macro_export]
macro_rules! log_trace {
    ($($arg:tt)*) => { $crate::log!($crate::logging::Level::Trace, $($arg)*) };
}
