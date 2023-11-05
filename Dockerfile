# (c) Copyright 2019-2023 OLX

# Rust versions for the vips packages can be found here...
# https://pkgs.alpinelinux.org/packages?name=vips&branch=v3.18

ARG ALPINE_DOCKER_VER=3.18.4
ARG ALPINE_VER=3.18
ARG VIPS_VER=8.14.3-r0
ARG RUST_VER=1.73.0

FROM rust:${RUST_VER}-alpine${ALPINE_VER} AS build

WORKDIR /usr/src/dali
RUN apk add --update --no-cache \
    --repository https://dl-cdn.alpinelinux.org/alpine/v${ALPINE_VER}/community \
    --repository https://dl-cdn.alpinelinux.org/alpine/v${ALPINE_VER}/main \
    musl-dev=1.2.4-r2 \
    vips-dev=${VIPS_VER}

COPY . .

RUN RUSTFLAGS="-C target-feature=-crt-static $(pkg-config vips --libs)" cargo build --release

FROM alpine:${ALPINE_DOCKER_VER}

RUN apk add --update --no-cache \
    --repository https://dl-cdn.alpinelinux.org/alpine/v${ALPINE_VER}/community \
    --repository https://dl-cdn.alpinelinux.org/alpine/v${ALPINE_VER}/main \
    vips=${VIPS_VER} \
    vips-heif=${VIPS_VER} \
    openssl

COPY --from=build /usr/src/dali/target/release/dali /usr/local/bin/dali

USER nobody

CMD ["dali"]
