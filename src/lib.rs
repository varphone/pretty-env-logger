#![cfg_attr(test, deny(warnings))]
#![deny(missing_docs)]
#![doc(html_root_url = "https://docs.rs/pretty_env_logger/0.5.0")]

//! A logger configured via an environment variable which writes to standard
//! error with nice colored output for log levels.
//!
//! ## Example
//!
//! ```
//! extern crate pretty_env_logger;
//! #[macro_use] extern crate log;
//!
//! fn main() {
//!     pretty_env_logger::init();
//!
//!     trace!("a trace example");
//!     debug!("deboogging");
//!     info!("such information");
//!     warn!("o_O");
//!     error!("boom");
//! }
//! ```
//!
//! Run the program with the environment variable `RUST_LOG=trace`.
//!
//! ## Defaults
//!
//! The defaults can be setup by calling `init()` or `try_init()` at the start
//! of the program.
//!
//! ## Enable logging
//!
//! This crate uses [env_logger][] internally, so the same ways of enabling
//! logs through an environment variable are supported.
//!
//! [env_logger]: https://docs.rs/env_logger

#[doc(hidden)]
pub extern crate env_logger;

extern crate log;

use std::borrow::Cow;
use std::fmt;
use std::sync::atomic::{AtomicUsize, Ordering};

use env_logger::{
    fmt::style::{AnsiColor, Style},
    Builder,
};
use log::Level;

/// Initializes the global logger with a pretty env logger.
///
/// This should be called early in the execution of a Rust program, and the
/// global logger may only be initialized once. Future initialization attempts
/// will return an error.
///
/// # Panics
///
/// This function fails to set the global logger if one has already been set.
pub fn init() {
    try_init().unwrap();
}

/// Initializes the global logger with a timed pretty env logger.
///
/// This should be called early in the execution of a Rust program, and the
/// global logger may only be initialized once. Future initialization attempts
/// will return an error.
///
/// # Panics
///
/// This function fails to set the global logger if one has already been set.
pub fn init_timed() {
    try_init_timed().unwrap();
}

/// Initializes the global logger with a pretty env logger.
///
/// This should be called early in the execution of a Rust program, and the
/// global logger may only be initialized once. Future initialization attempts
/// will return an error.
///
/// # Errors
///
/// This function fails to set the global logger if one has already been set.
pub fn try_init() -> Result<(), log::SetLoggerError> {
    try_init_custom_env("RUST_LOG")
}

/// Initializes the global logger with a timed pretty env logger.
///
/// This should be called early in the execution of a Rust program, and the
/// global logger may only be initialized once. Future initialization attempts
/// will return an error.
///
/// # Errors
///
/// This function fails to set the global logger if one has already been set.
pub fn try_init_timed() -> Result<(), log::SetLoggerError> {
    try_init_timed_custom_env("RUST_LOG")
}

/// Initialized the global logger with a pretty env logger, with a custom variable name.
///
/// This should be called early in the execution of a Rust program, and the
/// global logger may only be initialized once. Future initialization attempts
/// will return an error.
///
/// # Panics
///
/// This function fails to set the global logger if one has already been set.
pub fn init_custom_env(environment_variable_name: &str) {
    try_init_custom_env(environment_variable_name).unwrap();
}

/// Initialized the global logger with a pretty env logger, with a custom variable name.
///
/// This should be called early in the execution of a Rust program, and the
/// global logger may only be initialized once. Future initialization attempts
/// will return an error.
///
/// # Errors
///
/// This function fails to set the global logger if one has already been set.
pub fn try_init_custom_env(environment_variable_name: &str) -> Result<(), log::SetLoggerError> {
    let mut builder = formatted_builder();

    if let Ok(s) = ::std::env::var(environment_variable_name) {
        builder.parse_filters(&s);
    }

    builder.try_init()
}

/// Initialized the global logger with a timed pretty env logger, with a custom variable name.
///
/// This should be called early in the execution of a Rust program, and the
/// global logger may only be initialized once. Future initialization attempts
/// will return an error.
///
/// # Errors
///
/// This function fails to set the global logger if one has already been set.
pub fn try_init_timed_custom_env(
    environment_variable_name: &str,
) -> Result<(), log::SetLoggerError> {
    let mut builder = formatted_timed_builder();

    if let Ok(s) = ::std::env::var(environment_variable_name) {
        builder.parse_filters(&s);
    }

    builder.try_init()
}

/// Returns a `env_logger::Builder` for further customization.
///
/// This method will return a colored and formatted `env_logger::Builder`
/// for further customization. Refer to env_logger::Build crate documentation
/// for further details and usage.
pub fn formatted_builder() -> Builder {
    let mut builder = Builder::new();

    builder.format(|f, record| {
        use std::io::Write;

        let target = record.target();
        let max_width = max_target_width(target);

        let style = f.default_level_style(record.level());
        let level = colored_level(style, record.level());

        let style = f.default_level_style(record.level());
        let target = padded_target(style, target, max_width);

        writeln!(f, " {} {} > {}", level, target, record.args(),)
    });

    builder
}

/// Returns a `env_logger::Builder` for further customization.
///
/// This method will return a colored and time formatted `env_logger::Builder`
/// for further customization. Refer to env_logger::Build crate documentation
/// for further details and usage.
pub fn formatted_timed_builder() -> Builder {
    let mut builder = Builder::new();

    builder.format(|f, record| {
        use std::io::Write;
        let target = record.target();
        let max_width = max_target_width(target);

        let style = f.default_level_style(record.level());
        let level = colored_level(style, record.level());

        let style = f.default_level_style(record.level());
        let target = padded_target(style, target, max_width);

        let time = f.timestamp_millis();

        writeln!(f, " {} {} {} > {}", time, level, target, record.args(),)
    });

    builder
}

struct Padded<T> {
    value: T,
    width: usize,
}

impl<T: fmt::Display> fmt::Display for Padded<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{: <width$}", self.value, width = self.width)
    }
}

struct StyledValue<'a, T> {
    style: Cow<'a, Style>,
    value: T,
}

impl<T: fmt::Display> fmt::Display for StyledValue<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}{}{:#}", self.style, self.value, self.style)
    }
}

static MAX_MODULE_WIDTH: AtomicUsize = AtomicUsize::new(0);

fn max_target_width(target: &str) -> usize {
    let max_width = MAX_MODULE_WIDTH.load(Ordering::Relaxed);
    if max_width < target.len() {
        MAX_MODULE_WIDTH.store(target.len(), Ordering::Relaxed);
        target.len()
    } else {
        max_width
    }
}

fn colored_level<'r>(style: Style, level: Level) -> StyledValue<'r, &'static str> {
    let (style, value) = match level {
        Level::Trace => (style.fg_color(Some(AnsiColor::Magenta.into())), "TRACE"),
        Level::Debug => (style.fg_color(Some(AnsiColor::Blue.into())), "DEBUG"),
        Level::Info => (style.fg_color(Some(AnsiColor::Green.into())), "INFO "),
        Level::Warn => (style.fg_color(Some(AnsiColor::Yellow.into())), "WARN "),
        Level::Error => (style.fg_color(Some(AnsiColor::Red.into())), "ERROR"),
    };
    StyledValue {
        style: Cow::Owned(style),
        value,
    }
}

fn padded_target<'r>(style: Style, target: &str, width: usize) -> StyledValue<'r, Padded<&'_ str>> {
    StyledValue {
        style: Cow::Owned(style),
        value: Padded {
            value: target,
            width,
        },
    }
}
