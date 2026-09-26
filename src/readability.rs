//! Surface readability (spec 001 M2): Flesch Reading Ease, Flesch-Kincaid Grade, Gunning Fog and
//! average sentence length, computed from counts that add up across sentences.

use std::iter::Sum;
use std::ops::Add;

/// The counts a text's surface metrics are computed from; they add up across sentences.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SurfaceCounts {
    pub words: usize,
    pub sentences: usize,
    pub syllables: usize,
    pub complex_words: usize,
}

impl Add for SurfaceCounts {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self {
            words: self.words + other.words,
            sentences: self.sentences + other.sentences,
            syllables: self.syllables + other.syllables,
            complex_words: self.complex_words + other.complex_words,
        }
    }
}

impl Sum for SurfaceCounts {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Self::default(), Add::add)
    }
}

/// The surface scores of a text with at least one word and one sentence.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Readability {
    pub flesch_reading_ease: f64,
    pub flesch_kincaid_grade: f64,
    pub gunning_fog: f64,
    pub average_sentence_length: f64,
}

/// The scores of `counts`, or `None` when there is no word or no sentence to divide by.
pub fn readability(counts: SurfaceCounts) -> Option<Readability> {
    if counts.words == 0 || counts.sentences == 0 {
        return None;
    }
    let words = counts.words as f64;
    let asl = words / counts.sentences as f64;
    let syllables_per_word = counts.syllables as f64 / words;
    let complex_per_word = counts.complex_words as f64 / words;
    Some(Readability {
        flesch_reading_ease: 206.835 - 1.015 * asl - 84.6 * syllables_per_word,
        flesch_kincaid_grade: 0.39 * asl + 11.8 * syllables_per_word - 15.59,
        gunning_fog: 0.4 * (asl + 100.0 * complex_per_word),
        average_sentence_length: asl,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn counts(
        words: usize,
        sentences: usize,
        syllables: usize,
        complex_words: usize,
    ) -> SurfaceCounts {
        SurfaceCounts {
            words,
            sentences,
            syllables,
            complex_words,
        }
    }

    fn scores(c: SurfaceCounts) -> Readability {
        readability(c).expect("words and sentences are non-zero")
    }

    /// Given 100 words, 5 sentences and 150 syllables
    /// When the readability is computed
    /// Then FRE is 59.635 and FKGL is 9.91, within 1e-9
    #[test]
    fn witness_100_words_5_sentences_150_syllables() {
        let r = scores(counts(100, 5, 150, 0));
        assert!((r.flesch_reading_ease - 59.635).abs() < 1e-9, "{r:?}");
        assert!((r.flesch_kincaid_grade - 9.91).abs() < 1e-9, "{r:?}");
        assert!((r.average_sentence_length - 20.0).abs() < 1e-9, "{r:?}");
    }

    fn scorable() -> impl Strategy<Value = SurfaceCounts> {
        (1usize..5_000, 1usize..500, 0usize..8, 0usize..=100)
            .prop_map(|(w, s, extra, pct)| counts(w, s, w + w * extra / 2, w * pct / 100))
    }

    /// A count that is zero a quarter of the time, so the absence boundary is always exercised.
    fn often_zero(max: usize) -> impl Strategy<Value = usize> {
        prop_oneof![1 => Just(0usize), 3 => 1..max]
    }

    fn any_counts() -> impl Strategy<Value = SurfaceCounts> {
        (
            often_zero(1_000),
            often_zero(100),
            0usize..5_000,
            0usize..1_000,
        )
            .prop_map(|(w, s, syl, c)| counts(w, s, syl, c))
    }

    proptest! {
        /// Given counts with at least one word and one sentence
        /// When the readability is computed
        /// Then Gunning Fog is 0.4·(words/sentences + 100·complex/words) and ASL is words/sentences
        #[test]
        fn fog_matches_the_formula(c in scorable()) {
            let r = scores(c);
            let (w, s, cw) = (c.words as f64, c.sentences as f64, c.complex_words as f64);
            prop_assert!((r.gunning_fog - 0.4 * (w / s + 100.0 * cw / w)).abs() < 1e-9);
            prop_assert!((r.average_sentence_length - w / s).abs() < 1e-9);
        }

        /// Given counts with at least one word and one sentence
        /// When syllables increase by δ ≥ 1 at fixed words and sentences
        /// Then FRE is strictly lower and FKGL strictly higher
        #[test]
        fn more_syllables_lower_fre_higher_fkgl(c in scorable(), delta in 1usize..1_000) {
            let (before, after) = (scores(c), scores(SurfaceCounts { syllables: c.syllables + delta, ..c }));
            prop_assert!(after.flesch_reading_ease < before.flesch_reading_ease);
            prop_assert!(after.flesch_kincaid_grade > before.flesch_kincaid_grade);
        }

        /// Given counts with at least one word and one sentence
        /// When the same words are split into more sentences (shorter sentences, same syllables per word)
        /// Then FRE is strictly higher and FKGL strictly lower
        #[test]
        fn longer_sentences_lower_fre_higher_fkgl(c in scorable(), more in 1usize..100) {
            let (before, after) = (scores(c), scores(SurfaceCounts { sentences: c.sentences + more, ..c }));
            prop_assert!(after.flesch_reading_ease > before.flesch_reading_ease);
            prop_assert!(after.flesch_kincaid_grade < before.flesch_kincaid_grade);
        }

        /// Given any counts
        /// When the readability is computed
        /// Then it is absent exactly when there is no word or no sentence, and finite otherwise
        #[test]
        fn metrics_absent_iff_no_word_or_no_sentence(c in any_counts()) {
            let r = readability(c);
            prop_assert_eq!(r.is_none(), c.words == 0 || c.sentences == 0);
            if let Some(r) = r {
                prop_assert!([r.flesch_reading_ease, r.flesch_kincaid_grade, r.gunning_fog, r.average_sentence_length]
                    .iter().all(|x| x.is_finite()));
            }
        }

        /// Given any three counts
        /// When they are added
        /// Then addition is associative and commutative, zero is neutral, and a sum equals the folded additions
        #[test]
        fn surface_counts_form_a_commutative_monoid(a in any_counts(), b in any_counts(), c in any_counts()) {
            prop_assert_eq!((a + b) + c, a + (b + c));
            prop_assert_eq!(a + b, b + a);
            prop_assert_eq!(a + SurfaceCounts::default(), a);
            prop_assert_eq!([a, b, c].into_iter().sum::<SurfaceCounts>(), a + b + c);
        }
    }

    /// Given zero words, or zero sentences
    /// When the readability is computed
    /// Then the metrics are absent
    #[test]
    fn zero_words_or_sentences_are_absent() {
        assert_eq!(readability(counts(0, 3, 0, 0)), None);
        assert_eq!(readability(counts(12, 0, 20, 1)), None);
        assert_eq!(readability(SurfaceCounts::default()), None);
    }
}
