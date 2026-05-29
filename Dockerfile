# (c) Copyright 2019-2026 OLX
# We are manually installing and configuring libvips and each required package because previously when trying to use
# the community built bundles (i.e. vips and vips-heif) the performace of Dali has been significantly degraded.
FROM rust:1.95.0-alpine3.23 AS build

WORKDIR /usr/src/dali
RUN apk add --update --no-cache --repository https://dl-cdn.alpinelinux.org/alpine/v3.23/main \
    build-base=0.5-r3 \
    clang18=18.1.8-r6 	 \
    clang16-libclang=16.0.6-r9 \
    expat-dev=2.7.5-r0 \
    giflib-dev=5.2.2-r1 \
    glib-dev=2.86.3-r0 \
    lcms2-dev=2.19-r0 \
    libexif-dev=0.6.26-r0 \
    libheif-dev=1.21.2-r0 \
    libimagequant-dev=4.2.2-r0 \
    libjpeg-turbo-dev=3.1.2-r0 \
    libpng-dev=1.6.58-r1 \
    librsvg-dev=2.61.2-r0 \
    libwebp-dev=1.6.0-r0 \
    openssl-dev=3.5.6-r0 \
    orc-dev=0.4.41-r0 \
    pkgconf=2.5.1-r0 \
    tiff-dev=4.7.1-r0 \
    meson=1.9.1-r0 \
    samurai=1.2-r7

RUN wget https://github.com/libvips/libvips/archive/refs/tags/v8.18.2.tar.gz && \
    mkdir /vips && \
    tar xvzf v8.18.2.tar.gz -C /vips --strip-components 1 && \
    cd /vips && \
    meson setup build --buildtype=release --prefix=/usr/local -Ddebug=false -Dopenexr=disabled && \
    ninja -C build && \
    ninja -C build install && \
    rm -rf vips v8.18.2.tar.gz

COPY . .

ARG DALI_FEATURES=reqwest
RUN RUSTFLAGS="-C target-feature=-crt-static $(pkg-config vips --libs)" cargo build --features ${DALI_FEATURES} --release

FROM alpine:3.23
ENV GI_TYPELIB_PATH=/usr/lib/girepository-1.0

# With the next command, the libvips bianries are copied from the previous stage hence we don't have to install it again.
# The default location where the `configure` script installs libvips is `/usr/local/lib` unless overridden by `--prefix`
# which we don't do.
# !!! Also it's essential for the next `COPY` command to be executed before adding the rest of the packages thus the
# libvips binaries are included in the ldconfig cache.
COPY --from=build /usr/local/lib /usr/local/lib

RUN apk add --update --no-cache  \
    --repository=https://dl-cdn.alpinelinux.org/alpine/v3.20/main  \
    --repository=https://dl-cdn.alpinelinux.org/alpine/v3.20/community \
      expat=2.7.5-r0 \
      giflib=5.2.2-r1 \
      glib=2.86.3-r0 \
      lcms2=2.19-r0 \
      libde265=1.0.16-r0 \
      libexif=0.6.26-r0 \
      libgsf=1.14.55-r0 \
      libheif=1.21.2-r0 \
      libimagequant=4.2.2-r0 \
      libjpeg-turbo=3.1.2-r0 \
      libpng=1.6.58-r1 \
      librsvg=2.61.2-r0 \
      libwebp=1.6.0-r0 \
      libwebpdemux=1.6.0-r0 \
      libwebpmux=1.6.0-r0 \
      openssl=3.5.6-r0 \
      orc=0.4.41-r0 \
      tiff=4.7.1-r0

COPY --from=build /usr/src/dali/target/release/dali /usr/local/bin/dali

# Running as root is dangerous as it can lead to several unexpected side effects
USER nobody
CMD ["dali"]
