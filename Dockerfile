# (c) Copyright 2019-2023 OLX
FROM rust:1.73.0-alpine3.17 as build

WORKDIR /usr/src/dali
RUN apk add --update --no-cache \
    --repository https://dl-cdn.alpinelinux.org/alpine/v3.17/community \
    --repository https://dl-cdn.alpinelinux.org/alpine/v3.17/main \
    musl-dev=1.2.3-r4 \
    vips-dev=8.13.3-r1

COPY . .

RUN RUSTFLAGS="-C target-feature=-crt-static $(pkg-config vips --libs)" cargo build --release

FROM alpine:3.18.4

RUN apk add --update --no-cache \
    --repository https://dl-cdn.alpinelinux.org/alpine/v3.17/community \
    --repository https://dl-cdn.alpinelinux.org/alpine/v3.17/main \
    vips=8.13.3-r1 \
    vips-heif=8.13.3-r1 \
    openssl

COPY --from=build /usr/src/dali/target/release/dali /usr/local/bin/dali

CMD ["dali"]