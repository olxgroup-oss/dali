# (c) Copyright 2019-2025 OLX
# We are manually installing and configuring libvips and each required package because previously when trying to use
# the community built bundles (i.e. vips and vips-heif) the performace of Dali has been significantly degraded.
FROM rust:1.81.0-alpine3.20 AS build

WORKDIR /usr/src/dali
RUN apk add --update --no-cache --repository https://dl-cdn.alpinelinux.org/alpine/v3.20/main \
    build-base=0.5-r3 \
    clang17=17.0.6-r1 \
    clang16-libclang=16.0.6-r5 \
    expat-dev=2.7.0-r0 \
    giflib-dev=5.2.2-r0 \
    glib-dev=2.80.5-r0 \
    lcms2-dev=2.16-r0 \
    libexif-dev=0.6.24-r2 \
    libheif-dev=1.17.6-r1 \
    libimagequant-dev=4.2.2-r0 \
    libjpeg-turbo-dev=3.0.3-r0 \
    libpng-dev=1.6.44-r0 \
    librsvg-dev=2.58.5-r0 \
    libwebp-dev=1.3.2-r0 \
    openssl-dev=3.3.3-r0 \
    orc-dev=0.4.40-r0 \
    pkgconf=2.2.0-r0 \
    tiff-dev=4.6.0t-r0 \
    meson=1.4.0-r2 \
    samurai=1.2-r5

RUN wget https://github.com/libvips/libvips/archive/refs/tags/v8.16.1.tar.gz && \
    mkdir /vips && \
    tar xvzf v8.16.1.tar.gz -C /vips --strip-components 1 && \
    cd /vips && \
    meson setup build --buildtype=release --prefix=/usr/local -Ddebug=false -Dopenexr=disabled && \
    ninja -C build && \
    ninja -C build install && \
    rm -rf vips v8.16.1.tar.gz

COPY . .

ARG DALI_FEATURES=reqwest
RUN RUSTFLAGS="-C target-feature=-crt-static $(pkg-config vips --libs)" cargo build --features ${DALI_FEATURES} --release

FROM alpine:3.20.0
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
      expat=2.7.0-r0 \
      giflib=5.2.2-r0 \
      glib=2.80.5-r0 \
      lcms2=2.16-r0 \
      libde265=1.0.15-r0 \
      libexif=0.6.24-r2 \
      libgsf=1.14.52-r0 \
      libheif=1.17.6-r1 \
      libimagequant=4.2.2-r0 \
      libjpeg-turbo=3.0.3-r0 \
      libpng=1.6.44-r0 \
      librsvg=2.58.5-r0 \
      libwebp=1.3.2-r0 \
      libwebpdemux=1.3.2-r0 \
      libwebpmux=1.3.2-r0 \
      openssl=3.3.3-r0 \
      orc=0.4.40-r0 \
      tiff=4.6.0t-r0

COPY --from=build /usr/src/dali/target/release/dali /usr/local/bin/dali

# Running as root is dangerous as it can lead to several unexpected side effects
USER nobody
CMD ["dali"]
