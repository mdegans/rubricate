//! The one way a scanner reads its input.

/// A position in the source, and a count of the work done to get there.
///
/// Every scanner reads through a `Cursor`. It only moves forward, it only
/// stops on character boundaries, and it counts each character it hands
/// out, which is how the tests hold every language to linear time: a
/// scanner that looked at the same text again and again would count more
/// steps than the source has characters, however fast the machine.
#[derive(Debug, Clone)]
pub(crate) struct Cursor<'a> {
    source: &'a str,
    pos: usize,
    steps: usize,
}

impl<'a> Cursor<'a> {
    pub(crate) fn new(source: &'a str) -> Self {
        Self {
            source,
            pos: 0,
            steps: 0,
        }
    }

    /// The byte offset reached, always a character boundary
    pub(crate) fn pos(&self) -> usize {
        self.pos
    }

    /// How many characters have been handed out or tested. Read only by
    /// the tests that hold each language to linear work
    #[cfg(test)]
    pub(crate) fn steps(&self) -> usize {
        self.steps
    }

    pub(crate) fn is_done(&self) -> bool {
        self.pos >= self.source.len()
    }

    /// What is left, without consuming it
    pub(crate) fn rest(&self) -> &'a str {
        &self.source[self.pos..]
    }

    /// The next character, without consuming it
    pub(crate) fn peek(&mut self) -> Option<char> {
        self.steps += 1;
        self.rest().chars().next()
    }

    /// Consume one character
    pub(crate) fn bump(&mut self) -> Option<char> {
        let c = self.rest().chars().next()?;
        self.steps += 1;
        self.pos += c.len_utf8();
        Some(c)
    }

    /// Consume characters while `keep` holds; how many bytes that was
    pub(crate) fn eat_while(&mut self, mut keep: impl FnMut(char) -> bool) -> usize {
        let start = self.pos;
        for c in self.source[start..].chars() {
            self.steps += 1;
            if !keep(c) {
                break;
            }
            self.pos += c.len_utf8();
        }
        self.pos - start
    }

    /// Consume `prefix` if the rest starts with it
    pub(crate) fn eat(&mut self, prefix: &str) -> bool {
        self.steps += prefix.chars().count().max(1);
        if self.rest().starts_with(prefix) {
            self.pos += prefix.len();
            true
        } else {
            false
        }
    }

    /// The first character after any that `skip` holds for, consuming
    /// nothing. The characters looked at are counted: a scanner may look
    /// ahead over a run once, never again and again
    pub(crate) fn peek_past(&mut self, mut skip: impl FnMut(char) -> bool) -> Option<char> {
        for c in self.rest().chars() {
            self.steps += 1;
            if !skip(c) {
                return Some(c);
            }
        }
        None
    }

    /// Consume everything that is left
    pub(crate) fn finish(&mut self) {
        self.steps += 1;
        self.pos = self.source.len();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_only_moves_forward_on_character_boundaries() {
        let mut cursor = Cursor::new("aé🙂 b");
        assert_eq!(cursor.peek(), Some('a'));
        assert_eq!(cursor.pos(), 0);
        assert_eq!(cursor.bump(), Some('a'));
        assert_eq!(cursor.bump(), Some('é'));
        assert_eq!(cursor.pos(), 3);
        assert_eq!(cursor.rest(), "🙂 b");
        assert_eq!(cursor.eat_while(|c| c != ' '), 4);
        assert!(!cursor.eat("b"));
        assert!(cursor.eat(" "));
        assert_eq!(cursor.peek_past(|c| c == 'x'), Some('b'));
        assert_eq!(cursor.peek_past(|c| c == 'b'), None);
        assert_eq!(cursor.pos(), 8, "looking ahead consumes nothing");
        assert!(!cursor.is_done());
        cursor.finish();
        assert!(cursor.is_done());
        assert_eq!(cursor.bump(), None);
        assert_eq!(cursor.peek(), None);
        assert_eq!(cursor.eat_while(|_| true), 0);
    }

    /// Every read is counted, the ones that consume nothing included
    #[test]
    fn it_counts_what_it_reads() {
        let mut cursor = Cursor::new("abc   :");
        assert_eq!(cursor.steps(), 0);
        cursor.eat_while(char::is_alphabetic);
        assert_eq!(
            cursor.steps(),
            4,
            "three letters and the space that stopped it"
        );
        cursor.peek_past(char::is_whitespace);
        assert_eq!(cursor.steps(), 8);
        cursor.eat("zzz");
        assert_eq!(cursor.steps(), 11);
        cursor.peek();
        cursor.bump();
        cursor.finish();
        assert_eq!(cursor.steps(), 14);
    }
}
