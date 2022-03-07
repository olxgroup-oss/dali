# (c) Copyright 2019-2020 OLX
FROM rust:1.59.0-slim as build

WORKDIR /usr/src/dali

RUN apt-get update && apt install -y \
        libvips-dev \
        libvips \
        make

COPY . .

RUN cargo build --release

FROM debian:stable-20220228-slim

COPY --from=build /usr/src/dali/target/release/dali /usr/local/bin/dali

RUN apt-get update && apt install -y \
        libvips && \
    rm -rf /var/lib/apt/lists/*

CMD ["dali"]
