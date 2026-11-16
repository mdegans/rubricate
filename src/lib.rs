//! Syntax highlighting by rule, not by parsing.
//!
//! To rubricate a manuscript was to mark its headings and key words in red
//! so a reader could find their way: by rule, by hand, with no attempt to
//! understand the text. This crate does that for code blocks. It splits a
//! source string into [`Token`]s, each with a [`Kind`] from a small closed
//! set, and leaves the drawing to the caller.
//!
//! ```
//! use rubricate::{Kind, Language, tokens};
//!
//! let source = r#"{"answer": 42}"#;
//! let kinds: Vec<(Kind, &str)> =
//!     tokens(Language::Json, source).map(|token| (token.kind, token.text)).collect();
//! assert_eq!(
//!     kinds,
//!     [
//!         (Kind::Plain, "{"),
//!         (Kind::Attribute, r#""answer""#),
//!         (Kind::Plain, ": "),
//!         (Kind::Number, "42"),
//!         (Kind::Plain, "}"),
//!     ]
//! );
//! // Map each kind to a class of your own; never take one from the input.
//! // Joined in order, the texts are the source.
//! let joined: String = kinds.iter().map(|(_, text)| *text).collect();
//! assert_eq!(joined, source);
//! ```
//!
//! # What it promises
//!
//! - **The input comes back.** The tokens' texts, joined in order, are the
//!   source, byte for byte. No token is empty, and no two neighbours share a
//!   kind.
//! - **Linear time.** Each scanner reads forward through a cursor that
//!   counts what it reads; the work is bounded by a constant times the
//!   source's length, on any input, and the tests check the count. It is
//!   meant for untrusted text on a public server.
//! - **It cannot fail.** There is no error to handle. Text a scanner does
//!   not understand is [`Kind::Plain`]; an unterminated string runs to the
//!   end of the source.
//! - **No dependencies, no `unsafe`, no allocation, `no_std`.** The same
//!   code runs on a server and in a browser, so both draw the same thing.
//!
//! # What it does not
//!
//! It is not a parser. It does not know whether code is valid, it does not
//! follow scopes, and where a language is ambiguous without parsing it
//! guesses the common case. It covers the languages people paste into posts
//! and comments, not every language there is.
//!
//! # Languages
//!
//! [`Language::Json`] (JSON, JSON Lines, JSON with comments) and
//! [`Language::Text`], which is all [`Kind::Plain`]. More come one at a
//! time. [`Language::from_name`] is how a caller with a code fence's info
//! string asks for one; a name it does not know is `None`, and the caller
//! draws the block plain.
#![no_std]
#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod cursor;
mod json;

use cursor::Cursor;

/// What a token is, for the caller to turn into a colour.
///
/// The set is closed on purpose: a caller's table from kind to class can be
/// a `match` with no fallback, so a class can never come from the input. A
/// new kind is a breaking change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Kind {
    /// Anything no rule claimed, whitespace and punctuation included
    Plain,
    /// A word the language reserves (`fn`, `SELECT`, `true`)
    Keyword,
    /// A string or character literal, quotes included
    String,
    /// A comment, markers included
    Comment,
    /// A number
    Number,
    /// A name being called or defined as a function
    Function,
    /// A type's name
    Type,
    /// A key, an attribute's name, a decorator
    Attribute,
    /// An operator
    Operator,
    /// A markup tag's name (`<div`)
    Tag,
    /// Emphasis, a heading, a link in a markup language
    Markup,
    /// A line a diff adds
    Inserted,
    /// A line a diff removes
    Deleted,
}

impl Kind {
    /// Every kind, in declaration order
    pub const ALL: [Kind; 13] = [
        Kind::Plain,
        Kind::Keyword,
        Kind::String,
        Kind::Comment,
        Kind::Number,
        Kind::Function,
        Kind::Type,
        Kind::Attribute,
        Kind::Operator,
        Kind::Tag,
        Kind::Markup,
        Kind::Inserted,
        Kind::Deleted,
    ];

    /// A short lowercase name (`"keyword"`), stable across releases: fit
    /// for a CSS class's suffix
    pub const fn name(self) -> &'static str {
        match self {
            Kind::Plain => "plain",
            Kind::Keyword => "keyword",
            Kind::String => "string",
            Kind::Comment => "comment",
            Kind::Number => "number",
            Kind::Function => "function",
            Kind::Type => "type",
            Kind::Attribute => "attribute",
            Kind::Operator => "operator",
            Kind::Tag => "tag",
            Kind::Markup => "markup",
            Kind::Inserted => "inserted",
            Kind::Deleted => "deleted",
        }
    }
}

/// A language this crate has rules for.
///
/// Unlike [`Kind`], this list grows: it is `non_exhaustive`, so a new
/// language is a minor release and a caller's `match` keeps a fallback.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Language {
    /// Plain text: one [`Kind::Plain`] token
    Text,
    /// JSON, JSON Lines, and JSON with comments: keys told from values
    Json,
}

impl Language {
    /// Every language, in declaration order
    pub const ALL: [Language; 2] = [Language::Text, Language::Json];

    /// The language a code fence's info string names (`"text"`, `"TXT"`),
    /// ASCII case ignored. `None` for a name this crate has no rules for:
    /// draw the block plain
    pub fn from_name(name: &str) -> Option<Self> {
        Language::ALL.into_iter().find(|language| {
            language
                .names()
                .iter()
                .any(|known| known.eq_ignore_ascii_case(name))
        })
    }

    /// The canonical name (`"text"`)
    pub const fn name(self) -> &'static str {
        self.names()[0]
    }

    /// Every name [`from_name`](Self::from_name) accepts, the canonical one
    /// first
    pub const fn names(self) -> &'static [&'static str] {
        match self {
            Language::Text => &["text", "txt", "plain", "plaintext"],
            Language::Json => &["json", "jsonc", "json5", "jsonl", "ndjson"],
        }
    }
}

/// A piece of the source and what it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Token<'a> {
    /// What it is
    pub kind: Kind,
    /// Its text, a slice of the source: never empty
    pub text: &'a str,
}

/// Split `source` into tokens by `language`'s rules.
///
/// The tokens' texts joined in order are `source`; none is empty; no two
/// neighbours share a kind. Lazy: nothing is scanned until the iterator is
/// asked.
pub fn tokens(language: Language, source: &str) -> Tokens<'_> {
    Tokens {
        language,
        source,
        cursor: Cursor::new(source),
        ahead: None,
    }
}

/// The iterator [`tokens`] returns.
#[derive(Debug, Clone)]
pub struct Tokens<'a> {
    language: Language,
    source: &'a str,
    cursor: Cursor<'a>,
    /// A piece already scanned that belongs to the next token
    ahead: Option<(Kind, usize, usize)>,
}

impl<'a> Tokens<'a> {
    /// Scan one piece: its kind and its byte range. `None` at the end.
    ///
    /// A scanner must consume at least one character. If one ever did not,
    /// this takes a character as plain text instead of looping: the crate's
    /// promises do not rest on every scanner being right.
    fn piece(&mut self) -> Option<(Kind, usize, usize)> {
        if self.cursor.is_done() {
            return None;
        }
        let start = self.cursor.pos();
        let mut kind = scan(self.language, &mut self.cursor);
        if self.cursor.pos() == start {
            debug_assert!(false, "a scanner consumed nothing at byte {start}");
            self.cursor.bump();
            kind = Kind::Plain;
        }
        Some((kind, start, self.cursor.pos()))
    }
}

impl<'a> Iterator for Tokens<'a> {
    type Item = Token<'a>;

    fn next(&mut self) -> Option<Token<'a>> {
        let (kind, start, mut end) = self.ahead.take().or_else(|| self.piece())?;
        // Neighbouring pieces of one kind are one token
        while let Some(next) = self.piece() {
            if next.0 == kind {
                end = next.2;
            } else {
                self.ahead = Some(next);
                break;
            }
        }
        Some(Token {
            kind,
            text: &self.source[start..end],
        })
    }
}

impl core::iter::FusedIterator for Tokens<'_> {}

/// One piece of `language` from the cursor: consume at least one character
/// and say what it was.
fn scan(language: Language, cursor: &mut Cursor<'_>) -> Kind {
    match language {
        Language::Text => {
            cursor.finish();
            Kind::Plain
        }
        Language::Json => json::scan(cursor),
    }
}

#[cfg(test)]
mod tests {
    extern crate std;

    use std::string::String;
    use std::vec::Vec;

    use super::*;

    /// Awkward inputs every language is run over
    const SAMPLES: &[&str] = &[
        "",
        " ",
        "\n",
        "\r\n",
        "a",
        "plain text",
        "tabs\tand\ttabs",
        "line one\nline two\r\nline three\n",
        "\"unterminated",
        "'c' \"s\" `t` /* c */ // d\n# e",
        "\\\\\\\"\\'",
        "é ü 日本語 🙂 \u{200b} \u{202e}rtl",
        "<a href=\"x\">&amp;</a>",
        "{\"key\": [1, 2.5e3, true, null]}",
        "+ added\n- removed\n@@ -1 +1 @@",
    ];

    /// A small deterministic generator: no dev-dependency for random text
    struct Rng(u64);

    impl Rng {
        fn next(&mut self) -> u64 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            self.0
        }

        fn text(&mut self, len: usize) -> String {
            const ALPHABET: &[&str] = &[
                "a", "Z", "0", "9", " ", "\t", "\n", "\r\n", "\"", "'", "`", "\\", "/", "*", "#",
                "-", "+", "<", ">", "=", "{", "}", "[", "]", "(", ")", ":", ",", ".", "_", "@",
                "é", "日", "🙂", "\u{200b}", "fn", "def", "SELECT", "true", "null", "//", "/*",
                "*/", "\"\"\"", "<!--", "-->",
            ];
            let mut out = String::new();
            for _ in 0..len {
                out.push_str(ALPHABET[(self.next() % ALPHABET.len() as u64) as usize]);
            }
            out
        }
    }

    /// What every language must hold for `source`
    fn holds(language: Language, source: &str) {
        let all: Vec<Token<'_>> = tokens(language, source).collect();
        let joined: String = all.iter().map(|token| token.text).collect();
        assert_eq!(joined, source, "{language:?}: the input comes back");
        assert!(
            all.iter().all(|token| !token.text.is_empty()),
            "{language:?}: no token is empty"
        );
        assert!(
            all.windows(2).all(|pair| pair[0].kind != pair[1].kind),
            "{language:?}: no two neighbours share a kind"
        );
    }

    /// The work a language does on `source`, in cursor steps
    fn steps(language: Language, source: &str) -> usize {
        let mut all = tokens(language, source);
        all.by_ref().for_each(drop);
        all.cursor.steps()
    }

    #[test]
    fn the_input_comes_back_for_every_language() {
        let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
        for language in Language::ALL {
            for source in SAMPLES {
                holds(language, source);
            }
            for round in 0..500 {
                holds(language, &rng.text(round % 97));
            }
        }
    }

    /// The bound every language is held to: a constant number of steps for
    /// each character, whatever the input. Counted, not timed, so the test
    /// says the same thing on a slow machine
    #[test]
    fn the_work_is_linear_for_every_language() {
        const PER_CHAR: usize = 8;
        let hostile: &[&str] = &[
            "\"", "'", "`", "\\", "/*", "<!--", "\"\"\"", "\n", " ", "#", "((((", "\"\n", "a\\\"",
            "{[", "*/", "//",
        ];
        let mut rng = Rng(42);
        for language in Language::ALL {
            for unit in hostile {
                for repeats in [1, 10, 1_000, 20_000] {
                    let source = unit.repeat(repeats);
                    let chars = source.chars().count();
                    let work = steps(language, &source);
                    assert!(
                        work <= PER_CHAR * chars + 16,
                        "{language:?}: {work} steps for {chars} characters of {unit:?}"
                    );
                }
            }
            let source = rng.text(20_000);
            let chars = source.chars().count();
            assert!(
                steps(language, &source) <= PER_CHAR * chars + 16,
                "{language:?}: random text"
            );
        }
    }

    #[test]
    fn text_is_one_plain_token() {
        let all: Vec<Token<'_>> = tokens(Language::Text, "fn main() {}").collect();
        assert_eq!(
            all,
            [Token {
                kind: Kind::Plain,
                text: "fn main() {}"
            }]
        );
        assert_eq!(tokens(Language::Text, "").count(), 0);
    }

    #[test]
    fn a_language_is_found_by_any_of_its_names() {
        assert_eq!(Language::from_name("text"), Some(Language::Text));
        assert_eq!(Language::from_name("TXT"), Some(Language::Text));
        assert_eq!(Language::from_name("PlainText"), Some(Language::Text));
        assert_eq!(Language::from_name("klingon"), None);
        assert_eq!(Language::from_name(""), None);
        for language in Language::ALL {
            assert_eq!(Language::from_name(language.name()), Some(language));
        }
    }

    /// Names are an API: a caller builds class names from them
    #[test]
    fn names_are_distinct_and_plain() {
        let mut kinds: Vec<&str> = Kind::ALL.iter().map(|kind| kind.name()).collect();
        assert!(
            kinds
                .iter()
                .all(|name| name.bytes().all(|b| b.is_ascii_lowercase()))
        );
        kinds.sort_unstable();
        kinds.dedup();
        assert_eq!(kinds.len(), Kind::ALL.len());

        let mut names: Vec<&str> = Language::ALL
            .iter()
            .flat_map(|l| l.names().iter().copied())
            .collect();
        let count = names.len();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), count, "no name belongs to two languages");
    }

    /// The iterator stays finished, and a clone picks up where it was
    #[test]
    fn the_iterator_is_fused_and_cloneable() {
        let mut all = tokens(Language::Text, "abc");
        let copy = all.clone();
        assert!(all.next().is_some());
        assert!(all.next().is_none());
        assert!(all.next().is_none());
        assert_eq!(copy.count(), 1);
    }
}
