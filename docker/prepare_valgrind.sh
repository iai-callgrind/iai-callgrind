#!/bin/bash

set -ex

apt-get update && apt-get install --assume-yes --no-install-recommends wget

cd

mkdir valgrind
cd valgrind
wget https://sourceware.org/pub/valgrind/valgrind-"${IAI_CALLGRIND_CROSS_VALGRIND_VERSION}".tar.bz2
tar xf valgrind-"${IAI_CALLGRIND_CROSS_VALGRIND_VERSION}".tar.bz2
