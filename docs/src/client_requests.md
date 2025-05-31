# Valgrind Client Requests

Iai-Callgrind ships with its own interface to the [Valgrind's Client Request
Mechanism](https://valgrind.org/docs/manual/manual-core-adv.html#manual-core-adv.clientreq).
Iai-Callgrind's client requests have zero overhead (relative to the "C"
implementation of Valgrind) on many targets which are also natively supported by
valgrind. In short, Iai-Callgrind provides a complete and performant
implementation of Valgrind Client Requests.

## Installation

Client requests are deactivated by default but can be activated with the
`client_requests` feature.

```toml
[dev-dependencies]
iai-callgrind = { version = "0.14.1", features = ["client_requests"] }
```

If you need the client requests in your production code, you don't want them to
do anything when not running under valgrind with Iai-Callgrind benchmarks. You
can achieve that by adding Iai-Callgrind with the `client_requests_defs` feature
to your runtime dependencies and with the `client_requests` feature to your
`dev-dependencies` like so:

```toml
[dependencies]
iai-callgrind = { version = "0.14.1", default-features = false, features = [
    "client_requests_defs"
] }

[dev-dependencies]
iai-callgrind = { version = "0.14.1", features = ["client_requests"] }
```

With just the `client_requests_defs` feature activated, the client requests
compile down to nothing and don't add any overhead to your production code. It
simply provides the "definitions", method signatures and macros without body.
Only with the activated `client_requests` feature they will be actually
executed. Note that the client requests do not depend on any other part of
Iai-Callgrind, so you could even use the client requests without the rest of
Iai-Callgrind.

When building Iai-Callgrind with client requests, the valgrind header files must
exist in your standard include path (most of the time `/usr/include`). This is
usually the case if you've installed valgrind with your distribution's package
manager. If not, you can point the `IAI_CALLGRIND_VALGRIND_INCLUDE` or
`IAI_CALLGRIND_<triple>_VALGRIND_INCLUDE` environment variables to the include
path. So, if the headers can be found in `/home/foo/repo/valgrind/{valgrind.h,
callgrind.h, ...}`, the correct include path would be
`IAI_CALLGRIND_VALGRIND_INCLUDE=/home/foo/repo` (not `/home/foo/repo/valgrind`)

## Usage

Use them in your code for example like so:

```rust
# extern crate iai_callgrind;
use iai_callgrind::client_requests;

# fn main() {
fn main() {
    // Start callgrind event counting if not already started earlier
    client_requests::callgrind::start_instrumentation();

    // do something important

    // Switch event counting off
    client_requests::callgrind::stop_instrumentation();
}
# }
```

### Library Benchmarks

In [library benchmarks](./benchmarks/library_benchmarks.md) you might need to
use [`EntryPoint::None`][EntryPoint] in order to make the client requests work
as expected:

```rust
# extern crate iai_callgrind;
use iai_callgrind::{main, library_benchmark_group, library_benchmark};
use std::hint::black_box;

pub mod my_lib {
     #[inline(never)]
     fn bubble_sort(input: Vec<i32>) -> Vec<i32> {
         // The algorithm
#        input
     }

     pub fn pre_bubble_sort(input: Vec<i32>) -> Vec<i32> {
         println!("Doing something before the function call");
         iai_callgrind::client_requests::callgrind::start_instrumentation();

         let result = bubble_sort(input);

         iai_callgrind::client_requests::callgrind::stop_instrumentation();
         result
     }
}

#[library_benchmark]
#[bench::small(vec![3, 2, 1])]
#[bench::bigger(vec![5, 4, 3, 2, 1])]
fn bench_function(array: Vec<i32>) -> Vec<i32> {
    black_box(my_lib::pre_bubble_sort(array))
}

library_benchmark_group!(name = my_group; benchmarks = bench_function);
# fn main() {
main!(library_benchmark_groups = my_group);
# }
```

The default [`EntryPoint`][EntryPoint] sets the [`--toggle-collect`][Callgrind
Arguments] to the benchmark function (here `bench_function`) and
`--collect-at-start=no`. So, `Callgrind` starts collecting the events when
entering the benchmark function, not the moment `start_instrumentation` is
called. This behaviour can be remedied with `EntryPoint::None`:

```rust
# extern crate iai_callgrind;
use iai_callgrind::{
    main, library_benchmark_group, library_benchmark, LibraryBenchmarkConfig,
    client_requests, EntryPoint
};
use std::hint::black_box;

pub mod my_lib {
     #[inline(never)]
     fn bubble_sort(input: Vec<i32>) -> Vec<i32> {
         // The algorithm
#        input
     }

     pub fn pre_bubble_sort(input: Vec<i32>) -> Vec<i32> {
         println!("Doing something before the function call");
         iai_callgrind::client_requests::callgrind::start_instrumentation();

         let result = bubble_sort(input);

         iai_callgrind::client_requests::callgrind::stop_instrumentation();
         result
     }
}

#[library_benchmark(
    config = LibraryBenchmarkConfig::default()
        .callgrind_args(["--collect-at-start=no"])
        .entry_point(EntryPoint::None)
)]
#[bench::small(vec![3, 2, 1])]
#[bench::bigger(vec![5, 4, 3, 2, 1])]
fn bench_function(array: Vec<i32>) -> Vec<i32> {
    black_box(my_lib::pre_bubble_sort(array))
}

library_benchmark_group!(name = my_group; benchmarks = bench_function);
# fn main() {
main!(library_benchmark_groups = my_group);
# }
```

As the standard toggle is now switched off and the option
`--collect-at-start=no` is also omitted, you must specify
`--collect-at-start=no` manually in
`LibraryBenchmarkConfig::raw_callgrind_args`.

Please see the
[`docs`](https://docs.rs/iai-callgrind/0.14.1/iai_callgrind/client_requests) for
more details!

[Callgrind Arguments]: https://valgrind.org/docs/manual/cl-manual.html#cl-manual.options

[EntryPoint]: https://docs.rs/iai-callgrind/0.14.1/iai_callgrind/enum.EntryPoint.html
