#!/bin/sh

case "$1" in
    *.gbnf )
        mkdir -p tmp
        echo "Generating Lark grammar from GBNF $1 to tmp/gbnf.lark"
        ../scripts/gbnf_to_lark.py "$1" > tmp/gbnf.lark
        LARK=tmp/gbnf.lark
        ;;
    *.lark )
        LARK="$1"
        ;;
    test )
        ../scripts/gbnf_to_lark.py tmp/grammars/*.gbnf
        cargo run --bin lark_test -- tmp/grammars/*.lark
        exit $?
        ;;
    *)
        echo "Usage: $0 <lark_file> [args...]"
        exit 1
        ;;
esac

cargo run --bin lark_test -- $LARK

