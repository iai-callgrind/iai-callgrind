# Differences to library benchmarks

In this section we're going through the differences to [library
benchmarks](../library_benchmarks.md). This assumes that you already know how to
set up library benchmarks and it is recommended to learn the very basics about
library benchmarks, starting with
[Quickstart](../binary_benchmarks/quickstart.md), [Anatomy of a library
benchmark](../library_benchmarks/anatomy.md) and [The macros in more
detail](../library_benchmarks/macros.md). Then come back to this section.

## Name changes

Coming from library benchmarks, the names with `library` in it change to the
same name but `library` with `binary` replaced, so the `#[library_benchmark]`
attribute's name changes to `#[binary_benchmark]` and `library_benchmark_group!`
changes to `binary_benchmark_group!`, the config arguments take a
`BinaryBenchmarkConfig` instead of a `LibraryBenchmarkConfig`...

A quick reference of available macros in binary benchmarks:

* `#[binary_benchmark]` and its inner attributes `#[bench]` and `#[benches]`:
  The exact pendant to the `#[library_benchmark]` attribute macro.
* `binary_benchmark_group!`: Just the name of the macro has changed.
* `binary_benchmark_attribute!`: An additional macro if you intend to
  [migrate](./low_level.md#intermixing-high-level-and-low-level-api) from the high-level to the low-level
  api
* `main!`: The same macro as in library benchmarks but the name of the
  `library_benchmark_groups` parameter changed to `binary_benchmark_groups`.

To see all macros in action have a look at the example below.

## The return value of the benchmark function

The maybe most important difference is, that the `#[binary_benchmark]` annotated
function always needs to return an `iai_callgrind::Command`. Note this function
builds the command which is going to be benchmarked but doesn't executed it,
yet. So, the code in this function does not attribute to the event counts of the
actual benchmark.

```rust
# extern crate iai_callgrind;
# macro_rules! env { ($m:tt) => {{ "/some/path" }} }
use iai_callgrind::{binary_benchmark, binary_benchmark_group, main};
use std::path::PathBuf;

#[binary_benchmark]
#[bench::foo("foo.txt")]
#[bench::bar("bar.json")]
fn bench_binary(path: &str) -> iai_callgrind::Command {
    // We can put any code in this function which is needed to configure and
    // build the `Command`.
    let path = PathBuf::from(path);

    // Here, if the `path` ends with `.txt` we want to see
    // the `Stdout` output of the `Command` in the benchmark output. In all other 
    // cases, the `Stdout` of the `Command` is redirected to a `File` with the
    // same name as the input `path` but with the extension `out`.
    let stdout = if path.extension().unwrap() == "txt" {
        iai_callgrind::Stdio::Inherit
    } else {
        iai_callgrind::Stdio::File(path.with_extension("out"))
    };
    iai_callgrind::Command::new(env!("CARGO_BIN_EXE_my-foo"))
        .stdout(stdout)
        .arg(path)
        .build()
}

binary_benchmark_group!(name = my_group; benchmarks = bench_binary);
# fn main() {
main!(binary_benchmark_groups = my_group);
# }
```

## `setup` and `teardown`

Since we can put any code building the `Command` in the function itself, the
`setup` and `teardown` of `#[binary_benchmark]`, `#[bench]` and `#[benches]`
work differently.

```rust
# extern crate iai_callgrind;
# macro_rules! env { ($m:tt) => {{ "/some/path" }} }
use iai_callgrind::{binary_benchmark, binary_benchmark_group, main};

fn create_file() {
    std::fs::write("foo.txt", "some content").unwrap();
}

#[binary_benchmark]
#[bench::foo(args = ("foo.txt"), setup = create_file())]
fn bench_binary(path: &str) -> iai_callgrind::Command {
    iai_callgrind::Command::new(env!("CARGO_BIN_EXE_my-foo"))
        .arg(path)
        .build()
}

binary_benchmark_group!(name = my_group; benchmarks = bench_binary);
# fn main() {
main!(binary_benchmark_groups = my_group);
# }
```

`setup`, which is here the expression `create_file()`, is not evaluated right
away and the return value of `setup` is not used as input for the `function`!
Instead, the expression in `setup` is getting evaluated and executed just before
the benchmarked `Command` is __executed__. Similarly, `teardown` is executed
after the `Command` is __executed__.

In the example above, `setup` creates always the same file and is pretty static.
It's possible to use the same arguments for `setup` (`teardown`) __and__ the
`function` using the path (or file pointer) to a function as you're used to from
library benchmarks:

```rust
# extern crate iai_callgrind;
# macro_rules! env { ($m:tt) => {{ "/some/path" }} }
use iai_callgrind::{binary_benchmark, binary_benchmark_group, main};

fn create_file(path: &str) {
    std::fs::write(path, "some content").unwrap();
}

fn delete_file(path: &str) {
    std::fs::remove_file(path).unwrap();
}

#[binary_benchmark]
// Note the missing parentheses for `setup` of the function `create_file` which
// tells Iai-Callgrind to pass the `args` to the `setup` function AND the
// function `bench_binary`
#[bench::foo(args = ("foo.txt"), setup = create_file)]
// Same for `teardown`
#[bench::bar(args = ("bar.txt"), setup = create_file, teardown = delete_file)]
fn bench_binary(path: &str) -> iai_callgrind::Command {
    iai_callgrind::Command::new(env!("CARGO_BIN_EXE_my-foo"))
        .arg(path)
        .build()
}

binary_benchmark_group!(name = my_group; benchmarks = bench_binary);
# fn main() {
main!(binary_benchmark_groups = my_group);
# }
```
