//! Nominalizations (spec 001 M4): nouns derived from verbs or adjectives, recognised by suffix,
//! length and a committed stoplist.

use std::iter::Sum;
use std::ops::Add;

use crate::dependency::Token;

/// The committed stoplist: one lower-case lemma per line, `#` starts a comment line.
const STOPLIST: &str = include_str!("../data/nominalization-stoplist.txt");

/// The endings of a nominalization's lower-cased lemma.
const SUFFIXES: [&str; 6] = ["tion", "sion", "ment", "ance", "ence", "ity"];

/// The fewest letters a nominalization's lemma has.
const MIN_LETTERS: usize = 7;

/// Lemmas with a nominalization suffix that are not read as nominalizations.
#[derive(Debug, Clone)]
pub struct Stoplist {
    /// Sorted and deduplicated.
    lemmas: Vec<&'static str>,
}

impl Stoplist {
    /// The committed list, `data/nominalization-stoplist.txt`.
    pub fn committed() -> Self {
        let mut lemmas: Vec<&'static str> = entries().collect();
        lemmas.sort_unstable();
        lemmas.dedup();
        Self { lemmas }
    }

    /// Whether `lemma` (lower-case) is on the list.
    pub fn contains(&self, lemma: &str) -> bool {
        self.lemmas.binary_search(&lemma).is_ok()
    }
}

/// The lemma of a word without a parse (spec 001 M4): lower-cased, a trailing possessive `'s`,
/// `’s`, `'` or `’` removed, then a plural `-ies` read as `-y`, or else one final `-s` removed
/// unless it follows another `s`.
pub fn surface_lemma(word: &str) -> String {
    singular(without_possessive(&word.to_lowercase()))
}

const POSSESSIVES: [&str; 4] = ["'s", "’s", "'", "’"];

fn without_possessive(word: &str) -> &str {
    POSSESSIVES
        .iter()
        .find_map(|possessive| word.strip_suffix(possessive))
        .unwrap_or(word)
}

fn singular(word: &str) -> String {
    if let Some(stem) = word.strip_suffix("ies").filter(|stem| !stem.is_empty()) {
        format!("{stem}y")
    } else if word.ends_with("ss") {
        word.to_owned()
    } else {
        word.strip_suffix('s').unwrap_or(word).to_owned()
    }
}

/// Whether `lemma`, lower-cased, ends in a nominalization suffix, has at least 7 letters and is
/// not on the stoplist.
pub fn is_nominalization(lemma: &str, stoplist: &Stoplist) -> bool {
    let lemma = lemma.to_lowercase();
    has_suffix(&lemma) && letters(&lemma) >= MIN_LETTERS && !stoplist.contains(&lemma)
}

fn has_suffix(lemma: &str) -> bool {
    SUFFIXES.iter().any(|suffix| lemma.ends_with(suffix))
}

fn letters(lemma: &str) -> usize {
    lemma.chars().filter(|c| c.is_alphabetic()).count()
}

/// The number of nominalizations among `words`, each read through its surface lemma.
pub fn surface_nominalizations<'a>(
    words: impl IntoIterator<Item = &'a str>,
    stoplist: &Stoplist,
) -> usize {
    words
        .into_iter()
        .filter(|word| is_nominalization(&surface_lemma(word), stoplist))
        .count()
}

/// The number of nominalizations among a parse's `NOUN` tokens, each read through its lemma.
pub fn parsed_nominalizations(tokens: &[Token], stoplist: &Stoplist) -> usize {
    tokens
        .iter()
        .filter(|token| token.upostag == "NOUN" && is_nominalization(&token.lemma, stoplist))
        .count()
}

/// Nominalizations and words; they add up across sentences.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct NominalizationCount {
    pub nominalizations: usize,
    pub words: usize,
}

impl NominalizationCount {
    /// Nominalizations per word, absent when there is no word.
    pub fn ratio(&self) -> Option<f64> {
        (self.words > 0).then(|| self.nominalizations as f64 / self.words as f64)
    }
}

impl Add for NominalizationCount {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self {
            nominalizations: self.nominalizations + other.nominalizations,
            words: self.words + other.words,
        }
    }
}

impl Sum for NominalizationCount {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Self::default(), Add::add)
    }
}

/// The entries of the committed file, in file order: trimmed lines that are neither blank nor
/// comments.
fn entries() -> impl Iterator<Item = &'static str> {
    STOPLIST
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    use crate::words::words;

    /// The spec's suffix list, restated so the tests do not read the code's constant.
    const SPEC_SUFFIXES: [&str; 6] = ["tion", "sion", "ment", "ance", "ence", "ity"];

    fn token(form: &str, lemma: &str, upostag: &str) -> Token {
        Token {
            id: 1,
            form: form.to_owned(),
            lemma: lemma.to_owned(),
            upostag: upostag.to_owned(),
            head: 0,
            deprel: "root".to_owned(),
        }
    }

    /// `e` with a plural ending: `-y` → `-ies`, anything else `+s`.
    fn plural(entry: &str) -> String {
        entry
            .strip_suffix('y')
            .map_or_else(|| format!("{entry}s"), |stem| format!("{stem}ies"))
    }

    /// Given every committed stoplist entry, and its plural
    /// When each is checked through the surface lemma
    /// Then none is a nominalization
    #[test]
    fn stoplisted_lemmas_and_their_plurals_are_not_nominalizations() {
        let stoplist = Stoplist::committed();
        for entry in entries() {
            assert!(!is_nominalization(entry, &stoplist), "{entry}");
            let plural = plural(entry);
            assert!(
                !is_nominalization(&surface_lemma(&plural), &stoplist),
                "{plural}"
            );
        }
    }

    /// Given the NOMZ sentence's words, and `rations` (lemma `ration`, 6 letters)
    /// When nominalizations are counted without a parse
    /// Then the sentence has 4 (commission, decisions, implementation, consideration) and
    /// `rations` none
    #[test]
    fn surface_witnesses() {
        let stoplist = Stoplist::committed();
        let text = "We commission a review of the committee's decisions because their \
                    implementation needs careful consideration.";
        assert_eq!(surface_nominalizations(words(text), &stoplist), 4);
        assert_eq!(surface_nominalizations(["rations"], &stoplist), 0);
    }

    /// Given parsed tokens: a `VERB` `commission`, a `NOUN` `decisions` with lemma `decision`, a
    /// `NOUN` `cities` with the stoplisted lemma `city`, and a `NOUN` `motions` lemma `motion`
    /// When nominalizations are counted from the parse
    /// Then only the `NOUN` whose lemma qualifies counts
    #[test]
    fn parsed_count_reads_noun_lemmas() {
        let stoplist = Stoplist::committed();
        let tokens = [
            token("commission", "commission", "VERB"),
            token("decisions", "decision", "NOUN"),
            token("cities", "city", "NOUN"),
            token("motions", "motion", "NOUN"),
        ];
        assert_eq!(parsed_nominalizations(&tokens, &stoplist), 1);
    }

    /// Given words with and without plural, possessive and capital letters
    /// When their surface lemma is taken
    /// Then possessives go first, `-ies` reads as `-y`, one final `-s` goes unless after `s`
    #[test]
    fn surface_lemma_witnesses() {
        for (word, lemma) in [
            ("activities", "activity"),
            ("Decisions", "decision"),
            ("business", "business"),
            ("cities", "city"),
            ("class", "class"),
            ("implementation", "implementation"),
            ("rations", "ration"),
            ("decision's", "decision"),
            ("decisions'", "decision"),
            ("activity’s", "activity"),
            ("decisions’", "decision"),
        ] {
            assert_eq!(surface_lemma(word), lemma, "{word:?}");
        }
    }

    fn lemma_with_suffix() -> impl Strategy<Value = String> {
        ("[a-z]{3,10}", prop::sample::select(SPEC_SUFFIXES.to_vec()))
            .prop_map(|(stem, suffix)| format!("{stem}{suffix}"))
    }

    fn mixed_case_word() -> impl Strategy<Value = String> {
        let built = (
            "[a-zA-Zéö]{0,8}",
            prop::sample::select(SPEC_SUFFIXES.to_vec()),
            prop::sample::select(vec!["", "s", "S"]),
            any::<bool>(),
        )
            .prop_map(|(stem, suffix, plural, upper)| {
                let suffix = if upper {
                    suffix.to_uppercase()
                } else {
                    suffix.to_owned()
                };
                format!("{stem}{suffix}{plural}")
            });
        let listed = (
            prop::sample::select(entries().collect::<Vec<_>>()),
            any::<u32>(),
        )
            .prop_map(|(entry, mask)| {
                entry
                    .chars()
                    .enumerate()
                    .map(|(i, c)| {
                        if mask >> (i % 32) & 1 == 1 {
                            c.to_ascii_uppercase()
                        } else {
                            c
                        }
                    })
                    .collect::<String>()
            });
        prop_oneof![3 => built, 1 => listed]
    }

    fn any_count() -> impl Strategy<Value = NominalizationCount> {
        (0usize..50, prop_oneof![1 => Just(0usize), 3 => 1usize..100]).prop_map(
            |(nominalizations, words)| NominalizationCount {
                nominalizations,
                words,
            },
        )
    }

    proptest! {
        /// Given any lower-case lemma ending in a nominalization suffix
        /// When its plural (`+s`, and for `-ity` also `-ies`) is lemmatized
        /// Then the lemma comes back
        #[test]
        fn plurals_lemmatize_to_their_singular(lemma in lemma_with_suffix()) {
            let plural = format!("{lemma}s");
            prop_assert_eq!(surface_lemma(&plural), lemma.clone());
            if let Some(stem) = lemma.strip_suffix('y') {
                let plural = format!("{stem}ies");
                prop_assert_eq!(surface_lemma(&plural), lemma.clone());
            }
        }

        /// Given any word of ASCII letters in mixed case (plus é, ö) with a suffix and an optional
        /// plural `s`, or a stoplisted lemma in mixed case
        /// When it and its upper-case form are checked through the surface lemma
        /// Then both give the same verdict
        #[test]
        fn the_verdict_ignores_letter_case(word in mixed_case_word()) {
            let stoplist = Stoplist::committed();
            let verdict = |w: &str| is_nominalization(&surface_lemma(w), &stoplist);
            prop_assert_eq!(verdict(&word), verdict(&word.to_uppercase()), "{}", word);
        }

        /// Given a lemma with a suffix and fewer than 7 letters
        /// When it is checked
        /// Then it is not a nominalization
        #[test]
        fn short_lemmas_are_not_nominalizations(
            (stem, suffix) in prop::sample::select(SPEC_SUFFIXES.to_vec())
                .prop_flat_map(|suffix| (prop::collection::vec(prop::char::range('a', 'z'), 0..7 - suffix.len()), Just(suffix))),
        ) {
            let lemma = format!("{}{suffix}", stem.iter().collect::<String>());
            prop_assert!(lemma.chars().count() < 7, "{}", lemma);
            prop_assert!(!is_nominalization(&lemma, &Stoplist::committed()), "{}", lemma);
        }

        /// Given a lemma with a suffix, at least 7 letters and not on the stoplist
        /// When it is checked
        /// Then it is a nominalization
        #[test]
        fn long_unlisted_lemmas_are_nominalizations(lemma in lemma_with_suffix()) {
            let stoplist = Stoplist::committed();
            prop_assume!(lemma.chars().count() >= 7 && !stoplist.contains(&lemma));
            prop_assert!(is_nominalization(&lemma, &stoplist), "{}", lemma);
        }

        /// Given any three counts
        /// When they are added
        /// Then addition is associative and commutative, zero is neutral, and a sum is the fold
        #[test]
        fn counts_form_a_commutative_monoid(a in any_count(), b in any_count(), c in any_count()) {
            prop_assert_eq!((a + b) + c, a + (b + c));
            prop_assert_eq!(a + b, b + a);
            prop_assert_eq!(a + NominalizationCount::default(), a);
            prop_assert_eq!([a, b, c].into_iter().sum::<NominalizationCount>(), a + b + c);
        }

        /// Given any count
        /// When its ratio is taken
        /// Then it is absent exactly when there is no word, otherwise nominalizations / words, in
        /// [0, 1] whenever nominalizations ≤ words
        #[test]
        fn ratio_is_absent_iff_no_words(c in any_count()) {
            prop_assert_eq!(c.ratio().is_none(), c.words == 0);
            if let Some(r) = c.ratio() {
                prop_assert!((r - c.nominalizations as f64 / c.words as f64).abs() < 1e-12);
                if c.nominalizations <= c.words {
                    prop_assert!((0.0..=1.0).contains(&r), "{}", r);
                }
            }
        }

        /// Given any word without an apostrophe, optionally with a suffix and a plural `s`
        /// When a possessive `'s`, `’s`, `'` or `’` is appended
        /// Then its surface lemma is the word's own
        #[test]
        fn possessives_do_not_change_the_lemma(
            stem in "[a-zA-Z]{1,10}",
            suffix in prop::sample::select(vec!["", "tion", "ity", "ies", "s"]),
            possessive in prop::sample::select(vec!["'s", "’s", "'", "’"]),
        ) {
            let word = format!("{stem}{suffix}");
            prop_assert_eq!(surface_lemma(&format!("{word}{possessive}")), surface_lemma(&word));
        }
    }

    /// Given the committed stoplist file
    /// When its entries are read
    /// Then every entry is non-empty, lower-case, free of whitespace and listed once, and comment
    /// and blank lines yield no entry
    #[test]
    fn committed_entries_are_unique_lower_case_lemmas() {
        let raw: Vec<&str> = entries().collect();
        assert!(!raw.is_empty());
        for entry in &raw {
            assert!(!entry.is_empty(), "{entry:?}");
            assert_eq!(*entry, entry.to_lowercase(), "{entry:?}");
            assert!(!entry.contains(char::is_whitespace), "{entry:?}");
            assert!(!entry.starts_with('#'), "{entry:?}");
        }
        let mut sorted = raw.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), raw.len(), "a line is listed twice");
        let stoplist = Stoplist::committed();
        assert!(raw.iter().all(|entry| stoplist.contains(entry)));
    }

    /// Given the committed stoplist
    /// When pybiber's `city` and vernier's `version`, a comment word and a non-entry are looked up
    /// Then only the two entries are found
    #[test]
    fn committed_stoplist_holds_its_entries_only() {
        let stoplist = Stoplist::committed();
        assert!(stoplist.contains("city"));
        assert!(stoplist.contains("version"));
        assert!(!stoplist.contains("#"));
        assert!(!stoplist.contains("research"));
        assert!(!stoplist.contains("decision"));
        assert!(!stoplist.contains(""));
    }
}
