# Basic usage

It's possible to pass arguments to Iai-Callgrind separated by `--` (`cargo bench
-- ARGS`). If you're running into the error `Unrecognized Option`, see
[Troubleshooting](../troubleshooting/running-cargo-bench-results-in-an-unrecognized-option-error.md).
For a complete rundown of possible arguments, execute `cargo bench --bench
<benchmark> -- --help`. Almost all command-line arguments have a corresponding
environment variable. The environment variables which don't have a corresponding
command-line argument are:

- `IAI_CALLGRIND_COLOR`: [Control the colored output of Iai-Callgrind](./output/color.md) (Default
  is `auto`)
- `IAI_CALLGRIND_LOG`: [Define the log level](./output/logging.md) (Default is `WARN`)
