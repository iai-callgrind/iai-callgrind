FROM debian:bookworm-slim

ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:$PATH \
    RUST_VERSION=1.77.0 \
    QEMU_VERSION=8.2.2 \
    VALGRIND_VERSION=3.22.0 \
    IAI_CALLGRIND_VERSION=0.10.2


ENV CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER="x86_64-linux-gnu-gcc" \
    CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER="aarch64-linux-gnu-gcc"

COPY runner /

RUN set -eux; \
	apt-get update; \
	apt-get install -y --no-install-recommends \
		ca-certificates \
		wget \
		git \
		dropbear \
		bzip2 \
		cpio \
		automake \
		pkg-config \
		libglib2.0-0 \
		libglib2.0-dev \
		libattr1-dev \
		libslirp0 \
		libslirp-dev \
		make \
		xz-utils \
		python3 python3-venv python3-setuptools \
		ninja-build \
		gcc \
		gcc-x86-64-linux-gnu \
		gcc-aarch64-linux-gnu \
        libc6-dev-amd64-cross \
		libc6-dev-arm64-cross \
		binutils \
		libc6-dev \
	; \
    dpkgArch="$(dpkg --print-architecture)"; \
    case "${dpkgArch##*-}" in \
        amd64) rustArch='x86_64-unknown-linux-gnu' ;; \
        arm64) rustArch='aarch64-unknown-linux-gnu' ;; \
        *) echo >&2 "unsupported architecture: ${dpkgArch}"; exit 1 ;; \
    esac; \
    # rust
    wget -q "https://static.rust-lang.org/rustup/archive/1.27.0/${rustArch}/rustup-init"; \
    chmod +x rustup-init; \
    ./rustup-init -y --no-modify-path \
        --profile minimal \
        --default-toolchain $RUST_VERSION \
        --default-host ${rustArch} \
        --target x86_64-unknown-linux-gnu aarch64-unknown-linux-gnu \
    ; \
    rm rustup-init; \
    chmod -R a+w $RUSTUP_HOME $CARGO_HOME; \
    # cargo (for cargo metadata)
    git clone --depth 1 --branch rust-${RUST_VERSION} https://github.com/rust-lang/cargo.git; \
    cd cargo; \
    cargo build --features=vendored-openssl --release --target x86_64-unknown-linux-gnu; \
    cargo build --features=vendored-openssl --release --target aarch64-unknown-linux-gnu; \
    cd ..; \
    # qemu
    wget -q "https://download.qemu.org/qemu-$QEMU_VERSION.tar.xz"; \
    tar xJf qemu-$QEMU_VERSION.tar.xz; \
    cd qemu-$QEMU_VERSION; \
    ./configure \
      --prefix="/qemu" \
      --with-pkgversion="mezha" \
      --target-list=aarch64-softmmu,x86_64-softmmu \
      --without-default-features \
      --enable-lto \
      --enable-strip \
      --enable-attr \
      --enable-virtfs \
      --enable-slirp \
      --enable-tcg \
      --enable-plugins \
    ; \
    make -j$(nproc) && make install; \
    cd ..; \
    rm -rf qemu-$QEMU_VERSION qemu-$QEMU_VERSION.tar.xz; \
    # valgrind
    wget -q "https://sourceware.org/pub/valgrind/valgrind-$VALGRIND_VERSION.tar.bz2"; \
    tar xjf valgrind-$VALGRIND_VERSION.tar.bz2;  \
    cd valgrind-$VALGRIND_VERSION; \
    ./configure \
        --prefix="/usr/" \
        --host="x86_64-linux-gnu" \
        --enable-only64bit \
        --enable-lto \
    ; \
    make -j$(nproc) && make install DESTDIR=/x86_64; \
    cd ..; \
    rm -rf valgrind-$VALGRIND_VERSION; \
    tar xjf valgrind-$VALGRIND_VERSION.tar.bz2;  \
    cd valgrind-$VALGRIND_VERSION; \
    ./configure \
        --prefix="/usr/" \
        --host="aarch64-linux-gnu" \
        --enable-only64bit \
        --enable-lto \
    ; \
    make -j$(nproc) && make install DESTDIR=/aarch64; \
    cd ..; \
    mv /x86_64/usr /x86_64/valgrind; \
    mv /aarch64/usr /aarch64/valgrind; \
    rm -rf valgrind-$VALGRIND_VERSION valgrind-$VALGRIND_VERSION.tar.bz2; \
    # iai-callgrind
    wget -q https://github.com/iai-callgrind/iai-callgrind/releases/download/v${IAI_CALLGRIND_VERSION}/iai-callgrind-runner-v${IAI_CALLGRIND_VERSION}-aarch64-unknown-linux-gnu.tar.gz; \
    tar -xzf iai-callgrind-runner-v${IAI_CALLGRIND_VERSION}-aarch64-unknown-linux-gnu.tar.gz; \
    wget -q https://github.com/iai-callgrind/iai-callgrind/releases/download/v${IAI_CALLGRIND_VERSION}/iai-callgrind-runner-v${IAI_CALLGRIND_VERSION}-x86_64-unknown-linux-gnu.tar.gz; \
    tar -xzf iai-callgrind-runner-v${IAI_CALLGRIND_VERSION}-x86_64-unknown-linux-gnu.tar.gz; \
    # ssh
    mkdir /root/.ssh; \
    dropbearkey -t rsa -f /root/.ssh/dropbear_rsa_host_key; \
    SSH_PUBLIC_KEY=$(dropbearkey -y -f /root/.ssh/dropbear_rsa_host_key | grep "^ssh-rsa" | awk '{print $2}'); \
    echo "localhost ssh-rsa ${SSH_PUBLIC_KEY}" >> /root/.ssh/known_hosts; \
    # prepare images
    bash -c 'mkdir -p /qemu/images/{amd64,arm64}/{deps,root/{modules,sys,dev,proc,sbin,bin,etc/dropbear,root,usr/{bin,sbin,lib,libexec},var/log,tmp}}'; \
    chmod -R 777 "/qemu/images/"; \
    for arch in amd64 arm64; do \
        case "$arch" in \
            amd64) nArch='x86_64' ;; \
            arm64) nArch='aarch64' ;; \
        esac; \
        cd "/qemu/images/$arch"; \
        dpkg --add-architecture $arch; \
	    apt-get update; \
	    cd deps; \
	    apt-get download -d --no-install-recommends \
            linux-image-6.1.0-12-$arch:$arch \
            libc6:$arch \
            libc6-dbg:$arch \
            libstdc++6:$arch \
            libgcc-s1:$arch \
            libcrypt1:$arch \
            busybox:$arch \
            ncurses-base \
            zlib1g:$arch \
            libgmp10:$arch \
            libtommath1:$arch \
            libtomcrypt1:$arch \
            dropbear-bin:$arch \
            ; \
        for deb in *deb; do \
            dpkg -x "${deb}" "../root"; \
        done; \
	    dpkg --remove-architecture "$arch" || true; \
	    cd ..; \
        rm -rf deps; \
	    cp "root/boot/vmlinu"* kernel; \
        rm -rf "root/boot"; \
        cd root; \
        cp -v \
            lib/modules/6.1.0-12-$arch/kernel/drivers/net/net_failover.ko \
            lib/modules/6.1.0-12-$arch/kernel/drivers/net/virtio_net.ko \
            lib/modules/6.1.0-12-$arch/kernel/drivers/virtio/virtio_mmio.ko \
            lib/modules/6.1.0-12-$arch/kernel/drivers/virtio/virtio_pci_legacy_dev.ko \
            lib/modules/6.1.0-12-$arch/kernel/drivers/virtio/virtio_pci_modern_dev.ko \
            lib/modules/6.1.0-12-$arch/kernel/drivers/virtio/virtio_pci.ko \
            lib/modules/6.1.0-12-$arch/kernel/fs/netfs/netfs.ko \
            lib/modules/6.1.0-12-$arch/kernel/fs/9p/9p.ko \
            lib/modules/6.1.0-12-$arch/kernel/fs/fscache/fscache.ko \
            lib/modules/6.1.0-12-$arch/kernel/net/9p/9pnet.ko \
            lib/modules/6.1.0-12-$arch/kernel/net/9p/9pnet_virtio.ko \
            lib/modules/6.1.0-12-$arch/kernel/net/core/failover.ko \
            modules; \
        if [ "$arch" = "amd64" ]; then \
            cp -v \
                lib/modules/6.1.0-12-$arch/kernel/drivers/virtio/virtio.ko \
                lib/modules/6.1.0-12-$arch/kernel/drivers/virtio/virtio_ring.ko \
                modules; \
        fi; \
        rm -rf "lib/modules"; \
        # rustlib
        ls -1 /usr/local/rustup/toolchains/${RUST_VERSION}-${rustArch}/lib/rustlib/${nArch}-unknown-linux-gnu/lib/libstd-*.so | xargs -i cp {} usr/lib/; \
        # cargo
        mv /cargo/target/${nArch}-unknown-linux-gnu/release/cargo usr/bin/cargo; \
        # iai-callgrind-runner
        mv /iai-callgrind-runner-v${IAI_CALLGRIND_VERSION}-${nArch}-unknown-linux-gnu/iai-callgrind-runner usr/bin/iai-callgrind-runner; \
        # valgrind
        mv /${nArch}/valgrind/bin/valgrind usr/bin/valgrind; \
        mv /${nArch}/valgrind/lib/valgrind usr/lib/valgrind; \
        find /${nArch}/valgrind/libexec/valgrind -type f \
          -name "mips-*" -o \
          -name "mips64-*" -o \
          -name "power-*" -o \
          -name "power64-*" -o \
          -name "powerpc-*" -o \
          -name "s390-*" -o \
          -name "s390x-*" -o \
          -name "i386-*" -o \
          -name "32bit-*" -o \
          -name "valgrind-monitor*" -o \
          -name "dh_view*" \
          | xargs rm -f; \
        mv /${nArch}/valgrind/libexec/valgrind usr/libexec/valgrind; \
        if [ "${rustArch}" = "${nArch}-unknown-linux-gnu" ]; then \
            mv /${nArch}/valgrind/bin/* /usr/bin; \
        fi; \
        find /${nArch}/valgrind/ -type d \
          -name bin -o \
          -name lib -o \
          -name libexec -o \
          -name share \
          | xargs rm -rf; \
        # configure
        touch var/log/lastlog; \
        echo "root::0:0:root:/root:/bin/sh" >> etc/passwd; \
        echo "127.0.0.1 localhost qemu" >> etc/hosts; \
        echo "cd /app" >> etc/profile; \
        echo "qemu" >> etc/hostname; \
        cp /root/.ssh/dropbear_rsa_host_key etc/dropbear/dropbear_rsa_host_key; \
        echo '#!'"/bin/busybox sh" >> init; \
        echo "" >> init; \
        echo "set -e" >> init; \
        echo "" >> init; \
        echo "/bin/busybox --install" >> init; \
        echo "" >> init; \
        echo "mount -t devtmpfs devtmpfs /dev" >> init; \
        echo "mount -t proc none /proc" >> init; \
        echo "mount -t sysfs none /sys" >> init; \
        echo "mkdir /dev/pts" >> init; \
        echo "mount -t devpts none /dev/pts/" >> init; \
        echo "" >> init; \
        echo "insmod /modules/failover.ko" >> init; \
        echo "insmod /modules/net_failover.ko" >> init; \
        if [ "$arch" = "amd64" ]; then \
            echo "insmod /modules/virtio.ko" >> init; \
            echo "insmod /modules/virtio_ring.ko" >> init; \
        fi; \
        echo "insmod /modules/virtio_mmio.ko" >> init; \
        echo "insmod /modules/virtio_pci_legacy_dev.ko" >> init; \
        echo "insmod /modules/virtio_pci_modern_dev.ko" >> init; \
        echo "insmod /modules/virtio_pci.ko" >> init; \
        echo "insmod /modules/virtio_net.ko" >> init; \
        echo "insmod /modules/netfs.ko" >> init; \
        echo "insmod /modules/fscache.ko" >> init; \
        echo "insmod /modules/9pnet.ko" >> init; \
        echo "insmod /modules/9pnet_virtio.ko" >> init; \
        echo "insmod /modules/9p.ko" >> init; \
        echo "" >> init; \
        echo "ifconfig lo 127.0.0.1" >> init; \
        echo "ifconfig eth0 10.0.2.15" >> init; \
        echo "route add default gw 10.0.2.2 eth0" >> init; \
        echo "" >> init; \
        echo "mkdir /app" >> init; \
        echo "mount -t 9p -o trans=virtio app /app -oversion=9p2000.u" >> init; \
        echo "" >> init; \
        # TODO: add setarch to busybox (iai-callgrind-runner supports it)
        echo "echo 0 > /proc/sys/kernel/randomize_va_space" >> init; \
        echo "" >> init; \
        echo "exec dropbear -F -E -B" >> init; \
        chmod +x init; \
        find . | cpio --create --format='newc' --quiet | gzip >../initrd.gz; \
        cd ..; \
        rm -rf root; \
        mv /qemu/images/${arch} /qemu/images/${nArch}; \
    done; \
    rm /root/.ssh/dropbear_rsa_host_key; \
    # prepare workspace
    mkdir -p /app; \
    # clear
    rm -rf /cargo; \
    rm -rf /iai-callgrind-runner-*; \
    rm -rf /usr/local/cargo/registry; \
    apt-get remove -y --auto-remove \
        perl \
		wget \
		bzip2 \
		cpio \
		automake \
		pkg-config \
		libglib2.0-dev \
		libattr1-dev \
		libslirp-dev \
		make \
		xz-utils \
		python3 python3-venv python3-setuptools \
		ninja-build \
		gcc \
		binutils \
		libc6-dev \
    ; \
    apt-get install -y --no-install-recommends \
        gcc-x86-64-linux-gnu \
		gcc-aarch64-linux-gnu \
    ; \
	rm -rf /var/lib/apt/lists/*

ENV CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUNNER="/runner x86_64" \
    CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_RUNNER="/runner aarch64" \
    IAI_CALLGRIND_aarch64_unknown_linux_gnu_VALGRIND_INCLUDE="/aarch64/valgrind/include" \
    IAI_CALLGRIND_x86_64_unknown_linux_gnu_VALGRIND_INCLUDE="/x86_64/valgrind/include"

WORKDIR /app
