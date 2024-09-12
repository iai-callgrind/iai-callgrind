# Low-level api

I'm not going into full detail of the low-level api here since it is fully
documented in the [api
Documentation](https://docs.rs/iai-callgrind/0.13.4/iai_callgrind/index.html).

## The basic structure

The entry point of the low-level api is the `binary_benchmark_group`

```rust
# extern crate iai_callgrind;
# macro_rules! env { ($m:tt) => {{ "/some/path" }} }
use iai_callgrind::{
     binary_benchmark, binary_benchmark_attribute, binary_benchmark_group, main,
     BinaryBenchmark, Bench
};

binary_benchmark_group!(
    name = my_group;
    benchmarks = |group: &mut BinaryBenchmarkGroup| {
        group.binary_benchmark(BinaryBenchmark::new("bench_binary")
            .bench(Bench::new("some_id")
                .command(iai_callgrind::Command::new(env!("CARGO_BIN_EXE_my-foo"))
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

The low-level api mirrors the high-level api, "structifying" the macros.

The `binary_benchmark_group!` is also a struct now, the `BinaryBenchmarkGroup`.
It cannot be instantiated. Instead, it is passed as argument to the expression
of the `benchmarks` parameter in a `binary_benchmark_group`. You can choose any
name instead of `group`, we just have used `group` throughout the examples.

There's the shorter `benchmarks = |group| /* ... */` instead of `benchmarks =
|group: &mut BinaryBenchmarkGroup| /* ... */`. We use the more verbose variant
in the examples because it is more informative for benchmarking starters.

Furthermore, the `#[library_benchmark]` macro correlates with
`iai_callgrind::LibraryBenchmark` and `#[bench]` with `iai_callgrind::Bench`.
The parameters of the macros are now functions in the respective structs. The
return value of the benchmark function, the `iai-callgrind::Command`, is now
also a function `iai-callgrind::Bench::command`.

Note there is no `iai-callgrind::Benches` struct since specifying multiple
commands with `iai_callgrind::Bench::command` behaves exactly the same way as
the `#[benches]` attribute. So, the `file` parameter of `#[benches]` is a part
of `iai-callgrind::Bench` and can be used with the `iai-callgrind::Bench::file`
function.

## Intermixing high-level and low-level api

It is recommended to start with the high-level api using the
`#[binary_benchmark]` attribute, since you can fall back to the low-level api in
a few steps with the `binary_benchmark_attribute!` macro as shown below. The
other way around is much more involved.

```rust
# extern crate iai_callgrind;
# macro_rules! env { ($m:tt) => {{ "/some/path" }} }
use iai_callgrind::{
     binary_benchmark, binary_benchmark_attribute, binary_benchmark_group, main,
     BinaryBenchmark, Bench
};

#[binary_benchmark]
#[bench::some_id("foo")]
fn attribute_benchmark(arg: &str) -> iai_callgrind::Command {
    iai_callgrind::Command::new(env!("CARGO_BIN_EXE_my-binary"))
        .arg(arg)
        .build()
}

binary_benchmark_group!(
    name = low_level;
    benchmarks = |group: &mut BinaryBenchmarkGroup| {
        group
            .binary_benchmark(binary_benchmark_attribute!(attribute_benchmark))
            .binary_benchmark(
                BinaryBenchmark::new("low_level_benchmark")
                    .bench(
                        Bench::new("some_id").command(
                            iai_callgrind::Command::new(env!("CARGO_BIN_EXE_my-binary"))
                                .arg("bar")
                                .build()
                        )
                    )
            )
    }
);

# fn main() {
main!(binary_benchmark_groups = low_level);
# }
```

As shown above, there's no need to transcribe the function `attribute_benchmark`
with the `#[binary_benchmark]` attribute into the low-level api structures. Just
keep it as it is and add it to a the `group` with
`group.binary_benchmark(binary_benchmark_attribute(attribute_benchmark))`.
That's it! You can continue hacking on your benchmarks in the low-level api.
