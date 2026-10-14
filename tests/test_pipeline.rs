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
        red_check: false,
        workspace: None,
        exec_cmd: None,
        json: false,
    };

    let res = run_pipeline(&opts).expect("run_pipeline should succeed");
    assert!(res.task_id.starts_with("TASK-"));
    assert!(res.jit_invariants.contains("JIT Invariant Activation"));
    assert_eq!(res.exec_passed, None);
    assert!(!res.verified);
    assert!(!res.tuple_settled);
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
fn test_run_pipeline_with_exec_cmd_integration() {
    let opts = PipelineOptions {
        task: "integration test exec command".to_string(),
        domain: "INT_EXEC_TEST".to_string(),
        node: None,
        verify: false,
        red_check: false,
        workspace: None,
        exec_cmd: Some("echo hello".to_string()),
        json: false,
    };

    let res = run_pipeline(&opts).expect("run_pipeline with exec_cmd should succeed");
    assert_eq!(res.exec_passed, Some(true));
    assert!(!res.verified);

    if res.tuple_id.is_some() {
        let _ = StdCommand::new("k0maru-router")
            .args([
                "tuple",
                "in",
                "--kind",
                "TASK_READY",
                "--domain",
                "INT_EXEC_TEST",
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
            "--exec",
            "echo hello",
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
    assert_eq!(parsed.exec_passed, Some(true));
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

#[test]
fn test_cli_run_with_red_check_flag() {
    let mut cmd = Command::cargo_bin("k0maru-harness").unwrap();
    let assert = cmd
        .args([
            "run",
            "test red check flag",
            "--domain",
            "CLI_RED_TEST",
            "--red-check",
            "--no-verify",
            "--json",
        ])
        .assert()
        .success();

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let parsed: PipelineResult =
        serde_json::from_str(&stdout).expect("Failed to deserialize PipelineResult from stdout");
    assert!(parsed.task_id.starts_with("TASK-"));

    let _ = StdCommand::new("k0maru-router")
        .args([
            "tuple",
            "in",
            "--kind",
            "TASK_READY",
            "--domain",
            "CLI_RED_TEST",
        ])
        .output();
}

#[test]
fn test_pre_stdout_log_sink_and_format_wire_summary() {
    let opts = PipelineOptions {
        task: "tick 005 log sink and wire summary".to_string(),
        domain: "TICK_005_LOG".to_string(),
        node: None,
        verify: false,
        red_check: false,
        workspace: None,
        exec_cmd: Some("echo hello-pre-stdout-sink".to_string()),
        json: false,
    };

    let res = run_pipeline(&opts).expect("run_pipeline should succeed");
    let log_file = res.log_path.as_ref().expect("log_path must be populated");
    assert!(log_file.exists(), "Log file must exist in ~/.k0maru/logs/");
    let log_contents = std::fs::read_to_string(log_file).unwrap();
    assert!(log_contents.contains("hello-pre-stdout-sink"));

    let wire = k0maru_harness::pipeline::format_wire_summary(&res);
    assert!(wire.starts_with(&format!("!Σ:PIPELINE|{}|", res.task_id)));
    assert!(wire.contains("|exec:PASS|verify:SKIP|settled:false|"));

    if res.tuple_id.is_some() {
        let _ = StdCommand::new("k0maru-router")
            .args([
                "tuple",
                "in",
                "--kind",
                "TASK_READY",
                "--domain",
                "TICK_005_LOG",
            ])
            .output();
    }
}

#[test]
fn test_inspect_receipt_wire_and_cli_subcommand() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp receipts dir");
    let receipt_file = temp_dir.path().join("TICK-005-MOCK.json");
    let sample_receipt = serde_json::json!({
        "task_id": "TICK-005-MOCK",
        "verdict": "success",
        "git_commit": "3775cf7abcdef123456",
        "signature_sha256": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
        "checks": {
            "tests": {
                "status": "passed",
                "exit_code": 0,
                "total": 30,
                "failed": 0
            },
            "red_check": {
                "status": "passed",
                "exit_code": 101
            }
        }
    });
    std::fs::write(&receipt_file, sample_receipt.to_string()).unwrap();

    let wire =
        k0maru_harness::pipeline::inspect_receipt_wire("TICK-005-MOCK", Some(temp_dir.path()))
            .expect("inspect_receipt_wire should succeed");
    assert_eq!(
        wire,
        "!Σ:RECEIPT|TICK-005-MOCK|status:PASS|red_check:PASS|commit:3775cf7abcde|sig:e3b0c44298fc"
    );
    assert!(wire.len() < 160);

    let mut cmd = Command::cargo_bin("k0maru-harness").unwrap();
    cmd.args([
        "receipt",
        "TICK-005-MOCK",
        "--dir",
        temp_dir.path().to_str().unwrap(),
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains(
        "!Σ:RECEIPT|TICK-005-MOCK|status:PASS|red_check:PASS|commit:3775cf7abcde|sig:e3b0c44298fc",
    ));
}

#[test]
fn test_cli_run_wire_flag() {
    let mut cmd = Command::cargo_bin("k0maru-harness").unwrap();
    let assert = cmd
        .args([
            "run",
            "cli wire test task",
            "--domain",
            "CLI_WIRE_TEST",
            "--exec",
            "echo wire-sink",
            "--no-verify",
            "--wire",
        ])
        .assert()
        .success();

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), 1, "--wire must output a single K-DSL line");
    assert!(lines[0].starts_with("!Σ:PIPELINE|TASK-"));
    assert!(lines[0].contains("|exec:PASS|verify:SKIP|"));

    let _ = StdCommand::new("k0maru-router")
        .args([
            "tuple",
            "in",
            "--kind",
            "TASK_READY",
            "--domain",
            "CLI_WIRE_TEST",
        ])
        .output();
}
