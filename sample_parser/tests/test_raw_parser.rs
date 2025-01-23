use lazy_static::lazy_static;
use llguidance::{
    api::TopLevelGrammar, earley::SlicedBiasComputer, toktrie::InferenceCapabilities,
    ParserFactory, TokenParser,
};
use sample_parser::get_tok_env;

lazy_static! {
    static ref PARSER_FACTORY: ParserFactory = {
        let env = get_tok_env();
        let mut fact = ParserFactory::new(
            &env,
            InferenceCapabilities {
                ff_tokens: false,
                backtrack: false,
                conditional_ff_tokens: false,
                fork: false,
            },
            &SlicedBiasComputer::general_slices(),
        )
        .unwrap();
        fact.set_stderr_log_level(2);
        fact.set_buffer_log_level(0);
        fact
    };
}

fn make_parser(lark: &str) -> TokenParser {
    let grm = TopLevelGrammar::from_lark(lark.to_string());
    let mut parser = PARSER_FACTORY.create_parser(grm).unwrap();
    parser.start_without_prompt();
    parser
}

fn consume(parser: &mut TokenParser, tok: u32) {
    let n = parser.consume_token(tok).unwrap();
    assert!(n == 0);
}

#[test]
fn test_ff_tokens() {
    let mut p = make_parser(
        r#"
            start: <[1111]> <[311]> ( <[366]> | "s" ) <[311]> <[1111]>
        "#,
    );
    let t = p.compute_ff_tokens();
    assert_eq!(t, vec![1111, 311]);
    let n = p.validate_tokens_raw(&t).unwrap();
    assert_eq!(n, 2);
    consume(&mut p, 1111);
    consume(&mut p, 311);

    let n = p.validate_tokens_raw(&vec![366, 311, 1111]).unwrap();
    assert_eq!(n, 3);

    let n = p.validate_tokens_raw(&vec![29879, 311, 1111]).unwrap();
    assert_eq!(n, 3);

    consume(&mut p, 29879);

    let t = p.compute_ff_tokens();
    assert_eq!(t, vec![311, 1111]);
    let n = p.validate_tokens_raw(&t).unwrap();
    assert_eq!(n, 2);
}
