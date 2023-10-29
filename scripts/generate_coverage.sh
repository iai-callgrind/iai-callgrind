#!/bin/bash

# spell-checker: ignore readelf Cdebuginfo Cinstrument Csplit bindir libdir

# Helper script to generate coverage data with grcov using the coverage profile
# from `.cargo/config` to simplify cleanup of stale coverage data.

root_dir="$(cd "$(dirname "$0")" && cd ..))" || exit 1

export RUSTFLAGS="-Cinstrument-coverage"
export LLVM_PROFILE_FILE="iai_callgrind_coverage-%p-%m.profraw"

bindir="$(dirname "$(rustc --print target-libdir)")/bin"
if [[ ! -e "${bindir}/llvm-cov" ]]; then
  echo "Install llvm-tools or llvm-tools-preview with rustup first."
  exit 1
fi

# Clean old coverage data
rm -rfv 'target/coverage'
find . -type f \( -iname '*.profraw' -o -iname 'lcov.info' \) -print0 | xargs -0 rm -fv

cargo build --all-features --profile coverage
cargo test --all-features --profile coverage

grcov . \
  --branch \
  --llvm-path "$bindir" \
  --binary-path target/coverage \
  --ignore-not-existing \
  --output-type lcov \
  --source-dir . \
  --excl-start 'cov:\s*excl-start' \
  --excl-stop 'cov:\s*excl-stop' \
  --excl-line '^\s*((debug_)?assert(_eq|_ne)?!|#\[derive\(|.*cov:\s*excl-line)' \
  --ignore '**/examples/*' \
  --ignore '/*' \
  --ignore '[a-zA-Z]:/*' \
  --output-path lcov.info && test -e lcov.info
