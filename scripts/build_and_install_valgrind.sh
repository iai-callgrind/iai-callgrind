#!/bin/sh -ex
# spell-checker: ignore autogen

VALGRIND_VERSION="${1:-3.25.1}"

apt-get update && apt-get install --assume-yes --no-install-recommends wget lbzip2

cd

mkdir valgrind
cd valgrind
wget https://sourceware.org/pub/valgrind/valgrind-"${VALGRIND_VERSION}".tar.bz2
tar xf valgrind-"${VALGRIND_VERSION}".tar.bz2

cd valgrind-"${VALGRIND_VERSION}"

./autogen.sh

./configure --prefix=/usr
make -j4
sudo make -j4 install
