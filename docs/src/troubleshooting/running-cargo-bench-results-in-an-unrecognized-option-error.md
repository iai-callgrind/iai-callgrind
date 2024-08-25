# Running cargo bench results in an "Unrecognized Option" error

For

```shell
cargo bench -- --some-valid-arg
```

to work you can either specify the
benchmark with `--bench BENCHMARK`, for example

```shell
cargo bench --bench my_iai_benchmark -- --callgrind-args="--collect-bus=yes"
```

or add the following to your `Cargo.toml`:

```toml
[lib]
bench = false
```

and if you have binaries

```toml
[[bin]]
name = "my-binary"
path = "src/bin/my-binary.rs"
bench = false
```

Setting `bench = false` disables the creation of the implicit default `libtest`
harness which is added even if you haven't used `#[bench]` functions in your
library or binary. Naturally, the default harness doesn't know of the
Iai-Callgrind arguments and aborts execution printing the `Unrecognized
Option` error.

If you cannot or don't want to add `bench = false` to your `Cargo.toml`, you can
alternatively use environment variables. For every [command-line
argument](../cli_and_env/basics.md) exists a corresponding environment variable.
