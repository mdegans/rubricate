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

// The scaffold's one language uses only part of this; the rest is here so
// that the first real scanner is written against a reviewed cursor.
#[allow(dead_code)]
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

    /// How many characters have been handed out or tested
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

    /// Consume everything that is left
    pub(crate) fn finish(&mut self) {
        self.steps += 1;
        self.pos = self.source.len();
    }
}
