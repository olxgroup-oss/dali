#!/bin/bash
cargo build
./target/debug/dali >> /dev/null &
PID=$!
docker run --rm -v $(pwd)/tests/resources/:/usr/share/nginx/html/ -p 9000:80 --name dali-http-source -d nginx
cargo test
RCODE=$?
docker stop dali-http-source
kill $PID
wait $PID
exit $RCODE