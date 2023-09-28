use std::io::Write;

use colored::{control, Colorize};
use env_logger::Env;
use iai_callgrind_runner::error::IaiCallgrindError;
use iai_callgrind_runner::runner::envs;
use iai_callgrind_runner::util::write_all_to_stderr;
use log::{error, warn};
use version_compare::Cmp;

fn print_warnings() {
    if std::env::var("IAI_ALLOW_ASLR").is_ok() {
        warn!("The IAI_ALLOW_ASLR environment variable changed to IAI_CALLGRIND_ALLOW_ASLR");
    }

    if std::env::var("RUST_LOG").is_ok() {
        warn!(
            "The RUST_LOG environment variable to set the log level changed to IAI_CALLGRIND_LOG"
        );
    }
}

fn main() {
    // Configure the colored crate to respect IAI_CALLGRIND_COLOR and CARGO_TERM_COLOR
    let iai_callgrind_color = std::env::var(envs::IAI_CALLGRIND_COLOR).ok();
    if let Some(var) = iai_callgrind_color
        .as_ref()
        .or(std::env::var(envs::CARGO_TERM_COLOR).ok().as_ref())
    {
        if var == "never" {
            control::set_override(false);
        } else if var == "always" {
            control::set_override(true);
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

    print_warnings();
    match iai_callgrind_runner::runner::run() {
        Ok(_) => {}
        Err(error) => {
            match error {
                IaiCallgrindError::VersionMismatch(cmp, runner_version, library_version) => {
                    match cmp {
                        Cmp::Lt => error!(
                            "iai-callgrind-runner ({}) is older than iai-callgrind ({}). Please \
                             update iai-callgrind-runner by calling 'cargo install --version {} iai-callgrind-runner'",
                            runner_version, library_version, library_version
                        ),
                        Cmp::Gt => error!(
                            "iai-callgrind-runner ({}) is newer than iai-callgrind ({}). Please \
                             update iai-callgrind by calling 'cargo install --version {} iai-callgrind-runner'",
                            runner_version, library_version, library_version
                        ),
                        Cmp::Ne => error!(
                            "No version information found for iai-callgrind but \
                             iai-callgrind-runner ({0}) is >= '0.3.0'. Please update \
                             iai-callgrind to '{0}' by calling 'cargo install --version <latest version> iai-callgrind-runner'",
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
                IaiCallgrindError::InvalidCallgrindBoolArgument((option, value)) => {
                    error!(
                        "Invalid callgrind argument for --{option}: '{value}'. Valid values are \
                         'yes' or 'no'"
                    );
                }
            }
            std::process::exit(1)
        }
    }
}
