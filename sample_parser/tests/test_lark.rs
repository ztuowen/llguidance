use anyhow::Result;
use llguidance::{api::TopLevelGrammar, TokenParser};
use sample_parser::*;

fn make_parser(lark: &str) -> Result<TokenParser> {
    let grm = TopLevelGrammar::from_lark(lark.to_string());
    let mut parser = get_parser_factory().create_parser(grm)?;
    parser.start_without_prompt();
    Ok(parser)
}

fn consume(parser: &mut TokenParser, tok: u32) {
    let n = parser.consume_token(tok).unwrap();
    assert!(n == 0);
}

fn lark_ok(lark: &str) {
    match make_parser(lark) {
        Err(e) => panic!("unexpected error: {}, grm:\n{}", e, lark),
        Ok(_) => {}
    }
}

fn lark_err_test(lark: &str, err: &str) {
    match make_parser(lark) {
        Err(e) => {
            let e = format!("{}", e);
            if !e.contains(err) {
                panic!(
                    "unexpected error: {}, expecting {:?}; grm:\n{}",
                    e, err, lark
                );
            }
        }
        Ok(_) => panic!("expected error: {}; grm:\n{}", err, lark),
    }
}

fn lark_str_test(lark: &str, should_accept: bool, s: &str) {
    let trie = get_tok_env().tok_trie();
    let tokens = get_tok_env().tokenize(s);
    println!(
        "\n\ntokens: {}, accpt={}\ngrm:\n{}\n",
        trie.tokens_dbg(&tokens),
        should_accept,
        lark
    );

    let mut p = make_parser(lark).unwrap();

    for tok in tokens.iter() {
        let m = p.compute_mask().unwrap();
        if m.is_allowed(*tok) {
            consume(&mut p, *tok);
        } else {
            if should_accept {
                panic!("unexpected token: {}", trie.token_dbg(*tok));
            }
            return;
        }
    }
    if p.is_accepting() {
        if !should_accept {
            panic!("unexpected accept");
        }
    } else {
        if should_accept {
            panic!("unexpected reject");
        }
    }
}

fn lark_str_test_many(lark: &str, passing: &[&str], failing: &[&str]) {
    for s in passing {
        lark_str_test(lark, true, s);
    }
    for s in failing {
        lark_str_test(lark, false, s);
    }
}

#[test]
fn test_dot_unicode() {
    lark_str_test_many(
        r#"start: /.../ "abc" /.../"#,
        &[
            "abcabcabc",
            "aaaabcccc",
            // NOTE: Also ensures that multi-byte characters still count as a single character
            "🔵🟠✅abc❌🟠🔵",
        ],
        &[
            "aaabcccc",
            "aaaaabcccc",
            "aaaabccc",
            "aaaabccccc",
            "🔵🟠✅❌abc❌✅🟠🔵",
            "🔵🟠abc🟠🔵",
        ],
    );
}

#[test]
fn test_lark_syntax_general() {
    lark_err_test(r#"root: "abc" "def""#, "no start");

    lark_err_test(
        r#"
            start: foo{7,6}
            foo: "a" | "b"
        "#,
        "range end must be >= start",
    );
    lark_err_test(
        r#"
            start: foo{-1,}
            foo: "a" | "b"
        "#,
        "range start must be >= 0",
    );
    lark_err_test(
        r#"
            start: foo{0,-1}
            foo: "a" | "b"
        "#,
        "range end must be >= start",
    );

    lark_err_test(
        r#"
            start: FOO
            FOO: ("a" | "b"){7,6}
        "#,
        "range end must be >= start",
    );
    lark_err_test(
        r#"
            start: FOO
            FOO: ("a" | "b"){-1,}
        "#,
        "range start must be >= 0",
    );
    lark_err_test(
        r#"
            start: FOO
            FOO: ("a" | "b"){0,-1}
        "#,
        "range end must be >= start",
    );

    lark_err_test(
        r#"
            start: FOO
            FOO: "a" | BAR
            BAR: "b" FOO
        "#,
        "circular reference in token",
    );

    lark_ok(
        r#"
            start: foo
            foo: "a" | bar
            bar: "b" foo
        "#,
    );

    lark_err_test(
        r#"
            start: FOO
            BAR: "b"
        "#,
        "unknown name",
    );

    lark_err_test(
        r#"
            start: foo
            bar: "b"
        "#,
        "unknown name",
    );

    lark_err_test(
        r#"
            start: BAR
            BAR: BAZ "a"
        "#,
        r#"token "BAZ" not found"#,
    );

    lark_ok(
        r#"
            %import common.INT
            start: INT
        "#,
    );
    lark_err_test(
        r#"
            %import common.BLAH
            start: BLAH
        "#,
        "Unknown common",
    );

    lark_err_test(r#" start: /[abc/ "#, "invalid regex");
    lark_ok(r#" start: /[abc]/ "#);
    lark_err_test(r#" start: /[abc]/l "#, "l-flag is not supported");

    lark_err_test(
        r#"
            start: FOO
            FOO: @1
        "#,
        "cannot be used in terminals",
    );
    lark_err_test(
        r#"
            start: FOO
            FOO: %json { }
        "#,
        "cannot be used in terminals",
    );
    lark_err_test(
        r#"
            start: FOO
            FOO: <[1234]>
        "#,
        "cannot be used in terminals",
    );
    lark_err_test(
        r#"
            start: FOO
            FOO: <|assistant|>
        "#,
        "cannot be used in terminals",
    );
    lark_err_test(
        r#"
            start: "A" | <|foobarbaz|>
        "#,
        "unknown special token",
    );

    lark_err_test(
        r#" start: "ab".."c" "#,
        "range start must be a single character",
    );
    lark_err_test(
        r#" start: "a".."cd" "#,
        "range end must be a single character",
    );
    lark_err_test(r#"  start: "d".."a" "#, "invalid range order");

    lark_err_test(r#"start: <[100-200-300]>"#, "invalid token range");
    lark_ok(r#"start: <[100-200,300-4002]>"#);
    lark_err_test(r#"start: <[100-200,100-200-300]>"#, "invalid token range");
    lark_err_test(r#"start: <[,]>"#, "empty token range");
    lark_err_test(r#"start: <[200-100]>"#, "invalid token range");
    lark_err_test(r#"start: <[200 - 100]>"#, "lexer error");

    lark_err_test(
        r#"
            start: foo
            foo: "a" | "b"
            foo: "c"
        "#,
        "duplicate rule",
    );
    lark_err_test(
        r#"
            start: FOO
            FOO: "a" | "b"
            FOO: "c"
        "#,
        "duplicate token",
    );
}

#[test]
fn test_lark_syntax_perc() {
    lark_err_test(r#"start: %json {"#, "EOF while parsing an object");
    lark_err_test(r#"start: %json { foo"#, "key must be a string");
    lark_err_test(r#"start: %json []"#, "failed to compile JSON schema");
    lark_err_test(
        r#"start: %json { "if": {} }"#,
        "failed to compile JSON schema",
    );

    lark_err_test(
        r#"
            %llguidance { "no_forcing": "yadda-dada"}
            start: "a" | "b"
        "#,
        "failed to parse %llguidance declaration",
    );

    lark_ok(r#" start: %lexeme { "substring_words": "foo bar" } "#);
    lark_ok(r#" start: %lexeme { "substring_chars": "foo bar" } "#);
    lark_ok(r#" start: %lexeme { "substring_chunks": ["foo", "bar"] } "#);

    lark_err_test(
        r#" start: %lexeme { "substring_words": true } "#,
        "failed to parse %lexeme declaration",
    );

    lark_err_test(r#" start: %lexeme { "foobar": true } "#, "unknown field");

    lark_err_test(
        r#" start: %lexeme { "substring_words": "aa", "substring_chars": "bb" } "#,
        "only one field can be set on %lexeme declaration",
    );

    lark_err_test(
        r#" start: %lexeme {  } "#,
        "no fields set on %lexeme declaration",
    );
}

#[test]
fn test_lark_syntax_attributes() {
    lark_ok(
        r#" start: foo
            foo[lazy]: /.*/ "#,
    );

    lark_ok(
        r#" start: foo
            foo[lazy,max_tokens=12]: /.*/ "#,
    );

    lark_ok(
        r#" start: foo
            foo[capture,lazy]: /.*/ "#,
    );

    lark_ok(
        r#" start: foo
            foo[capture , lazy]: /.*/ "#,
    );

    lark_ok(
        r#" start: foo
            foo[stop = "foobar"]: /.*/ "#,
    );

    lark_err_test(
        r#" start: foo
            foo[foobar=12]: /.*/ "#,
        "Unknown attribute",
    );

    lark_err_test(
        r#" start: foo
            foo[lazy="foo"]: /.*/ "#,
        "Expected token",
    );

    lark_err_test(
        r#" start: foo
            foo[max_tokens="foo"]: /.*/ "#,
        "Expected token",
    );
}

#[test]
fn test_repeat() {
    lark_str_test_many(
        r#"start:  ab{3,5}
           ab:  "a" | "b"
        "#,
        &["aba", "abaa", "aaaaa", "aabaa"],
        &["aa", "ab", "aaaaaa"],
    );

    lark_str_test_many(
        r#"start:  ab{3,}
           ab:  "a" | "b"
        "#,
        &["aba", "abaa", "aaaaa", "aabaa", "aaaaaa"],
        &["aa", "ab"],
    );

    lark_str_test_many(
        r#"start:  ab{,5}
           ab:  "a" | "b"
        "#,
        &["", "aa", "b", "aba", "abaa", "aaaaa", "aabaa"],
        &["aaaaaa"],
    );
}

#[test]
fn test_lexeme_substring_general() {
    lark_str_test_many(
        r#" start: "A" %lexeme { "substring_words": "foo bar baz" } "B" "#,
        &[
            "AfooB",
            "Abar bazB",
            "AbazB",
            "Afoo bar bazB",
            "Afoo bar B",
            "A bar bazB",
            "AB",
        ],
        &["Afoo bar baz", "AfoB"],
    );

    lark_str_test_many(
        r#" start: "A" %lexeme { "substring_chunks": ["foo", " bar", " baz"] } "B" "#,
        &[
            "AfooB",
            "A bar bazB",
            "A bazB",
            "Afoo bar bazB",
            "Afoo barB",
            "AB",
            "A bar bazB",
        ],
        &["Afoo bar baz", "AfoB"],
    );
}

#[test]
fn test_lexeme_substring_chars_ascii() {
    lark_str_test_many(
        r#"start: %lexeme { "substring_chars": "The quick brown fox jumps over the lazy dog." }"#,
        &[
            "The quick brown fox jumps over the lazy dog.",
            "The quick brown fox",
            "he quick brow",
            "fox jump",
            "dog.",
        ],
        &["brown fx"],
    );
}

#[test]
fn test_lexeme_substring_chars_unicode() {
    lark_str_test_many(
        r#"start: %lexeme { "substring_chars": "빠른 갈색 여우가 게으른 개를 뛰어넘었다." }"#,
        &[
            "빠른 갈색 여우가 게으른 개를 뛰어넘었다.",
            "빠른 갈색 여우가 게으른",
            "른 갈색 여우",
            "여우가 게으",
            "뛰어넘었다.",
        ],
        &["갈색 여가"],
    );
}

#[test]
fn test_lexeme_substring_words_ascii() {
    lark_str_test_many(
        r#"start: %lexeme { "substring_words": "The quick brown fox jumps over the lazy dog." }"#,
        &[
            "The quick brown fox jumps over the lazy dog.",
            "The quick brown fox",
            "dog.",
        ],
        &["he quick brow", "fox jump", "brown fx"],
    );
}

#[test]
fn test_lexeme_substring_words_unicode() {
    lark_str_test_many(
        r#"start: %lexeme { "substring_words": "빠른 갈색 여우가 게으른 개를 뛰어넘었다." }"#,
        &[
            "빠른 갈색 여우가 게으른 개를 뛰어넘었다.",
            "빠른 갈색 여우가 게으른",
            "뛰어넘었다.",
        ],
        &["른 갈색 여우", "여우가 게으", "갈색 여가"],
    );
}
