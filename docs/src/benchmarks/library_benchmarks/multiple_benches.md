# Specifying multiple benches at once

Multiple benches can be specified at once with the
[`#[benches]`](macros.md#the-benches-attribute) attribute.

## The `#[benches]` attribute in more detail

Let's start with an example:

```rust
# extern crate iai_callgrind;
# mod my_lib { pub fn bubble_sort(value: Vec<i32>) -> Vec<i32> { value } }
use iai_callgrind::{library_benchmark, library_benchmark_group, main};
use std::hint::black_box;
use my_lib::bubble_sort;

fn setup_worst_case_array(start: i32) -> Vec<i32> {
    if start.is_negative() {
        (start..0).rev().collect()
    } else {
        (0..start).rev().collect()
    }
}

#[library_benchmark]
#[benches::multiple(vec![1], vec![5])]
#[benches::with_setup(args = [1, 5], setup = setup_worst_case_array)]
fn bench_bubble_sort_with_benches_attribute(input: Vec<i32>) -> Vec<i32> {
    black_box(bubble_sort(input))
}

library_benchmark_group!(name = my_group; benchmarks = bench_bubble_sort_with_benches_attribute);
# fn main () {
main!(library_benchmark_groups = my_group);
# }
```

Usually the `arguments` are passed directly to the benchmarking function as it
can be seen in the `#[benches::multiple(/* arguments */)]` case. In
`#[benches::with_setup(/* ... */)]`, the arguments are passed to the `setup` function
instead. The above `#[library_benchmark]` is pretty much the same as

```rust
# extern crate iai_callgrind;
# mod my_lib { pub fn bubble_sort(value: Vec<i32>) -> Vec<i32> { value } }
use iai_callgrind::{library_benchmark, library_benchmark_group, main};
use std::hint::black_box;
use my_lib::bubble_sort;

fn setup_worst_case_array(start: i32) -> Vec<i32> {
    if start.is_negative() {
        (start..0).rev().collect()
    } else {
        (0..start).rev().collect()
    }
}

#[library_benchmark]
#[bench::multiple_0(vec![1])]
#[bench::multiple_1(vec![5])]
#[bench::with_setup_0(setup_worst_case_array(1))]
#[bench::with_setup_1(setup_worst_case_array(5))]
fn bench_bubble_sort_with_benches_attribute(input: Vec<i32>) -> Vec<i32> {
    black_box(bubble_sort(input))
}

library_benchmark_group!(name = my_group; benchmarks = bench_bubble_sort_with_benches_attribute);
# fn main () {
main!(library_benchmark_groups = my_group);
# }
```

but a lot more concise especially if a lot of values are passed to the same
`setup` function.

### The `file` parameter

Reading inputs from a file allows for example sharing the same inputs between
different benchmarking frameworks like `criterion` or if you simply have a long
list of inputs you might find it more convenient to read them from a file.

The `file` parameter, exclusive to the `#[benches]` attribute, does exactly that
and reads the specified file line by line creating a benchmark from each line.
The line is passed to the benchmark function as `String` or if the `setup`
parameter is also present to the `setup` function. A small example assuming you
have a file `benches/inputs` (relative paths are interpreted to the workspace
root) with the following content

```text
1
11
111
```

then

```rust,ignore
# extern crate iai_callgrind;
# mod my_lib { pub fn string_to_u64(value: String) -> Result<u64, String> { Ok(1) } }
use iai_callgrind::{library_benchmark, library_benchmark_group, main};
use std::hint::black_box;

#[library_benchmark]
#[benches::from_file(file = "benches/inputs")]
fn some_bench(line: String) -> Result<u64, String> {
    black_box(my_lib::string_to_u64(line))
}

library_benchmark_group!(name = my_group; benchmarks = some_bench);
# fn main() {
main!(library_benchmark_groups = my_group);
# }
```

The above is roughly equivalent to the following but with the `args` parameter

```rust
# extern crate iai_callgrind;
# mod my_lib { pub fn string_to_u64(value: String) -> Result<u64, String> { Ok(1) } }
use iai_callgrind::{library_benchmark, library_benchmark_group, main};
use std::hint::black_box;

#[library_benchmark]
#[benches::from_args(args = [1.to_string(), 11.to_string(), 111.to_string()])]
fn some_bench(line: String) -> Result<u64, String> {
    black_box(my_lib::string_to_u64(line))
}

library_benchmark_group!(name = my_group; benchmarks = some_bench);
# fn main() {
main!(library_benchmark_groups = my_group);
# }
```

The true power of the `file` parameter comes with the `setup` function because
you can format the lines in the file as you like and convert each line in the
`setup` function to the format as you need it in the benchmark. For example if
you decided to go with a csv like format in the file `benches/inputs`

```text
255;255;255
0;0;0
```

and your library has a function which converts from RGB to HSV color space:

```rust,ignore
# extern crate iai_callgrind;
# mod my_lib { pub fn rgb_to_hsv(a: u8, b: u8, c:u8) -> (u16, u8, u8) { (a.into(), b, c) } }
use iai_callgrind::{library_benchmark, library_benchmark_group, main};
use std::hint::black_box;

fn decode_line(line: String) -> (u8, u8, u8) {
    if let &[a, b, c] = line.split(";")
        .map(|s| s.parse::<u8>().unwrap())
        .collect::<Vec<u8>>()
        .as_slice() 
    {
        (a, b, c)
    } else {
        panic!("Wrong input format in line '{line}'");
    }
}

#[library_benchmark]
#[benches::from_file(file = "benches/inputs", setup = decode_line)]
fn some_bench((a, b, c): (u8, u8, u8)) -> (u16, u8, u8) {
    black_box(my_lib::rgb_to_hsv(black_box(a), black_box(b), black_box(c)))
}

library_benchmark_group!(name = my_group; benchmarks = some_bench);
# fn main() {
main!(library_benchmark_groups = my_group);
# }
```
