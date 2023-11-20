use clap::{ArgAction, Parser};

use super::summary::SummaryFormat;
use crate::api::RawArgs;

/// The command line arguments the user provided after `--` when running cargo bench
///
/// These arguments are not the command line arguments passed to `iai-callgrind-runner`. We collect
/// the command line arguments in the `iai-callgrind::main!` macro without the binary as first
/// argument, that's why `no_binary_name` is set to `true`.
/// TODO: ADD environment variables
#[derive(Parser, Debug, Clone)]
#[clap(
    author,
    version,
    about = "High-precision and consistent benchmarking framework/harness for Rust",
    long_about = None,
    no_binary_name = true,
)]
pub struct CommandLineArgs {
    /// `--bench` usually shows up as last argument set by cargo and not by us. This argument is
    /// useless, so we sort it out and never make use of it.
    #[clap(long = "bench", hide = true, action = ArgAction::SetTrue, required = false)]
    pub _bench: bool,

    #[clap(
        long = "callgrind-args",
        required = false,
        value_parser = parse_args,
        takes_value = true,
        help = "Arguments to pass through to Callgrind"
    )]
    pub callgrind_args: Option<RawArgs>,

    #[clap(
        long = "save-summary",
        value_enum,
        required = false,
        default_missing_value = "json",
        env = "IAI_CALLGRIND_SAVE_SUMMARY",
        help = "Save a summary for each benchmark run"
    )]
    pub save_summary: Option<SummaryFormat>,
}

/// This function parses a space separated list of raw argument strings into [`crate::api::RawArgs`]
fn parse_args(value: &str) -> Result<RawArgs, String> {
    shlex::split(value)
        .ok_or_else(|| "Failed to split callgrind args".to_owned())
        .map(RawArgs::new)
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::api::RawArgs;

    #[rstest]
    #[case::empty("", &[])]
    #[case::single_key_value("--some=yes", &["--some=yes"])]
    #[case::two_key_value("--some=yes --other=no", &["--some=yes", "--other=no"])]
    #[case::single_escaped("--some='yes and no'", &["--some=yes and no"])]
    #[case::double_escaped("--some='\"yes and no\"'", &["--some=\"yes and no\""])]
    #[case::multiple_escaped("--some='yes and no' --other='no and yes'", &["--some=yes and no", "--other=no and yes"])]
    fn test_parse_callgrind_args(#[case] value: &str, #[case] expected: &[&str]) {
        let actual = parse_args(value).unwrap();
        assert_eq!(actual, RawArgs::from_iter(expected));
    }
}