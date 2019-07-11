//! Module for calculating matches and scores.

/// Indicate if the given string matches the query.
///
/// A match occurs when the query is a subsequence
/// of the string, case-insensitive.
pub fn subsequence_match(query: &str, string: &str) -> bool {
    let string = string.to_ascii_lowercase();
    let mut s: &str = string.as_ref();
    let chars = query.chars();
    for c in chars {
        match s.find(c) {
            None => return false,
            Some(p) => s = &s[(p + 1)..],
        };
    }
    return true;
}

#[test]
fn test_subsequence_match() {
    assert!(subsequence_match("", ""));
    assert!(subsequence_match("", "foo"));
    assert!(subsequence_match("foo", "foo"));
    assert!(subsequence_match("bar", "BAR"));
    assert!(subsequence_match("bar", "bazaar"));
    assert!(subsequence_match("bar", "BaZaAR"));
    assert!(!subsequence_match("foo", ""));
    assert!(!subsequence_match("foo", "fo"));
    assert!(!subsequence_match("bar", "bra"));
    assert!(!subsequence_match("baaaar", "bar"));
}
