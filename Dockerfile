# (c) Copyright 2019-2020 OLX
FROM docker.pkg.github.com/olxgroup-oss/dali/base-rust-image:latest as build

WORKDIR /usr/src/dali

COPY . .

# this flag ensures that proc macro can be compiled in musl targets
RUN RUSTFLAGS="-C target-feature=-crt-static $(pkg-config vips --libs)" cargo install --jobs 2 --path .

FROM docker.pkg.github.com/olxgroup-oss/dali/base-dali:latest

COPY --from=build /usr/local/cargo/bin/dali /usr/local/bin/dali

CMD dali
