#!/usr/bin/env bash
# spellchecker:ignore libc

# We need the debug symbols of the libc6-dbg package available in the qemu image
# to be able to run Valgrind's memcheck. We can use the `/linux-image.sh` script
# from cross which is still present within the cross image.
set -xe

cd /

# shellcheck disable=SC2016
if ! grep 'libc6-dbg' /linux-image.sh; then
  sed -Ei 's#^(\s*["])(libc6.*)\\$#\1\2\\\n\1libc6-dbg:${arch}" \\#' /linux-image.sh
  arch="${IAI_CALLGRIND_CROSS_TARGET%%-*}"
  /linux-image.sh "$arch"
fi
