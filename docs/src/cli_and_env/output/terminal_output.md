<!-- markdownlint-disable MD041 MD033 -->
# Just an example

Make a change

<pre>
   <code class="hljs"><span style="color:#0A0">test_bin_bench_sandbox::my_group::with_sandbox</span> <span style="color:#0AA">sandbox_with_fixture</span><span style="color:#0AA">:</span><b><span style="color:#00A">("one_line.fix", true) -> target/release/file-exists one_line.fix true</span></b>
  Instructions:     <b>         340625</b>|340625          (<span style="color:#555">No change</span>)
  L1 Hits:          <b>         455224</b>|455224          (<span style="color:#555">No change</span>)
  L2 Hits:          <b>            731</b>|731             (<span style="color:#555">No change</span>)
  RAM Hits:         <b>           4044</b>|4044            (<span style="color:#555">No change</span>)
  Total read+write: <b>         459999</b>|459999          (<span style="color:#555">No change</span>)
  Estimated Cycles: <b>         600419</b>|600419          (<span style="color:#555">No change</span>)
<span style="color:#0A0">test_bin_bench_sandbox::my_group::with_sandbox</span> <span style="color:#0AA">sandbox_without_fixture</span><span style="color:#0AA">:</span><b><span style="color:#00A">("one_line.fix", false) -> target/release/file-exists one_line.fix false</span></b>
  Instructions:     <b>         340594</b>|340594          (<span style="color:#555">No change</span>)
  L1 Hits:          <b>         455169</b>|455169          (<span style="color:#555">No change</span>)
  L2 Hits:          <b>            731</b>|731             (<span style="color:#555">No change</span>)
  RAM Hits:         <b>           4041</b>|4041            (<span style="color:#555">No change</span>)
  Total read+write: <b>         459941</b>|459941          (<span style="color:#555">No change</span>)
  Estimated Cycles: <b>         600259</b>|600259          (<span style="color:#555">No change</span>)
<span style="color:#0A0">test_bin_bench_sandbox::my_group::without_sandbox</span> <span style="color:#0AA">check_file</span><span style="color:#0AA">:</span><b><span style="color:#00A">("benches/fixtures/one_line.fix", true) -> target/release/file-exists benches/fixtures/one_line.fix true</span></b>
  Instructions:     <b>         340493</b>|340493          (<span style="color:#555">No change</span>)
  L1 Hits:          <b>         455062</b>|455062          (<span style="color:#555">No change</span>)
  L2 Hits:          <b>            732</b>|732             (<span style="color:#555">No change</span>)
  RAM Hits:         <b>           4046</b>|4046            (<span style="color:#555">No change</span>)
  Total read+write: <b>         459840</b>|459840          (<span style="color:#555">No change</span>)
  Estimated Cycles: <b>         600332</b>|600332          (<span style="color:#555">No change</span>)
<span style="color:#0A0">test_bin_bench_sandbox::my_group::with_current_dir</span> <b><span style="color:#00A">() -> target/release/file-exists bar.txt true</span></b>
  Instructions:     <b>         340552</b>|340552          (<span style="color:#555">No change</span>)
  L1 Hits:          <b>         455139</b>|455139          (<span style="color:#555">No change</span>)
  L2 Hits:          <b>            730</b>|730             (<span style="color:#555">No change</span>)
  RAM Hits:         <b>           4041</b>|4041            (<span style="color:#555">No change</span>)
  Total read+write: <b>         459910</b>|459910          (<span style="color:#555">No change</span>)
  Estimated Cycles: <b>         600224</b>|600224          (<span style="color:#555">No change</span>)</code>
</pre>

```rust
# extern crate iai_callgrind;
use iai_callgrind::{main, library_benchmark_group, library_benchmark};

fn my_teardown(value: u64) {
    eprintln!("Error output during teardown: {value}");
}

fn to_be_benchmarked(value: u64) -> u64 {
    println!("Output to stdout: {value}");
    value + 10
}

#[library_benchmark]
#[bench::some_id(args = (10), teardown = my_teardown)]
fn my_bench(value: u64) -> u64 {
    to_be_benchmarked(value)
}

library_benchmark_group!(
    name = my_bench_group;
    benchmarks = my_bench
);

# fn main() {
main!(library_benchmark_groups = my_bench_group);
# }
```

```text
Just some text
```
