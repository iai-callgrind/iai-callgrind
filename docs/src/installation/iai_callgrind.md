# Iai-Callgrind

Iai-Callgrind is divided into the library `iai-callgrind` and the benchmark runner
`iai-callgrind-runner`.

## Installation of the library

To start with Iai-Callgrind, add the following to your `Cargo.toml` file:

```toml
[dev-dependencies]
iai-callgrind = "0.13.2"
```

or run

```bash
cargo add --dev iai-callgrind@0.13.2
```

## Installation of the benchmark runner

To be able to run the benchmarks you'll also need the `iai-callgrind-runner`
binary installed somewhere in your `$PATH`. Otherwise, there is no need to
interact with `iai-callgrind-runner` as it is just an implementation detail.

### From Source

```shell
cargo install --version 0.13.2 iai-callgrind-runner
```

There's also the possibility to install the binary somewhere else and point the
`IAI_CALLGRIND_RUNNER` environment variable to the absolute path of the
`iai-callgrind-runner` binary like so:

```shell
cargo install --version 0.13.2 --root /tmp iai-callgrind-runner
IAI_CALLGRIND_RUNNER=/tmp/bin/iai-callgrind-runner cargo bench --bench my-bench
```

### Binstall

The `iai-callgrind-runner` binary is
[pre-built](https://github.com/iai-callgrind/iai-callgrind/releases/tag/v0.13.2)
for most platforms supported by valgrind and easily installable with
[binstall](https://github.com/cargo-bins/cargo-binstall)

```shell
cargo binstall iai-callgrind-runner@0.13.2
```

## Updating

When updating the `iai-callgrind` library, you'll also need to update
`iai-callgrind-runner` and vice-versa or else the benchmark runner will exit
with an error.

### In the Github CI

Since the `iai-callgrind-runner` version must match the `iai-callgrind` library
version it's best to automate this step in the CI. A job step in the github
actions CI could look like this

```yaml
- name: Install iai-callgrind-runner
  run: |
    version=$(cargo metadata --format-version=1 |\
      jq '.packages[] | select(.name == "iai-callgrind").version' |\
      tr -d '"'
    )
    cargo install iai-callgrind-runner --version $version
```

Or, speed up the overall installation time with `binstall` using the
[taiki-e/install-action](https://github.com/taiki-e/install-action)

```yaml
- uses: taiki-e/install-action@cargo-binstall
- name: Install iai-callgrind-runner
  run: |
    version=$(cargo metadata --format-version=1 |\
      jq '.packages[] | select(.name == "iai-callgrind").version' |\
      tr -d '"'
    )
    cargo binstall --no-confirm iai-callgrind-runner --version $version
```
