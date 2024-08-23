# Machine-readable output

With `--output-format=default|json|pretty-json` (env:
`IAI_CALLGRIND_OUTPUT_FORMAT`) you can change the terminal output format to the
machine-readable json format. The json schema fully describing the json output
is stored in
[summary.v2.schema.json](https://github.com/iai-callgrind/iai-callgrind/blob/main/iai-callgrind-runner/schemas/summary.v2.schema.json).
Each line of json output (if not `pretty-json`) is a summary of a single
benchmark and you may want to combine all benchmarks in an array. You can do so
for example with `jq`

`cargo bench -- --output-format=json | jq -s`

which transforms `{...}\n{...}` into `[{...},{...}]`.

Instead of, or in addition to changing the terminal output, it's possible to
save a summary file for each benchmark with `--save-summary=json|pretty-json`
(env: `IAI_CALLGRIND_SAVE_SUMMARY`). The `summary.json` files are stored next to
the usual benchmark output files in the `target/iai` directory.
