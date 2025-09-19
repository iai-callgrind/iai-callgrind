# Structure of a library benchmark

We're reusing our example from the [Quickstart](./quickstart.md) section.

```rust
# extern crate iai_callgrind;
use iai_callgrind::{main, library_benchmark_group, library_benchmark};
use std::hint::black_box;

fn fibonacci(n: u64) -> u64 {
    match n {
        0 => 1,
        1 => 1,
        n => fibonacci(n - 1) + fibonacci(n - 2),
    }
}

#[library_benchmark]
#[bench::short(10)]
#[bench::long(30)]
fn bench_fibonacci(value: u64) -> u64 {
    black_box(fibonacci(value))
}

library_benchmark_group!(
    name = bench_fibonacci_group;
    benchmarks = bench_fibonacci
);

# fn main() {
main!(library_benchmark_groups = bench_fibonacci_group);
# }
```

First of all, you need a public function in your library which you want to
benchmark. In this example this is the `fibonacci` function which, for the sake
of simplicity, lives in the benchmark file itself but doesn't have to. If it
had been located in `my_lib::fibonacci`, you simply import that function
with `use my_lib::fibonacci` and go on as shown above. Next, you need a
`library_benchmark_group!` in which you specify the names of the benchmark
functions. Finally, the benchmark harness is created by the `main!` macro.

## The benchmark function

The benchmark function has to be annotated with the
[`#[library_benchmark]`](./macros.md) attribute. The
[`#[bench]`](./macros.md) attribute is an inner attribute of the
`#[library_benchmark]` attribute. It consists of a mandatory id (the `ID` part
in `#[bench::ID(/* ... */)]`) and in its most basic form, an optional list of
arguments which are passed to the benchmark function as parameters. Naturally,
the parameters of the benchmark function must match the argument list of the
`#[bench]` attribute. It is always a good idea to return something from the
benchmark function, here it is the computed `u64` value from the `fibonacci`
function wrapped in a `black_box`. See the docs of
[`std::hint::black_box`](https://doc.rust-lang.org/std/hint/fn.black_box.html)
for more information about its usage. Simply put, _all_ values and variables in
the benchmarking function (but not in your library function) need to be wrapped
in a `black_box` except for the input parameters (here `value`) because
Gungraun already does that. But, it is no error to `black_box` the `value`
again.

The `bench` attribute takes any expression which includes function calls. The
following would have worked too and is one way to avoid the costs of the setup
code being attributed to the benchmarked function.

```rust
# extern crate iai_callgrind;
use iai_callgrind::{main, library_benchmark_group, library_benchmark};
use std::hint::black_box;

fn some_setup_func(value: u64) -> u64 {
    value + 10
}

fn fibonacci(n: u64) -> u64 {
    match n {
        0 => 1,
        1 => 1,
        n => fibonacci(n - 1) + fibonacci(n - 2),
    }
}

#[library_benchmark]
#[bench::short(10)]
// Note the usage of the `some_setup_func` in the argument list of this #[bench]
#[bench::long(some_setup_func(20))]
fn bench_fibonacci(value: u64) -> u64 {
    black_box(fibonacci(value))
}

library_benchmark_group!(
   name = bench_fibonacci_group;
   benchmarks = bench_fibonacci
);

# fn main() {
main!(library_benchmark_groups = bench_fibonacci_group);
# }
```

The perhaps most crucial part in setting up library benchmarks is to keep the
body of benchmark functions clean from any setup or teardown code. There are
other ways to avoid setup and teardown code in the benchmark function,
which are discussed in full detail in the [setup and
teardown](./setup_and_teardown.md) section.

## The group

The name of the benchmark functions, here the only benchmark function
`bench_fibonacci`, which should be benchmarked need to be specified in a
`library_benchmark_group!` in the `benchmarks` parameter. You can create as many
groups as you like, and you can use it to organize related benchmarks. Each group
needs a unique `name`.

## The main macro

Each group you want to be benchmarked needs to be specified in the
`library_benchmark_groups` parameter of the `main!` macro and you're all set.
