# (c) Copyright 2019-2026 OLX
# We are manually installing and configuring libvips and each required package because previously when trying to use
# the community built bundles (i.e. vips and vips-heif) the performace of Dali has been significantly degraded.
FROM rust:1.96.0-alpine3.24 AS build

WORKDIR /usr/src/dali
RUN apk add --update --no-cache --repository https://dl-cdn.alpinelinux.org/alpine/v3.24/main \
    --repository https://dl-cdn.alpinelinux.org/alpine/v3.24/community \
    build-base=0.5-r4 \
    clang18=18.1.8-r10 \
    clang16-libclang=16.0.6-r11 \
    expat-dev=2.8.2-r0 \
    giflib-dev=5.2.2-r1 \
    glib-dev=2.88.1-r2 \
    # [CHANGE 1] highway-dev enables SIMD acceleration in libvips (resize, colour conversion, etc.)
    # Without it, libvips falls back to scalar code paths that can be 2-4x slower.
    highway-dev=1.3.0-r2 \
    lcms2-dev=2.19-r0 \
    libexif-dev=0.6.26-r0 \
    libheif-dev=1.23.0-r0 \
    libimagequant-dev=4.2.2-r0 \
    libjpeg-turbo-dev=3.1.3-r0 \
    libpng-dev=1.6.58-r1 \
    librsvg-dev=2.62.3-r0 \
    libwebp-dev=1.6.0-r0 \
    openssl-dev=3.5.7-r0 \
    orc-dev=0.4.42-r0 \
    pkgconf=2.9.95-r0 \
    tiff-dev=4.7.1-r0 \
    meson=1.11.1-r0 \
    samurai=1.3-r0

RUN wget https://github.com/libvips/libvips/archive/refs/tags/v8.18.2.tar.gz && \
    mkdir /vips && \
    tar xvzf v8.18.2.tar.gz -C /vips --strip-components 1 && \
    cd /vips && \
    # highway is auto-detected by meson; no extra flag needed — just having it installed is enough.
    meson setup build --buildtype=release --prefix=/usr/local -Ddebug=false -Dopenexr=disabled && \
    ninja -C build && \
    ninja -C build install && \
    rm -rf vips v8.18.2.tar.gz

COPY . .

ARG DALI_FEATURES=reqwest
RUN RUSTFLAGS="-C target-feature=-crt-static $(pkg-config vips --libs)" cargo build --features ${DALI_FEATURES} --release

# ---- final image ----
FROM alpine:3.24
ENV GI_TYPELIB_PATH=/usr/lib/girepository-1.0

# With the next command, the libvips binaries are copied from the previous stage hence we don't have to install it again.
# The default location where the `configure` script installs libvips is `/usr/local/lib` unless overridden by `--prefix`
# which we don't do.
# !!! Also it's essential for the next `COPY` command to be executed before adding the rest of the packages thus the
# libvips binaries are included in the ldconfig cache.
COPY --from=build /usr/local/lib /usr/local/lib

RUN apk add --update --no-cache \
    --repository=https://dl-cdn.alpinelinux.org/alpine/v3.24/main \
    --repository=https://dl-cdn.alpinelinux.org/alpine/v3.24/community \
      expat=2.8.2-r0 \
      giflib=5.2.2-r1 \
      glib=2.88.1-r2 \
      # [CHANGE 2] jemalloc replaces musl's default allocator at runtime via LD_PRELOAD (see ENV below).
      # musl's malloc uses a single global lock under contention, which bottlenecks libvips's
      # multi-threaded pipeline. jemalloc uses per-thread arenas, eliminating that contention.
      jemalloc=5.3.0-r6 \
      lcms2=2.19-r0 \
      libde265=1.0.18-r0 \
      libexif=0.6.26-r0 \
      libgsf=1.14.58-r0 \
      libheif=1.23.0-r0 \
      libimagequant=4.2.2-r0 \
      libjpeg-turbo=3.1.3-r0 \
      libpng=1.6.58-r1 \
      librsvg=2.62.3-r0 \
      libwebp=1.6.0-r0 \
      libwebpdemux=1.6.0-r0 \
      libwebpmux=1.6.0-r0 \
      openssl=3.5.7-r0 \
      orc=0.4.42-r0 \
      libhwy=1.3.0-r2 \
      tiff=4.7.1-r0

# Inject jemalloc as the allocator for the dali process.
ENV LD_PRELOAD=/usr/lib/libjemalloc.so.2

# Tune jemalloc for a long-running containerised image-processing service:
#   background_thread  — a dedicated thread returns memory to the OS so processing threads never block on it.
#   dirty_decay_ms     — how long (ms) before jemalloc releases dirty (recently freed) pages; 1000ms is a
#                        balanced default. Lower values reduce RSS; higher values reduce syscall overhead.
#   muzzy_decay_ms     — same for "muzzy" (OS-returned-but-not-purged) pages.
#   narenas            — number of allocation arenas. Defaults to 4×CPU which is excessive in a container;
#                        2 is enough to avoid lock contention without wasting per-arena bookkeeping memory.
ENV MALLOC_CONF="background_thread:true,dirty_decay_ms:1000,muzzy_decay_ms:1000,narenas:2"

COPY --from=build /usr/src/dali/target/release/dali /usr/local/bin/dali

# Running as root is dangerous as it can lead to several unexpected side effects
USER nobody
CMD ["dali"]
