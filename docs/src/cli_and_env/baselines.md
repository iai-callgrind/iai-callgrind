<!-- markdownlint-disable MD041 MD033 -->
# Comparing with baselines

Usually, two consecutive benchmark runs let Iai-Callgrind compare these two
runs. It's sometimes desirable to compare the current benchmark run against a
static reference, instead. For example, if you're working longer on the
implementation of a feature, you may wish to compare against a baseline from
another branch or the commit from which you started off hacking on your new
feature to make sure you haven't introduced performance regressions.
Iai-Callgrind offers such custom baselines. If you are familiar with
[criterion.rs](https://bheisler.github.io/criterion.rs/book/user_guide/command_line_options.html#baselines),
the following command line arguments should also be very familiar to you:

- `--save-baseline=NAME` (env: `IAI_CALLGRIND_SAVE_BASELINE`): Compare against
  the `NAME` baseline if present and then overwrite it.
- `--baseline=NAME` (env: `IAI_CALLGRIND_BASELINE`): Compare against the `NAME`
  baseline without overwriting it
- `--load-baseline=NAME` (env: `IAI_CALLGRIND_LOAD_BASELINE`): Load the `NAME`
  baseline as the `new` data set instead of creating a new one. This option
  needs also `--baseline=NAME` to be present.

If `NAME` is not present, `NAME` defaults to `default`.

For example to create a static reference from the main branch and compare it:

```shell
git checkout main
cargo bench --bench <benchmark> -- --save-baseline=main
git checkout feature
# ... HACK ... HACK
cargo bench --bench <benchmark> -- --baseline main
```

Sticking to the above execution sequence,

```shell
cargo bench --bench my_benchmark -- --save-baseline=main
```

prints something like that with an additional line `Baselines` in the output.

<pre><code class="hljs"><span style="color:#0A0">my_benchmark::my_group::bench_library</span>
  Baselines:        <b>           main</b>|main
  Instructions:     <b>            280</b>|N/A             (<span style="color:#555">*********</span>)
  L1 Hits:          <b>            374</b>|N/A             (<span style="color:#555">*********</span>)
  LL Hits:          <b>              1</b>|N/A             (<span style="color:#555">*********</span>)
  RAM Hits:         <b>              6</b>|N/A             (<span style="color:#555">*********</span>)
  Total read+write: <b>            381</b>|N/A             (<span style="color:#555">*********</span>)
  Estimated Cycles: <b>            589</b>|N/A             (<span style="color:#555">*********</span>)

Iai-Callgrind result: <b><span style="color:#0A0">Ok</span></b>. 1 without regressions; 0 regressed; 1 benchmarks finished in 0.49333s</code></pre>

After you've made some changes to your code, running

```shell
cargo bench --bench my_benchmark -- --baseline=main`
```

prints something like the following:

<pre><code class="hljs"><span style="color:#0A0">my_benchmark::my_group::bench_library</span>
  Baselines:                       |main
  Instructions:     <b>            214</b>|280             (<b><span style="color:#42c142">-23.5714%</span></b>) [<b><span style="color:#42c142">-1.30841x</span></b>]
  L1 Hits:          <b>            287</b>|374             (<b><span style="color:#42c142">-23.2620%</span></b>) [<b><span style="color:#42c142">-1.30314x</span></b>]
  LL Hits:          <b>              1</b>|1               (<span style="color:#555">No change</span>)
  RAM Hits:         <b>              6</b>|6               (<span style="color:#555">No change</span>)
  Total read+write: <b>            294</b>|381             (<b><span style="color:#42c142">-22.8346%</span></b>) [<b><span style="color:#42c142">-1.29592x</span></b>]
  Estimated Cycles: <b>            502</b>|589             (<b><span style="color:#42c142">-14.7708%</span></b>) [<b><span style="color:#42c142">-1.17331x</span></b>]

Iai-Callgrind result: <b><span style="color:#0A0">Ok</span></b>. 1 without regressions; 0 regressed; 1 benchmarks finished in 0.49333s</code></pre>
