# Sandbox

The
[`Sandbox`](https://docs.rs/iai-callgrind/0.15.0/iai_callgrind/struct.Sandbox.html)
is a temporary directory which is created before the execution of the `setup`
and deleted after the `teardown`. `setup`, the `Command` and `teardown` are
executed inside this temporary directory. This simply describes the order of the
execution but the `setup` or `teardown` don't need to be present.

## Why using a Sandbox?

A `Sandbox` can help mitigating differences in benchmark results on different
machines. As long as `$TMP_DIR` is unset or set to `/tmp`, the temporary
directory has a constant length on unix machines (except android
which uses `/data/local/tmp`). The directory itself is created with a constant
length but random name like `/tmp/.a23sr8fk`.

It is not implausible that an executable has different event counts just because
the directory it is executed in has a different length. For example, if a member
of your project has set up the project in `/home/bob/workspace/our-project`
running the benchmarks in this directory, and the ci runs the benchmarks in
`/runner/our-project`, the event counts might differ. If possible, the
benchmarks should be run in a constant environment. For example [clearing the
environment variables](../important.md) is also such a measure.

Other good reasons for using a `Sandbox` are convenience, e.g. if you create
files during the `setup` and `Command` run and do not want to delete all files
manually. Or, maybe more importantly, if the `Command` is destructive and
deletes files, it is usually safer to run such a `Command` in a temporary
directory where it cannot cause damage to your or other file systems.

The `Sandbox` is deleted after the benchmark, regardless of whether the
benchmark run was successful or not. The latter is not guaranteed if you only
rely on `teardown`, as `teardown` is only executed if the `Command` returns
without error.

```rust
# extern crate iai_callgrind;
# macro_rules! env { ($m:tt) => {{ "/some/path" }} }
use iai_callgrind::{
    binary_benchmark, binary_benchmark_group, main, BinaryBenchmarkConfig, Sandbox
};

fn create_file(path: &str) {
    std::fs::write(path, "some content").unwrap();
}

#[binary_benchmark]
#[bench::foo(
    args = ("foo.txt"),
    config = BinaryBenchmarkConfig::default().sandbox(Sandbox::new(true)),
    setup = create_file
)]
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

In this example, as part of the `setup`, the `create_file` function with the
argument `foo.txt` is executed in the `Sandbox` before the `Command` is
executed. The `Command` is executed in the same `Sandbox` and therefore the file
`foo.txt` with the content `some content` exists thanks to the `setup`. After
the execution of the `Command`, the `Sandbox` is completely removed, deleting
all files created during `setup`, the `Command` execution (and `teardown` if it
had been present in this example).

Since `setup` is run in the sandbox, you can't copy fixtures from your project's
workspace into the sandbox that easily anymore. The `Sandbox` can be configured
to copy `fixtures` into the temporary directory with `Sandbox::fixtures`:

```rust
# extern crate iai_callgrind;
# macro_rules! env { ($m:tt) => {{ "/some/path" }} }
use iai_callgrind::{
    binary_benchmark, binary_benchmark_group, main, BinaryBenchmarkConfig, Sandbox
};

#[binary_benchmark]
#[bench::foo(
    args = ("foo.txt"),
    config = BinaryBenchmarkConfig::default()
        .sandbox(Sandbox::new(true)
            .fixtures(["benches/foo.txt"])),
)]
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

The above will copy the fixture file `foo.txt` in the `benches` directory into
the sandbox root as `foo.txt`. Relative paths in `Sandbox::fixtures` are
interpreted relative to the workspace root. In a multi-crate workspace this is
the directory with the top-level `Cargo.toml` file. Paths in `Sandbox::fixtures`
are not limited to files, they can be directories, too.

If you have more complex demands, you can access the workspace root via the
environment variable `_WORKSPACE_ROOT` in `setup` and `teardown`. Suppose, there
is a fixture located in `/home/the_project/foo_crate/benches/fixtures/foo.txt`
with `the_project` being the workspace root and `foo_crate` a workspace member
with the `my-foo` executable. If the command is expected to create a file
`bar.json`, which needs further inspection after the benchmarks have run, let's
copy it into a temporary directory `tmp` (which may or may not exist) in
`foo_crate`:

```rust
# extern crate iai_callgrind;
# macro_rules! env { ($m:tt) => {{ "/some/path" }} }
use iai_callgrind::{
    binary_benchmark, binary_benchmark_group, main, BinaryBenchmarkConfig, Sandbox
};
use std::path::PathBuf;

fn copy_fixture(path: &str) {
    let workspace_root = PathBuf::from(std::env::var_os("_WORKSPACE_ROOT").unwrap());
    std::fs::copy(
        workspace_root.join("foo_crate").join("benches").join("fixtures").join(path),
        path
    );
}

// This function will fail if `bar.json` does not exist, which is fine as this
// file is expected to be created by `my-foo`. So, if this file does not exist,
// an error will occur and the benchmark will fail. Although benchmarks are not
// expected to test the correctness of the application, the `teardown` can be
// used to check postconditions for a successful command run.
fn copy_back(path: &str) {
    let workspace_root = PathBuf::from(std::env::var_os("_WORKSPACE_ROOT").unwrap());
    let dest_dir = workspace_root.join("foo_crate").join("tmp");
    if !dest_dir.exists() {
        std::fs::create_dir(&dest_dir).unwrap();
    }
    std::fs::copy(path, dest_dir.join(path));
}

#[binary_benchmark]
#[bench::foo(
    args = ("foo.txt"),
    config = BinaryBenchmarkConfig::default().sandbox(Sandbox::new(true)),
    setup = copy_fixture,
    teardown = copy_back("bar.json")
)]
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
