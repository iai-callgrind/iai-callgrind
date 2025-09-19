#!/bin/bash

# spell-checker: ignore readelf Cdebuginfo Cinstrument Csplit bindir libdir

# Helper script to generate coverage data with grcov using the coverage profile
# from `.cargo/config` to simplify cleanup of stale coverage data.

root_dir="$(cd "$(dirname "$0")" && cd ..))" || exit 1

just_clean=0
if [[ "$1" == "--clean" ]]; then
  just_clean=1
fi
# Valgrind sometimes exits in benchmarks with the error
#
# valgrind: m_debuginfo/readelf.c:718 (get_elf_symbol_info): Assertion 'in_rx' failed.
#
# The error disappeared after adding -Cdebuginfo=2 to RUSTFLAGS and running all
# cargo commands with the nightly toolchain
export RUSTFLAGS="-Cinstrument-coverage -Cdebuginfo=2" # -Csplit-debuginfo=off
export LLVM_PROFILE_FILE="iai_callgrind_coverage-%p-%m.profraw"

bindir="$(dirname "$(rustc --print target-libdir)")/bin"
if [[ ! -e "${bindir}/llvm-cov" ]]; then
  echo "Install llvm-tools or llvm-tools-preview with rustup first."
  exit 1
fi

# Clean old coverage data
rm -rfv 'target/coverage'
find . -type f \( -iname '*.profraw' -o -iname 'lcov.info' \) -print0 | xargs -0 rm -fv

if [[ $just_clean == 1 ]]; then
  echo "Success cleaning coverage data"
  exit 0
fi

cargo +nightly build --all-features --profile coverage --all-targets

IAI_CALLGRIND_RUNNER=$(realpath -e target/coverage/gungraun-runner)
IAI_CALLGRIND_LOG=debug

export IAI_CALLGRIND_RUNNER IAI_CALLGRIND_LOG

cargo +nightly test --all-features --profile coverage --no-fail-fast
cargo +nightly bench --all-features --profile coverage --no-fail-fast

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
  --output-path lcov.info

if [[ ! -e lcov.info ]]; then
  echo "No lcov.info file found after running grcov"
  exit 1
fi
