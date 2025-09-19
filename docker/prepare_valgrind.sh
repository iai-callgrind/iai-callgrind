#!/bin/bash

set -ex

apt-get update && apt-get install --assume-yes --no-install-recommends wget lbzip2

cd

mkdir valgrind
cd valgrind
wget https://sourceware.org/pub/valgrind/valgrind-"${GUNGRAUN_CROSS_VALGRIND_VERSION}".tar.bz2
tar xf valgrind-"${GUNGRAUN_CROSS_VALGRIND_VERSION}".tar.bz2
