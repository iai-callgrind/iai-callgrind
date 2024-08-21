# Prerequisites

In order to use Iai-Callgrind, you must have [Valgrind](https://www.valgrind.org) installed. This
means that Iai-Callgrind cannot be used on platforms that are not supported by Valgrind.

## Debug Symbols

It's required to run the iai-callgrind benchmarks with debugging symbols
switched on. For example in your `~/.cargo/config` or your project's
`Cargo.toml`:

```toml
[profile.bench]
debug = true
```

Now, all benchmarks which are run with `cargo bench` include the debug symbols.
(See also [Cargo
Profiles](https://doc.rust-lang.org/cargo/reference/profiles.html) and [Cargo
Config](https://doc.rust-lang.org/cargo/reference/config.html)).

It's required that settings like `strip = true` or other configuration options
stripping the debug symbols need to be disabled explicitly for the `bench`
profile if you have changed this option for the `release` profile. For example:

```toml
[profile.release]
strip = true

[profile.bench]
debug = true
strip = false
```

## Valgrind Client Requests

If you want to make use of the mighty [Valgrind Client Request
Mechanism](https://valgrind.org/docs/manual/manual-core-adv.html#manual-core-adv.clientreq)
shipped with Iai-Callgrind, you also need `libclang` (clang >= 5.0) installed.
See also the requirements of
[bindgen](https://rust-lang.github.io/rust-bindgen/requirements.html)) and of
[cc](https://github.com/rust-lang/cc-rs).

More details on the usage and requirements of Valgrind Client Requests in
[this](../client_requests.md) chapter of the guide.

## Installation of Valgrind

Iai-Callgrind is intentionally independent from a specific version of valgrind.
However, Iai-Callgrind was only tested with versions of valgrind >= `3.20.0`. It
is therefore highly recommended to use a recent version of valgrind. Bugs get
fixed, the supported platforms are expanded ... Also, if you want or need to,
[building valgrind from
source](https://sourceware.org/git/?p=valgrind.git;a=blob;f=README;h=eabcc6ad88c8cab6dfe73cfaaaf5543023c2e941;hb=HEAD)
is usually a straight-forward process. Just make sure the `valgrind` binary is
in your `$PATH` so that `iai-callgrind` can find it.

### Installation of valgrind with your package manager

#### Alpine Linux

```bash
apk add just
```

#### Arch Linux

```bash
pacman -Sy valgrind
```

#### Debian/Ubuntu

```bash
apt-get install valgrind
```

#### Fedora Linux

```bash
dnf install valgrind
```

#### FreeBSD

```bash
pkg install valgrind
```

#### Valgrind is available for the following distributions

[![Packaging status](https://repology.org/badge/vertical-allrepos/valgrind.svg)](https://repology.org/project/valgrind/versions)
