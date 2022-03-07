#!/bin/bash
cargo build --no-default-features --features "hyper_client"
./target/debug/dali >> /dev/null &
PID=$!
docker run --rm -v $(pwd)/tests/resources/:/usr/share/nginx/html/ -p 9000:80 --name dali-http-source -d nginx
cargo test --no-default-features --features "hyper_client"
RCODE=$?
kill ${PID}
wait ${PID}
cargo build --no-default-features --features "awc_client"
./target/debug/dali >> /dev/null &
PID=$!
cargo test --no-default-features --features "awc_client"
AWCRCODE=$?
docker stop dali-http-source
kill ${PID}
wait ${PID}
if [[ ${RCODE} -ne 0 || ${AWCRCODE} -ne 0 ]]; then
    exit 1
fi
exit 0