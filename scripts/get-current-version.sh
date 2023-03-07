#!/bin/sh

awk -F ' = ' '$1 ~ /version/ { gsub(/[\"]/, "", $2); printf("%s",$2) }' Cargo.toml