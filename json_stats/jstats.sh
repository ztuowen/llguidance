#!/bin/sh

if [ -z "$1" ]; then
    for folder in ../.. .. ../../tmp ; do
        if test -d $folder/jsonschemabench/maskbench/data; then
            DEFAULT_ARGS=$folder/jsonschemabench/maskbench/data/
            break
        fi
    done
else
    DEFAULT_ARGS=
fi

if [ -z "$PERF" ]; then
    cargo run --release -- $DEFAULT_ARGS "$@"
else
    PERF='perf record -F 9999 -g'
    RUSTFLAGS='-C force-frame-pointers=y' cargo build --profile perf
    $PERF ../target/perf/json_stats $DEFAULT_ARGS "$@"
    echo "perf report -g graph,0.05,caller"
    echo "perf report -g graph,0.05,callee"
fi
