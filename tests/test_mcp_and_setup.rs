use assert_cmd::Command;
use k0maru_harness::mcp::{handle_mcp_request, mcp_tool_definitions, serve_mcp_stdio};
use k0maru_harness::setup::run_setup_with_home;
use predicates::prelude::*;
use serde_json::{json, Value};
use std::io::Cursor;

#[test]
fn test_mcp_protocol_lifecycle_and_stdio() {
    let defs = mcp_tool_definitions();
    assert_eq!(defs.len(), 5);

    let input_lines = [
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}).to_string(),
        json!({"jsonrpc":"2.0","method":"notifications/initialized"}).to_string(),
        json!({"jsonrpc":"2.0","id":2,"method":"tools/list"}).to_string(),
        json!({
            "jsonrpc":"2.0",
            "id":3,
            "method":"tools/call",
            "params":{"name":"harness_probe","arguments":{}}
        })
        .to_string(),
    ]
    .join("\n")
        + "\n";

    let reader = Cursor::new(input_lines.into_bytes());
    let mut output = Vec::new();
    serve_mcp_stdio(reader, &mut output).expect("serve_mcp_stdio should succeed");

    let out_str = String::from_utf8(output).unwrap();
    let lines: Vec<&str> = out_str.lines().collect();
    // notifications/initialized produces no output line, so 3 responses expected
    assert_eq!(lines.len(), 3);

    let r1: Value = serde_json::from_str(lines[0]).unwrap();
    assert_eq!(r1["id"], 1);
    assert_eq!(r1["result"]["protocolVersion"], "2024-11-05");
    assert_eq!(r1["result"]["serverInfo"]["name"], "k0maru-harness");

    let r2: Value = serde_json::from_str(lines[1]).unwrap();
    assert_eq!(r2["id"], 2);
    assert_eq!(r2["result"]["tools"].as_array().unwrap().len(), 5);

    let r3: Value = serde_json::from_str(lines[2]).unwrap();
    assert_eq!(r3["id"], 3);
    assert_eq!(r3["result"]["isError"], false);
    let text = r3["result"]["content"][0]["text"].as_str().unwrap();
    assert!(text.starts_with("!Σ:PROBE|status:"));
}

#[test]
fn test_mcp_harness_receipt_tool_call() {
    let tmp = tempfile::tempdir().unwrap();
    let receipt_file = tmp.path().join("TICK-007.json");
    std::fs::write(
        &receipt_file,
        serde_json::to_string(&json!({
            "task_id": "TICK-007",
            "status": "PASS",
            "git_commit": "1122334455667788",
            "signature_sha256": "aabbccddeeff0011",
            "checks": {
                "test": { "exit_code": 0 },
                "red_check": { "status": "PASS" }
            }
        }))
        .unwrap(),
    )
    .unwrap();

    let req = json!({
        "jsonrpc": "2.0",
        "id": 99,
        "method": "tools/call",
        "params": {
            "name": "harness_receipt",
            "arguments": {
                "task_id": "TICK-007",
                "dir": tmp.path().to_string_lossy()
            }
        }
    });

    let resp = handle_mcp_request(&req).expect("response expected");
    assert_eq!(resp["result"]["isError"], false);
    assert_eq!(
        resp["result"]["content"][0]["text"].as_str().unwrap(),
        "!Σ:RECEIPT|TICK-007|status:PASS|red_check:PASS|commit:112233445566|sig:aabbccddeeff"
    );
}

#[test]
fn test_setup_with_temp_home_and_cli_mcp_subcommand() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path();
    let vault = home.join("Documents/SecondBrain");
    std::fs::create_dir_all(&vault).unwrap();

    let wire = run_setup_with_home(home, Some(&vault)).expect("setup should succeed");
    assert!(wire.starts_with("!Σ:SETUP|mcp_config:UPDATED|schemas:5|dirs:OK|probe:"));
    assert!(home.join(".gemini/config/mcp_config.json").is_file());
    assert!(home
        .join(".gemini/antigravity-cli/mcp/k0maru-harness/harness_run.json")
        .is_file());

    // Also test CLI `k0maru-harness mcp` via assert_cmd
    let mut cmd = Command::cargo_bin("k0maru-harness").unwrap();
    cmd.arg("mcp")
        .write_stdin("{\"jsonrpc\":\"2.0\",\"id\":7,\"method\":\"initialize\",\"params\":{}}\n")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "\"protocolVersion\":\"2024-11-05\"",
        ));
}
