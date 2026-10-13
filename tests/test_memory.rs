use assert_cmd::Command;
use k0maru_harness::memory::{clean_snippet, format_jit_prompt, JitActivationResult, JitHit};
use predicates::prelude::*;

#[test]
fn test_snippet_cleaning_and_prompt_formatting() {
    let raw_snippet =
        "  # Invariant Header \n\n All commits MUST pass guard check. \r\n Branch from dev. \t ";
    let cleaned = clean_snippet(raw_snippet);
    assert_eq!(
        cleaned,
        "# Invariant Header All commits MUST pass guard check. Branch from dev."
    );

    let hits = vec![
        JitHit {
            path: "10_Projects/Git.md".to_string(),
            title: "Git Invariants".to_string(),
            score: 0.98,
            snippet: raw_snippet.to_string(),
        },
        JitHit {
            path: "20_Cards/Locking.md".to_string(),
            title: "Concurrency".to_string(),
            score: 0.88,
            snippet: "Lock before write.".to_string(),
        },
    ];

    let prompt = format_jit_prompt("git workflow", &hits);
    assert!(prompt.starts_with("### 🧠 [JIT Invariant Activation: git workflow]"));
    assert!(prompt.contains("- [[10_Projects/Git.md]] (Git Invariants): # Invariant Header All commits MUST pass guard check. Branch from dev."));
    assert!(prompt.contains("- [[20_Cards/Locking.md]] (Concurrency): Lock before write."));
}

#[test]
fn test_token_estimation_consistency() {
    let prompt =
        "### 🧠 [JIT Invariant Activation: test]\n- [[test.md]] (Test): This is a short invariant.";
    let estimate = k0maru_harness::memory::estimate_tokens(prompt);
    assert_eq!(estimate, prompt.chars().count() / 4);
    assert!(estimate > 0);
}

#[test]
fn test_cli_memory_jit_json() {
    let mut cmd = Command::cargo_bin("k0maru-harness").unwrap();
    let assert = cmd
        .args(["memory", "jit", "--decision", "git branching", "--json"])
        .assert()
        .success();

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let parsed: JitActivationResult =
        serde_json::from_str(&stdout).expect("Failed to parse JSON output");

    assert_eq!(parsed.decision_point, "git branching");
    assert!(parsed
        .formatted_prompt
        .contains("JIT Invariant Activation: git branching"));
    assert_eq!(
        parsed.token_estimate,
        parsed.formatted_prompt.chars().count() / 4
    );
}

#[test]
fn test_cli_memory_jit_human_readable() {
    let mut cmd = Command::cargo_bin("k0maru-harness").unwrap();
    cmd.args(["memory", "jit", "--decision", "git branching"])
        .assert()
        .success()
        .stdout(predicate::str::starts_with(
            "### 🧠 [JIT Invariant Activation: git branching]",
        ));
}

#[test]
fn test_fallback_on_empty_query() {
    let result = k0maru_harness::memory::jit_activate("nonexistent_decision_xyz_999", 2, None);
    assert!(result.is_ok());
    let res = result.unwrap();
    assert_eq!(res.decision_point, "nonexistent_decision_xyz_999");
    assert!(res
        .formatted_prompt
        .contains("JIT Invariant Activation: nonexistent_decision_xyz_999"));
}
