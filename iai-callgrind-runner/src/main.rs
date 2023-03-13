use colored::{control, Colorize};
use env_logger::Env;
use iai_callgrind_runner::IaiCallgrindError;
use log::error;
use std::io::Write;
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
            // .blue(),
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
        Ok(_) => std::process::exit(0),
        Err(error) => {
            match error {
                IaiCallgrindError::VersionMismatch(cmp, runner_version, library_version) => match cmp {
                    Cmp::Lt => error!(
                        "iai-callgrind-runner ({}) is older than iai-callgrind ({}). Please update iai-callgrind-runner",
                        runner_version, library_version
                    ),
                    Cmp::Gt => error!(
                        "iai-callgrind-runner ({}) is newer than iai-callgrind ({}). Please update iai-callgrind",
                        runner_version, library_version
                    ),
                    _ => unreachable!(),
                },
                IaiCallgrindError::LaunchError(error) =>
                    error!(
                        "Unexpected error when launching valgrind: {}\n\
                        Please make sure Valgrind is installed and in your $PATH", error),
                IaiCallgrindError::CallgrindLaunchError(output) => {
                    print!("{}", String::from_utf8_lossy(output.stderr.as_slice()));
                    error!(
                        "Error launching callgrind: Exit code was: {}",
                        output.status.code().unwrap());
                }
            }
            std::process::exit(1);
        }
    }
}
