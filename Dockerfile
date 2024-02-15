# (c) Copyright 2019-2024 OLX
# We are manually installing and configuring libvips and each required package because previously when trying to use
# the community built bundles (i.e. vips and vips-heif) the performace of Dali has been significantly degraded.
FROM rust:1.74.0-alpine3.18 as build

WORKDIR /usr/src/dali
RUN apk add --update --no-cache --repository https://dl-cdn.alpinelinux.org/alpine/v3.18/main \
    build-base=0.5-r3 \
    clang=16.0.6-r1 \
    clang16-libclang=16.0.6-r1 \
    expat-dev=2.6.0-r0 \
    giflib-dev=5.2.1-r4 \
    glib-dev=2.76.6-r0 \
    lcms2-dev=2.15-r2 \
    libexif-dev=0.6.24-r1 \
    libheif-dev=1.16.2-r0 \
    libimagequant-dev=4.2.0-r0 \
    libjpeg-turbo-dev=2.1.5.1-r3 \
    libpng-dev=1.6.39-r3 \
    librsvg-dev=2.56.3-r0 \
    libwebp-dev=1.3.2-r0 \
    openssl-dev=3.1.4-r5 \
    orc-dev=0.4.34-r0 \
    pkgconf=1.9.5-r0 \
    tiff-dev=4.5.1-r0

RUN wget https://github.com/libvips/libvips/releases/download/v8.13.3/vips-8.13.3.tar.gz && \
    mkdir /vips && \
    tar xvzf vips-8.13.3.tar.gz -C /vips --strip-components 1 && \
    cd /vips && \
    ./configure --enable-debug=no --without-OpenEXR --disable-static --enable-silent-rule && \
    make && \
    make install && \
    rm -rf vips vips-8.13.3.tar.gz

COPY . .

ARG DALI_FEATURES=reqwest
RUN RUSTFLAGS="-C target-feature=-crt-static $(pkg-config vips --libs)" cargo build --features ${DALI_FEATURES} --release

FROM alpine:3.18.4
ENV GI_TYPELIB_PATH=/usr/lib/girepository-1.0

# With the next command, the libvips bianries are copied from the previous stage hence we don't have to install it again.
# The default location where the `configure` script installs libvips is `/usr/local/lib` unless overridden by `--prefix`
# which we don't do.
# !!! Also it's essential for the next `COPY` command to be executed before adding the rest of the packages thus the
# libvips binaries are included in the ldconfig cache.
COPY --from=build /usr/local/lib /usr/local/lib

RUN apk add --update --no-cache  \
    --repository=https://dl-cdn.alpinelinux.org/alpine/v3.18/main  \
    --repository=https://dl-cdn.alpinelinux.org/alpine/v3.18/community \
      expat=2.6.0-r0 \
      giflib=5.2.1-r4 \
      glib=2.76.6-r0 \
      lcms2=2.15-r2 \
      libde265=1.0.15-r0 \
      libexif=0.6.24-r1 \
      libgsf=1.14.50-r1 \
      libheif=1.16.2-r0 \
      libimagequant=4.2.0-r0 \
      libjpeg-turbo=2.1.5.1-r3 \
      libpng=1.6.39-r3 \
      librsvg=2.56.3-r0 \
      libwebp=1.3.2-r0 \
      openssl=3.1.4-r5 \
      orc=0.4.34-r0 \
      tiff=4.5.1-r0

COPY --from=build /usr/src/dali/target/release/dali /usr/local/bin/dali

# Running as root is dangerous as it can lead to several unexpected side effects
USER nobody
CMD ["dali"]
