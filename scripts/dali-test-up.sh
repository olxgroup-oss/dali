#!/bin/bash

run_test_with_feature() {
    cargo run --no-default-features --features "${1}" >> /dev/null &
    PID=$!
    wait_until_ready localhost 8080
    cargo test --no-default-features --features "${1}"
    RCODE=$?
    stop_process ${PID} localhost 8080
}

wait_until_ready() {
    timeout 30 sh -c 'until nc -z $0 $1; do sleep 1; done' $1 $2
}

wait_until_notready() {
    timeout 30 sh -c 'until ! nc -z $0 $1; do sleep 1; done' $1 $2
}

stop_process() {
    kill -SIGTERM ${1}
    wait_until_notready $2 $3
}

setup() {
    docker run --rm -v $(pwd)/tests/resources/:/usr/share/nginx/html/ -p 9000:80 --name dali-http-source -d nginx
    wait_until_ready localhost 9000
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
