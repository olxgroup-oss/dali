# (c) Copyright 2019-2020 OLX
ARG BUILD_IMAGE=docker.pkg.github.com/olxgroup-oss/dali/base-rust-image
ARG BASE_IMAGE=docker.pkg.github.com/olxgroup-oss/dali/base-dali

FROM ${BUILD_IMAGE}:latest as build

WORKDIR /usr/src/dali

COPY . .

# this flag ensures that proc macro can be compiled in musl targets
RUN RUSTFLAGS="-C target-feature=-crt-static $(pkg-config vips --libs)" cargo install --path .

FROM ${BASE_IMAGE}:latest

COPY --from=build /usr/local/cargo/bin/dali /usr/local/bin/dali

CMD dali
