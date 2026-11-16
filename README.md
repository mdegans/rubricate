# rubricate

Syntax highlighting by rule, not by parsing.

To rubricate a manuscript was to mark its headings and key words in red so a
reader could find their way: by rule, by hand, with no attempt to understand
the text. This crate does that for code blocks. It splits a source string into
tokens, each with a kind from a small closed set (keyword, string, comment,
number, …), and leaves the drawing to you.

```rust
use rubricate::{Language, tokens};

let source = "plain text, for now";
if let Some(language) = Language::from_name("text") {
    for token in tokens(language, source) {
        // token.kind: map it to a class of your own
        // token.text: a slice of `source`
        print!("{}", token.text);
    }
}
```

## What it promises

- **The input comes back.** The tokens' texts, joined in order, are the source,
  byte for byte. No token is empty; no two neighbours share a kind.
- **Linear time, on any input.** Every scanner reads forward through a cursor
  that counts what it reads, and the tests check the count, not a clock. It is
  meant for untrusted text on a public server.
- **It cannot fail.** No error type. What a scanner does not understand is
  plain text; an unterminated string runs to the end.
- **No dependencies, no `unsafe`, no allocation, `no_std`.** The same code runs
  natively and as WebAssembly, so a server and a browser draw the same thing.

## What it does not

It is not a parser. It does not know whether code is valid or follow scopes,
and where a language cannot be read without parsing it guesses the common
case. If you need exact highlighting, use a tree-sitter or TextMate-grammar
highlighter. If you need something small that is safe to run on whatever
people paste, this is for that.

## Languages

`0.1.0` has one, `text`, which is all plain: the promises above are in place
and tested before any rules are. Planned, most-pasted first: JSON, Python,
Rust, SQL, diff, shell, TOML and YAML, CSV, XML and HTML, Markdown.

## Using the kinds

`Kind` is a closed enum, so your table from kind to style can be a `match`
with no fallback, and a class name never comes from the input:

```rust
use rubricate::Kind;

fn class(kind: Kind) -> &'static str {
    match kind {
        Kind::Plain => "",
        Kind::Keyword => "tok-keyword",
        Kind::String => "tok-string",
        Kind::Comment => "tok-comment",
        Kind::Number => "tok-number",
        Kind::Function => "tok-function",
        Kind::Type => "tok-type",
        Kind::Attribute => "tok-attribute",
        Kind::Operator => "tok-operator",
        Kind::Tag => "tok-tag",
        Kind::Markup => "tok-markup",
        Kind::Inserted => "tok-inserted",
        Kind::Deleted => "tok-deleted",
    }
}
```

## Contributing

Every change is reviewed by someone other than its author before it merges.
A new language comes with real code as fixtures and passes the two tests every
language shares: the input comes back, and the work is linear.

## Licence

MIT.
