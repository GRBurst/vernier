//! The GitHub Actions example (spec 001 M5): it runs `vernier check` and cannot swallow its exit
//! code.

use std::path::Path;

/// The example workflow's text.
// why: a test helper; clippy's allow-unwrap-in-tests covers only `#[test]` items (audit 003).
#[allow(clippy::unwrap_used)]
fn example() -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("docs/examples/github-actions.yml");
    std::fs::read_to_string(path).unwrap()
}

/// The command of every `run:` step, without comments.
fn run_commands(workflow: &str) -> Vec<&str> {
    workflow
        .lines()
        .map(str::trim_start)
        .filter(|line| !line.starts_with('#'))
        .filter_map(|line| line.trim_start_matches("- ").strip_prefix("run:"))
        .map(str::trim)
        .collect()
}

/// Given the committed GitHub Actions example
/// When its steps are read
/// Then one step runs `vernier check`, no step sets `continue-on-error`, and no command masks a
/// non-zero exit (`|| true`, `|| exit 0`, `set +e`), so exit 1 fails the job
#[test]
fn the_github_actions_example_fails_the_job_on_a_flag() {
    let workflow = example();
    let commands = run_commands(&workflow);
    assert!(
        commands.iter().any(|c| c.starts_with("vernier check")),
        "{commands:?}"
    );
    let active: Vec<&str> = workflow
        .lines()
        .filter(|line| !line.trim_start().starts_with('#'))
        .collect();
    assert!(
        !active.iter().any(|line| line.contains("continue-on-error")),
        "{workflow}"
    );
    for masked in ["|| true", "|| exit 0", "set +e"] {
        assert!(
            !commands.iter().any(|c| c.contains(masked)),
            "{masked}: {commands:?}"
        );
    }
}
