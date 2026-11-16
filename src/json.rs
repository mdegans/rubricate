//! JSON, and what people paste as JSON: JSON Lines, and JSON with comments.

use crate::Kind;
use crate::cursor::Cursor;

/// One piece of JSON.
///
/// - A string is an [`Attribute`](Kind::Attribute) when a colon follows it
///   (an object's key) and a [`String`](Kind::String) otherwise. A string
///   ends at its closing quote or at the end of its line: JSON has no
///   multi-line strings, so an unterminated one does not swallow the rest.
/// - `true`, `false` and `null` are [`Keyword`](Kind::Keyword)s; any other
///   bare word is plain.
/// - A number is JSON's: an optional minus, digits, an optional fraction,
///   an optional exponent.
/// - `//` and `/* */` comments are recognised, though JSON proper has none.
/// - Everything else (braces, brackets, commas, colons, space) is plain.
pub(crate) fn scan(cursor: &mut Cursor<'_>) -> Kind {
    let Some(first) = cursor.peek() else {
        return Kind::Plain;
    };
    match first {
        '"' => string(cursor),
        '/' if cursor.eat("//") => {
            cursor.eat_while(|c| c != '\n');
            Kind::Comment
        }
        '/' if cursor.eat("/*") => {
            block_comment(cursor);
            Kind::Comment
        }
        '-' | '0'..='9' => number(cursor),
        c if is_word_start(c) => {
            let start = cursor.rest();
            let len = cursor.eat_while(is_word);
            match &start[..len] {
                "true" | "false" | "null" => Kind::Keyword,
                _ => Kind::Plain,
            }
        }
        _ => {
            // A run of what no rule above starts with
            cursor.bump();
            cursor.eat_while(|c| !starts_a_rule(c));
            Kind::Plain
        }
    }
}

fn is_word_start(c: char) -> bool {
    c.is_alphabetic() || c == '_'
}

fn is_word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Whether `c` can begin anything but plain text
fn starts_a_rule(c: char) -> bool {
    matches!(c, '"' | '/' | '-' | '0'..='9') || is_word_start(c)
}

/// From an opening quote to its close or the end of the line; a key if a
/// colon follows
fn string(cursor: &mut Cursor<'_>) -> Kind {
    cursor.bump();
    loop {
        match cursor.peek() {
            None | Some('\n') => return Kind::String,
            Some('"') => {
                cursor.bump();
                break;
            }
            Some('\\') => {
                cursor.bump();
                // The escaped character, unless the line ends first
                if !matches!(cursor.peek(), None | Some('\n')) {
                    cursor.bump();
                }
            }
            Some(_) => {
                cursor.eat_while(|c| !matches!(c, '"' | '\\' | '\n'));
            }
        }
    }
    // The space between a key and its colon is looked at here and scanned
    // again as plain text: twice at most, so still linear
    match cursor.peek_past(|c| c == ' ' || c == '\t') {
        Some(':') => Kind::Attribute,
        _ => Kind::String,
    }
}

/// After `/*`, to `*/` or the end
fn block_comment(cursor: &mut Cursor<'_>) {
    loop {
        cursor.eat_while(|c| c != '*');
        if cursor.eat("*/") || cursor.bump().is_none() {
            return;
        }
    }
}

/// A JSON number, or a lone `-` as plain text
fn number(cursor: &mut Cursor<'_>) -> Kind {
    let minus = cursor.eat("-");
    if cursor.eat_while(|c| c.is_ascii_digit()) == 0 {
        // A minus with no digits after it
        debug_assert!(minus);
        return Kind::Plain;
    }
    // A fraction only if a digit follows the point
    let mut after = cursor.clone();
    if after.eat(".") && after.eat_while(|c| c.is_ascii_digit()) > 0 {
        *cursor = after;
    }
    // An exponent only if a digit follows its sign
    let mut after = cursor.clone();
    if after.eat("e") || after.eat("E") {
        let _signed = after.eat("+") || after.eat("-");
        if after.eat_while(|c| c.is_ascii_digit()) > 0 {
            *cursor = after;
        }
    }
    Kind::Number
}

#[cfg(test)]
mod tests {
    extern crate std;

    use std::vec::Vec;

    use crate::Kind::{Attribute, Comment, Keyword, Number, Plain, String as Str};
    use crate::{Kind, Language, tokens};

    fn json(source: &str) -> Vec<(Kind, &str)> {
        tokens(Language::Json, source)
            .map(|token| (token.kind, token.text))
            .collect()
    }

    #[test]
    fn keys_are_told_from_values() {
        assert_eq!(
            json(r#"{"kind": "note", "tags" : ["a", "b"]}"#),
            [
                (Plain, "{"),
                (Attribute, r#""kind""#),
                (Plain, ": "),
                (Str, r#""note""#),
                (Plain, ", "),
                (Attribute, r#""tags""#),
                (Plain, " : ["),
                (Str, r#""a""#),
                (Plain, ", "),
                (Str, r#""b""#),
                (Plain, "]}"),
            ]
        );
    }

    /// A key and its colon are on one line; a string on the line above a
    /// colon is a value
    #[test]
    fn a_colon_on_the_next_line_makes_no_key() {
        assert_eq!(json("\"a\"\n: 1")[0], (Str, "\"a\""));
        assert_eq!(json("\"a\"\t :1")[0], (Attribute, "\"a\""));
    }

    #[test]
    fn escapes_do_not_end_a_string() {
        assert_eq!(json(r#""a \" b""#), [(Str, r#""a \" b""#)]);
        assert_eq!(json(r#""back\\""#), [(Str, r#""back\\""#)]);
        assert_eq!(json(r#""é é 🙂""#), [(Str, r#""é é 🙂""#)]);
        assert_eq!(json(r#""a:b": "c:d""#)[0], (Attribute, r#""a:b""#));
    }

    /// An unterminated string stops at the end of its line, so one missing
    /// quote does not colour the rest of the block
    #[test]
    fn an_unterminated_string_ends_with_its_line() {
        assert_eq!(
            json("\"open\n\"next\": 1"),
            [
                (Str, "\"open"),
                (Plain, "\n"),
                (Attribute, "\"next\""),
                (Plain, ": "),
                (Number, "1")
            ]
        );
        assert_eq!(json("\"open"), [(Str, "\"open")]);
        assert_eq!(
            json("\"ends in a backslash\\"),
            [(Str, "\"ends in a backslash\\")]
        );
        assert_eq!(json("\"a\\\nb")[0], (Str, "\"a\\"));
    }

    #[test]
    fn numbers_are_jsons() {
        for number in ["0", "-1", "42", "3.14", "-2.5e3", "1E-9", "6e+2", "0.0"] {
            assert_eq!(json(number), [(Number, number)], "{number}");
        }
        // What is not a number stays out of it
        assert_eq!(json("1."), [(Number, "1"), (Plain, ".")]);
        assert_eq!(json("1e"), [(Number, "1"), (Plain, "e")]);
        assert_eq!(json("1e+"), [(Number, "1"), (Plain, "e+")]);
        assert_eq!(json("-"), [(Plain, "-")]);
        assert_eq!(json("--1"), [(Plain, "-"), (Number, "-1")]);
        assert_eq!(
            json("1.2.3"),
            [(Number, "1.2"), (Plain, "."), (Number, "3")]
        );
        assert_eq!(
            json("[1,-2]"),
            [
                (Plain, "["),
                (Number, "1"),
                (Plain, ","),
                (Number, "-2"),
                (Plain, "]")
            ]
        );
    }

    #[test]
    fn only_jsons_words_are_keywords() {
        assert_eq!(
            json("[true, false, null]"),
            [
                (Plain, "["),
                (Keyword, "true"),
                (Plain, ", "),
                (Keyword, "false"),
                (Plain, ", "),
                (Keyword, "null"),
                (Plain, "]")
            ]
        );
        // A word that only starts like one, or has one inside, is plain;
        // so are other languages' spellings
        for word in [
            "truest",
            "nullable",
            "is_true",
            "True",
            "None",
            "undefined",
            "x1",
        ] {
            assert_eq!(json(word), [(Plain, word)], "{word}");
        }
    }

    #[test]
    fn comments_are_recognised() {
        assert_eq!(
            json("{ // why\n\"a\": 1 /* inline */ }"),
            [
                (Plain, "{ "),
                (Comment, "// why"),
                (Plain, "\n"),
                (Attribute, "\"a\""),
                (Plain, ": "),
                (Number, "1"),
                (Plain, " "),
                (Comment, "/* inline */"),
                (Plain, " }"),
            ]
        );
        assert_eq!(
            json("/* never closed * /"),
            [(Comment, "/* never closed * /")]
        );
        assert_eq!(json("/**/1"), [(Comment, "/**/"), (Number, "1")]);
        assert_eq!(json("/***/"), [(Comment, "/***/")]);
        // A slash that opens nothing is plain, and so is what follows it
        assert_eq!(json("a / b"), [(Plain, "a / b")]);
        // Nothing inside a string is a comment
        assert_eq!(json(r#""http://x/*y*/""#), [(Str, r#""http://x/*y*/""#)]);
    }

    /// One object a line, as logs and data sets are pasted
    #[test]
    fn json_lines_read_line_by_line() {
        let all = json("{\"n\": 1}\n{\"n\": 2}\n");
        assert_eq!(all.iter().filter(|(kind, _)| *kind == Attribute).count(), 2);
        assert_eq!(all.iter().filter(|(kind, _)| *kind == Number).count(), 2);
    }

    /// A nested document of the shape agents post: every key an attribute,
    /// every value its own kind, and nothing else coloured
    #[test]
    fn a_whole_document() {
        let source = r#"{
  "critique_type": "structural_absence",
  "provision": "Article IV §4 (obligations tracking)",
  "weights": [0.375, 0.3125, -1e-3],
  "binding": false,
  "note": null,
  "nested": {"depth": 2, "quote": "she said \"no\""}
}"#;
        let all = json(source);
        let of = |kind: Kind| -> Vec<&str> {
            all.iter()
                .filter(|(k, _)| *k == kind)
                .map(|(_, text)| *text)
                .collect()
        };
        assert_eq!(
            of(Attribute),
            [
                "\"critique_type\"",
                "\"provision\"",
                "\"weights\"",
                "\"binding\"",
                "\"note\"",
                "\"nested\"",
                "\"depth\"",
                "\"quote\""
            ]
        );
        assert_eq!(
            of(Str),
            [
                "\"structural_absence\"",
                "\"Article IV §4 (obligations tracking)\"",
                "\"she said \\\"no\\\"\""
            ]
        );
        assert_eq!(of(Number), ["0.375", "0.3125", "-1e-3", "2"]);
        assert_eq!(of(Keyword), ["false", "null"]);
        assert!(of(Comment).is_empty());
    }

    #[test]
    fn its_names() {
        for name in ["json", "JSON", "jsonc", "json5", "jsonl", "ndjson"] {
            assert_eq!(Language::from_name(name), Some(Language::Json), "{name}");
        }
    }
}
