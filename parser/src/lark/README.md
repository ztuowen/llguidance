# Lark-like syntax

LLGuidance supports a variant of syntax used by Python [Lark parsing toolkit](https://github.com/lark-parser/lark).
We also provide a [gbnf_to_lark.py script](../../../scripts/gbnf_to_lark.py) to convert from 
[GBNF](https://github.com/ggerganov/llama.cpp/blob/master/grammars/README.md) format used in
[llama.cpp](https://github.com/ggerganov/llama.cpp).
These makes it easier to get started with a new grammar,
and provide a familiar syntax, however neither is a drop-in replacement for Lark or GBNF.

For a general intro to Lark syntax, see:

- [How to write a DSL](https://blog.erezsh.com/how-to-write-a-dsl-in-python-with-lark/) blog post;
  ignore the part about syntax trees
- [Lark documentation](https://lark-parser.readthedocs.io/en/latest/);
  LLGuidance uses Earley parser, with an equivalent of `lexer='dynamic'` setting in Lark

## Extensions to Lark syntax

Following are the extensions to Lark syntax.

### Minor syntax changes

- `expr{M,N}` can be used instead of `expr~M..N`
  (with either `M` or `N` being optional; `expr{N}` is also supported)
- both `//` and `#` can be used for comments
- `-` is valid in identifiers

### Inline JSON Schemas

You can also inline JSON schema in the lark grammar, using `%json { ... }` syntax
(it behaves like a non-terminal).

For example, this defines a JSON function calling schema for Meta Llama 3.1,
according to [their website](https://www.llama.com/docs/model-cards-and-prompt-formats/llama3_1/#json-based-tool-calling):

```lark
start: TEXT | fun_call
TEXT: /[^{](.|\n)*/
fun_call: %json {
  "type": "object",
  "properties": {
    "name": { "const": "get_weather" },
    "parameters": {
      "type": "object",
      "properties": {
        "city": { "type": "string" }
      },
      "required": ["city"]
    }
  },
  "required": ["name", "parameters"]
}
```

This will accepting strings like `There is no function I can call`
and `{"name":"get_weather", "parameters": {"city":"Seattle"}}`.
If the string starts with `{`, it will be forced to follow the given JSON schema.

If you have more functions, you should use `%json { "anyOf": [ ... ] }`.
Do not use `fun_call1 | fun_call2`, as it [currently doesn't work](https://github.com/guidance-ai/llguidance/issues/113).

### Special tokens

Special tokens can referenced via `<token_name>` syntax (i.e., any string between `<` and `>`),
for example `<|ENDOFTEXT|>`.
They cannot be used inside of terminals, but can be used in regular rules.
The exact set of available tokens depends on the tokenizer used.

You can also use numeric token ids, as in `<[128010]>` (this is `<|python_tag|>` in Meta Llama tokenizer).
Tou can also use ranges like `<[128000-128255]>` for all Llama special tokens, or
even lists of ranges like `<[128000-128100,128130-128170]>`; ranges are inclusive.

For example, this is how to constrain JSON function calling for Meta Llama 3.1,
according to their [source repo](https://github.com/meta-llama/llama-models/blob/main/models/llama3_1/prompt_format.md#model-response-format-5) (and yes, it's [different](https://github.com/meta-llama/llama-models/issues/266) than the website).

```lark
start: TEXT | fun_call
TEXT: /[^{](.|\n)*/
fun_call: <|python_tag|> json_body <|eom_id|>
json_body: %json {
  # ... same as above ...
}
```

Here, we also insist on regular text responses not starting with `{` to avoid confusing the model,
but normally the JSON response should start with the special `<|python_tag|>` token.

Another example is function calling in Llama for Brave search or Wolfram Alpha:

```lark
start: normal_text | brave | wolfram
normal_text: /(.|\n)*/
brave: <|python_tag|> "brave_search.call(query=" JSON_STRING ")" <|eom_id|>
wolfram: <|python_tag|> "wolfram_alpha.call(query=" JSON_STRING ")" <|eom_id|>
JSON_CHAR: /(\\([\"\\\/bfnrt]|u[a-fA-F0-9]{4})|[^\"\\\x00-\x1F\x7F])/
JSON_STRING: "\"" JSON_CHAR* "\""
```

Note that is is [critical for performance](#terminals-vs-rules) for the `JSON_STRING` to be uppercase,
so that it is treated as a terminal (single lexeme, or regex and not a context-free grammar production).

BTW, in this case you may want to replace the JSON string definition
with a definition of a Python string, depending on how the model was trained.

### Lexeme options

These features are mostly for compatibility with [Guidance](https://github.com/guidance-ai/guidance).

`max_tokens`, `temperature` and `stop` can be specified on rules, but the rule body must be a token expression,
for example: `mygen[stop="\n", max_tokens=10, temperature=0.7]: /.*/`

If `stop` is specified (possibly as `""`) the rule is treated as `gen()` in Guidance
(the lexeme is lazy); otherwise it is treated as `lexeme()` (greedy).

### Grammar options

Certain grammar options can be set by using `%llguidnace { ... }`,
by passing it a JSON object with the options;
see `LLGuidanceOptions` in [api.rs](../api.rs#L24).
Example: `%llguidance { "no_forcing": true }`.
It can be specified multiple times, with the options being merged.

You can also start the grammar file with `%llguidance {}` to indicate
that llguidance should be used to process the grammar.

### Multiple grammars

The input to LLGuidance consists of a list of grammars. This can be accessed via
[LLGuidance API](../api.rs). Each of these can be a Lark grammar, a JSON schema,
or a grammar in the API format. With the introduction of `%json` in Lark syntax
there is less need now for using multiple grammars, but it is still supported.

Inside of Lark grammar, you can reference other grammars using syntax like `@17`
refering to grammar at index 17 in the `grammars` list, or `@my_grammar`,
refering to grammar with `"name": "my_grammar"`.
The top-level grammar is at index 0.

You can specify temperature for subgrammar by referencing it via
`my_temp_json[temperature=0.7]: @json` syntax.

Example:

```json
{
  "grammars": [
    {
      "lark_grammar": "start: /(.|\\n)*/ | fun\nfun: <|python_tag|> @fcall <|eom_id|>",
    },
    {"name": "fcall", "json_schema": { ... }}
  ]
}
```

### Unsupported Lark features

Following features of Lark syntax are currently not supported:

- lookarounds in lexer regexes
- lazy modifier (`?`) in lexer regexes; you [can use](#lexeme-options) `[stop=""]` to make the entire lexeme lazy
- priorities of terminals
- templates
- imports (other than built-in `%import common`)
- regexes use Rust `regex` crate [syntax](https://docs.rs/regex/latest/regex/#syntax), not Python's `re` (though they are similar)
- certain string syntax, see [issue](https://github.com/microsoft/llguidance/issues/54)

## Terminals vs rules

A Lark file defines a context-free grammar.
Another, more well-known, type of grammar is a regular grammar, which is used by regular expressions.
Regular grammars are more limited, but also faster to parse.
Therefore, typically programming languages are parsed in two stages:

- first, a lexer converts the input bytes into a stream of lexemes or terminals (aka tokens, but we avoid that term to avoid confusion with LLM tokens)
- the parser then uses a context-free grammar to parse the lexemes into higher-level structures

Thus, the lexer matches strings, while the parser matches the structure.

Most often, the programming languages allow some sort of white-space between lexemes,
which is used to separate them, but otherwise ignored.
For example, the strings `foo+bar` and `foo + bar` are typically equivalent
(lexed as `foo`, `+`, `bar`), while `fo o + bar` is not.
Similarly, `x+=12` is equivalent to `x += 12`, but not to `x + = 12` nor to `x += 1 2`.
Lexemes in these cases are the fragments between which the white-space is allowed.
Note that `"xy"` and `"x y"` are not equivalent - the whole string literal is a single lexeme.

You can use `%ignore` directive to specify which kind of white-space
(or other strings, including comments) to ignore.
For example, for JSON one would use `%ignore /[ \t\n\r]+/`.
By default nothing is ignored.

In Lark, string literals like `"while"` or `"if"` are lexemes,
and so are regular expressions like `/[0-9]+/`, enclosed in `/`...`/`.

Lark also allows using grammar operators like `|` or `*` outside of regular expressions or strings,
so you can also say `/[0-9]/+` or `/[0-9]/{1,}`.
The meaning of this depends on whether you're defining a terminals (uppercase) or a rule (lowercase).
Any uppercase symbol (like `INT: /[0-9]/+` or `NUMBER: INT | FLOAT`) is treated
as a terminal (lexeme), and in particular white-space is not allowed inside of it.
Any lowercase symbol (like `digits: /[0-9]/+`) is treated as a rule,
meaning `digits` will match `123` but also `1 2 3` or `1 23` (and treat them as the same).
**Putting a repeating operator like `+` or `*` on a single-character lexeme is typically not what you want and will slow down the parser, up to the point it will refuse to run.**

All the terminals (uppersase identifiers) are compiled into regular expressions,
meaning they can't be recursive (refer to themselves, directly or indirectly).

In practice, in the grammar you can use string literals like `"while"`
to refer to keyword, but when defining a lexeme with a regular expression,
it's best to name it in uppercase, like `IDENTIFIER: /[a-zA-Z_][a-zA-Z0-9_]*/`,
and then use `IDENTIFIER`.
Sometimes it can be useful to name the character classes (again, making sure everything is uppercase), as in:

```lark
ID_START: /[a-zA-Z_]/
ID_CHAR: /[a-zA-Z0-9_]/
IDENTIFIER: ID_START ID_CHAR*
```
