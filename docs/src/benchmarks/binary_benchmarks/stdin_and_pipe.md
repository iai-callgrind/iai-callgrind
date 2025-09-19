# The Command's stdin and simulating piped input

The behaviour of `Stdin` of the `Command` can be changed, almost the same way as
the `Stdin` of a `std::process::Command` with the only difference, that we use
the enums `iai_callgrind::Stdin` and `iai_callgrind::Stdio`. These enums provide
the variants `Inherit` (the equivalent of `std::process::Stdio::inherit`),
`Pipe` (the equivalent of `std::process::Stdio::piped`) and so on. There's also
`File` which takes a `PathBuf` to the file which is used as `Stdin` for the
`Command`. This corresponds to a redirection in the shell as in `my-foo <
path/to/file`.

Moreover, `iai_callgrind::Stdin` provides the `Stdin::Setup` variant specific to
Gungraun:

Applications may change their behaviour if the input or the `Stdin` of the
`Command` is coming from a pipe as in `echo "some content" | my-foo`. To be able
to benchmark such cases, it is possible to use the output of `setup` to `Stdout`
or `Stderr` as `Stdin` for the `Command`.

```rust
# extern crate iai_callgrind;
# macro_rules! env { ($m:tt) => {{ "/some/path" }} }
use iai_callgrind::{binary_benchmark, binary_benchmark_group, main, Stdin, Pipe};

fn setup_pipe() {
    println!(
        "The output to `Stdout` here will be the input or `Stdin` of the `Command`"
    );
}

#[binary_benchmark]
#[bench::foo(setup = setup_pipe())]
fn bench_binary() -> iai_callgrind::Command {
    iai_callgrind::Command::new(env!("CARGO_BIN_EXE_my-foo"))
        .stdin(Stdin::Setup(Pipe::Stdout))
        .build()
}

binary_benchmark_group!(name = my_group; benchmarks = bench_binary);
# fn main() {
main!(binary_benchmark_groups = my_group);
# }
```

Usually, `setup` then the `Command` and then `teardown` are executed
sequentially, each waiting for the previous process to exit successfully (See
also [Configure the exit code of the Command](./configuration/exit_code.md)). If
the `Command::stdin` changes to `Stdin::Setup`, `setup` and the `Command` are
executed in parallel and Gungraun waits first for the `Command` to exit,
then `setup`. After the successful exit of `setup`, `teardown` is executed.

Since `setup` and `Command` are run in parallel if `Stdin::Setup` is used, it is
sometimes necessary to delay the execution of the `Command`. Please see the
[`delay`](./configuration/delay.md) chapter for more details.
