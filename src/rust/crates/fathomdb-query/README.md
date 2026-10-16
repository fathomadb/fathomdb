# fathomdb-query

The text-query compiler used by **FathomDB**, a local-first embedded retrieval engine built on
SQLite.

## Is this the crate you want?

Probably not, unless you are working *on* FathomDB rather than *with* it. Applications should depend
on [`fathomdb`](https://crates.io/crates/fathomdb) and call `Engine::search`, which uses this crate
internally.

It is published separately because it is a **leaf** — its one dependency is `sha2` — and because
the one thing it does is independently useful: turning arbitrary user text into an SQLite FTS5 `MATCH`
expression that is safe to interpolate.

## Status: pre-1.0, beta

The 0.8.x line is under active development. The public surface can change between minor releases.

## What it does

`compile_text_query` lowercases the input, splits on non-alphanumeric characters, drops a small
English stopword list, de-duplicates the rest in first-seen order, and OR-joins them as quoted FTS5
literals. Tokens of every length are kept, so "vitamin D3" searches for `d3`:

```rust
use fathomdb_query::compile_text_query;

let compiled = compile_text_query("status of the alpha");
assert_eq!(compiled.match_expression, "\"status\" OR \"alpha\"");
```

Two properties are the point:

- **Injection safety.** Splitting on non-alphanumeric characters removes every FTS5 control
  character (`*`, `"`, `:`, `^`, `(`, `)`, `,`) before quoting, so the emitted expression contains
  only literals.
- **OR, not AND.** Requiring every token to be present near-zeroes recall on natural-language
  questions. OR-ing the content tokens lets `bm25()` rank by overlap instead.

A query with no content tokens at all (all stopwords, all symbols, or all sub-three-character) falls
back to an OR over the raw whitespace tokens as quoted phrases, so it still searches rather than
matching nothing.

## Custom stop words

`QueryStopWords` describes a stop-word setting: a `base` list (`StopWordBase::Fathomdb`, the
built-in list, or `StopWordBase::None`), `extra` words to add and `keep` words to remove. Each word
must be one lowercase alphanumeric token. `QueryStopWords::validate` checks the setting and returns
the effective `StopWordSet`, or a `StopWordError` naming the field, the index and the reason.
`compile_text_query_with` compiles with that set; with the built-in set it returns exactly what
`compile_text_query` does:

```rust
use fathomdb_query::{compile_text_query_with, QueryStopWords};

let setting = QueryStopWords { keep: vec!["the".to_string()], ..QueryStopWords::default() };
let stop_words = setting.validate().expect("valid setting");
let compiled = compile_text_query_with("status of the alpha", &stop_words);
assert_eq!(compiled.match_expression, "\"status\" OR \"the\" OR \"alpha\"");
```

`StopWordSet::sha256` identifies a set: the SHA-256 of its words sorted and joined by newlines.
`DEFAULT_STOP_WORDS_SHA256` is that hash for the built-in list.

## Install

```bash
cargo add fathomdb-query
```

## License

MIT. See the `LICENSE` file shipped in this crate.

Source, issues and full documentation: <https://github.com/fathomadb/fathomdb>
