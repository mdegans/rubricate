# Changelog

## 0.1.0

The scaffold: the API and its promises, before any rules.

- `tokens(language, source)` returns an iterator of `Token { kind, text }`.
  The texts joined in order are the source; no token is empty; no two
  neighbours share a kind.
- `Kind`, a closed set of thirteen, with stable lowercase names.
- `Language`, with one member, `Text` (all plain), and `Language::from_name`
  for a code fence's info string.
- Linear time by construction: scanners read through a cursor that counts its
  steps, and a test holds every language to a constant number per character
  on hostile input.
- No dependencies, `#![forbid(unsafe_code)]`, `no_std`, no allocation.
