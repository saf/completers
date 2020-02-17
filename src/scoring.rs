//! Module for calculating matches and scores.

use std::borrow::Borrow;

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

/// A single entry in the scoring table.
///
/// See the description of the score() routine for details
/// on how scoring is implemented.
#[derive(Clone, Copy, Default)]
struct ScoringEntry {
    /// Score when the current position is "taken" into the matching subsequence.
    take: u64,

    /// Score when the current position is omitted from the matching subsequence.
    leave: u64,
}

impl std::fmt::Display for ScoringEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "y{}/n{}", self.take, self.leave)
    }
}

/// Return the indices of word start characters in the given string.
fn word_start_indices<C: Borrow<char>>(chars: impl Iterator<Item = C>) -> Vec<usize> {
    let mut previous_char_is_letter = false;
    let mut result = Vec::new();
    for (i, c) in chars.enumerate() {
        if !previous_char_is_letter && c.borrow().is_alphanumeric() {
            previous_char_is_letter = true;
            result.push(i);
        } else if !c.borrow().is_alphanumeric() {
            previous_char_is_letter = false;
        }
    }
    result
}

#[test]
fn test_word_start_indices() {
    let check = |s: &str, expected: Vec<usize>| assert_eq!(word_start_indices(s.chars()), expected);
    check("foo", vec![0]);
    check("foo bar", vec![0, 4]);
    check("foo_bar", vec![0, 4]);
    check("directory/subdir/file.ext", vec![0, 10, 17, 22]);
}

/// Settings for scoring.
///
/// This aims to represent the configuration of assigning scores
/// which may favor word starts or consecutive characters.
pub struct ScoringSettings {
    letter_match: u64,
    subsequent_bonus: u64,
    word_start_bonus: u64,
}
/// An array to store the scores for prefixes of the
/// query and the candidate string.
///
/// The scores are calculated
/// with a dynamic programming algorithm using the following
/// score functions:
///    T[i, j] - the score for the prefixes query[..=i] and candidate[..=j]
///              if we take the last character of both prefixes into the
///              match subsequence; this is 0 if query[i] and candidate[j]
///              are different characters.
///    L[i, j] - the score for the prefixes query[..=] and candidate[..=j]
///              if we do not take the last character of both prefixes
///              into the match subsequence.
///
/// The step functions are as follows:
///    T[i, j] = max {
///        T[i-1, j-1] + LETTER + SUBSEQ [+ WORD]
///            (if previous chars match, i.e. T[i-1, j-1] is non-zero),
///        L[i-1, j-1] + LETTER [+ WORD]
///    }
///
///    When we "take" a character from the query, we advance
///    in both the candidate and the query; therefore, we take
///    the value from the previous row and the previous column,
///    and apply relevant bonuses. If we "take" a character, we
///    account for it in the score, but we cannot use the same
///    character again. It may turn out that it is more beneficial
///    to leave a matching character in the query for now and use
///    it later on, when it earns us better bonuses. Therefore,
///    we need two "timelines" going forward: one where we Took
///    the character, and one where we Left it in the query.
///
///   N[i, j] = max { T[i, j-1], N[i, j-1] }
///
///    When we do not take a character from the query, we only
///    advance in the candidate but not in the query, so we take
///    the values from the same "row" (i.e., same query prefix).
///    We are going to lose the subsequence bonus, so we simply
///    lump the preceding values of Take and Leave together,
///    as it will make no difference for next characters whether
///    we Took or Left the previous character.
struct ScoringArray<'a> {
    candidate_chars: Vec<char>,
    query_chars: Vec<char>,
    word_start_indices: Vec<usize>,
    settings: &'a ScoringSettings,
    array: Vec<Vec<ScoringEntry>>,
}

impl ScoringArray<'_> {
    /// Create a new array.
    pub fn new(
        candidate_chars: Vec<char>,
        query_chars: Vec<char>,
        word_start_indices: Vec<usize>,
        scoring_settings: &ScoringSettings,
    ) -> ScoringArray {
        ScoringArray {
            candidate_chars: candidate_chars,
            query_chars: query_chars,
            word_start_indices: word_start_indices,
            settings: scoring_settings,
            array: Vec::new(),
        }
    }

    /// Return the word start bonus for the given index into the "candidate".
    fn word_start_bonus(&self, candidate_index: usize) -> u64 {
        if self.word_start_indices.contains(&candidate_index) {
            self.settings.word_start_bonus
        } else {
            0
        }
    }

    /// Score for the given prefix of query and candidate if character is "taken"
    /// into the match.
    fn take_score(&self, query_index: usize, candidate_index: usize) -> u64 {
        if self.query_chars[query_index] != self.candidate_chars[candidate_index] {
            return 0;
        }

        let score_from_prev = if query_index > 0 && candidate_index > 0 {
            let prev = self.array[query_index - 1][candidate_index - 1];
            let take_prev_score = if prev.take > 0 {
                prev.take + self.settings.subsequent_bonus
            } else {
                0
            };
            std::cmp::max(take_prev_score, prev.leave)
        } else {
            0
        };
        score_from_prev + self.settings.letter_match + self.word_start_bonus(candidate_index)
    }

    /// Compute the score if we do not take the current character
    /// into the match.
    fn leave_score(&self, query_index: usize, candidate_index: usize) -> u64 {
        if candidate_index > 0 {
            let prev = &self.array[query_index][candidate_index - 1];
            std::cmp::max(prev.take, prev.leave)
        } else {
            0
        }
    }

    /// Computes a single entry in the scoring table.
    fn compute_entry(&self, query_index: usize, candidate_index: usize) -> ScoringEntry {
        ScoringEntry {
            take: self.take_score(query_index, candidate_index),
            leave: self.leave_score(query_index, candidate_index),
        }
    }

    /// Compute all values of the scoring array.
    pub fn compute(&mut self) {
        for qi in 0..self.query_chars.len() {
            self.array.push(Vec::new());
            for ci in 0..self.candidate_chars.len() {
                let entry = self.compute_entry(qi, ci);
                self.array[qi].push(entry);
            }
        }
    }

    /// Return the score computed in the array.
    ///
    /// Because the array entries represent scores for prefixes, the overall
    /// score is the score from the last array cell in the last row.
    pub fn score(&self) -> u64 {
        let empty = vec![];
        let array_end = self
            .array
            .last()
            .unwrap_or(&empty)
            .last()
            .unwrap_or(&ScoringEntry { take: 0, leave: 0 });
        std::cmp::max(array_end.take, array_end.leave)
    }
}

impl std::fmt::Display for ScoringArray<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "   {}", self.candidate_chars.iter().collect::<String>())?;
        for (i, row) in self.array.iter().enumerate() {
            write!(f, "{}  ", self.query_chars[i])?;
            for entry in row {
                write!(f, "{} ", entry)?;
            }
            writeln!(f, "")?;
        }
        std::fmt::Result::Ok(())
    }
}

/// Return the score for the given query and candidate.
pub fn score(candidate: &str, query: &str, settings: &ScoringSettings) -> u64 {
    if query.len() > candidate.len() {
        return 0;
    }
    let candidate_chars = candidate.chars().collect::<Vec<_>>();
    let query_chars = query.chars().collect::<Vec<_>>();
    let word_starts = word_start_indices(candidate_chars.iter());

    let mut scoring_array = ScoringArray::new(candidate_chars, query_chars, word_starts, settings);
    scoring_array.compute();
    scoring_array.score()
}

#[test]
fn test_scoring_plain() {
    let settings = ScoringSettings {
        letter_match: 1,
        subsequent_bonus: 0,
        word_start_bonus: 0,
    };
    assert_eq!(score("", "", &settings), 0);
    assert_eq!(score("foo", "", &settings), 0);
    assert_eq!(score("foo", "f", &settings), 1);
    assert_eq!(score("foo", "o", &settings), 1);
    assert_eq!(score("foo", "fo", &settings), 2);
    assert_eq!(score("foo", "oo", &settings), 2);
    assert_eq!(score("foo", "foo", &settings), 3);
    assert_eq!(score("foo", "ooo", &settings), 2);
    assert_eq!(score("bar", "br", &settings), 2);

    assert_eq!(score("foo", "fooo", &settings), 0);
}

#[test]
fn test_scoring_word_start_bonus() {
    let settings = ScoringSettings {
        letter_match: 1,
        subsequent_bonus: 0,
        word_start_bonus: 3,
    };
    assert_eq!(score("", "", &settings), 0);
    assert_eq!(score("foo", "", &settings), 0);
    assert_eq!(score("foo", "f", &settings), 4);
    assert_eq!(score("foo", "o", &settings), 1);
    assert_eq!(score("foo", "fo", &settings), 5);
    assert_eq!(score("foo", "oo", &settings), 2);
    assert_eq!(score("foo bar", "fb", &settings), 8);
    assert_eq!(score("foo/bar", "foba", &settings), 10);
    assert_eq!(score("foo/bar", "fa", &settings), 5);
    assert_eq!(score("foo/bar", "oa", &settings), 2);
}

#[test]
fn test_scoring_subsequent_bonus() {
    let settings = ScoringSettings {
        letter_match: 1,
        subsequent_bonus: 3,
        word_start_bonus: 0,
    };
    assert_eq!(score("", "", &settings), 0);
    assert_eq!(score("foo", "", &settings), 0);
    assert_eq!(score("foo", "f", &settings), 1);
    assert_eq!(score("foo", "fo", &settings), 5);
    assert_eq!(score("foo", "oo", &settings), 5);
    assert_eq!(score("foo", "foo", &settings), 9);
    assert_eq!(score("bar", "ar", &settings), 5);
    assert_eq!(score("bar", "br", &settings), 2);
    assert_eq!(score("bar", "bar", &settings), 9);
    assert_eq!(score("foo/bar", "ob", &settings), 2);
}
