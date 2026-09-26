//! Nominalizations (spec 001 M4): nouns derived from verbs or adjectives, recognised by suffix,
//! length and a committed stoplist.

/// The committed stoplist: one lower-case lemma per line, `#` starts a comment line.
const STOPLIST: &str = include_str!("../data/nominalization-stoplist.txt");

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

    const SUFFIXES: [&str; 6] = ["tion", "sion", "ment", "ance", "ence", "ity"];

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
        ("[a-z]{3,10}", prop::sample::select(SUFFIXES.to_vec()))
            .prop_map(|(stem, suffix)| format!("{stem}{suffix}"))
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
