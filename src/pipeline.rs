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

/// Runs a task through the complete end-to-end K0maru harness pipeline:
///
/// 1. Probes component availability across the 4 decoupled micro-kernels.
/// 2. Performs JIT invariant activation via cognitive memory hub (<180 tokens).
/// 3. Emits `TASK_READY` Linda tuple via `k0maru-router` (or gracefully degrades if unavailable).
/// 4. Optionally executes `opts.exec_cmd` via `sh -c` in `workspace_path` with JIT env vars.
/// 5. If `opts.verify` is true, executes deterministic physical verification via `k0maru-verifier`.
/// 6. If `verified` is true and `k0maru-router` is available, emits `TASK_COMPLETED` settlement tuple.
pub fn run_pipeline(opts: &PipelineOptions) -> Result<PipelineResult, HarnessError> {
    let task_id = format!("TASK-{}", chrono::Utc::now().format("%Y%m%d%H%M%S"));
    let workspace_path = opts.workspace.as_deref().unwrap_or_else(|| Path::new("."));

    // Phase 1: Probe component readiness
    let probe_report = crate::probe::ComponentProbe::run_full_probe();
    let probe_passed = probe_report.ready;

    // Phase 2: JIT Invariant Activation
    let jit_res = crate::memory::jit_activate(&opts.task, 3, None)?;
    let jit_invariants = jit_res.formatted_prompt;

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

    // Phase 3.5: Optional Command Execution (`--exec <CMD>`)
    let exec_passed = if let Some(ref cmd_str) = opts.exec_cmd {
        let output = Command::new("sh")
            .arg("-c")
            .arg(cmd_str)
            .current_dir(workspace_path)
            .env("K0MARU_TASK_ID", &task_id)
            .env("K0MARU_JIT_INVARIANTS", &jit_invariants)
            .output();

        match output {
            Ok(out) => Some(out.status.success()),
            Err(_) => Some(false),
        }
    } else {
        None
    };

    // Phase 4: Deterministic Physical Verification Gate
    let (verified, receipt_summary) = if opts.verify {
        if let Some(verifier_bin) = crate::probe::find_binary("k0maru-verifier") {
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
                Err(e) => (false, Some(format!("Failed to execute verifier: {}", e))),
            }
        } else {
            (false, Some("k0maru-verifier binary not found".to_string()))
        }
    } else {
        (false, None)
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
