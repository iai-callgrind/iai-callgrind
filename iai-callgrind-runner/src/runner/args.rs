use clap::builder::BoolishValueParser;
use clap::{ArgAction, Parser};

use super::format::OutputFormat;
use super::summary::{BaselineName, SummaryFormat};
use crate::api::{EventKind, RawArgs, RegressionConfig};

/// The command line arguments the user provided after `--` when running cargo bench
///
/// These arguments are not the command line arguments passed to `iai-callgrind-runner`. We collect
/// the command line arguments in the `iai-callgrind::main!` macro without the binary as first
/// argument, that's why `no_binary_name` is set to `true`.
#[derive(Parser, Debug, Clone)]
#[clap(
    author,
    version,
    about = "High-precision and consistent benchmarking framework/harness for Rust",
    long_about = None,
    no_binary_name = true,
)]
pub struct CommandLineArgs {
    /// `--bench` usually shows up as last argument set by cargo and not by us.
    ///
    /// This argument is useless, so we sort it out and never make use of it.
    #[clap(long = "bench", hide = true, action = ArgAction::SetTrue, required = false)]
    pub _bench: bool,

    /// The raw arguments to pass through to Callgrind
    ///
    /// This is a space separated list of command-line-arguments specified as if they were
    /// passed directly to valgrind.
    ///
    /// Examples:
    ///   * --callgrind-args=--dump-instr=yes
    ///   * --callgrind-args='--dump-instr=yes --collect-systime=yes'
    #[clap(
        long = "callgrind-args",
        required = false,
        value_parser = parse_args,
        takes_value = true,
        verbatim_doc_comment
    )]
    pub callgrind_args: Option<RawArgs>,

    /// Save a machine-readable summary of each benchmark run in json format next to the usual
    /// benchmark output
    #[clap(
        long = "save-summary",
        value_enum,
        required = false,
        default_missing_value = "json",
        env = "IAI_CALLGRIND_SAVE_SUMMARY"
    )]
    pub save_summary: Option<SummaryFormat>,

    /// Allow ASLR (Address Space Layout Randomization)
    ///
    /// If possible, ASLR is disabled on platforms that support it (linux, freebsd) because ASLR
    /// could noise up the callgrind cache simulation results a bit. Setting this option to true
    /// runs all benchmarks with ASLR enabled.
    ///
    /// See also https://docs.kernel.org/admin-guide/sysctl/kernel.html?highlight=randomize_va_space#randomize-va-space
    #[clap(
        long = "allow-aslr",
        env = "IAI_CALLGRIND_ALLOW_ASLR",
        default_missing_value = "yes",
        value_parser = BoolishValueParser::new(),
        )]
    pub allow_aslr: Option<bool>,

    /// Set performance regression limits for specific `EventKinds`
    ///
    /// This is a `,` separate list of EventKind=limit (key=value) pairs with the limit being a
    /// positive or negative percentage. If positive, a performance regression check for this
    /// `EventKind` fails if the limit is exceeded. If negative, the regression check fails if the
    /// value comes below the limit. The `EventKind` is matched case insensitive. For a list of
    /// valid `EventKinds` see the docs: https://docs.rs/iai-callgrind/latest/iai_callgrind/enum.EventKind.html
    ///
    /// Examples: --regression='ir=0.0' or --regression='ir=0, EstimatedCycles=10'
    #[clap(
        required = false,
        long = "regression",
        env = "IAI_CALLGRIND_REGRESSION",
        value_parser = parse_regression_config
    )]
    pub regression: Option<RegressionConfig>,

    /// If true, the first failed performance regression check fails the whole benchmark run
    ///
    /// This option requires --regression=... or IAI_CALLGRIND_REGRESSION=... to be present.
    #[clap(
        long = "regression-fail-fast",
        env = "IAI_CALLGRIND_REGRESSION_FAIL_FAST",
        requires = "regression",
        default_missing_value = "yes",
        value_parser = BoolishValueParser::new()
    )]
    pub regression_fail_fast: Option<bool>,

    /// Compare against this baseline if present and then overwrite it
    #[clap(
        long = "save-baseline",
        env = "IAI_CALLGRIND_SAVE_BASELINE",
        default_missing_value = "default",
        conflicts_with_all = &["baseline", "LOAD_BASELINE"]
    )]
    pub save_baseline: Option<BaselineName>,

    /// Compare against this baseline if present but do not overwrite it
    #[clap(
        long = "baseline",
        env = "IAI_CALLGRIND_BASELINE",
        default_missing_value = "default"
    )]
    pub baseline: Option<BaselineName>,

    /// Load this baseline as the new data set instead of creating a new one
    #[clap(
        id = "LOAD_BASELINE",
        long = "load-baseline",
        requires = "baseline",
        env = "IAI_CALLGRIND_LOAD_BASELINE"
    )]
    pub load_baseline: Option<BaselineName>,

    /// The terminal output format in default human-readable format or in machine-readable json
    /// format
    ///
    /// # The JSON Output Format
    ///
    /// The json terminal output schema is the same as the schema with the `--save-summary`
    /// argument when saving to a `summary.json` file. All other output than the json output goes
    /// to stderr and only the summary output goes to stdout. When not printing pretty json, each
    /// line is a dictionary summarizing a single benchmark. You can combine all lines
    /// (benchmarks) into an array for example with `jq`
    ///
    /// `cargo bench -- --output-format=json | jq -s`
    ///
    /// which transforms `{...}\n{...}` into `[{...},{...}]`
    #[clap(
        long = "output-format",
        value_enum,
        required = false,
        default_value = "default",
        env = "IAI_CALLGRIND_OUTPUT_FORMAT"
    )]
    pub output_format: OutputFormat,
}

/// This function parses a space separated list of raw argument strings into [`crate::api::RawArgs`]
fn parse_args(value: &str) -> Result<RawArgs, String> {
    shlex::split(value)
        .ok_or_else(|| "Failed to split callgrind args".to_owned())
        .map(RawArgs::new)
}

fn parse_regression_config(value: &str) -> Result<RegressionConfig, String> {
    let value = value.trim();
    if value.is_empty() {
        return Err("No limits found: At least one limit must be specified".to_owned());
    }

    let regression_config = if value.eq_ignore_ascii_case("default") {
        RegressionConfig::default()
    } else {
        let mut limits = vec![];

        for split in value.split(',') {
            let split = split.trim();

            if let Some((key, value)) = split.split_once('=') {
                let (key, value) = (key.trim(), value.trim());
                let event_kind = EventKind::from_str_ignore_case(key)
                    .ok_or_else(|| -> String { format!("Unknown event kind: '{key}'") })?;

                let pct = value.parse::<f64>().map_err(|error| -> String {
                    format!("Invalid percentage for '{key}': {error}")
                })?;
                limits.push((event_kind, pct));
            } else {
                return Err(format!("Invalid format of key/value pair: '{split}'"));
            }
        }

        RegressionConfig {
            limits,
            ..Default::default()
        }
    };

    Ok(regression_config)
}

impl From<&CommandLineArgs> for Option<RegressionConfig> {
    fn from(value: &CommandLineArgs) -> Self {
        let mut config = value.regression.clone();
        if let Some(config) = config.as_mut() {
            config.fail_fast = value.regression_fail_fast;
        }
        config
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::api::EventKind::*;
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

    #[rstest]
    #[case::regression_default("default", vec![])]
    #[case::regression_default_case_insensitive("DefAulT", vec![])]
    #[case::regression_only("Ir=10", vec![(Ir, 10f64)])]
    #[case::regression_case_insensitive("EstIMATedCycles=10", vec![(EstimatedCycles, 10f64)])]
    #[case::multiple_regression("Ir=10,EstimatedCycles=5", vec![(Ir, 10f64), (EstimatedCycles, 5f64)])]
    #[case::multiple_regression_with_whitespace("Ir= 10 ,  EstimatedCycles = 5", vec![(Ir, 10f64), (EstimatedCycles, 5f64)])]
    fn test_parse_regression_config(
        #[case] regression_var: &str,
        #[case] expected_limits: Vec<(EventKind, f64)>,
    ) {
        let expected = RegressionConfig {
            limits: expected_limits,
            fail_fast: None,
        };

        let actual = parse_regression_config(regression_var).unwrap();
        assert_eq!(actual, expected);
    }

    #[rstest]
    #[case::regression_wrong_format_of_key_value_pair(
        "Ir:10",
        "Invalid format of key/value pair: 'Ir:10'"
    )]
    #[case::regression_unknown_event_kind("WRONG=10", "Unknown event kind: 'WRONG'")]
    #[case::regression_invalid_percentage(
        "Ir=10.0.0",
        "Invalid percentage for 'Ir': invalid float literal"
    )]
    #[case::regression_empty_limits("", "No limits found: At least one limit must be specified")]
    fn test_try_regression_config_from_env_then_error(
        #[case] regression_var: &str,
        #[case] expected_reason: &str,
    ) {
        assert_eq!(
            &parse_regression_config(regression_var).unwrap_err(),
            expected_reason,
        );
    }
}
