ARG IAI_CALLGRIND_CROSS_TARGET

FROM ghcr.io/cross-rs/${IAI_CALLGRIND_CROSS_TARGET}:main
ARG DEBIAN_FRONTEND=noninteractive

COPY prepare_valgrind.sh /
RUN /prepare_valgrind.sh

ARG IAI_CALLGRIND_CROSS_TARGET
COPY build_valgrind.sh /
RUN /build_valgrind.sh

COPY cleanup_valgrind.sh /
RUN /cleanup_valgrind.sh

ARG IAI_CALLGRIND_CROSS_TARGET
COPY linux-image.sh /
COPY build_custom_image.sh /
RUN /build_custom_image.sh

ARG IAI_CALLGRIND_CROSS_TARGET
ENV IAI_CALLGRIND_CROSS_TARGET=${IAI_CALLGRIND_CROSS_TARGET}
ENV IAI_CALLGRIND_CROSS_VALGRIND_DESTDIR="/target/valgrind/${IAI_CALLGRIND_CROSS_TARGET}"
ENV IAI_CALLGRIND_CROSS_VALGRIND_TEMPDIR="/valgrind/target"
