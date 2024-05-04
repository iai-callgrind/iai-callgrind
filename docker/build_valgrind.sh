#!/bin/bash
# spell-checker: ignore DESTDIR autogen

set -ex

export CC="${CROSS_TOOLCHAIN_PREFIX}gcc"
export LD="${CROSS_TOOLCHAIN_PREFIX}ld"
export AR="${CROSS_TOOLCHAIN_PREFIX}ar"

which "$CC" "$LD" "$AR"

[[ -n "$IAI_CALLGRIND_CROSS_TARGET" ]] || {
  echo "IAI_CALLGRIND_CROSS_TARGET environment variable is not defined"
  exit 1
}

case $IAI_CALLGRIND_CROSS_TARGET in
*-*-*-*) ;;
*-*-*) ;;
*)
  echo "Invalid target specification for IAI_CALLGRIND_CROSS_TARGET: '$IAI_CALLGRIND_CROSS_TARGET'" >&2
  exit 1
  ;;
esac

cd ~/valgrind/valgrind-"${IAI_CALLGRIND_CROSS_VALGRIND_VERSION}"

dest_dir="/valgrind"
target_dir="/target/valgrind/${IAI_CALLGRIND_CROSS_TARGET}"

mkdir "$dest_dir"

./autogen.sh

# According to valgrind/configure file, the IAI_CALLGRIND_CROSS_TARGET is
# supported as is for the --host variable. If the target is not supported by
# valgrind, configure will exit with an error.
./configure --prefix="$target_dir" \
  --host="$IAI_CALLGRIND_CROSS_TARGET"

make -j4
make -j4 install DESTDIR="$dest_dir"
