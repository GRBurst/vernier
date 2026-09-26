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
