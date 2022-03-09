# (c) Copyright 2019-2020 OLX
FROM rust:1.59.0-slim as build

WORKDIR /usr/src/dali

RUN apt-get update && apt install -y \
        libvips-dev=8.10.5-2 \
        libvips42=8.10.5-2 \
        make

COPY . .

RUN cargo build --release

FROM debian:stable-20220228-slim

COPY --from=build /usr/src/dali/target/release/dali /usr/local/bin/dali

RUN apt-get update && apt install -y \
        libvips42=8.10.5-2 && \
    rm -rf /var/lib/apt/lists/*

CMD ["dali"]