#!/bin/sh

awk -F ' = ' '$1 ~ /version/ { printf("%s",$2) }' Cargo.toml