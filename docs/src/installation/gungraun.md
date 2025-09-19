# Gungraun

Gungraun is divided into the library `gungraun` and the benchmark runner
`gungraun-runner`.

## Installation of the library

To start with Gungraun, add the following to your `Cargo.toml` file:

```toml
[dev-dependencies]
gungraun = "0.16.1"
```

or run

```bash
cargo add --dev gungraun@0.16.1
```

## Installation of the benchmark runner

To be able to run the benchmarks you'll also need the `gungraun-runner`
binary installed somewhere in your `$PATH`. Otherwise, there is no need to
interact with `gungraun-runner` as it is just an implementation detail.

### From Source

```shell
cargo install --version 0.16.1 gungraun-runner
```

There's also the possibility to install the binary somewhere else and point the
`GUNGRAUN_RUNNER` environment variable to the absolute path of the
`gungraun-runner` binary like so:

```shell
cargo install --version 0.16.1 --root /tmp gungraun-runner
GUNGRAUN_RUNNER=/tmp/bin/gungraun-runner cargo bench --bench my-bench
```

### Binstall

The `gungraun-runner` binary is
[pre-built](https://github.com/gungraun/gungraun/releases/tag/v0.16.1)
for most platforms supported by valgrind and easily installable with
[binstall](https://github.com/cargo-bins/cargo-binstall)

```shell
cargo binstall gungraun-runner@0.16.1
```

## Updating

When updating the `gungraun` library, you'll also need to update
`gungraun-runner` and vice-versa or else the benchmark runner will exit
with an error.

### In the Github CI

Since the `gungraun-runner` version must match the `gungraun` library
version it's best to automate this step in the CI. A job step in the github
actions CI could look like this

```yaml
- name: Install gungraun-runner
  run: |
    version=$(cargo metadata --format-version=1 |\
      jq '.packages[] | select(.name == "gungraun").version' |\
      tr -d '"'
    )
    cargo install gungraun-runner --version $version
```

Or, speed up the overall installation time with `binstall` using the
[taiki-e/install-action](https://github.com/taiki-e/install-action)

```yaml
- uses: taiki-e/install-action@cargo-binstall
- name: Install gungraun-runner
  run: |
    version=$(cargo metadata --format-version=1 |\
      jq '.packages[] | select(.name == "gungraun").version' |\
      tr -d '"'
    )
    cargo binstall --no-confirm gungraun-runner --version $version
```
