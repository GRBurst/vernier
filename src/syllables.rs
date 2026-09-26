//! Syllables and complex words (spec 001 M2): a rule-based syllable counter,
//! ported by hand from the regex heuristic the research measured (datascience.SE answer 89312).
//!
//! The heuristic is `max(1, vowel runs − exceptions + additions)` on the lowercased word, with
//! ASCII `aeiouy` as vowels. The regex crate cannot express its lookahead (`ia(?!n$)`), so each
//! alternative is a small matcher and the additions are scanned like Python's `re.findall`.

/// The number of syllables in `word`; at least 1.
pub fn count_syllables(word: &str) -> usize {
    let lower: Vec<char> = word.to_lowercase().chars().collect();
    (vowel_runs(&lower) + additions(&lower))
        .saturating_sub(exceptions(&lower))
        .max(1)
}

fn is_vowel(c: char) -> bool {
    matches!(c, 'a' | 'e' | 'i' | 'o' | 'u' | 'y')
}

fn is_aeiou(c: char) -> bool {
    matches!(c, 'a' | 'e' | 'i' | 'o' | 'u')
}

/// Whether `w[i..]` starts with the characters of `literal`.
fn has_at(w: &[char], i: usize, literal: &str) -> bool {
    literal
        .chars()
        .enumerate()
        .all(|(k, c)| w.get(i + k) == Some(&c))
}

/// Whether the character at `i` exists and satisfies `class`.
fn is_at(w: &[char], i: usize, class: fn(char) -> bool) -> bool {
    w.get(i).is_some_and(|&c| class(c))
}

/// Maximal runs of `[aeiouy]`.
fn vowel_runs(w: &[char]) -> usize {
    (0..w.len())
        .filter(|&k| is_at(w, k, is_vowel) && (k == 0 || !is_at(w, k - 1, is_vowel)))
        .count()
}

/// 1 if `w` matches `[^aeiou]e[sd]?$` or `[^e]ely$`, else 0 (both are anchored at the end,
/// so at most one match exists).
fn exceptions(w: &[char]) -> usize {
    let n = w.len();
    // `[^aeiou]e` with the `e` at `e`: a character exists before it and is no vowel.
    let silent_e = |e: usize| e >= 1 && has_at(w, e, "e") && is_at(w, e - 1, |c| !is_aeiou(c));
    let final_e = n >= 2 && silent_e(n - 1);
    let e_sd = n >= 3 && matches!(w.get(n - 1), Some('s' | 'd')) && silent_e(n - 2);
    let ely = n >= 4 && has_at(w, n - 3, "ely") && !has_at(w, n - 4, "e");
    usize::from(final_e || e_sd || ely)
}

/// The number of non-overlapping, leftmost-first matches of the additions, like `re.findall`.
fn additions(w: &[char]) -> usize {
    let (mut i, mut n) = (0, 0);
    while i < w.len() {
        match ADDITIONS.iter().find_map(|alternative| alternative(w, i)) {
            Some(len) => {
                n += 1;
                i += len;
            }
            None => i += 1,
        }
    }
    n
}

/// The match length at `i`, if the alternative matches there.
type Alternative = fn(&[char], usize) -> Option<usize>;

/// The alternatives of `[^aeioulr][lr]e[sd]?$|[csgz]es$|[td]ed$|.y[aeiou]|ia(?!n$)|eo|ism$|[^aeiou]ire$|[^gq]ua`, in order.
const ADDITIONS: [Alternative; 9] = [
    lr_e_end,
    csgz_es_end,
    td_ed_end,
    any_y_vowel,
    ia_not_ian_end,
    eo,
    ism_end,
    consonant_ire_end,
    ua_not_after_gq,
];

/// `[^aeioulr][lr]e[sd]?$`
fn lr_e_end(w: &[char], i: usize) -> Option<usize> {
    let head = is_at(w, i, |c| !is_aeiou(c) && c != 'l' && c != 'r')
        && is_at(w, i + 1, |c| c == 'l' || c == 'r')
        && has_at(w, i + 2, "e");
    let len = if i + 3 == w.len() {
        3
    } else if i + 4 == w.len() && is_at(w, i + 3, |c| c == 's' || c == 'd') {
        4
    } else {
        0
    };
    (head && len > 0).then_some(len)
}

/// `[csgz]es$`
fn csgz_es_end(w: &[char], i: usize) -> Option<usize> {
    let hit = is_at(w, i, |c| matches!(c, 'c' | 's' | 'g' | 'z'))
        && has_at(w, i + 1, "es")
        && i + 3 == w.len();
    hit.then_some(3)
}

/// `[td]ed$`
fn td_ed_end(w: &[char], i: usize) -> Option<usize> {
    let hit = is_at(w, i, |c| c == 't' || c == 'd') && has_at(w, i + 1, "ed") && i + 3 == w.len();
    hit.then_some(3)
}

/// `.y[aeiou]`
fn any_y_vowel(w: &[char], i: usize) -> Option<usize> {
    let hit = is_at(w, i, |_| true) && has_at(w, i + 1, "y") && is_at(w, i + 2, is_aeiou);
    hit.then_some(3)
}

/// `ia(?!n$)`
fn ia_not_ian_end(w: &[char], i: usize) -> Option<usize> {
    let is_ian_end = has_at(w, i + 2, "n") && i + 3 == w.len();
    (has_at(w, i, "ia") && !is_ian_end).then_some(2)
}

/// `eo`
fn eo(w: &[char], i: usize) -> Option<usize> {
    has_at(w, i, "eo").then_some(2)
}

/// `ism$`
fn ism_end(w: &[char], i: usize) -> Option<usize> {
    (has_at(w, i, "ism") && i + 3 == w.len()).then_some(3)
}

/// `[^aeiou]ire$`
fn consonant_ire_end(w: &[char], i: usize) -> Option<usize> {
    let hit = is_at(w, i, |c| !is_aeiou(c)) && has_at(w, i + 1, "ire") && i + 4 == w.len();
    hit.then_some(4)
}

/// `[^gq]ua`
fn ua_not_after_gq(w: &[char], i: usize) -> Option<usize> {
    let hit = is_at(w, i, |c| c != 'g' && c != 'q') && has_at(w, i + 1, "ua");
    hit.then_some(3)
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    /// The Python spike's `regex_counter` on the first 40 words of `cmudict-500.tsv`
    /// (printed by `measurements/sample_cmudict.py`).
    const SPIKE_FIRST_40: [(&str, usize); 40] = [
        ("abrupt", 2),
        ("accountable", 4),
        ("achievements", 4),
        ("advising", 3),
        ("agency", 3),
        ("aggravated", 4),
        ("alone", 2),
        ("alonso", 3),
        ("amid", 2),
        ("anatomy", 4),
        ("angela", 3),
        ("angle", 2),
        ("anwar", 2),
        ("appeal", 2),
        ("approach", 2),
        ("arlene", 2),
        ("artie", 2),
        ("assignment", 3),
        ("athena", 3),
        ("athletes", 2),
        ("atrocities", 4),
        ("ballerina", 4),
        ("ballot", 2),
        ("bands", 1),
        ("banter", 2),
        ("barefoot", 3),
        ("barry", 2),
        ("bartholomew", 4),
        ("basement", 3),
        ("basque", 2),
        ("beaten", 2),
        ("belief", 2),
        ("benedict", 3),
        ("bid", 1),
        ("billionaire", 3),
        ("bird", 1),
        ("birthplace", 2),
        ("bjorn", 1),
        ("blades", 1),
        ("blair", 1),
    ];

    /// The spike's `regex_counter` on words that exercise every addition and exception,
    /// and on the CMUdict words where a match must be consumed before the scan goes on
    /// (`cronyism`, `dandyism`, `mccarthyism`, `shiyuan`); printed by the same script.
    const SPIKE_WITNESSES: [(&str, usize); 21] = [
        ("people", 3),
        ("places", 2),
        ("wanted", 2),
        ("trying", 2),
        ("special", 3),
        ("asian", 2),
        ("someone", 3),
        ("mechanism", 4),
        ("fire", 2),
        ("actually", 4),
        ("quality", 3),
        ("lately", 2),
        ("the", 1),
        ("freely", 2),
        ("biased", 2),
        ("cronyism", 3),
        ("dandyism", 3),
        ("mccarthyism", 3),
        ("shiyuan", 2),
        ("nth", 1),
        ("gps", 1),
    ];

    /// Given the words of the committed spike tables
    /// When their syllables are counted
    /// Then every count equals the Python spike's regex counter (the port is faithful)
    #[test]
    fn agrees_with_the_python_spike() {
        for (word, expected) in SPIKE_FIRST_40.iter().chain(SPIKE_WITNESSES.iter()) {
            assert_eq!(count_syllables(word), *expected, "{word}");
        }
    }

    fn with_a_letter() -> impl Strategy<Value = String> {
        "[a-zA-Z0-9'éıßü]{0,8}[a-zA-Z][a-zA-Z0-9'éıßü]{0,8}"
    }

    proptest! {
        /// Given any word containing a letter
        /// When its syllables are counted
        /// Then the count is at least 1
        #[test]
        fn at_least_one_for_letters(word in with_a_letter()) {
            prop_assert!(count_syllables(&word) >= 1);
        }

        /// Given a word of digits only
        /// When its syllables are counted
        /// Then the count is exactly 1
        #[test]
        fn digits_only_is_one(word in "[0-9]{1,12}") {
            prop_assert_eq!(count_syllables(&word), 1);
        }

        /// Given a word whose upper case lowercases back to its lower case
        /// When it and its upper case are counted
        /// Then the counts are equal
        // why: the filter keeps the law true; `ß` uppercases to `SS` (then matches `[csgz]es$`)
        // and `ı` to `I` (then a vowel), so case changes the letters of those words.
        #[test]
        fn case_does_not_change_the_count(
            word in with_a_letter()
                .prop_filter("case round-trips", |w| w.to_uppercase().to_lowercase() == w.to_lowercase())
        ) {
            prop_assert_eq!(count_syllables(&word), count_syllables(&word.to_uppercase()));
        }
    }
}
