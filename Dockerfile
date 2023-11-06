# (c) Copyright 2019-2023 OLX

# Rust versions for the vips packages can be found here...
# https://pkgs.alpinelinux.org/packages?name=vips*&branch=v3.18&repo=&arch=x86_64&maintainer=
# https://pkgs.alpinelinux.org/packages?name=musl*&branch=v3.18&repo=&arch=x86_64&maintainer=

FROM rust:1.73.0-alpine3.18 AS build

WORKDIR /usr/src/dali
RUN apk add --update --no-cache \
    --repository https://dl-cdn.alpinelinux.org/alpine/v3.18/community \
    --repository https://dl-cdn.alpinelinux.org/alpine/v3.18/main \
    musl-dev=1.2.4-r2 \
    vips-dev=8.14.3-r0

COPY . .

RUN RUSTFLAGS="-C target-feature=-crt-static $(pkg-config vips --libs)" cargo build --release

FROM alpine:3.18.4

RUN apk add --update --no-cache \
    --repository https://dl-cdn.alpinelinux.org/alpine/v3.18/community \
    --repository https://dl-cdn.alpinelinux.org/alpine/v3.18/main \
    vips=8.14.3-r0 \
    vips-heif=8.14.3-r0 \
    openssl

COPY --from=build /usr/src/dali/target/release/dali /usr/local/bin/dali

USER nobody

CMD ["dali"]
