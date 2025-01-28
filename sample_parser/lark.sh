#!/bin/sh

mkdir -p tmp

case "$1" in
    *.gbnf )
        echo "Generating Lark grammar from GBNF $1 to tmp/gbnf.lark"
        ../scripts/gbnf_to_lark.py "$1" > tmp/gbnf.lark
        LARK=tmp/gbnf.lark
        ;;
    *.lark )
        LARK="$1"
        ;;
    test )
        set -xe
        ../scripts/gbnf_to_lark.py data/from-llama.cpp/*.gbnf
        cargo run --bin lark_test -- data/from-llama.cpp/*.lark
        exit $?
        ;;
    *)
        echo "Usage: $0 <lark_file> [args...]"
        exit 1
        ;;
esac

cargo run --bin lark_test -- $LARK
