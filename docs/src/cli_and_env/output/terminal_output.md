<!-- markdownlint-disable MD041 MD033 -->

# Showing terminal output of benchmarks

Per default, all terminal output of the benchmark function, `setup` and
`teardown` is captured and therefore not shown during a benchmark run.

## Using the log level

The most basic possibility to show any captured output, is to use
[`IAI_CALLGRIND_LOG=info`](./logging.md). This includes a lot of other output,
too.

## Tell Iai-Callgrind to not capture the output

Another nicer possibility is, to tell Iai-Callgrind to not capture output with
the `--nocapture` (env: `IAI_CALLGRIND_NOCAPTURE`) option. This is currently
restricted to the `callgrind` run to prevent showing the same output multiple
times. So, any terminal output of [other tool runs](../../tools.md) is still
captured.

The `--nocapture` flag takes the special values `stdout` and `stderr` in
addition to `true` and `false`:

`--nocapture=true|false|stdout|stderr`

In the `--nocapture=stdout` case, terminal output to `stdout` is not captured
and shown during the benchmark run but output to `stderr` is discarded.
Likewise, `--nocapture=stderr` shows terminal output to `stderr` but discards
output to `stdout`.

Let's take as example a library benchmark `benches/my_benchmark.rs`

```rust
# extern crate iai_callgrind;
use iai_callgrind::{library_benchmark, library_benchmark_group, main};
use std::hint::black_box;

fn print_to_stderr(value: u64) {
    eprintln!("Error output during teardown: {value}");
}

fn add_10_and_print(value: u64) -> u64 {
    let value = value + 10;
    println!("Output to stdout: {value}");

    value
}

#[library_benchmark]
#[bench::some_id(args = (10), teardown = print_to_stderr)]
fn bench_library(value: u64) -> u64 {
    black_box(add_10_and_print(value))
}

library_benchmark_group!(name = my_group; benchmarks = bench_library);
# fn main() {
main!(library_benchmark_groups = my_group);
# }
```

If the above benchmark is run with `cargo bench --bench my_benchmark --
--nocapture`, the output of Iai-Callgrind will look like this:

<pre><code class="hljs"><span style="color:#0A0">my_benchmark::my_group::bench_library</span> <span style="color:#0AA">some_id</span><span style="color:#0AA">:</span><b><span style="color:#00A">10</span></b>
Output to stdout: 20
Error output during teardown: 20
<span style="color:#A50">-</span> <span style="color:#A50">end of stdout/stderr</span>
  Instructions:     <b>            851</b>|N/A             (<span style="color:#555">*********</span>)
  L1 Hits:          <b>           1193</b>|N/A             (<span style="color:#555">*********</span>)
  L2 Hits:          <b>              5</b>|N/A             (<span style="color:#555">*********</span>)
  RAM Hits:         <b>             66</b>|N/A             (<span style="color:#555">*********</span>)
  Total read+write: <b>           1264</b>|N/A             (<span style="color:#555">*********</span>)
  Estimated Cycles: <b>           3528</b>|N/A             (<span style="color:#555">*********</span>)</code></pre>

Everything between the headline and the `- end of stdout/stderr` line is output
from your benchmark. The `- end of stdout/stderr` line changes depending on the
options you have given. For example in the `--nocapture=stdout` case this line
indicates your chosen option with `- end of stdout`.

Note that independently of the value of the `--nocapture` option, all logging
output of a valgrind tool itself is stored in files in the output directory of
the benchmark. Since Iai-Callgrind needs the logging output of valgrind tools
stored in files, there is no option to disable the creation of these log files.
But, if anything goes sideways you might be glad to have the log files around.
