<!-- markdownlint-disable MD041 MD033 -->

# setup and teardown

`setup` and `teardown` are your bread and butter in library benchmarks. The
benchmark functions need to be as clean as possible and almost always only
contain the function call to the function of your library which you want to
benchmark.

## Setup

In an ideal world you don't need any setup code, and you can pass arguments to
the function as they are.

But, for example if a function expects a `File` and not a `&str` with the path
to the file you need `setup` code. Gungraun has an easy-to-use system in
place to allow you to run any setup code before the function is executed and
this `setup` code is not attributed to the metrics of the benchmark.

If the `setup` parameter is specified, the `setup` function takes the arguments
from the `#[bench]` (or `#[benches]`) attributes and the benchmark function
receives the return value of the `setup` function as parameter. This is a small
indirection with great effect. The effect is best shown with an example:

```rust
# extern crate iai_callgrind;
# mod my_lib { pub fn count_bytes_fast(_file: std::fs::File) -> u64 { 1 } }
use iai_callgrind::{library_benchmark, library_benchmark_group, main};

use std::hint::black_box;
use std::path::PathBuf;
use std::fs::File;

fn open_file(path: &str) -> File {
    File::open(path).unwrap()
}

#[library_benchmark]
#[bench::first(args = ("path/to/file"), setup = open_file)]
fn count_bytes_fast(file: File) -> u64 {
    black_box(my_lib::count_bytes_fast(file))
}

library_benchmark_group!(name = my_group; benchmarks = count_bytes_fast);
# fn main() {
main!(library_benchmark_groups = my_group);
# }
```

You can actually see the effect of using a setup function in the output of the
benchmark. Let's assume the above benchmark is in a file
`benches/my_benchmark.rs`, then running

```shell
IAI_CALLGRIND_NOCAPTURE=true cargo bench
```

result in the benchmark output like below.

<pre><code class="hljs"><span style="color:#0A0">my_benchmark::my_group::count_bytes_fast</span> <span style="color:#0AA">first</span><span style="color:#0AA">:</span><b><span style="color:#00A">open_file("path/to/file")</span></b>
  Instructions:     <b>        1630162</b>|N/A             (<span style="color:#555">*********</span>)
  L1 Hits:          <b>        2507933</b>|N/A             (<span style="color:#555">*********</span>)
  LL Hits:          <b>              2</b>|N/A             (<span style="color:#555">*********</span>)
  RAM Hits:         <b>             11</b>|N/A             (<span style="color:#555">*********</span>)
  Total read+write: <b>        2507946</b>|N/A             (<span style="color:#555">*********</span>)
  Estimated Cycles: <b>        2508328</b>|N/A             (<span style="color:#555">*********</span>)

Gungraun result: <b><span style="color:#0A0">Ok</span></b>. 1 without regressions; 0 regressed; 1 benchmarks finished in 0.49333s</code></pre>

The description in the headline contains `open_file("path/to/file")`, your setup
function `open_file` with the value of the parameter it is called with.

If you need to specify the same `setup` function for all (or almost all)
`#[bench]` and `#[benches]` in a `#[library_benchmark]` you can use the `setup`
parameter of the `#[library_benchmark]`:

```rust
# extern crate iai_callgrind;
# mod my_lib { pub fn count_bytes_fast(_file: std::fs::File) -> u64 { 1 } }
use iai_callgrind::{library_benchmark, library_benchmark_group, main};

use std::hint::black_box;
use std::path::PathBuf;
use std::fs::File;
use std::io::{Seek, SeekFrom};

fn open_file(path: &str) -> File {
    File::open(path).unwrap()
}

fn open_file_with_offset(path: &str, offset: u64) -> File {
    let mut file = File::open(path).unwrap();
    file.seek(SeekFrom::Start(offset)).unwrap();
    file
}

#[library_benchmark(setup = open_file)]
#[bench::small("path/to/small")]
#[bench::big("path/to/big")]
#[bench::with_offset(args = ("path/to/big", 100), setup = open_file_with_offset)]
fn count_bytes_fast(file: File) -> u64 {
    black_box(my_lib::count_bytes_fast(file))
}

library_benchmark_group!(name = my_group; benchmarks = count_bytes_fast);
# fn main() {
main!(library_benchmark_groups = my_group);
# }
```

The above will use the `open_file` function in the `small` and `big` benchmarks
and the `open_file_with_offset` function in the `with_offset` benchmark.

## Teardown

What about `teardown` and why should you use it? Usually the `teardown` isn't
needed but for example if you intend to make the result from the benchmark
visible in the benchmark output, the `teardown` is the perfect place to do so.

The `teardown` function takes the return value of the benchmark function as its
argument:

```rust
# extern crate iai_callgrind;
# mod my_lib { pub fn count_bytes_fast(_file: std::fs::File) -> u64 { 1 } }
use iai_callgrind::{library_benchmark, library_benchmark_group, main};

use std::hint::black_box;
use std::path::PathBuf;
use std::fs::File;

fn open_file(path: &str) -> File {
    File::open(path).unwrap()
}

fn print_bytes_read(num_bytes: u64) {
    println!("bytes read: {num_bytes}");
}

#[library_benchmark]
#[bench::first(
    args = ("path/to/big"),
    setup = open_file,
    teardown = print_bytes_read
)]
fn count_bytes_fast(file: File) -> u64 {
    black_box(my_lib::count_bytes_fast(file))
}

library_benchmark_group!(name = my_group; benchmarks = count_bytes_fast);
# fn main() {
main!(library_benchmark_groups = my_group);
# }
```

Note Gungraun captures all output per default. In order to actually see the
output of the benchmark, `setup` and `teardown` functions, it is required to run
the benchmarks with the flag `--nocapture` or set the environment variable
`IAI_CALLGRIND_NOCAPTURE=true`. Let's assume the above benchmark is in a file
`benches/my_benchmark.rs`, then running

```shell
IAI_CALLGRIND_NOCAPTURE=true cargo bench
```

results in output like the below

<pre><code class="hljs"><span style="color:#0A0">my_benchmark::my_group::count_bytes_fast</span> <span style="color:#0AA">first</span><span style="color:#0AA">:</span><b><span style="color:#00A">open_file("path/to/big")</span></b>
bytes read: 25078
<span style="color:#A50">-</span> <span style="color:#A50">end of stdout/stderr</span>
  Instructions:     <b>        1630162</b>|N/A             (<span style="color:#555">*********</span>)
  L1 Hits:          <b>        2507931</b>|N/A             (<span style="color:#555">*********</span>)
  LL Hits:          <b>              2</b>|N/A             (<span style="color:#555">*********</span>)
  RAM Hits:         <b>             13</b>|N/A             (<span style="color:#555">*********</span>)
  Total read+write: <b>        2507946</b>|N/A             (<span style="color:#555">*********</span>)
  Estimated Cycles: <b>        2508396</b>|N/A             (<span style="color:#555">*********</span>)

Gungraun result: <b><span style="color:#0A0">Ok</span></b>. 1 without regressions; 0 regressed; 1 benchmarks finished in 0.49333s</code></pre>

The output of the `teardown` function is now visible in the benchmark output
above the `- end of stdout/stderr` line.
