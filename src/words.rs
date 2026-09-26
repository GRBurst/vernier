//! Word counting (spec 001, Definitions: a word is a UAX #29 word segment
//! containing at least one alphabetic or numeric character).

use unicode_segmentation::UnicodeSegmentation;

/// The number of words in `text`.
pub fn count_words(text: &str) -> usize {
    // `unicode_words` keeps exactly the UAX #29 segments with an alphanumeric character.
    text.unicode_words().count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn word() -> impl Strategy<Value = String> {
        prop::sample::select(vec![
            "cat", "Déjà", "naïve", "42", "3.14", "don't", "Straße", "x2",
        ])
        .prop_map(str::to_owned)
    }

    fn non_word() -> impl Strategy<Value = String> {
        let piece = prop::sample::select(vec![" ", ",", ".", "!", "—", "\n", "\t", "(", ")", "*"]);
        prop::collection::vec(piece, 0..12).prop_map(|p| p.concat())
    }

    proptest! {
        /// Given two texts that each end and start on a word
        /// When they are joined by a space
        /// Then the word count of the join is the sum of their counts
        #[test]
        fn counts_add_over_a_space(a in prop::collection::vec(word(), 0..8), b in prop::collection::vec(word(), 0..8)) {
            let (a, b) = (a.join(" "), b.join(" "));
            prop_assert_eq!(count_words(&format!("{a} {b}")), count_words(&a) + count_words(&b));
        }

        /// Given n single-segment words
        /// When they are joined by single spaces
        /// Then the count is n
        #[test]
        fn counts_each_single_segment_word_once(words in prop::collection::vec(word(), 0..16)) {
            prop_assert_eq!(count_words(&words.join(" ")), words.len());
        }

        /// Given a text of punctuation and whitespace only
        /// When its words are counted
        /// Then the count is zero
        #[test]
        fn punctuation_and_whitespace_are_not_words(text in non_word()) {
            prop_assert_eq!(count_words(&text), 0);
        }

        /// Given a text
        /// When punctuation and whitespace are added around it
        /// Then its word count does not change
        #[test]
        fn surrounding_punctuation_does_not_change_the_count(
            words in prop::collection::vec(word(), 0..8), before in non_word(), after in non_word()
        ) {
            let text = words.join(" ");
            prop_assert_eq!(count_words(&format!("{before} {text} {after}")), count_words(&text));
        }
    }

    /// Given "Hello, wörld! It's 2026." (one witness)
    /// When its words are counted
    /// Then there are 4
    #[test]
    fn a_counting_witness() {
        assert_eq!(count_words("Hello, wörld! It's 2026."), 4);
    }
}
