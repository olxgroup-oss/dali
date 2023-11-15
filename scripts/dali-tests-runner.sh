#!/bin/bash
# requires docker service to be running

features="${1:-hyper_client}"
binary="dali"
binary_extra=""
if [ "${features}" == "awc_client" ];then
    binary_extra="_awc"
fi

binary="${binary}${binary_extra}"

run_tests() {
    HTTP_HOST=localhost:9000 RUN_MODE=default cargo run --bin ${binary} --features "${features}" >> /dev/null &
    PID=$!
    wait_until_ready localhost 8080
    HTTP_HOST=localhost:9000 cargo test --bin ${binary} --features "${features}"
    RCODE=$?
    stop_process ${PID} localhost 8080
}

wait_until_ready() {
  # We want this to output $1 and $2 without expansion
  # shellcheck disable=SC2016
  timeout 30 sh -c 'until nc -z $0 $1; do sleep 1; done' "$1" "$2"
}

wait_until_notready() {
  # We want this to output $1 and $2 without expansion
  # shellcheck disable=SC2016
    timeout 30 sh -c 'until ! nc -z $0 $1; do sleep 1; done' "$1" "$2"
}

stop_process() {
    kill -SIGTERM "${1}"
    wait_until_notready "$2" "$3"
}

setup() {
    docker run --rm -v ./tests/resources/:/usr/share/nginx/html/ -p 9000:80 --name dali-http-nginx-source -d nginx:1.25.3-alpine-slim
    wait_until_ready localhost 9000
}

teardown() {
    docker stop dali-http-nginx-source
}

setup

run_tests
RCODE=$?

teardown

if [[ ${RCODE} -ne 0 ]]; then
    exit 1
fi
exit 0
