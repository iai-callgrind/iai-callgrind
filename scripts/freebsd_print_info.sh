#!/usr/bin/env sh

set -e

# shellcheck disable=SC1090
. ~/.cargo/env

pwd
ls -lah
whoami
env
freebsd-version

valgrind --version

rustup --version
rustup show
rustup component list --installed
rustc --version --verbose
