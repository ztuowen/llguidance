use lazy_static::lazy_static;
use llguidance::{
    api::{GrammarWithLexer, TopLevelGrammar},
    earley::SlicedBiasComputer,
    toktrie::{InferenceCapabilities, TokEnv, TokenId},
    Constraint, ParserFactory,
};

/// Check that the grammar generates the expected output.
///
/// Output is a list of strings, each of which is a sequence of tokens.
/// Tokens in the string are separated with "‧".
/// Strings at even positions are "forced tokens", and strings at odd positions
/// are "generated tokens".
/// We check that the grammars forces the forced tokens (first of which is the
/// prompt), and that it allows in the mask the generated tokens.
///
/// These tests are "recorded" by passing "test_trace": true in the llguidance
/// request and post-processing.
fn check_grammar(
    factory: &ParserFactory,
    prompt_str: &str,
    grammar: TopLevelGrammar,
    output: &[&str],
    temp: f32,
) -> Constraint {
    let parser = factory.create_parser(grammar).unwrap();
    let mut constraint = Constraint::new(parser);

    let tok_env = factory.tok_env();

    let prompt = constraint.process_prompt(tok_env.tokenize(prompt_str));
    check_eq(tok_env, "prompt", &prompt, output[0]);

    let mut idx = 1;
    let mut gen_tokens = tokenize_trace(tok_env, output[idx]);
    let mut seen_temp = temp == 0.0;

    for _ in 0..200 {
        let res = constraint.compute_mask().unwrap().clone();

        if let Some(t) = res.temperature {
            assert!(
                t == temp || t == 0.0,
                "Expected temperature {} got {}",
                temp,
                t
            );
            if t == temp {
                seen_temp = true;
            }
        }

        if res.is_stop() {
            assert!(idx >= output.len() - 1, "Expected more output at {}", idx);
            assert!(gen_tokens.is_empty(), "Expected more tokens to generate");
            break;
        }

        let mut bt: u32;
        let mut toks: Vec<TokenId>;

        if let Some(mask) = &res.sample_mask {
            if gen_tokens.is_empty() {
                panic!("No more tokens to generate");
            }

            loop {
                let (is_allowed, tok) = gen_tokens[0];
                if is_allowed {
                    break;
                }
                assert!(
                    !mask.is_allowed(tok),
                    "Token {} {} should not be allowed",
                    tok,
                    tok_env.tok_trie().token_dbg(tok)
                );
                assert!(constraint.validate_tokens_raw(&[tok]).unwrap() == 0);
                gen_tokens.remove(0);
            }

            let (_, tok) = gen_tokens[0];
            assert!(
                mask.is_allowed(tok),
                "Token {} {} should be allowed",
                tok,
                tok_env.tok_trie().token_dbg(tok)
            );

            let rest_allowed = gen_tokens
                .iter()
                .filter(|(is_allowed, _)| *is_allowed)
                .map(|(_, tok)| *tok)
                .collect::<Vec<_>>();

            let num_ok = constraint.validate_tokens_raw(&rest_allowed).unwrap();
            if num_ok < rest_allowed.len() {
                panic!(
                    "Expected {} tokens to be allowed; got {}; {}",
                    rest_allowed.len(),
                    num_ok,
                    tok_env.tok_trie().tokens_dbg(&rest_allowed)
                );
            }
            gen_tokens.remove(0);

            let res = constraint.commit_token(Some(tok)).unwrap();
            bt = res.backtrack;
            toks = res.ff_tokens.clone();
            if toks.is_empty() || toks[0] != tok {
                if idx + 1 < output.len() && output[idx + 1].starts_with("1↶") {
                    // fast-forward with fake backtrack
                    assert!(bt == 0 || res.ff_tokens.is_empty());
                    bt = 1;
                    // go to forced byte checking
                } else {
                    if toks.is_empty() {
                        panic!("Expected {}; got nothing", tok);
                    } else {
                        panic!("Expected token {} got {}", tok, toks[0]);
                    }
                }
            } else if toks.len() > 1 {
                // we got fast-forwarded to the next entry,
                // delete the generated tokens and leave the rest for forced
                // bytes checking below
                toks.remove(0);
                // go to forced byte checking
            } else {
                assert!(bt == 0);
                assert!(toks.len() == 1);
                continue; // normal path
            }
        } else {
            let res = constraint.commit_token(None).unwrap();
            bt = res.backtrack;
            toks = res.ff_tokens.clone();
        }

        // forced byte checking
        assert!(gen_tokens.is_empty(), "Expected more tokens to generate");

        idx += 1;
        let mut expected = output[idx];
        if expected.contains("↶") {
            let parts: Vec<&str> = expected.split("↶").collect();
            assert!(parts.len() == 2);
            expected = parts[1];
            assert!(
                bt == parts[0].parse::<u32>().unwrap(),
                "Expected backtrack {} got {}",
                parts[0],
                bt
            );
        }
        check_eq(tok_env, &format!("step {}", idx), &toks, expected);
        idx += 1;
        if idx < output.len() {
            gen_tokens = tokenize_trace(tok_env, output[idx]);
        }
    }

    assert!(seen_temp, "Expected temperature {} not seen", temp);

    constraint
}

fn check_eq(tok_env: &TokEnv, label: &str, tokens: &[TokenId], expected_tokens: &str) {
    let trie = tok_env.tok_trie();
    let actual_tokens = trie.test_trace_tokens(tokens);
    println!(
        "Checking {}: exp:{:?} got:{:?}",
        label, expected_tokens, actual_tokens
    );
    assert_eq!(
        actual_tokens, expected_tokens,
        "Tokens mismatch in {}",
        label
    );
}

fn tokenize_trace(tok_env: &TokEnv, s: &str) -> Vec<(bool, TokenId)> {
    let trie = tok_env.tok_trie();
    println!("Tokenizing {:?}", s);
    let mut result = Vec::new();
    if s.is_empty() {
        return result;
    }

    // Split by both ‧ and × to catch all tokens
    let words = s.split(|c| c == '‧' || c == '✖').collect::<Vec<&str>>();
    let mut char_pos = 0;

    for word in words {
        if word.is_empty() {
            char_pos += 1;
            continue;
        }

        // Determine if this token started with ‧ (true) or × (false)
        let is_allowed = if char_pos > 0 {
            let char_before = s.chars().nth(char_pos - 1).unwrap_or('✖');
            char_before == '‧'
        } else {
            true // First token assumed to start with ‧
        };

        if word == "≺EOS≻" {
            result.push((is_allowed, trie.eos_token()));
        } else if let Some(t) = trie.get_special_token(word) {
            result.push((is_allowed, t));
        } else {
            let tt = trie.greedy_tokenize(word.as_bytes());
            assert!(
                tt.len() == 1,
                "Expected single token for {:?} got {}",
                word,
                trie.test_trace_tokens(&tt)
            );
            result.push((is_allowed, tt[0]));
        }

        char_pos += word.chars().count() + 1; // +1 for the separator
    }

    result
}

lazy_static! {
    static ref PARSER_FACTORY: ParserFactory = {
        let env = toktrie_hf_tokenizers::ByteTokenizerEnv::from_name("microsoft/Phi-3.5-mini-instruct", None)
            .unwrap()
            .to_env();
        let mut fact = ParserFactory::new(&env,
            InferenceCapabilities {
                ff_tokens: true, // can the engine append multiple tokens?
                backtrack: true, // can the engine remove generated tokens?
                conditional_ff_tokens: false, // not used
                fork: false,                  // not used
            }, &SlicedBiasComputer::general_slices()).unwrap();
        fact.set_stderr_log_level(2);
        fact.set_buffer_log_level(0);
        fact
    };
}

pub fn check_lark_grammar_prompt(lark: &str, prompt_str: &str, output: &[&str]) -> Constraint {
    let grm = TopLevelGrammar::from_lark(lark.to_string());
    println!("\nChecking grammar:\n{}\nagainst: {:?}", lark, output);
    let temp = find_temperature(lark);
    check_grammar(&PARSER_FACTORY, prompt_str, grm, output, temp)
}

pub fn check_lark_grammar(lark: &str, output: &[&str]) -> Constraint {
    check_lark_grammar_prompt(lark, "", output)
}

fn find_temperature(lark: &str) -> f32 {
    lark.find("temperature=")
        .map(|i| {
            let i = i + "temperature=".len();
            let mut end = i;
            while end < lark.len()
                && (lark.as_bytes()[end].is_ascii_digit() || lark.as_bytes()[end] == b'.')
            {
                end += 1;
            }
            lark[i..end].parse::<f32>().unwrap()
        })
        .unwrap_or(0.0)
}

pub fn check_lark_grammar_nested(lark: &str, sub_lark: &str, output: &[&str]) -> Constraint {
    let temp = find_temperature(lark);
    let mut top_grm = TopLevelGrammar::from_lark(lark.to_string());
    let mut sub_grm = GrammarWithLexer::from_lark(sub_lark.to_string());
    sub_grm.name = Some("sub".to_string());
    top_grm.grammars.push(sub_grm);
    println!(
        "\nChecking nested grammars:\n{}\nNested:\n{}\nagainst: {:?}",
        lark, sub_lark, output
    );
    check_grammar(&PARSER_FACTORY, "", top_grm, output, temp)
}

pub fn check_lark_json(lark: &str, json_schema: serde_json::Value, output: &[&str]) -> Constraint {
    let schema_str = serde_json::to_string_pretty(&json_schema).unwrap();
    let mut top_grm = TopLevelGrammar::from_lark(lark.to_string());
    let mut sub_grm = GrammarWithLexer::from_json_schema(json_schema);
    sub_grm.name = Some("sub".to_string());
    top_grm.grammars.push(sub_grm);
    println!(
        "\nChecking lark+json:\n{}\nNested:\n{}\nagainst: {:?}",
        lark, schema_str, output
    );
    check_grammar(&PARSER_FACTORY, "", top_grm, output, 0.0)
}

pub fn check_capture(c: &Constraint, name: &str, expected: &str) {
    if let Some(bytes) = c.parser.get_capture(name) {
        let actual = String::from_utf8_lossy(bytes);
        assert_eq!(actual, expected, "Capture {} mismatch", name);
    } else {
        panic!("Capture {} not found", name);
    }
}

pub fn print_tokenized(s: &str) {
    let trie = PARSER_FACTORY.tok_env().tok_trie();
    let tokens = PARSER_FACTORY.tok_env().tokenize(s);
    println!("{:?}", trie.test_trace_tokens(&tokens));
}
