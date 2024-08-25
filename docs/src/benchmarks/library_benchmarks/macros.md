# The macros in more detail

This section is a brief reference to all the macros available in library
benchmarks. Feel free to come back here from other sections if you need a
reference. For the complete documentation of each macro see the [api
Documentation](https://docs.rs/iai-callgrind/0.13.0/iai_callgrind/).

For the following examples it is assumed that there is a file `lib.rs` in a
crate named `my_lib` with the following content:

```rust
pub fn bubble_sort(mut array: Vec<i32>) -> Vec<i32> {
    for i in 0..array.len() {
        for j in 0..array.len() - i - 1 {
            if array[j + 1] < array[j] {
                array.swap(j, j + 1);
            }
        }
    }
    array
}
```

## The `#[library_benchmark]` attribute

This attribute needs to be present on all benchmark functions specified in the
[`library_benchmark_group`](#the-library_benchmark_group-macro). The benchmark
function can then be further annotated with the inner
[`#[bench]`](#the-bench-attribute) or [`#[benches]`](#the-benches-attribute)
attributes.

```rust
# extern crate iai_callgrind;
# mod my_lib { pub fn bubble_sort(value: Vec<i32>) -> Vec<i32> { value } }
use iai_callgrind::{library_benchmark, library_benchmark_group, main};
use std::hint::black_box;

#[library_benchmark]
#[bench::one(vec![1])]
#[benches::multiple(vec![1, 2], vec![1, 2, 3], vec![1, 2, 3, 4])]
fn bench_bubble_sort(values: Vec<i32>) -> Vec<i32> {
    black_box(my_lib::bubble_sort(values))
}

library_benchmark_group!(name = bubble_sort_group; benchmarks = bench_bubble_sort);
# fn main() {
main!(library_benchmark_groups = bubble_sort_group);
# }
```

The following parameters are accepted:

- `config`: Takes a
  [`LibraryBenchmarkConfig`](https://docs.rs/iai-callgrind/0.13.0/iai_callgrind/struct.LibraryBenchmarkConfig.html)
- `setup`: A global setup function which is applied to all following [`#[bench]`](#the-bench-attribute)
  and [`#[benches]`](#the-benches-attribute) attributes if not overwritten by a `setup` parameter of these
  attributes.
- `teardown`: Similar to `setup` but takes a global `teardown` function.

```rust
# extern crate iai_callgrind;
# mod my_lib { pub fn bubble_sort(value: Vec<i32>) -> Vec<i32> { value } }
use iai_callgrind::{library_benchmark, library_benchmark_group, main, LibraryBenchmarkConfig};
use std::hint::black_box;

#[library_benchmark(
    config = LibraryBenchmarkConfig::default().truncate_description(None)
)]
#[bench::one(vec![1])]
fn bench_bubble_sort(values: Vec<i32>) -> Vec<i32> {
    black_box(my_lib::bubble_sort(values))
}

library_benchmark_group!(name = bubble_sort_group; benchmarks = bench_bubble_sort);
# fn main() {
main!(library_benchmark_groups = bubble_sort_group);
# }
```

### The `#[bench]` attribute

The basic structure is `#[bench::some_id(/* parameters */)]`. The part after the
`::` must be an id unique within the same `#[library_benchmark]`. This attribute
accepts the following parameters:

- `args`: A tuple with a list of arguments which are passed to the
  benchmark function. The parentheses also need to be present if there is only a
  single argument (`#[bench::my_id(args = (10))]`).
- `config`: Accepts a
  [`LibraryBenchmarkConfig`](https://docs.rs/iai-callgrind/0.13.0/iai_callgrind/struct.LibraryBenchmarkConfig.html)
- `setup`: A function which takes the arguments specified in the `args`
  parameter and passes its return value to the benchmark function.
- `teardown`: A function which takes the return value of the benchmark function.

If no other parameters besides `args` are present you can simply pass the
arguments as a list of values. So, instead of `#[bench::my_id(args = (10,
20))]`, you could also use the shorter `#[bench::my_id(10, 20)]`.

```rust
# extern crate iai_callgrind;
# mod my_lib { pub fn bubble_sort(value: Vec<i32>) -> Vec<i32> { value } }
use iai_callgrind::{library_benchmark, library_benchmark_group, main, LibraryBenchmarkConfig};
use std::hint::black_box;

// This function is used to create a worst case array we want to sort with our implementation of
// bubble sort
pub fn worst_case(start: i32) -> Vec<i32> {
    if start.is_negative() {
        (start..0).rev().collect()
    } else {
        (0..start).rev().collect()
    }
}

#[library_benchmark]
#[bench::one(vec![1])]
#[bench::worst_two(args = (vec![2, 1]))]
#[bench::worst_four(args = (4), setup = worst_case)]
fn bench_bubble_sort(value: Vec<i32>) -> Vec<i32> {
    black_box(my_lib::bubble_sort(value))
}

library_benchmark_group!(name = bubble_sort_group; benchmarks = bench_bubble_sort);
# fn main() {
main!(library_benchmark_groups = bubble_sort_group);
# }
```

### The `#[benches]` attribute

This attribute is used to specify multiple benchmarks at once. It accepts the
same parameters as the [`#[bench]`](#the-bench-attribute) attribute: `args`,
`config`, `setup` and `teardown` and additionally the `file` parameter which is
explained in detail [here](./multiple_benches.md). In contrast to the `args`
parameter in [`#[bench]`](#the-bench-attribute), `args` takes an array of
arguments.

```rust
# extern crate iai_callgrind;
# mod my_lib { pub fn bubble_sort(value: Vec<i32>) -> Vec<i32> { value } }
use iai_callgrind::{library_benchmark, library_benchmark_group, main, LibraryBenchmarkConfig};
use std::hint::black_box;

pub fn worst_case(start: i32) -> Vec<i32> {
    if start.is_negative() {
        (start..0).rev().collect()
    } else {
        (0..start).rev().collect()
    }
}

#[library_benchmark]
#[benches::worst_two_and_three(args = [vec![2, 1], vec![3, 2, 1]])]
#[benches::worst_four_to_nine(args = [4, 5, 6, 7, 8, 9], setup = worst_case)]
fn bench_bubble_sort(value: Vec<i32>) -> Vec<i32> {
    black_box(my_lib::bubble_sort(value))
}

library_benchmark_group!(name = bubble_sort_group; benchmarks = bench_bubble_sort);
# fn main() {
main!(library_benchmark_groups = bubble_sort_group);
# }
```

## The library_benchmark_group! macro

The `library_benchmark_group` macro accepts the following parameters (in this
order and separated by a semicolon):

- __`name`__ (mandatory): A unique name used to identify the group for the
  `main!` macro
- __`config`__ (optional): A
  [`LibraryBenchmarkConfig`](https://docs.rs/iai-callgrind/0.13.0/iai_callgrind/struct.LibraryBenchmarkConfig.html)
  which is applied to all benchmarks within the same group.
- __`compare_by_id`__ (optional): The default is false. If true, all benches in
  the benchmark functions specified in the `benchmarks` parameter are compared
  with each other as long as the ids (the part after the `::` in
  `#[bench::id(...)]`) match. See also [Comparing benchmark
  functions](./compare_by_id.md)
- __`setup`__ (optional): A setup function or any valid expression which is run
  before all benchmarks of this group
- __`teardown`__ (optional): A teardown function or any valid expression which
  is run after all benchmarks of this group
- __`benchmarks`__ (mandatory): A list of comma separated paths of benchmark
  functions which are annotated with `#[library_benchmark]`

Note the `setup` and `teardown` parameters are different to the ones of
`#[library_benchmark]`, `#[bench]` and `#[benches]`. They accept an expression
or function call as in `setup = group_setup_function()`. Also, these `setup` and
`teardown` functions are not overridden by the ones from any of the before
mentioned attributes.

## The main! macro

This macro is the entry point for Iai-Callgrind and creates the benchmark
harness. It accepts the following top-level arguments in this order (separated
by a semicolon):

- __`config`__ (optional): Optionally specify a
  [`LibraryBenchmarkConfig`](https://docs.rs/iai-callgrind/0.13.0/iai_callgrind/struct.LibraryBenchmarkConfig.html)
- __`setup`__ (optional): A setup function or any valid expression which is run
  before all benchmarks
- __`teardown`__ (optional): A setup function or any valid expression which is
  run after all benchmarks
- __`library_benchmark_groups`__ (mandatory): The name of one or more library
  benchmark groups. Multiple names are separated by a comma.

Like the `setup` and `teardown` of the
[`library_benchmark_group`](#the-library_benchmark_group-macro), these
parameters accept an expression and are not overridden by the `setup` and
`teardown` of the `library_benchmark_group`, `#[library_benchmark]`, `#[bench]`
or `#[benches]` attribute.
