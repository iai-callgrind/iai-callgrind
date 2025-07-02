<!-- markdownlint-disable MD025 MD042 MD033 -->

# Comparing benchmark functions

Comparing benchmark functions is supported via the optional
`library_benchmark_group!` argument `compare_by_id` (The default value for
`compare_by_id` is `false`). Only benches with the same `id` are compared, which
allows to single out cases which don't need to be compared. In the following
example, the `case_3` and `multiple` bench are compared with each other in
addition to the usual comparison with the previous run:

```rust
# extern crate iai_callgrind;
# mod my_lib { pub fn bubble_sort(_: Vec<i32>) -> Vec<i32> { vec![] } }
use iai_callgrind::{library_benchmark, library_benchmark_group, main};
use std::hint::black_box;

#[library_benchmark]
#[bench::case_3(vec![1, 2, 3])]
#[benches::multiple(args = [vec![1, 2], vec![1, 2, 3, 4]])]
fn bench_bubble_sort_best_case(input: Vec<i32>) -> Vec<i32> {
    black_box(my_lib::bubble_sort(input))
}

#[library_benchmark]
#[bench::case_3(vec![3, 2, 1])]
#[benches::multiple(args = [vec![2, 1], vec![4, 3, 2, 1]])]
fn bench_bubble_sort_worst_case(input: Vec<i32>) -> Vec<i32> {
    black_box(my_lib::bubble_sort(input))
}

library_benchmark_group!(
    name = bench_bubble_sort;
    compare_by_id = true;
    benchmarks = bench_bubble_sort_best_case, bench_bubble_sort_worst_case
);

# fn main() {
main!(library_benchmark_groups = bench_bubble_sort);
# }
```

Note if `compare_by_id` is `true`, all benchmark functions are compared with
each other, so you are not limited to two benchmark functions per comparison
group.

Here's the benchmark output of the above example to see what is happening:

<pre><code class="hljs"><span style="color:#0A0">my_benchmark::bubble_sort_group::bubble_sort_best_case</span> <span style="color:#0AA">case_2</span><span style="color:#0AA">:</span><b><span style="color:#00A">vec! [1, 2]</span></b>
  Instructions:     <b>             63</b>|N/A             (<span style="color:#555">*********</span>)
  L1 Hits:          <b>             86</b>|N/A             (<span style="color:#555">*********</span>)
  LL Hits:          <b>              1</b>|N/A             (<span style="color:#555">*********</span>)
  RAM Hits:         <b>              4</b>|N/A             (<span style="color:#555">*********</span>)
  Total read+write: <b>             91</b>|N/A             (<span style="color:#555">*********</span>)
  Estimated Cycles: <b>            231</b>|N/A             (<span style="color:#555">*********</span>)
<span style="color:#0A0">my_benchmark::bubble_sort_group::bubble_sort_best_case</span> <span style="color:#0AA">multiple_0</span><span style="color:#0AA">:</span><b><span style="color:#00A">vec! [1, 2, 3]</span></b>
  Instructions:     <b>             94</b>|N/A             (<span style="color:#555">*********</span>)
  L1 Hits:          <b>            123</b>|N/A             (<span style="color:#555">*********</span>)
  LL Hits:          <b>              1</b>|N/A             (<span style="color:#555">*********</span>)
  RAM Hits:         <b>              4</b>|N/A             (<span style="color:#555">*********</span>)
  Total read+write: <b>            128</b>|N/A             (<span style="color:#555">*********</span>)
  Estimated Cycles: <b>            268</b>|N/A             (<span style="color:#555">*********</span>)
<span style="color:#0A0">my_benchmark::bubble_sort_group::bubble_sort_best_case</span> <span style="color:#0AA">multiple_1</span><span style="color:#0AA">:</span><b><span style="color:#00A">vec! [1, 2, 3, 4]</span></b>
  Instructions:     <b>            136</b>|N/A             (<span style="color:#555">*********</span>)
  L1 Hits:          <b>            174</b>|N/A             (<span style="color:#555">*********</span>)
  LL Hits:          <b>              1</b>|N/A             (<span style="color:#555">*********</span>)
  RAM Hits:         <b>              4</b>|N/A             (<span style="color:#555">*********</span>)
  Total read+write: <b>            179</b>|N/A             (<span style="color:#555">*********</span>)
  Estimated Cycles: <b>            319</b>|N/A             (<span style="color:#555">*********</span>)
<span style="color:#0A0">my_benchmark::bubble_sort_group::bubble_sort_worst_case</span> <span style="color:#0AA">case_2</span><span style="color:#0AA">:</span><b><span style="color:#00A">vec! [2, 1]</span></b>
  Instructions:     <b>             66</b>|N/A             (<span style="color:#555">*********</span>)
  L1 Hits:          <b>             91</b>|N/A             (<span style="color:#555">*********</span>)
  LL Hits:          <b>              1</b>|N/A             (<span style="color:#555">*********</span>)
  RAM Hits:         <b>              4</b>|N/A             (<span style="color:#555">*********</span>)
  Total read+write: <b>             96</b>|N/A             (<span style="color:#555">*********</span>)
  Estimated Cycles: <b>            236</b>|N/A             (<span style="color:#555">*********</span>)
  <b><span style="color:#A50">Comparison with</span></b> <span style="color:#0A0">bubble_sort_best_case</span> <span style="color:#0AA">case_2</span>:<b><span style="color:#00A">vec! [1, 2]</span></b>
  Instructions:     <b>             63</b>|66              (<b><span style="color:#42c142">-4.54545%</span></b>) [<b><span style="color:#42c142">-1.04762x</span></b>]
  L1 Hits:          <b>             86</b>|91              (<b><span style="color:#42c142">-5.49451%</span></b>) [<b><span style="color:#42c142">-1.05814x</span></b>]
  LL Hits:          <b>              1</b>|1               (<span style="color:#555">No change</span>)
  RAM Hits:         <b>              4</b>|4               (<span style="color:#555">No change</span>)
  Total read+write: <b>             91</b>|96              (<b><span style="color:#42c142">-5.20833%</span></b>) [<b><span style="color:#42c142">-1.05495x</span></b>]
  Estimated Cycles: <b>            231</b>|236             (<b><span style="color:#42c142">-2.11864%</span></b>) [<b><span style="color:#42c142">-1.02165x</span></b>]
<span style="color:#0A0">my_benchmark::bubble_sort_group::bubble_sort_worst_case</span> <span style="color:#0AA">multiple_0</span><span style="color:#0AA">:</span><b><span style="color:#00A">vec! [3, 2, 1]</span></b>
  Instructions:     <b>            103</b>|N/A             (<span style="color:#555">*********</span>)
  L1 Hits:          <b>            138</b>|N/A             (<span style="color:#555">*********</span>)
  LL Hits:          <b>              1</b>|N/A             (<span style="color:#555">*********</span>)
  RAM Hits:         <b>              4</b>|N/A             (<span style="color:#555">*********</span>)
  Total read+write: <b>            143</b>|N/A             (<span style="color:#555">*********</span>)
  Estimated Cycles: <b>            283</b>|N/A             (<span style="color:#555">*********</span>)
  <b><span style="color:#A50">Comparison with</span></b> <span style="color:#0A0">bubble_sort_best_case</span> <span style="color:#0AA">multiple_0</span>:<b><span style="color:#00A">vec! [1, 2, 3]</span></b>
  Instructions:     <b>             94</b>|103             (<b><span style="color:#42c142">-8.73786%</span></b>) [<b><span style="color:#42c142">-1.09574x</span></b>]
  L1 Hits:          <b>            123</b>|138             (<b><span style="color:#42c142">-10.8696%</span></b>) [<b><span style="color:#42c142">-1.12195x</span></b>]
  LL Hits:          <b>              1</b>|1               (<span style="color:#555">No change</span>)
  RAM Hits:         <b>              4</b>|4               (<span style="color:#555">No change</span>)
  Total read+write: <b>            128</b>|143             (<b><span style="color:#42c142">-10.4895%</span></b>) [<b><span style="color:#42c142">-1.11719x</span></b>]
  Estimated Cycles: <b>            268</b>|283             (<b><span style="color:#42c142">-5.30035%</span></b>) [<b><span style="color:#42c142">-1.05597x</span></b>]
<span style="color:#0A0">my_benchmark::bubble_sort_group::bubble_sort_worst_case</span> <span style="color:#0AA">multiple_1</span><span style="color:#0AA">:</span><b><span style="color:#00A">vec! [4, 3, 2, 1]</span></b>
  Instructions:     <b>            154</b>|N/A             (<span style="color:#555">*********</span>)
  L1 Hits:          <b>            204</b>|N/A             (<span style="color:#555">*********</span>)
  LL Hits:          <b>              1</b>|N/A             (<span style="color:#555">*********</span>)
  RAM Hits:         <b>              4</b>|N/A             (<span style="color:#555">*********</span>)
  Total read+write: <b>            209</b>|N/A             (<span style="color:#555">*********</span>)
  Estimated Cycles: <b>            349</b>|N/A             (<span style="color:#555">*********</span>)
  <b><span style="color:#A50">Comparison with</span></b> <span style="color:#0A0">bubble_sort_best_case</span> <span style="color:#0AA">multiple_1</span>:<b><span style="color:#00A">vec! [1, 2, 3, 4]</span></b>
  Instructions:     <b>            136</b>|154             (<b><span style="color:#42c142">-11.6883%</span></b>) [<b><span style="color:#42c142">-1.13235x</span></b>]
  L1 Hits:          <b>            174</b>|204             (<b><span style="color:#42c142">-14.7059%</span></b>) [<b><span style="color:#42c142">-1.17241x</span></b>]
  LL Hits:          <b>              1</b>|1               (<span style="color:#555">No change</span>)
  RAM Hits:         <b>              4</b>|4               (<span style="color:#555">No change</span>)
  Total read+write: <b>            179</b>|209             (<b><span style="color:#42c142">-14.3541%</span></b>) [<b><span style="color:#42c142">-1.16760x</span></b>]
  Estimated Cycles: <b>            319</b>|349             (<b><span style="color:#42c142">-8.59599%</span></b>) [<b><span style="color:#42c142">-1.09404x</span></b>]

Iai-Callgrind result: <b><span style="color:#0A0">Ok</span></b>. 6 without regressions; 0 regressed; 6 benchmarks finished in 1.58123s</code></pre>

The procedure of the comparison algorithm:

1. Run all benches in the first benchmark function
2. Run the first bench in the second benchmark function and if there is a bench
   in the first benchmark function with the same id compare them
3. Run the second bench in the second benchmark function ...
4. ...
5. Run the first bench in the third benchmark function and if there is a bench
   in the first benchmark function with the same id compare them. If there is a
   bench with the same id in the second benchmark function compare them.
6. Run the second bench in the third benchmark function ...
7. and so on ... until all benches are compared with each other

Neither the order nor the amount of benches within the benchmark functions
matters, so it is not strictly necessary to mirror the bench ids of the first
benchmark function in the second, third, etc. benchmark function.
