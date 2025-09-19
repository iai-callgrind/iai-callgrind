# Basic usage

It's possible to pass arguments to Gungraun separated by `--` (`cargo bench
-- ARGS`). If you're running into the error `Unrecognized Option`, see
[Troubleshooting](../troubleshooting/running-cargo-bench-results-in-an-unrecognized-option-error.md).
For a complete rundown of possible arguments, execute `cargo bench --bench
<benchmark> -- --help`. Almost all command-line arguments have a corresponding
environment variable. The environment variables which don't have a corresponding
command-line argument are:

- `IAI_CALLGRIND_COLOR`: [Control the colored output of Gungraun](./output/color.md) (Default
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
      --list[=<LIST>]
          Print a list of all benchmarks. With this argument no benchmarks are executed.

          The output format is intended to be the same as the output format of the libtest harness.
          However, future changes of the output format by cargo might not be incorporated into
          iai-callgrind. As a consequence, it is not considered safe to rely on the output in
          scripts.

          [env: IAI_CALLGRIND_LIST=]
          [default: false]
          [possible values: true, false]

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
          option to benchmark library functions needs adjustments to the benchmarking functions with
          client-requests to measure the counts correctly. If you want to switch permanently to
          cachegrind, it is usually better to activate the `cachegrind` feature of iai-callgrind in
          your Cargo.toml. However, setting a tool with this option overrides cachegrind set with the
          iai-callgrind feature. See the guide for all details.

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

      --allow-aslr[=<ALLOW_ASLR>]
          Allow ASLR (Address Space Layout Randomization)

          If possible, ASLR is disabled on platforms that support it (linux, freebsd) because ASLR
          could noise up the callgrind cache simulation results a bit. Setting this option to true
          runs all benchmarks with ASLR enabled.

          See also
          <https://docs.kernel.org/admin-guide/sysctl/kernel.html?highlight=randomize_va_space#randomize-va-space>

          [env: IAI_CALLGRIND_ALLOW_ASLR=]
          [possible values: true, false]

      --home <HOME>
          Specify the home directory of iai-callgrind benchmark output files

          All output files are per default stored under the `$PROJECT_ROOT/target/iai` directory.
          This option lets you customize this home directory, and it will be created if it doesn't
          exist.

          [env: IAI_CALLGRIND_HOME=]

      --separate-targets[=<SEPARATE_TARGETS>]
          Separate iai-callgrind benchmark output files by target

          The default output path for files created by iai-callgrind and valgrind during the
          benchmark is

          `target/iai/$PACKAGE_NAME/$BENCHMARK_FILE/$GROUP/$BENCH_FUNCTION.$BENCH_ID`.

          This can be problematic if you're running the benchmarks not only for a single target
          because you end up comparing the benchmark runs with the wrong targets. Setting this option
          changes the default output path to

          `target/iai/$TARGET/$PACKAGE_NAME/$BENCHMARK_FILE/$GROUP/$BENCH_FUNCTION.$BENCH_ID`

          Although not as comfortable and strict, you could achieve a separation by target also with
          baselines and a combination of `--save-baseline=$TARGET` and `--baseline=$TARGET` if you
          prefer having all files of a single $BENCH in the same directory.

          [env: IAI_CALLGRIND_SEPARATE_TARGETS=]
          [default: false]
          [possible values: true, false]

      --baseline[=<BASELINE>]
          Compare against this baseline if present but do not overwrite it

          [env: IAI_CALLGRIND_BASELINE=]

      --load-baseline[=<LOAD_BASELINE>]
          Load this baseline as the new data set instead of creating a new one

          [env: IAI_CALLGRIND_LOAD_BASELINE=]

      --save-baseline[=<SAVE_BASELINE>]
          Compare against this baseline if present and then overwrite it

          [env: IAI_CALLGRIND_SAVE_BASELINE=]

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

      --nosummary[=<NOSUMMARY>]
          Suppress the summary showing regressions and execution time at the end of a benchmark run

          Note, that a summary is only printed if the `--output-format` is not JSON.

          The summary described by `--nosummary` is different from `--save-summary` and they do not
          affect each other.

          [env: IAI_CALLGRIND_NOSUMMARY=]
          [default: false]
          [possible values: true, false]

      --output-format <OUTPUT_FORMAT>
          The terminal output format in default human-readable format or in machine-readable json
          format

          # The JSON Output Format

          The json terminal output schema is the same as the schema with the `--save-summary`
          argument when saving to a `summary.json` file. All other output than the json output goes
          to stderr and only the summary output goes to stdout. When not printing pretty json, each
          line is a dictionary summarizing a single benchmark. You can combine all lines (benchmarks)
          into an array for example with `jq`

          `cargo bench -- --output-format=json | jq -s`

          which transforms `{...}\n{...}` into `[{...},{...}]`

          [env: IAI_CALLGRIND_OUTPUT_FORMAT=]
          [default: default]

          Possible values:
          - default:     The default terminal output
          - json:        Json terminal output
          - pretty-json: Pretty json terminal output

      --save-summary[=<SAVE_SUMMARY>]
          Save a machine-readable summary of each benchmark run in json format next to the usual
          benchmark output

          [env: IAI_CALLGRIND_SAVE_SUMMARY=]

          Possible values:
          - json:        The format in a space optimal json representation without newlines
          - pretty-json: The format in pretty printed json

      --tolerance[=<TOLERANCE>]
          Show changes only when they are above the `tolerance` level

          If no value is specified, the default value of `0.000_009_999_999_999_999_999` is based on
          the number of decimal places of the percentages displayed in the terminal output in case of
          differences.

          Negative tolerance values are converted to their absolute value.

          Examples:
          * --tolerance (applies the default value)
          * --tolerance=0.1 (set the tolerance level to `0.1`)

          [env: IAI_CALLGRIND_TOLERANCE=]

      --bbv-args <BBV_ARGS>
          The command-line arguments to pass through to the experimental BBV

          <https://valgrind.org/docs/manual/bbv-manual.html#bbv-manual.usage>. See also the
          description for --callgrind-args for more details and restrictions.

          Examples:
            * --bbv-args=--interval-size=10000
            * --bbv-args='--interval-size=10000 --instr-count-only=yes'

          [env: IAI_CALLGRIND_BBV_ARGS=]

      --cachegrind-args <CACHEGRIND_ARGS>
          The command-line arguments to pass through to Cachegrind

          <https://valgrind.org/docs/manual/cg-manual.html#cg-manual.cgopts>. See also the
          description for --callgrind-args for more details and restrictions.

          Examples:
            * --cachegrind-args=--intr-at-start=no
            * --cachegrind-args='--branch-sim=yes --instr-at-start=no'

          [env: IAI_CALLGRIND_CACHEGRIND_ARGS=]

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

      --dhat-args <DHAT_ARGS>
          The command-line arguments to pass through to DHAT

          <https://valgrind.org/docs/manual/dh-manual.html#dh-manual.options>. See also the
          description for --callgrind-args for more details and restrictions.

          Examples:
            * --dhat-args=--mode=ad-hoc

          [env: IAI_CALLGRIND_DHAT_ARGS=]

      --drd-args <DRD_ARGS>
          The command-line arguments to pass through to DRD

          <https://valgrind.org/docs/manual/drd-manual.html#drd-manual.options>. See also the
          description for --callgrind-args for more details and restrictions.

          Examples:
            * --drd-args=--exclusive-threshold=100
            * --drd-args='--exclusive-threshold=100 --free-is-write=yes'

          [env: IAI_CALLGRIND_DRD_ARGS=]

      --helgrind-args <HELGRIND_ARGS>
          The command-line arguments to pass through to Helgrind

          <https://valgrind.org/docs/manual/hg-manual.html#hg-manual.options>. See also the
          description for --callgrind-args for more details and restrictions.

          Examples:
            * --helgrind-args=--free-is-write=yes
            * --helgrind-args='--conflict-cache-size=100000 --free-is-write=yes'

          [env: IAI_CALLGRIND_HELGRIND_ARGS=]

      --massif-args <MASSIF_ARGS>
          The command-line arguments to pass through to Massif

          <https://valgrind.org/docs/manual/ms-manual.html#ms-manual.options>. See also the
          description for --callgrind-args for more details and restrictions.

          Examples:
            * --massif-args=--heap=no
            * --massif-args='--heap=no --threshold=2.0'

          [env: IAI_CALLGRIND_MASSIF_ARGS=]

      --memcheck-args <MEMCHECK_ARGS>
          The command-line arguments to pass through to Memcheck

          <https://valgrind.org/docs/manual/mc-manual.html#mc-manual.options>. See also the
          description for --callgrind-args for more details and restrictions.

          Examples:
            * --memcheck-args=--leak-check=full
            * --memcheck-args='--leak-check=yes --show-leak-kinds=all'

          [env: IAI_CALLGRIND_MEMCHECK_ARGS=]

      --valgrind-args <VALGRIND_ARGS>
          The command-line arguments to pass through to all tools

          The core valgrind command-line arguments
          <https://valgrind.org/docs/manual/manual-core.html#manual-core.options> which are
          recognized by all tools. More specific arguments for example set with --callgrind-args
          override the arguments with the same name specified with this option.

          Examples:
            * --valgrind-args=--time-stamp=yes
            * --valgrind-args='--error-exitcode=202 --num-callers=50'

          [env: IAI_CALLGRIND_VALGRIND_ARGS=]

      --cachegrind-limits <CACHEGRIND_LIMITS>
          Set performance regression limits for specific cachegrind metrics

          This is a `,` separate list of CachegrindMetric=limit or CachegrindMetrics=limit
          (key=value) pairs. See the description of --callgrind-limits for the details and
          <https://docs.rs/iai-callgrind/latest/iai_callgrind/enum.CachegrindMetrics.html>
          respectively
          <https://docs.rs/iai-callgrind/latest/iai_callgrind/enum.CachegrindMetric.html>
          for valid metrics and group members.

          See the the guide
          (https://gungraun.github.io/gungraun/latest/html/regressions.html) for all
          details or replace the format spec in `--callgrind-limits` with the following:

          group ::= "@" ( "default"
                        | "all"
                        | ("cachemisses" | "misses" | "ms")
                        | ("cachemissrates" | "missrates" | "mr")
                        | ("cachehits" | "hits" | "hs")
                        | ("cachehitrates" | "hitrates" | "hr")
                        | ("cachesim" | "cs")
                        | ("branchsim" | "bs")
                        )
          event ::= CachegrindMetric

          Examples:
          * --cachegrind-limits='ir=0.0%'
          * --cachegrind-limits='ir=10000,EstimatedCycles=10%'
          * --cachegrind-limits='@all=10%,ir=10000,EstimatedCycles=10%'

          [env: IAI_CALLGRIND_CACHEGRIND_LIMITS=]

      --callgrind-limits <CALLGRIND_LIMITS>
          Set performance regression limits for specific `EventKinds`

          This is a `,` separate list of EventKind=limit or CallgrindMetrics=limit (key=value) pairs
          with the limit being a soft limit if the number suffixed with a `%` or a hard limit if it
          is a bare number. It is possible to specify hard and soft limits in one go with the `|`
          operator (e.g. `ir=10%|10000`). Groups (CallgrindMetrics) are prefixed with `@`. List of
          allowed groups and events with their abbreviations:

          group ::= "@" ( "default"
                        | "all"
                        | ("cachemisses" | "misses" | "ms")
                        | ("cachemissrates" | "missrates" | "mr")
                        | ("cachehits" | "hits" | "hs")
                        | ("cachehitrates" | "hitrates" | "hr")
                        | ("cachesim" | "cs")
                        | ("cacheuse" | "cu")
                        | ("systemcalls" | "syscalls" | "sc")
                        | ("branchsim" | "bs")
                        | ("writebackbehaviour" | "writeback" | "wb")
                        )
          event ::= EventKind

          See the guide (https://gungraun.github.io/gungraun/latest/html/regressions.html)
          for more details, the docs of `CallgrindMetrics`
          (<https://docs.rs/iai-callgrind/latest/iai_callgrind/enum.CallgrindMetrics.html>) and
          `EventKind` <https://docs.rs/iai-callgrind/latest/iai_callgrind/enum.EventKind.html> for a
          list of metrics and groups with their members.

          A performance regression check for an `EventKind` fails if the limit is exceeded. If
          limits are defined and one or more regressions have occurred during the benchmark run,
          the whole benchmark is considered to have failed and the program exits with error and
          exit code `3`.

          Examples:
          * --callgrind-limits='ir=5.0%'
          * --callgrind-limits='ir=10000,EstimatedCycles=10%'
          * --callgrind-limits='@all=10%,ir=5%|10000'

          [env: IAI_CALLGRIND_CALLGRIND_LIMITS=]

      --dhat-limits <DHAT_LIMITS>
          Set performance regression limits for specific dhat metrics

          This is a `,` separate list of DhatMetrics=limit or DhatMetric=limit (key=value) pairs. See
          the description of --callgrind-limits for the details and
          <https://docs.rs/iai-callgrind/latest/iai_callgrind/enum.DhatMetrics.html> respectively
          <https://docs.rs/iai-callgrind/latest/iai_callgrind/enum.DhatMetric.html> for valid metrics
          and group members.

          See the the guide
          (https://gungraun.github.io/gungraun/latest/html/regressions.html) for all
          details or replace the format spec in `--callgrind-limits` with the following:

          group ::= "@" ( "default" | "all" )
          event ::=   ( "totalunits" | "tun" )
                    | ( "totalevents" | "tev" )
                    | ( "totalbytes" | "tb" )
                    | ( "totalblocks" | "tbk" )
                    | ( "attgmaxbytes" | "gb" )
                    | ( "attgmaxblocks" | "gbk" )
                    | ( "attendbytes" | "eb" )
                    | ( "attendblocks" | "ebk" )
                    | ( "readsbytes" | "rb" )
                    | ( "writesbytes" | "wb" )
                    | ( "totallifetimes" | "tl" )
                    | ( "maximumbytes" | "mb" )
                    | ( "maximumblocks" | "mbk" )

          `events` with a long name have their allowed abbreviations placed in the same parentheses.

          Examples:
          * --dhat-limits='totalbytes=0.0%'
          * --dhat-limits='totalbytes=10000,totalblocks=5%'
          * --dhat-limits='@all=10%,totalbytes=5000,totalblocks=5%'

          [env: IAI_CALLGRIND_DHAT_LIMITS=]

      --regression-fail-fast[=<REGRESSION_FAIL_FAST>]
          If true, the first failed performance regression check fails the whole benchmark run

          Note that if --regression-fail-fast is set to true, no summary is printed.

          [env: IAI_CALLGRIND_REGRESSION_FAIL_FAST=]
          [possible values: true, false]

      --cachegrind-metrics <CACHEGRIND_METRICS>...
          Define the cachegrind metrics and the order in which they are displayed

          This is a `,`-separated list of cachegrind metric groups and event kinds which are allowed
          to appear in the terminal output of cachegrind.

          See `--callgrind-metrics` for more details and
          <https://docs.rs/iai-callgrind/latest/iai_callgrind/enum.CachegrindMetrics.html>
          respectively
          <https://docs.rs/iai-callgrind/latest/iai_callgrind/enum.CachegrindMetric.html> for valid
          metrics and group members.

          The `group` names, their abbreviations if present and `event` kinds are exactly the same as
          described in the `--cachegrind-limits` option.

          Examples:
          * --cachegrind-metrics='ir' to show only `Instructions`
          * --cachegrind-metrics='@all' to show all possible cachegrind metrics
          * --cachegrind-metrics='@default,@mr' to show cache miss rates in addition to the defaults

          [env: IAI_CALLGRIND_CACHEGRIND_METRICS=]

      --callgrind-metrics <CALLGRIND_METRICS>...
          Define the callgrind metrics and the order in which they are displayed

          This is a `,`-separated list of callgrind metric groups and event kinds which are allowed
          to appear in the terminal output of callgrind. Group names need to be prefixed with '@'.
          The order matters and the callgrind metrics are shown in their insertion order of this
          option. More precisely, in case of duplicate metrics, the first specified one wins.

          The `group` names, their abbreviations if present and `event` kinds are exactly the same as
          described in the `--callgrind-limits` option.

          For a list of valid metrics, groups and their members see the docs of `CallgrindMetrics`
          (<https://docs.rs/iai-callgrind/latest/iai_callgrind/enum.CallgrindMetrics.html>) and
          `EventKind` <https://docs.rs/iai-callgrind/latest/iai_callgrind/enum.EventKind.html>.

          Note that setting the metrics here does not imply that these metrics are actually
          collected. This option just sets the order and appearance of metrics in case they are
          collected. To activate the collection of specific metrics you need to use
          `--callgrind-args`.

          Examples:
          * --callgrind-metrics='ir' to show only `Instructions`
          * --callgrind-metrics='@all' to show all possible callgrind metrics
          * --callgrind-metrics='@default,@mr' to show cache miss rates in addition to the defaults

          [env: IAI_CALLGRIND_CALLGRIND_METRICS=]

      --dhat-metrics <DHAT_METRICS>...
          Define the dhat metrics and the order in which they are displayed

          This is a `,`-separated list of dhat metric groups and event kinds which are allowed to
          appear in the terminal output of dhat.

          See `--callgrind-metrics` for more details and
          <https://docs.rs/iai-callgrind/latest/iai_callgrind/enum.DhatMetrics.html> respectively
          <https://docs.rs/iai-callgrind/latest/iai_callgrind/enum.DhatMetric.html> for valid metrics
          and group members.

          The `group` names, their abbreviations if present and `event` kinds are exactly the same as
          described in the `--dhat-limits` option.

          Examples:
          * --dhat-metrics='totalbytes' to show only `Total Bytes`
          * --dhat-metrics='@all' to show all possible dhat metrics
          * --dhat-metrics='@default,mb' to show maximum bytes in addition to the defaults

          [env: IAI_CALLGRIND_DHAT_METRICS=]

      --drd-metrics <DRD_METRICS>...
          Define the drd error metrics and the order in which they are displayed

          This is a `,`-separated list of error metrics which are allowed to appear in the terminal
          output of drd. The `group` and `event` are the same as for `--memcheck-metrics`.

          See `--callgrind-metrics` for more details and
          <https://docs.rs/iai-callgrind/latest/iai_callgrind/enum.ErrorMetric.html> for valid error
          metrics.

          Since this is a very small set of metrics, there is only one `group`: `@all`

          Examples:
          * --drd-metrics='errors' to show only `Errors`
          * --drd-metrics='@all' to show all possible error metrics (the default)
          * --drd-metrics='err,ctx' to show only errors and contexts

          [env: IAI_CALLGRIND_DRD_METRICS=]

      --helgrind-metrics <HELGRIND_METRICS>...
          Define the helgrind error metrics and the order in which they are displayed

          This is a `,`-separated list of error metrics which are allowed to appear in the terminal
          output of helgrind. The `group` and `event` are the same as for `--memcheck-metrics`.

          See `--callgrind-metrics` for more details and
          <https://docs.rs/iai-callgrind/latest/iai_callgrind/enum.ErrorMetric.html> for valid error
          metrics.

          Examples:
          * --helgrind-metrics='errors' to show only `Errors`
          * --helgrind-metrics='@all' to show all possible error metrics (the default)
          * --helgrind-metrics='err,ctx' to show only errors and contexts

          [env: IAI_CALLGRIND_HELGRIND_METRICS=]

      --memcheck-metrics <MEMCHECK_METRICS>...
          Define the memcheck error metrics and the order in which they are displayed

          This is a `,`-separated list of error metrics which are allowed to appear in the terminal
          output of memcheck.

          Since this is a very small set of metrics, there is only one `group`: `@all`

          group ::= "@all"
          event ::=   ( "errors" | "err" )
                    | ( "contexts" | "ctx" )
                    | ( "suppressederrors" | "serr")
                    | ( "suppressedcontexts" | "sctx" )

          See `--callgrind-metrics` for more details and
          <https://docs.rs/iai-callgrind/latest/iai_callgrind/enum.ErrorMetric.html> for valid
          metrics.

          Examples:
          * --memcheck-metrics='errors' to show only `Errors`
          * --memcheck-metrics='@all' to show all possible error metrics (the default)
          * --memcheck-metrics='err,ctx' to show only errors and contexts

          [env: IAI_CALLGRIND_MEMCHECK_METRICS=]

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
