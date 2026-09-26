#!/usr/bin/env python3
"""House-style linter for docs/specs/<NNN>-<slug>/spec.md.

Stdlib only. Fails closed: finding no spec is a failure, not a pass.
Exit 0 = clean, 1 = at least one error. Warnings never fail the gate.

Rules (id: what it refuses):
  spec-title          first line is not '# Spec <NNN> — <title>'
  spec-status         no '**Status:** Draft|Approved|Superseded' line
  spec-section        '**Scope:**' or '**Non-Goals:**' missing
  ms-heading          milestone heading not '## M<id> — <title> (Status: <S>)'
  ms-status           milestone status not PLANNED|IN PROGRESS|IMPLEMENTED|DONE
  ms-anchor           heading not followed by '<a id="M<id>"></a>'
  ms-duplicate        two milestones share an id
  ms-criteria         milestone has no '**Acceptance Criteria:**' checkbox
  ac-shall            criterion has no SHALL (EARS-style)
  ac-vague            criterion uses an unmeasurable adjective
  ms-done-unticked    DONE/IMPLEMENTED milestone has an unticked criterion
  clarify-approved    Approved spec still carries [NEEDS CLARIFICATION
  clarify-open        (warning) Draft spec carries [NEEDS CLARIFICATION
"""

from __future__ import annotations

import re
import sys
from dataclasses import dataclass
from pathlib import Path

SPEC_GLOB = "docs/specs/*/spec.md"
TITLE_RE = re.compile(r"^# Spec \d{3} — \S")
STATUS_RE = re.compile(r"^\*\*Status:\*\* (Draft|Approved|Superseded)\s*$")
MS_RE = re.compile(r"^## (M\d+[a-z]?) — (.+) \(Status: ([A-Z ]+)\)\s*$")
MS_STATUSES = {"PLANNED", "IN PROGRESS", "IMPLEMENTED", "DONE"}
BOX_RE = re.compile(r"^- \[( |x)\] (.*)$")
VAGUE = {
    "fast", "quick", "quickly", "simple", "easy", "easily", "robust",
    "efficient", "efficiently", "user-friendly", "intuitive", "appropriate",
    "appropriately", "reasonable", "adequate", "flexible", "seamless",
    "seamlessly", "scalable", "good", "nice", "clean",
}
CLARIFY = "[NEEDS CLARIFICATION"


@dataclass(frozen=True)
class Finding:
    line: int
    rule: str
    message: str
    severity: str = "error"


def _strip_code(text: str) -> str:
    """Drop inline code spans so backticked words are data, not prose."""
    return re.sub(r"`[^`]*`", "", text)


def _prose_lines(lines: list[str]) -> list[tuple[int, str]]:
    """Numbered lines outside fenced code blocks."""
    out, fenced = [], False
    for number, line in enumerate(lines, start=1):
        if line.lstrip().startswith("```"):
            fenced = not fenced
            continue
        if not fenced:
            out.append((number, line))
    return out


def _check_header(lines: list[str]) -> tuple[list[Finding], str | None]:
    findings = []
    if not lines or not TITLE_RE.match(lines[0]):
        findings.append(Finding(1, "spec-title", "first line must be '# Spec <NNN> — <title>'"))
    statuses = [m.group(1) for m in map(STATUS_RE.match, lines) if m]
    if not statuses:
        findings.append(Finding(1, "spec-status", "missing '**Status:** Draft|Approved|Superseded'"))
    for section in ("**Scope:**", "**Non-Goals:**"):
        if section not in lines:
            findings.append(Finding(1, "spec-section", f"missing {section} section"))
    return findings, (statuses[0] if statuses else None)


def _check_criterion(number: int, text: str) -> list[Finding]:
    findings = []
    prose = _strip_code(text)
    if "SHALL" not in prose:
        findings.append(Finding(number, "ac-shall", "criterion has no SHALL"))
    words = set(re.findall(r"[a-z][a-z-]*", prose.lower()))
    for word in sorted(words & VAGUE):
        findings.append(Finding(number, "ac-vague", f"unmeasurable word '{word}'"))
    return findings


def _check_milestone(block: list[tuple[int, str]]) -> list[Finding]:
    number, heading = block[0]
    match = MS_RE.match(heading)
    if not match:
        return [Finding(number, "ms-heading", "expected '## M<id> — <title> (Status: <S>)'")]
    ms_id, _, status = match.groups()
    findings = []
    if status not in MS_STATUSES:
        findings.append(Finding(number, "ms-status", f"'{status}' not in {sorted(MS_STATUSES)}"))
    nxt = next((line for _, line in block[1:] if line.strip()), "")
    if nxt.strip() != f'<a id="{ms_id}"></a>':
        findings.append(Finding(number, "ms-anchor", f'next line must be <a id="{ms_id}"></a>'))
    boxes = [(n, BOX_RE.match(line)) for n, line in block if BOX_RE.match(line)]
    has_section = any(line.strip() == "**Acceptance Criteria:**" for _, line in block)
    if not has_section or not boxes:
        findings.append(Finding(number, "ms-criteria", f"{ms_id} has no acceptance criteria"))
    for n, box in boxes:
        findings += _check_criterion(n, box.group(2))
        if status in {"DONE", "IMPLEMENTED"} and box.group(1) == " ":
            findings.append(Finding(n, "ms-done-unticked", f"{ms_id} is {status} but this is unticked"))
    return findings


def _milestone_blocks(prose: list[tuple[int, str]]) -> list[list[tuple[int, str]]]:
    blocks: list[list[tuple[int, str]]] = []
    for number, line in prose:
        if line.startswith("## "):
            blocks.append([(number, line)])
        elif blocks:
            blocks[-1].append((number, line))
    return [b for b in blocks if re.match(r"^## M\d", b[0][1])]


def check_text(text: str) -> list[Finding]:
    lines = text.splitlines()
    findings, status = _check_header(lines)
    prose = _prose_lines(lines)
    seen: dict[str, int] = {}
    for block in _milestone_blocks(prose):
        findings += _check_milestone(block)
        match = MS_RE.match(block[0][1])
        if match:
            ms_id = match.group(1)
            if ms_id in seen:
                findings.append(Finding(block[0][0], "ms-duplicate", f"{ms_id} also at line {seen[ms_id]}"))
            seen.setdefault(ms_id, block[0][0])
    for number, line in prose:
        if CLARIFY in _strip_code(line):
            if status == "Approved":
                findings.append(Finding(number, "clarify-approved", "Approved spec has an open question"))
            else:
                findings.append(Finding(number, "clarify-open", "open question", "warning"))
    return sorted(findings, key=lambda f: f.line)


def main(argv: list[str]) -> int:
    paths = [Path(p) for p in argv] or sorted(Path(".").glob(SPEC_GLOB))
    if not paths:
        print(f"FAIL: no spec found ({SPEC_GLOB})", file=sys.stderr)
        return 1
    errors = 0
    for path in paths:
        for f in check_text(path.read_text(encoding="utf-8")):
            print(f"{path}:{f.line}: {f.severity}[{f.rule}] {f.message}")
            errors += f.severity == "error"
    print(f"spec-check: {len(paths)} file(s), {errors} error(s)")
    return 1 if errors else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
