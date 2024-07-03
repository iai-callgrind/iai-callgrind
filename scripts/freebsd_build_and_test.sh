#!/usr/bin/env sh

set -e

# shellcheck disable=SC1090
. ~/.cargo/env

echo Prepare
cargo +stable install cargo-binstall
cargo +stable binstall --no-confirm cargo-hack

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
