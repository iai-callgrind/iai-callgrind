#!/bin/bash

set -ex

apt-get update && apt-get install --assume-yes --no-install-recommends wget

cd

mkdir valgrind
cd valgrind
wget https://sourceware.org/pub/valgrind/valgrind-3.22.0.tar.bz2
tar xf valgrind-3.22.0.tar.bz2
