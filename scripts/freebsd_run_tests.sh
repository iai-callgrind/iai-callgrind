#!/usr/bin/env sh

# spell-checker:ignore nocapture

set -e

# shellcheck disable=SC1090
. ~/.cargo/env

cargo test -p client-request-tests --test tests --release -- --nocapture
