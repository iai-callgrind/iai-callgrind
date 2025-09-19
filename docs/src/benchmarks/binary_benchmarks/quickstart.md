<!-- markdownlint-disable MD041 MD033 -->
# Quickstart

Suppose the crate's binary is called `my-foo` and this binary takes a file path
as positional argument. This first example shows the basic usage of the
high-level api with the `#[binary_benchmark]` attribute:

```rust
# extern crate gungraun;
# macro_rules! env { ($m:tt) => {{ "/some/path" }} }
use gungraun::{binary_benchmark, binary_benchmark_group, main};

#[binary_benchmark]
#[bench::some_id("foo.txt")]
fn bench_binary(path: &str) -> gungraun::Command {
    gungraun::Command::new(env!("CARGO_BIN_EXE_my-foo"))
        .arg(path)
        .build()
}

binary_benchmark_group!(
    name = my_group;
    benchmarks = bench_binary
);

# fn main() {
main!(binary_benchmark_groups = my_group);
# }
```

If you want to try out this example with your crate's binary, put the above code
into a file in `$WORKSPACE_ROOT/benches/binary_benchmark.rs`. Next, replace
`my-foo` in `env!("CARGO_BIN_EXE_my-foo")` with the name of a binary of your
crate.

Note the `env!` macro is a [rust](https://doc.rust-lang.org/std/macro.env.html)
builtin macro and `CARGO_BIN_EXE_<name>` is documented
[rust stdlib](https://doc.rust-lang.org/cargo/reference/environment-variables.html#environment-variables-cargo-sets-for-crates).

You should always use `env!("CARGO_BIN_EXE_<name>")` to determine the path to
the binary of your crate. Do not use relative paths like `target/release/my-foo`
since this might break your benchmarks in many ways. The environment variable
does exactly the right thing and the usage is short and simple.

Lastly, adjust the argument of the `Command` and add the following to your
`Cargo.toml`:

```toml
[[bench]]
name = "binary_benchmark"
harness = false
```

Running

```shell
cargo bench
```

presents you with something like the following:

<pre><code class="hljs"><span style="color:#0A0">binary_benchmark::my_group::bench_binary</span> <span style="color:#0AA">some_id</span><span style="color:#0AA">:</span><b><span style="color:#00A">("foo.txt") -> target/release/my-foo foo.txt</span></b>
  Instructions:     <b>         342129</b>|N/A             (<span style="color:#555">*********</span>)
  L1 Hits:          <b>         457370</b>|N/A             (<span style="color:#555">*********</span>)
  LL Hits:          <b>            734</b>|N/A             (<span style="color:#555">*********</span>)
  RAM Hits:         <b>           4096</b>|N/A             (<span style="color:#555">*********</span>)
  Total read+write: <b>         462200</b>|N/A             (<span style="color:#555">*********</span>)
  Estimated Cycles: <b>         604400</b>|N/A             (<span style="color:#555">*********</span>)

Gungraun result: <b><span style="color:#0A0">Ok</span></b>. 1 without regressions; 0 regressed; 1 benchmarks finished in 0.49333s</code></pre>

As opposed to library benchmarks, binary benchmarks have access to a [low-level
api](./low_level.md). Here, pretty much the same as the above high-level usage
but written in the low-level api:

```rust
# extern crate gungraun;
# macro_rules! env { ($m:tt) => {{ "/some/path" }} }
use gungraun::{BinaryBenchmark, Bench, binary_benchmark_group, main};

binary_benchmark_group!(
    name = my_group;
    benchmarks = |group: &mut BinaryBenchmarkGroup| {
        group.binary_benchmark(BinaryBenchmark::new("bench_binary")
            .bench(Bench::new("some_id")
                .command(gungraun::Command::new(env!("CARGO_BIN_EXE_my-foo"))
                    .arg("foo.txt")
                    .build()
                )
            )
        )
    }
);

# fn main() {
main!(binary_benchmark_groups = my_group);
# }
```

If in doubt, use the high-level api. You can still
[migrate](./low_level.md#intermixing-high-level-and-low-level-api) to the
low-level api very easily if you really need to. The other way around is more
involved.
