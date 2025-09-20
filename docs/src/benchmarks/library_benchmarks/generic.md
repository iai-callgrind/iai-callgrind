# Generic benchmark functions

Benchmark functions can be generic. And `setup` and `teardown` functions, too.
There's actually not much more to say about it since generic benchmark (`setup`
and `teardown`) functions behave exactly the same way as you would expect it
from any other generic function.

However, there is a common pitfall. If you have a function
`count_lines_in_file_fast` which expects as parameter a `PathBuf` and although
it is convenient especially when you have to specify many paths, don't do this:

```rust
# extern crate gungraun;
# mod my_lib { pub fn count_lines_in_file_fast(_path: std::path::PathBuf) -> u64 { 1 } }
use gungraun::{library_benchmark, library_benchmark_group, main};

use std::hint::black_box;
use std::path::PathBuf;

#[library_benchmark]
#[bench::first("path/to/file")]
fn generic_bench<T>(path: T) -> u64 where T: Into<PathBuf> {
    black_box(my_lib::count_lines_in_file_fast(black_box(path.into())))
}

library_benchmark_group!(name = my_group; benchmarks = generic_bench);
# fn main() {
main!(library_benchmark_groups = my_group);
# }
```

Since `path.into()` is called in the benchmark function itself, the conversion
from a `&str` to a `PathBuf` is attributed to the benchmark metrics. This is
almost never what you intended. You should instead convert the argument to a
`PathBuf` in a generic `setup` function like that:

```rust
# extern crate gungraun;
# mod my_lib { pub fn count_lines_in_file_fast(_path: std::path::PathBuf) -> u64 { 1 } }
use gungraun::{library_benchmark, library_benchmark_group, main};

use std::hint::black_box;
use std::path::PathBuf;

fn convert_to_pathbuf<T>(path: T) -> PathBuf where T: Into<PathBuf> {
    path.into()
}

#[library_benchmark]
#[bench::first(args = ("path/to/file"), setup = convert_to_pathbuf)]
fn not_generic_anymore(path: PathBuf) -> u64 {
    black_box(my_lib::count_lines_in_file_fast(path))
}

library_benchmark_group!(name = my_group; benchmarks = not_generic_anymore);
# fn main() {
main!(library_benchmark_groups = my_group);
# }
```

That way you can still enjoy the convenience to use string literals instead of
`PathBuf` in your `#[bench]` (or `#[benches]`) arguments and have clean
benchmark metrics.
