use std::path::{Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};

use crate::error::HarnessError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineOptions {
    pub task: String,
    pub domain: String,
    pub node: Option<String>,
    pub verify: bool,
    pub red_check: bool,
    pub workspace: Option<PathBuf>,
    #[serde(default)]
    pub exec_cmd: Option<String>,
    #[serde(default)]
    pub vault: Option<PathBuf>,
    pub json: bool,
}

impl Default for PipelineOptions {
    fn default() -> Self {
        Self {
            task: String::new(),
            domain: "DEFAULT".to_string(),
            node: None,
            verify: true,
            red_check: false,
            workspace: None,
            exec_cmd: None,
            vault: None,
            json: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineResult {
    pub task_id: String,
    pub probe_passed: bool,
    pub jit_invariants: String,
    pub tuple_id: Option<String>,
    #[serde(default)]
    pub exec_passed: Option<bool>,
    pub verified: bool,
    pub receipt_summary: Option<String>,
    #[serde(default)]
    pub tuple_settled: bool,
    #[serde(default)]
    pub log_path: Option<PathBuf>,
    #[serde(default)]
    pub lesson_recorded: Option<String>,
}

fn extract_tuple_id(text: &str) -> Option<String> {
    for line in text.lines() {
        if let Some(pos) = line.find("id=") {
            let rest = &line[pos + 3..];
            let id = rest.split_whitespace().next().unwrap_or("");
            if !id.is_empty() {
                return Some(id.to_string());
            }
        }
    }
    None
}

fn resolve_logs_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".k0maru")
        .join("logs")
}

fn resolve_default_receipts_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".k0maru")
        .join("receipts")
}

/// Classifies a pipeline failure into `(title, cause, standard)` for automatic `LESSONS.md` recording.
pub fn classify_pipeline_failure(
    domain: &str,
    exec_passed: Option<bool>,
    verified: bool,
    receipt_summary: Option<&str>,
    log_buffer: &str,
) -> Option<(String, String, String)> {
    if exec_passed == Some(false) {
        Some((
            format!("[{}] 子进程执行失败 (Exec Subprocess Failed)", domain),
            format!(
                "Pipeline --exec command exited with non-zero status in domain {}",
                domain
            ),
            "Ensure subprocess command and environment invariants pass before pipeline execution"
                .to_string(),
        ))
    } else if !verified && receipt_summary.is_some() {
        if receipt_summary.unwrap_or("").contains("red-check: failed")
            || log_buffer.contains("Popperian falsification failed")
        {
            Some((
                format!(
                    "[{}] 波普尔反向证伪失败 (Red-Check Falsification Failed)",
                    domain
                ),
                "New or modified tests passed on reverted baseline or contained hollow/tautological assertions"
                    .to_string(),
                "Write genuine falsifiable assertions that fail on baseline and pass on new implementation"
                    .to_string(),
            ))
        } else {
            Some((
                format!("[{}] 物理预言机验证未通过 (Verifier Gate Blocked)", domain),
                format!(
                    "k0maru-verifier blocked task in domain {}: {}",
                    domain,
                    receipt_summary.unwrap_or("unknown")
                ),
                "Fix compile/clippy/test failures and verify locally before submission".to_string(),
            ))
        }
    } else {
        None
    }
}

/// Formats a `PipelineResult` into a single-line K-DSL wire summary:
/// `!Σ:PIPELINE|<task_id>|probe:<OK/FAIL>|exec:<PASS/FAIL/SKIP>|verify:<PASS/FAIL/SKIP>|settled:<bool>|log:<log_path>|receipt:<receipt_summary>|lesson:<NONE_or_lesson_recorded>`
pub fn format_wire_summary(res: &PipelineResult) -> String {
    let probe_str = if res.probe_passed { "OK" } else { "FAIL" };
    let exec_str = match res.exec_passed {
        Some(true) => "PASS",
        Some(false) => "FAIL",
        None => "SKIP",
    };
    let verify_str = if res.verified {
        "PASS"
    } else if res.receipt_summary.is_some() {
        "FAIL"
    } else {
        "SKIP"
    };
    let log_str = res
        .log_path
        .as_ref()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "NONE".to_string());
    let receipt_str = res
        .receipt_summary
        .as_deref()
        .unwrap_or("NONE")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .replace('|', "/");
    let lesson_str = res.lesson_recorded.as_deref().unwrap_or("NONE");

    format!(
        "!Σ:PIPELINE|{}|probe:{}|exec:{}|verify:{}|settled:{}|log:{}|receipt:{}|lesson:{}",
        res.task_id,
        probe_str,
        exec_str,
        verify_str,
        res.tuple_settled,
        log_str,
        receipt_str,
        lesson_str
    )
}

/// Reads `<receipts_dir_or_~/.k0maru/receipts>/<task_id>.json` and formats a compact (<160 chars)
/// single-line K-DSL wire summary:
/// `!Σ:RECEIPT|<task_id>|status:<PASS/FAIL>|red_check:<status>|commit:<git_commit>|sig:<short_sha256>`
pub fn inspect_receipt_wire(
    task_id: &str,
    receipts_dir: Option<&Path>,
) -> Result<String, HarnessError> {
    let base_dir = receipts_dir
        .map(|p| p.to_path_buf())
        .unwrap_or_else(resolve_default_receipts_dir);
    let clean_id = task_id.trim_end_matches(".json");
    let receipt_path = base_dir.join(format!("{}.json", clean_id));

    let raw = std::fs::read_to_string(&receipt_path)?;
    let json: serde_json::Value = serde_json::from_str(&raw)?;

    let resolved_task_id = json
        .get("task_id")
        .and_then(|v| v.as_str())
        .unwrap_or(clean_id);

    let raw_status = json
        .get("status")
        .or_else(|| json.get("verdict"))
        .and_then(|v| v.as_str())
        .unwrap_or("FAIL");
    let test_exit_code = json
        .pointer("/checks/test/exit_code")
        .or_else(|| json.pointer("/checks/tests/exit_code"))
        .and_then(|v| v.as_i64())
        .unwrap_or(0);

    let status_pass = matches!(
        raw_status.to_ascii_lowercase().as_str(),
        "pass" | "passed" | "success" | "ok"
    ) && test_exit_code == 0;
    let status_str = if status_pass { "PASS" } else { "FAIL" };

    let red_check_raw = json
        .pointer("/checks/red_check/status")
        .and_then(|v| v.as_str())
        .unwrap_or("SKIP");
    let red_check_str = match red_check_raw.to_ascii_lowercase().as_str() {
        "pass" | "passed" | "success" | "ok" => "PASS".to_string(),
        "fail" | "failed" | "error" => "FAIL".to_string(),
        "skip" | "skipped" | "none" => "SKIP".to_string(),
        other => other.to_ascii_uppercase(),
    };

    let commit_raw = json
        .get("git_commit")
        .or_else(|| json.pointer("/artifacts/git_commit"))
        .and_then(|v| v.as_str())
        .unwrap_or("HEAD");
    let commit_short: String = commit_raw.chars().take(12).collect();

    let sig_raw = json
        .get("signature_sha256")
        .or_else(|| json.pointer("/artifacts/signature_sha256"))
        .or_else(|| json.pointer("/artifacts/git_patch_sha256"))
        .and_then(|v| v.as_str())
        .unwrap_or("none");
    let short_sig: String = sig_raw.chars().take(12).collect();

    Ok(format!(
        "!Σ:RECEIPT|{}|status:{}|red_check:{}|commit:{}|sig:{}",
        resolved_task_id, status_str, red_check_str, commit_short, short_sig
    ))
}

/// Runs a task through the complete end-to-end K0maru harness pipeline:
///
/// 1. Probes component availability across the 4 decoupled micro-kernels.
/// 2. Performs JIT invariant activation via cognitive memory hub (<180 tokens).
/// 3. Emits `TASK_READY` Linda tuple via `k0maru-router` (or gracefully degrades if unavailable).
/// 4. Optionally executes `opts.exec_cmd` via `sh -c` in `workspace_path` with JIT env vars, sinking stdout/stderr to `~/.k0maru/logs/<task_id>.log`.
/// 5. If `opts.verify` is true, executes deterministic physical verification via `k0maru-verifier`, sinking stdout/stderr to `~/.k0maru/logs/<task_id>.log`.
/// 6. Automatically captures failure lessons into `LESSONS.md` (`Seen 3×` state machine) when execution or verification fails.
/// 7. If `verified` is true and `k0maru-router` is available, emits `TASK_COMPLETED` settlement tuple.
pub fn run_pipeline(opts: &PipelineOptions) -> Result<PipelineResult, HarnessError> {
    static SEQ: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let seq = SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let now = chrono::Utc::now();
    let task_id = format!(
        "TASK-{}-{:03}-{}",
        now.format("%Y%m%d%H%M%S"),
        now.timestamp_subsec_millis(),
        seq
    );
    let workspace_path = opts.workspace.as_deref().unwrap_or_else(|| Path::new("."));

    // Prepare Pre-Stdout Log Sink (`~/.k0maru/logs/<task_id>.log`)
    let logs_dir = resolve_logs_dir();
    let _ = std::fs::create_dir_all(&logs_dir);
    let log_file_path = logs_dir.join(format!("{}.log", task_id));
    let mut log_buffer = format!(
        "=== [K0maru-Harness Pre-Stdout Log Sink] task_id={} domain={} ===\n",
        task_id, opts.domain
    );

    // Phase 1: Probe component readiness
    let probe_report = crate::probe::ComponentProbe::run_full_probe();
    let probe_passed = probe_report.ready;

    // Phase 2: JIT Invariant Activation
    let jit_res = crate::memory::jit_activate(&opts.task, 3, opts.vault.as_deref())?;
    let jit_invariants = jit_res.formatted_prompt;
    log_buffer.push_str(&format!(
        "\n--- [Phase 2: JIT Invariants] ---\n{}\n",
        jit_invariants
    ));

    // Phase 3: Linda Space Task Ready Tuple Emission & Multiplexer Pane Probe (`--driver auto`)
    let router_bin_opt = crate::probe::find_binary("k0maru-router");
    let tuple_id = if let Some(ref router_bin) = router_bin_opt {
        let mux_probe = if let Some(ref ws) = opts.workspace {
            let mut probe_cmd = Command::new(router_bin);
            probe_cmd
                .arg("probe")
                .arg("--driver")
                .arg("auto")
                .arg("--workspace")
                .arg(ws)
                .arg("--json");
            if let Some(ref n) = opts.node {
                probe_cmd.arg("--node").arg(n);
            }
            probe_cmd
                .output()
                .ok()
                .filter(|o| o.status.success())
                .and_then(|o| serde_json::from_slice::<serde_json::Value>(&o.stdout).ok())
        } else {
            None
        };

        let payload = serde_json::json!({
            "task": opts.task,
            "domain": opts.domain,
            "node": opts.node,
            "workspace": opts.workspace.as_ref().map(|w| w.display().to_string()),
            "mux_probe": mux_probe,
            "invariants": jit_invariants,
        });
        let payload_str = payload.to_string();

        let output = Command::new(router_bin)
            .arg("tuple")
            .arg("out")
            .arg("--kind")
            .arg("TASK_READY")
            .arg("--domain")
            .arg(&opts.domain)
            .arg("--task-id")
            .arg(&task_id)
            .arg("--payload")
            .arg(&payload_str)
            .arg("--json")
            .output();

        match output {
            Ok(out) if out.status.success() => {
                if let Ok(v) = serde_json::from_slice::<serde_json::Value>(&out.stdout) {
                    v.get("id")
                        .and_then(|id| id.as_str())
                        .map(|s| s.to_string())
                } else {
                    let txt = String::from_utf8_lossy(&out.stdout);
                    extract_tuple_id(&txt)
                }
            }
            Ok(out) => {
                let txt = String::from_utf8_lossy(&out.stdout);
                extract_tuple_id(&txt)
            }
            Err(_) => None,
        }
    } else {
        None
    };

    // Phase 3.5: Optional Command Execution (`--exec <CMD>`) — Sunk to log_buffer
    let exec_passed = if let Some(ref cmd_str) = opts.exec_cmd {
        log_buffer.push_str(&format!("\n--- [Phase 3.5: Exec '{}'] ---\n", cmd_str));
        let output = Command::new("sh")
            .arg("-c")
            .arg(cmd_str)
            .current_dir(workspace_path)
            .env("K0MARU_TASK_ID", &task_id)
            .env("K0MARU_JIT_INVARIANTS", &jit_invariants)
            .output();

        match output {
            Ok(out) => {
                log_buffer.push_str(&String::from_utf8_lossy(&out.stdout));
                log_buffer.push_str(&String::from_utf8_lossy(&out.stderr));
                Some(out.status.success())
            }
            Err(e) => {
                log_buffer.push_str(&format!("Exec error: {}\n", e));
                Some(false)
            }
        }
    } else {
        None
    };

    // Phase 4: Deterministic Physical Verification Gate — Sunk to log_buffer
    let (verified, receipt_summary) = if opts.verify {
        if let Some(verifier_bin) = crate::probe::find_binary("k0maru-verifier") {
            log_buffer.push_str("\n--- [Phase 4: k0maru-verifier] ---\n");
            let mut cmd = Command::new(&verifier_bin);
            cmd.arg("verify")
                .arg("--task-id")
                .arg(&task_id)
                .arg("--workspace")
                .arg(workspace_path)
                .arg("--json");

            if opts.red_check {
                cmd.arg("--red-check");
            }

            match cmd.output() {
                Ok(out) => {
                    log_buffer.push_str(&String::from_utf8_lossy(&out.stdout));
                    log_buffer.push_str(&String::from_utf8_lossy(&out.stderr));
                    let success = out.status.success();
                    let summary = if let Ok(json) =
                        serde_json::from_slice::<serde_json::Value>(&out.stdout)
                    {
                        let verdict = json
                            .get("verdict")
                            .and_then(|v| v.as_str())
                            .unwrap_or(if success { "success" } else { "failure" });
                        let total_tests =
                            json.pointer("/checks/tests/total").and_then(|v| v.as_u64());
                        let failed_tests = json
                            .pointer("/checks/tests/failed")
                            .and_then(|v| v.as_u64());
                        let red_status = json
                            .pointer("/checks/red_check/status")
                            .and_then(|v| v.as_str());
                        let red_suffix = match red_status {
                            Some(s) => format!(", red-check: {s}"),
                            None => String::new(),
                        };
                        match (total_tests, failed_tests) {
                            (Some(total), Some(failed)) => format!(
                                "Verdict: {}, tests: {}/{} passed{}",
                                verdict,
                                total.saturating_sub(failed),
                                total,
                                red_suffix
                            ),
                            _ => format!("Verdict: {}{}", verdict, red_suffix),
                        }
                    } else {
                        let txt = String::from_utf8_lossy(&out.stdout).trim().to_string();
                        if !txt.is_empty() {
                            txt
                        } else if success {
                            "Verification passed".to_string()
                        } else {
                            format!("Verification failed with status: {}", out.status)
                        }
                    };
                    (success, Some(summary))
                }
                Err(e) => {
                    log_buffer.push_str(&format!("Failed to execute verifier: {}\n", e));
                    (false, Some(format!("Failed to execute verifier: {}", e)))
                }
            }
        } else {
            (false, Some("k0maru-verifier binary not found".to_string()))
        }
    } else {
        (false, None)
    };

    // Phase 4.5: Automatic Lesson Capture (`LESSONS.md` Seen-3x Engine) on Failure
    let mut lesson_recorded = None;
    if exec_passed == Some(false) || (opts.verify && !verified) {
        let should_record = opts.vault.is_some()
            || std::env::var("K0MARU_VAULT_PATH").is_ok()
            || std::env::var("CARGO").is_err();
        if should_record {
            if let Some((title, cause, standard)) = classify_pipeline_failure(
                &opts.domain,
                exec_passed,
                verified,
                receipt_summary.as_deref(),
                &log_buffer,
            ) {
                if let Ok(rec) =
                    crate::memory::record_lesson(opts.vault.as_deref(), &title, &cause, &standard)
                {
                    let badge = format!("Seen_{}x(promoted:{})", rec.count, rec.promoted);
                    log_buffer.push_str(&format!(
                        "\n--- [Phase 4.5: Auto Lesson Recorded] {} -> {} ---\n",
                        rec.title, badge
                    ));
                    lesson_recorded = Some(badge);
                }
            }
        }
    }

    let log_path = if std::fs::write(&log_file_path, &log_buffer).is_ok() {
        Some(log_file_path)
    } else {
        None
    };

    // Phase 5: Linda Tuple Settlement (`TASK_COMPLETED`) when verified
    let tuple_settled = if verified {
        if let Some(ref router_bin) = router_bin_opt {
            let settle_payload = serde_json::json!({
                "task": opts.task,
                "domain": opts.domain,
                "verified": verified,
                "exec_passed": exec_passed,
                "receipt_summary": receipt_summary,
                "log_path": log_path,
                "lesson_recorded": lesson_recorded,
            });
            let out = Command::new(router_bin)
                .arg("tuple")
                .arg("out")
                .arg("--kind")
                .arg("TASK_COMPLETED")
                .arg("--domain")
                .arg(&opts.domain)
                .arg("--task-id")
                .arg(&task_id)
                .arg("--payload")
                .arg(settle_payload.to_string())
                .arg("--json")
                .output();
            matches!(out, Ok(o) if o.status.success())
        } else {
            false
        }
    } else {
        false
    };

    Ok(PipelineResult {
        task_id,
        probe_passed,
        jit_invariants,
        tuple_id,
        exec_passed,
        verified,
        receipt_summary,
        tuple_settled,
        log_path,
        lesson_recorded,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pipeline_options_default() {
        let opts = PipelineOptions::default();
        assert_eq!(opts.domain, "DEFAULT");
        assert!(opts.verify);
        assert!(opts.exec_cmd.is_none());
        assert!(opts.vault.is_none());
        assert!(!opts.json);
    }

    #[test]
    fn test_extract_tuple_id() {
        let text = "TUPLE OUT: id=tup_123456 kind=TASK_READY domain=TEST task_id=TASK-001\n";
        assert_eq!(extract_tuple_id(text), Some("tup_123456".to_string()));
        let empty = "no tuple here";
        assert_eq!(extract_tuple_id(empty), None);
    }

    #[test]
    fn test_run_pipeline_no_verify() {
        let opts = PipelineOptions {
            task: "unit test task".to_string(),
            domain: "HARNESS_TEST".to_string(),
            node: None,
            verify: false,
            red_check: false,
            workspace: None,
            exec_cmd: None,
            vault: None,
            json: false,
        };
        let res = run_pipeline(&opts);
        assert!(res.is_ok());
        let result = res.unwrap();
        assert!(result.task_id.starts_with("TASK-"));
        assert!(result.jit_invariants.contains("JIT Invariant Activation"));
        assert_eq!(result.exec_passed, None);
        assert!(!result.verified);
        assert!(!result.tuple_settled);
        assert_eq!(result.receipt_summary, None);
        assert_eq!(result.lesson_recorded, None);

        if result.tuple_id.is_some() {
            let _ = Command::new("k0maru-router")
                .args([
                    "tuple",
                    "in",
                    "--kind",
                    "TASK_READY",
                    "--domain",
                    "HARNESS_TEST",
                ])
                .output();
        }
    }

    #[test]
    fn test_run_pipeline_with_exec_cmd() {
        let opts = PipelineOptions {
            task: "unit test exec cmd".to_string(),
            domain: "HARNESS_EXEC_TEST".to_string(),
            node: None,
            verify: false,
            red_check: false,
            workspace: None,
            exec_cmd: Some("echo hello".to_string()),
            vault: None,
            json: false,
        };
        let res = run_pipeline(&opts).expect("run_pipeline with exec_cmd should succeed");
        assert_eq!(res.exec_passed, Some(true));
        assert!(!res.verified);

        if res.tuple_id.is_some() {
            let _ = Command::new("k0maru-router")
                .args([
                    "tuple",
                    "in",
                    "--kind",
                    "TASK_READY",
                    "--domain",
                    "HARNESS_EXEC_TEST",
                ])
                .output();
        }
    }
}
