//! The `iai-callgrind-runner` binary
use std::io::Write;

use colored::{control, Colorize};
use env_logger::Env;
use iai_callgrind_runner::error::Error;
use iai_callgrind_runner::runner::envs;
use log::{error, warn};

/// The main function of the `iai-callgrind-runner` binary
///
/// We initialize the logging interface and configure the usage of colors as early as possible here.
/// Then we're printing warnings with [`print_warnings`] and finally call the main
/// [`iai_callgrind_runner::runner::run`] library function catching and printing
/// [`iai_callgrind_runner::error::Error`]s.
fn main() {
    // Configure the colored crate to respect IAI_CALLGRIND_COLOR and CARGO_TERM_COLOR
    let iai_callgrind_color = std::env::var(envs::IAI_CALLGRIND_COLOR).ok();
    if let Some(var) = iai_callgrind_color
        .clone()
        .or_else(|| std::env::var(envs::CARGO_TERM_COLOR).ok())
    {
        if var == "never" {
            control::set_override(false);
        } else if var == "always" {
            control::set_override(true);
        } else {
            // do nothing
        }
    }

    // Configure the env_logger crate to respect IAI_CALLGRIND_COLOR and CARGO_TERM_COLOR
    env_logger::Builder::from_env(
        Env::default()
            .filter_or(envs::IAI_CALLGRIND_LOG, "warn")
            .write_style(
                iai_callgrind_color
                    .map_or_else(|| envs::CARGO_TERM_COLOR, |_| envs::IAI_CALLGRIND_COLOR),
            ),
    )
    .format(|buf, record| {
        writeln!(
            buf,
            "{}: {:<5}: {}",
            record
                .module_path()
                .unwrap_or_else(|| record.module_path_static().unwrap_or("???")),
            match record.level() {
                log::Level::Error => "Error".red().bold(),
                log::Level::Warn => "Warn".yellow().bold(),
                log::Level::Info => "Info".green().bold(),
                log::Level::Debug => "Debug".blue().bold(),
                log::Level::Trace => "Trace".cyan().bold(),
            },
            record.args()
        )
    })
    .init();

    print_warnings();
    match iai_callgrind_runner::runner::run() {
        Ok(()) => {}
        Err(error) => {
            if let Some(Error::RegressionError(is_fatal)) = error.downcast_ref::<Error>() {
                if *is_fatal {
                    error!("{error}");
                }
                std::process::exit(3)
            } else {
                error!("{error}");
                std::process::exit(1)
            }
        }
    }
}

/// Print warnings for deprecated usages of environment variables
fn print_warnings() {
    if std::env::var("IAI_ALLOW_ASLR").is_ok() {
        warn!("The IAI_ALLOW_ASLR environment variable changed to IAI_CALLGRIND_ALLOW_ASLR");
    }

    if std::env::var("RUST_LOG").is_ok() {
        warn!(
            "The RUST_LOG environment variable to set the log level changed to IAI_CALLGRIND_LOG"
        );
    }

    if std::env::var("IAI_CALLGRIND_REGRESSION").is_ok() {
        warn!(
            "The IAI_CALLGRIND_REGRESSION environment variable changed to \
             IAI_CALLGRIND_CALLGRIND_LIMITS"
        );
    }
}
