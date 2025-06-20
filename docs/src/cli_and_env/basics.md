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

## Exit Codes

- **0**: Success
- **1**: All other errors
- **2**: Parsing command-line arguments failed
- **3**: One or more regressions occurred

## The command-line arguments

For an update-to-date list run `cargo bench` with `--help` as described above.

```text
High-precision and consistent benchmarking framework/harness for Rust

Boolish command line arguments take also one of `y`, `yes`, `t`, `true`, `on`, `1`
instead of `true` and one of `n`, `no`, `f`, `false`, `off`, and `0` instead of
`false`

Usage: cargo bench ... [BENCHNAME] -- [OPTIONS]

Arguments:
  [BENCHNAME]
          If specified, only run benches containing this string in their names

          Note that a benchmark name might differ from the benchmark file name.

          [env: IAI_CALLGRIND_FILTER=]

Options:
      --default-tool <DEFAULT_TOOL>
          The default tool used to run the benchmarks

          The standard tool to run the benchmarks is callgrind but can be overridden with this
          option. Any valgrind tool can be used:
            * callgrind
            * cachegrind
            * dhat
            * memcheck
            * helgrind
            * drd
            * massif
            * exp-bbv

          This argument matches the tool case-insensitive. Note that using cachegrind with this
          option to benchmark library functions needs adjustments to the benchmarking functions
          with client-requests to measure the counts correctly. If you want to switch permanently
          to cachegrind, it is usually better to activate the `cachegrind` feature of
          iai-callgrind in your Cargo.toml. However, setting a tool with this option overrides
          cachegrind set with the iai-callgrind feature. See the guide for all details.

          [env: IAI_CALLGRIND_DEFAULT_TOOL=]

      --tools <TOOLS>...
          A comma separated list of tools to run additionally to callgrind or another default tool

          The tools specified here take precedence over the tools in the benchmarks. The valgrind
          tools which are allowed here are the same as the ones listed in the documentation of
          --default-tool.

          Examples
            * --tools dhat
            * --tools memcheck,drd

          [env: IAI_CALLGRIND_TOOLS=]

      --valgrind-args <VALGRIND_ARGS>
          The command-line arguments to pass through to all tools

          The core valgrind command-line arguments
          <https://valgrind.org/docs/manual/manual-core.html#manual-core.options> which are
          recognized
          by all tools. More specific arguments for example set with --callgrind-args override the
          arguments with the same name specified with this option.

          Examples:
            * --valgrind-args=--time-stamp=yes
            * --valgrind-args='--error-exitcode=202 --num-callers=50'

          [env: IAI_CALLGRIND_VALGRIND_ARGS=]

      --callgrind-args <CALLGRIND_ARGS>
          The command-line arguments to pass through to Callgrind

          <https://valgrind.org/docs/manual/cl-manual.html#cl-manual.options> and the core valgrind
          command-line arguments
          <https://valgrind.org/docs/manual/manual-core.html#manual-core.options>. Note that not all
          command-line arguments are supported especially the ones which change output paths.
          Unsupported arguments will be ignored printing a warning.

          Examples:
            * --callgrind-args=--dump-instr=yes
            * --callgrind-args='--dump-instr=yes --collect-systime=yes'

          [env: IAI_CALLGRIND_CALLGRIND_ARGS=]

      --cachegrind-args <CACHEGRIND_ARGS>
          The command-line arguments to pass through to Cachegrind

          <https://valgrind.org/docs/manual/cg-manual.html#cg-manual.cgopts>. See also the
          description
          for --callgrind-args for more details and restrictions.

          Examples:
            * --cachegrind-args=--intr-at-start=no
            * --cachegrind-args='--branch-sim=yes --instr-at-start=no'

          [env: IAI_CALLGRIND_CACHEGRIND_ARGS=]

      --dhat-args <DHAT_ARGS>
          The command-line arguments to pass through to DHAT

          <https://valgrind.org/docs/manual/dh-manual.html#dh-manual.options>. See also the
          description
          for --callgrind-args for more details and restrictions.

          Examples:
            * --dhat-args=--mode=ad-hoc

          [env: IAI_CALLGRIND_DHAT_ARGS=]

      --memcheck-args <MEMCHECK_ARGS>
          The command-line arguments to pass through to Memcheck

          <https://valgrind.org/docs/manual/mc-manual.html#mc-manual.options>. See also the
          description
          for --callgrind-args for more details and restrictions.

          Examples:
            * --memcheck-args=--leak-check=full
            * --memcheck-args='--leak-check=yes --show-leak-kinds=all'

          [env: IAI_CALLGRIND_MEMCHECK_ARGS=]

      --helgrind-args <HELGRIND_ARGS>
          The command-line arguments to pass through to Helgrind

          <https://valgrind.org/docs/manual/hg-manual.html#hg-manual.options>. See also the
          description
          for --callgrind-args for more details and restrictions.

          Examples:
            * --helgrind-args=--free-is-write=yes
            * --helgrind-args='--conflict-cache-size=100000 --free-is-write=yes'

          [env: IAI_CALLGRIND_HELGRIND_ARGS=]

      --drd-args <DRD_ARGS>
          The command-line arguments to pass through to DRD

          <https://valgrind.org/docs/manual/drd-manual.html#drd-manual.options>. See also the
          description
          for --callgrind-args for more details and restrictions.

          Examples:
            * --drd-args=--exclusive-threshold=100
            * --drd-args='--exclusive-threshold=100 --free-is-write=yes'

          [env: IAI_CALLGRIND_DRD_ARGS=]

      --massif-args <MASSIF_ARGS>
          The command-line arguments to pass through to Massif

          <https://valgrind.org/docs/manual/ms-manual.html#ms-manual.options>. See also the
          description
          for --callgrind-args for more details and restrictions.

          Examples:
            * --massif-args=--heap=no
            * --massif-args='--heap=no --threshold=2.0'

          [env: IAI_CALLGRIND_MASSIF_ARGS=]

      --bbv-args <BBV_ARGS>
          The command-line arguments to pass through to the experimental BBV

          <https://valgrind.org/docs/manual/bbv-manual.html#bbv-manual.usage>. See also the
          description
          for --callgrind-args for more details and restrictions.

          Examples:
            * --bbv-args=--interval-size=10000
            * --bbv-args='--interval-size=10000 --instr-count-only=yes'

          [env: IAI_CALLGRIND_BBV_ARGS=]

      --save-summary[=<SAVE_SUMMARY>]
          Save a machine-readable summary of each benchmark run in json format next to the usual
          benchmark output

          [env: IAI_CALLGRIND_SAVE_SUMMARY=]

          Possible values:
          - json:        The format in a space optimal json representation without newlines
          - pretty-json: The format in pretty printed json

      --allow-aslr[=<ALLOW_ASLR>]
          Allow ASLR (Address Space Layout Randomization)

          If possible, ASLR is disabled on platforms that support it (linux, freebsd) because ASLR
          could noise up the callgrind cache simulation results a bit. Setting this option to true
          runs all benchmarks with ASLR enabled.

          See also
          <https://docs.kernel.org/admin-guide/sysctl/kernel.html?highlight=randomize_va_space#randomize-va-space>

          [env: IAI_CALLGRIND_ALLOW_ASLR=]
          [possible values: true, false]

      --callgrind-limits <CALLGRIND_LIMITS>
          Set performance regression limits for specific `EventKinds`

          This is a `,` separate list of EventKind=limit (key=value) pairs with the limit being a
          positive or negative percentage. If positive, a performance regression check for this
          `EventKind` fails if the limit is exceeded. If negative, the regression check fails if the
          value comes below the limit. The `EventKind` is matched case-insensitive. For a list of
          valid `EventKinds` see the docs:
          <https://docs.rs/iai-callgrind/latest/iai_callgrind/enum.EventKind.html>

          If regressions are defined and one ore more regressions occurred during the benchmark run
          the program exits with error and exit code `3`.

          Examples: --callgrind-limits='ir=0.0' or --callgrind-limits='ir=0, EstimatedCycles=10'

          [env: IAI_CALLGRIND_CALLGRIND_LIMITS=]

      --cachegrind-limits <CACHEGRIND_LIMITS>
          Set performance regression limits for specific cachegrind metrics

          This is a `,` separate list of CachegrindMetric=limit (key=value) pairs. See the
          description of --callgrind-limits for the details and
          <https://docs.rs/iai-callgrind/latest/iai_callgrind/enum.CachegrindMetric.html> for valid
          metrics.

          Examples: --cachegrind-limits='ir=0.0' or --cachegrind-limits='ir=0, EstimatedCycles=10'

          [env: IAI_CALLGRIND_CACHEGRIND_LIMITS=]

      --regression-fail-fast[=<REGRESSION_FAIL_FAST>]
          If true, the first failed performance regression check fails the whole benchmark run

          Note that if --regression-fail-fast is set to true, no summary is printed.

          [env: IAI_CALLGRIND_REGRESSION_FAIL_FAST=]
          [possible values: true, false]

      --save-baseline[=<SAVE_BASELINE>]
          Compare against this baseline if present and then overwrite it

          [env: IAI_CALLGRIND_SAVE_BASELINE=]

      --baseline[=<BASELINE>]
          Compare against this baseline if present but do not overwrite it

          [env: IAI_CALLGRIND_BASELINE=]

      --load-baseline[=<LOAD_BASELINE>]
          Load this baseline as the new data set instead of creating a new one

          [env: IAI_CALLGRIND_LOAD_BASELINE=]

      --output-format <OUTPUT_FORMAT>
          The terminal output format in default human-readable format or in machine-readable json
          format

          # The JSON Output Format

          The json terminal output schema is the same as the schema with the `--save-summary`
          argument when saving to a `summary.json` file. All other output than the json output goes
          to stderr and only the summary output goes to stdout. When not printing pretty json, each
          line is a dictionary summarizing a single benchmark. You can combine all lines
          (benchmarks) into an array for example with `jq`

          `cargo bench -- --output-format=json | jq -s`

          which transforms `{...}\n{...}` into `[{...},{...}]`

          [env: IAI_CALLGRIND_OUTPUT_FORMAT=]
          [default: default]
          [possible values: default, json, pretty-json]

      --separate-targets[=<SEPARATE_TARGETS>]
          Separate iai-callgrind benchmark output files by target

          The default output path for files created by iai-callgrind and valgrind during the
          benchmark is

          `target/iai/$PACKAGE_NAME/$BENCHMARK_FILE/$GROUP/$BENCH_FUNCTION.$BENCH_ID`.

          This can be problematic if you're running the benchmarks not only for a single target
          because you end up comparing the benchmark runs with the wrong targets. Setting this
          option changes the default output path to

          `target/iai/$TARGET/$PACKAGE_NAME/$BENCHMARK_FILE/$GROUP/$BENCH_FUNCTION.$BENCH_ID`

          Although not as comfortable and strict, you could achieve a separation by target also with
          baselines and a combination of `--save-baseline=$TARGET` and `--baseline=$TARGET` if you
          prefer having all files of a single $BENCH in the same directory.

          [env: IAI_CALLGRIND_SEPARATE_TARGETS=]
          [default: false]
          [possible values: true, false]

      --home <HOME>
          Specify the home directory of iai-callgrind benchmark output files

          All output files are per default stored under the `$PROJECT_ROOT/target/iai` directory.
          This option lets you customize this home directory, and it will be created if it doesn't
          exist.

          [env: IAI_CALLGRIND_HOME=]

      --nocapture[=<NOCAPTURE>]
          Don't capture terminal output of benchmarks

          Possible values are one of [true, false, stdout, stderr].

          This option is currently restricted to the `callgrind` run of benchmarks. The output of
          additional tool runs like DHAT, Memcheck, ... is still captured, to prevent showing the
          same output of benchmarks multiple times. Use `IAI_CALLGRIND_LOG=info` to also show
          captured and logged output.

          If no value is given, the default missing value is `true` and doesn't capture stdout and
          stderr. Besides `true` or `false` you can specify the special values `stdout` or `stderr`.
          If `--nocapture=stdout` is given, the output to `stdout` won't be captured and the output
          to `stderr` will be discarded. Likewise, if `--nocapture=stderr` is specified, the output
          to `stderr` won't be captured and the output to `stdout` will be discarded.

          [env: IAI_CALLGRIND_NOCAPTURE=]
          [default: false]

      --list[=<LIST>]
          Print a list of all benchmarks. With this argument no benchmarks are executed.

          The output format is intended to be the same as the output format of the libtest harness.
          However, future changes of the output format by cargo might not be incorporated into
          iai-callgrind. As a consequence, it is not considered safe to rely on the output in
          scripts.

          [env: IAI_CALLGRIND_LIST=]
          [default: false]
          [possible values: true, false]

      --nosummary[=<NOSUMMARY>]
          Suppress the summary showing regressions and execution time at the end of a benchmark run

          Note, that a summary is only printed if the `--output-format` is not JSON.

          The summary described by `--nosummary` is different from `--save-summary` and they do not
          affect each other.

          [env: IAI_CALLGRIND_NOSUMMARY=]
          [default: false]
          [possible values: true, false]

  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version

  Exit codes:
      0: Success
      1: All other errors
      2: Parsing command-line arguments failed
      3: One or more regressions occurred
```
