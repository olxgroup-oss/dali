#!/bin/bash

run_test_with_feature() {
    cargo build --no-default-features --features "${1}"
    ./target/debug/dali >> /dev/null &
    PID=$!
    cargo test --no-default-features --features "${1}"
    RCODE=$?
    stop_process ${PID}
}

stop_process() {
    kill ${1}
    wait ${1}
}

setup() {
    docker run --rm -v $(pwd)/tests/resources/:/usr/share/nginx/html/ -p 9000:80 --name dali-http-source -d nginx
}

teardown() {
    docker stop dali-http-source
}

setup

run_test_with_feature "hyper_client"
RCODE=$?

run_test_with_feature "awc_client"
AWCRCODE=$?

teardown

if [[ ${RCODE} -ne 0 || ${AWCRCODE} -ne 0 ]]; then
    exit 1
fi
exit 0