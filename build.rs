// (c) Copyright 2022-2023 OLX

fn main() {
    pkg_config::Config::new().probe("vips").unwrap();
    println!("cargo:rustc-env=RUSTFLAGS=-C target-feature=-crt-static");
    println!("cargo:rerun-if-changed=build.rs");
}
