use std::io::Write;

use colored::{control, Colorize};
use env_logger::Env;
use iai_callgrind_runner::{write_all_to_stderr, IaiCallgrindError};
use log::error;
use version_compare::Cmp;

fn main() {
    // Configure the colored crate to respect CARGO_TERM_COLOR
    if let Some(var) = option_env!("CARGO_TERM_COLOR") {
        if var == "never" {
            control::set_override(false);
        } else if var == "always" {
            control::set_override(true);
        }
    }

    // Configure the env_logger crate to respect CARGO_TERM_COLOR
    env_logger::Builder::from_env(
        Env::default()
            .default_filter_or("warn")
            .write_style("CARGO_TERM_COLOR"),
    )
    .format(|buf, record| {
        writeln!(
            buf,
            "{}: {:<5}: {}",
            record
                .module_path()
                .unwrap_or(record.module_path_static().unwrap_or("???")),
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

    match iai_callgrind_runner::run() {
        Ok(_) => {}
        Err(error) => {
            match error {
                IaiCallgrindError::VersionMismatch(cmp, runner_version, library_version) => {
                    match cmp {
                        Cmp::Lt => error!(
                            "iai-callgrind-runner ({}) is older than iai-callgrind ({}). Please \
                             update iai-callgrind-runner",
                            runner_version, library_version
                        ),
                        Cmp::Gt => error!(
                            "iai-callgrind-runner ({}) is newer than iai-callgrind ({}). Please \
                             update iai-callgrind",
                            runner_version, library_version
                        ),
                        Cmp::Ne => error!(
                            "No version information found for iai-callgrind but \
                             iai-callgrind-runner ({0}) is >= '0.3.0'. Please update \
                             iai-callgrind to '{0}'",
                            runner_version
                        ),
                        _ => unreachable!(),
                    }
                }
                IaiCallgrindError::LaunchError(exec, error) => {
                    error!("Error executing '{}': {}", exec.display(), error)
                }
                IaiCallgrindError::BenchmarkLaunchError(output) => {
                    error!("Captured stderr:",);
                    write_all_to_stderr(&output.stderr);
                    error!(
                        "Error launching benchmark: Exit code was: {}",
                        output.status.code().unwrap()
                    );
                }
                IaiCallgrindError::Other(message) => {
                    error!("{}", message);
                }
            }
            std::process::exit(1)
        }
    }
}
