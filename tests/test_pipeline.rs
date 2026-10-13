use assert_cmd::Command;
use predicates::prelude::*;
use std::process::Command as StdCommand;

use k0maru_harness::pipeline::{run_pipeline, PipelineOptions, PipelineResult};

#[test]
fn test_run_pipeline_programmatic() {
    let opts = PipelineOptions {
        task: "integration test programmatic execution".to_string(),
        domain: "INT_TEST".to_string(),
        node: None,
        verify: false,
        workspace: None,
        json: false,
    };

    let res = run_pipeline(&opts).expect("run_pipeline should succeed");
    assert!(res.task_id.starts_with("TASK-"));
    assert!(res.jit_invariants.contains("JIT Invariant Activation"));
    assert!(!res.verified);
    assert_eq!(res.receipt_summary, None);

    // If router emitted a tuple, clean it up
    if res.tuple_id.is_some() {
        let _ = StdCommand::new("k0maru-router")
            .args([
                "tuple",
                "in",
                "--kind",
                "TASK_READY",
                "--domain",
                "INT_TEST",
            ])
            .output();
    }
}

#[test]
fn test_cli_run_json_output() {
    let mut cmd = Command::cargo_bin("k0maru-harness").unwrap();
    let assert = cmd
        .args([
            "run",
            "cli json test task",
            "--domain",
            "CLI_JSON_TEST",
            "--no-verify",
            "--json",
        ])
        .assert()
        .success();

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let parsed: PipelineResult =
        serde_json::from_str(&stdout).expect("Failed to deserialize PipelineResult from stdout");

    assert!(parsed.task_id.starts_with("TASK-"));
    assert!(parsed
        .jit_invariants
        .contains("JIT Invariant Activation: cli json test task"));
    assert!(!parsed.verified);

    // Clean up emitted tuple if present
    if parsed.tuple_id.is_some() {
        let _ = StdCommand::new("k0maru-router")
            .args([
                "tuple",
                "in",
                "--kind",
                "TASK_READY",
                "--domain",
                "CLI_JSON_TEST",
            ])
            .output();
    }
}

#[test]
fn test_cli_run_human_readable() {
    let mut cmd = Command::cargo_bin("k0maru-harness").unwrap();
    cmd.args([
        "run",
        "cli human readable test",
        "--domain",
        "CLI_HUMAN_TEST",
        "--no-verify",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("Task Pipeline Execution"))
    .stdout(predicate::str::contains(
        "Phase 1: Component Readiness Probe",
    ))
    .stdout(predicate::str::contains(
        "Phase 2: JIT Invariant Activation",
    ))
    .stdout(predicate::str::contains("Phase 3: Linda Space Task Ready"))
    .stdout(predicate::str::contains("Phase 4: Physical Verification"));

    // Clean up emitted tuple if present
    let _ = StdCommand::new("k0maru-router")
        .args([
            "tuple",
            "in",
            "--kind",
            "TASK_READY",
            "--domain",
            "CLI_HUMAN_TEST",
        ])
        .output();
}
