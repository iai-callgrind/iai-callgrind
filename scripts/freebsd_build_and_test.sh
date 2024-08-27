#!/usr/bin/env sh
# spell-checker:ignore tlsv

set -e

# shellcheck disable=SC1090
. ~/.cargo/env

echo Prepare

# Generic approach
# host=$(rustc -vV | grep '^host:' | cut -d' ' -f2)
# curl --proto '=https' --tlsv1.2 -fsSL "https://github.com/taiki-e/cargo-hack/releases/latest/download/cargo-hack-$host.tar.gz" | tar xzf - -C "$HOME/.cargo/bin"

# Download binary and install to $HOME/.cargo/bin
curl --proto '=https' --tlsv1.2 -fsSL "https://github.com/taiki-e/cargo-hack/releases/latest/download/cargo-hack-x86_64-unknown-freebsd.tar.gz" | tar xzf - -C "$HOME/.cargo/bin"

cargo update

echo Build with the feature powerset
just build-hack

# This excludes the ui tests
echo Run normal tests
cargo test --workspace --exclude client-request-tests

echo Run client request tests
cargo +stable test -p client-request-tests --test tests --release -- --nocapture

echo Run benchmark tests
just full-bench-test-all
