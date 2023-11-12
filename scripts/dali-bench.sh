#!/bin/bash
# requires docker service to be running

# Takes two parameters; or defaults are set for the benchmark filename to run
# defaults to the request_benchmarks
benchmark="${1:?"Please specify which benchmark file you want to test"}"
features="${2:-hyper_client}"

# We can run the server with  different features to test different server implementations
# current the is hyper_client, the default, and awc_client
# we need cargo-criterion crate installed so that we can compare the
# compare the server implementations

check_deps(){
  cargo criterion -h >/dev/null || {
    cat << EOT
    Please install the crate cargo-criterion by running:
    cargo install cargo-criterion
EOT
  } && { echo "cargo criterion dependency found."; }

  [ -f "benches/${1}.rs" ] || { echo "$1 not found"; exit 1;}

}

run_benchmark() {
    benchmark="${1}"
    features="${2:-hyper_client}"

    echo "Running benchmark $benchmark using the feature:$features"
    sleep 5

    HTTP_HOST=localhost:8080 RUN_MODE=default cargo run --features "${features}" >> /dev/null &
    PID=$!
    echo "Dali:[$features] is running on PID($PID)"

    wait_until_ready localhost 8080

    BENCH_HTTP_HOST="http://127.0.0.1:8080" BENCH_FILE_SERVER_HOST="http://127.0.0.1:9000" cargo bench --features "${features}" --bench "${benchmark}" -- --save-baseline "${features}"
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
    # docker: -p outside:inside
    docker run --rm -v ./tests/resources/:/usr/share/nginx/html/ -p 9000:80 --name dali-http-nginx-source -d nginx:1.23.3-alpine-slim
    wait_until_ready localhost 9000
}

teardown() {
    docker stop dali-http-nginx-source
}

setup

check_deps "${benchmark}"
run_benchmark "${benchmark}" "${features}"

RCODE=$?

teardown

if [[ ${RCODE} -ne 0 ]]; then
    exit 1
fi

cp -a "target/criterion/${benchmark}" reports/criterion/
open "target/criterion/${benchmark}/report/index.html"
