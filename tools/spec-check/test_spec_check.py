"""Tests for spec_check: one clean spec, then one mutation per rule.

Each test breaks exactly one thing and asserts that exactly that rule fires,
so a rule that goes silent or starts firing on clean text fails a test (A4).
Run: python3 -m unittest discover -s tools/spec-check
"""

import unittest

from spec_check import VAGUE, check_text

CLEAN = """# Spec 001 — Example

**Status:** Draft

**Scope:**

- One thing.

**Non-Goals:**

- THE tool SHALL NOT do another thing.

## M1 — First (Status: PLANNED)
<a id="M1"></a>

**Acceptance Criteria:**

- [ ] WHEN a file is given, THE tool SHALL print its word count.

```text
## M9 — inside a fence (Status: WHATEVER)
```

## M2 — Second (Status: DONE)
<a id="M2"></a>

**Acceptance Criteria:**

- [x] THE tool SHALL exit 0 on an empty file, even for `fast` input.
"""


def rules(text: str, severity: str = "error") -> list[str]:
    return [f.rule for f in check_text(text) if f.severity == severity]


class CleanSpec(unittest.TestCase):
    def test_clean_spec_has_no_findings(self):
        self.assertEqual(check_text(CLEAN), [])


class EachRuleFiresAlone(unittest.TestCase):
    CASES = {
        "spec-title": ("# Spec 001 — Example", "# Example"),
        "spec-status": ("**Status:** Draft", "Status: Draft"),
        "spec-section": ("**Scope:**", "Scope:"),
        "ms-heading": ("## M1 — First (Status: PLANNED)", "## M1 First"),
        "ms-status": ("(Status: PLANNED)", "(Status: MAYBE)"),
        "ms-anchor": ('<a id="M1"></a>', '<a id="M7"></a>'),
        "ms-duplicate": ("## M2 — Second", "## M1 — Second"),
        "ac-shall": ("THE tool SHALL print", "THE tool prints"),
        "ac-vague": ("print its word count", "print its word count quickly"),
        "ms-done-unticked": ("- [x] THE tool SHALL exit", "- [ ] THE tool SHALL exit"),
    }

    def test_each_mutation_fires_exactly_its_rule(self):
        for rule, (old, new) in self.CASES.items():
            with self.subTest(rule=rule):
                self.assertIn(old, CLEAN)
                found = rules(CLEAN.replace(old, new, 1))
                self.assertIn(rule, found)
                # ms-duplicate also breaks the anchor of the renamed milestone
                self.assertLessEqual(set(found) - {rule}, {"ms-anchor"})

    def test_missing_criteria_fires(self):
        text = CLEAN.replace("- [ ] WHEN a file is given, THE tool SHALL print its word count.\n", "")
        self.assertEqual(rules(text), ["ms-criteria"])

    def test_every_vague_word_fires(self):
        for word in sorted(VAGUE):
            with self.subTest(word=word):
                text = CLEAN.replace("its word count", f"its word count {word}", 1)
                self.assertEqual(rules(text), ["ac-vague"])


class Clarification(unittest.TestCase):
    MARKED = CLEAN.replace("- One thing.", "- One thing. [NEEDS CLARIFICATION: which?]")

    def test_draft_marker_is_only_a_warning(self):
        self.assertEqual(rules(self.MARKED), [])
        self.assertEqual(rules(self.MARKED, "warning"), ["clarify-open"])

    def test_approved_marker_is_an_error(self):
        text = self.MARKED.replace("**Status:** Draft", "**Status:** Approved")
        self.assertEqual(rules(text), ["clarify-approved"])

    def test_marker_in_code_span_is_data(self):
        text = CLEAN.replace("- One thing.", "- One thing, see `[NEEDS CLARIFICATION: x]`.")
        self.assertEqual(check_text(text), [])


class FailsClosed(unittest.TestCase):
    def test_no_spec_found_is_a_failure(self):
        import contextlib
        import io
        import os
        import tempfile

        from spec_check import main

        here, stderr = os.getcwd(), io.StringIO()
        with tempfile.TemporaryDirectory() as empty:
            os.chdir(empty)
            try:
                with contextlib.redirect_stderr(stderr):
                    self.assertEqual(main([]), 1)
            finally:
                os.chdir(here)
        self.assertIn("no spec found", stderr.getvalue())


if __name__ == "__main__":
    unittest.main()
