use std::io::{BufRead, Write};
use std::path::PathBuf;

use serde_json::{json, Value};

use crate::error::HarnessError;
use crate::memory::{jit_activate, record_lesson};
use crate::pipeline::{format_wire_summary, inspect_receipt_wire, run_pipeline, PipelineOptions};
use crate::probe::{Availability, ComponentProbe};

/// Returns the list of 5 compact K-DSL MCP tool definitions.
pub fn mcp_tool_definitions() -> Vec<Value> {
    vec![
        json!({
            "name": "harness_probe",
            "description": "Check readiness of all 4 K0maru micro-kernels, Linda tuple bus, and receipts ledger.",
            "inputSchema": {
                "type": "object",
                "properties": {},
                "required": []
            }
        }),
        json!({
            "name": "harness_jit",
            "description": "Activate JIT architectural invariants and lessons (<180 tokens) for a decision point.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "decision": {
                        "type": "string",
                        "description": "Decision point query string (e.g. 'git branching', 'sqlite lock')"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Maximum number of hits to retrieve (default: 3)"
                    }
                },
                "required": ["decision"]
            }
        }),
        json!({
            "name": "harness_record_lesson",
            "description": "Record or increment a failure lesson in LESSONS.md with Seen-3x auto-promotion to Living ADR.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "title": {
                        "type": "string",
                        "description": "Lesson title or anti-pattern summary"
                    },
                    "cause": {
                        "type": "string",
                        "description": "Incident description and root cause"
                    },
                    "standard": {
                        "type": "string",
                        "description": "Standard for doing it right (Done right)"
                    }
                },
                "required": ["title", "cause", "standard"]
            }
        }),
        json!({
            "name": "harness_run",
            "description": "Execute end-to-end harness pipeline with pre-stdout log sinking and 1-line K-DSL wire return (!Σ:PIPELINE|...).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "task": {
                        "type": "string",
                        "description": "Task description or specification"
                    },
                    "domain": {
                        "type": "string",
                        "description": "Target domain identifier (e.g. HARNESS, ROUTER)"
                    },
                    "exec_cmd": {
                        "type": "string",
                        "description": "Optional shell command to execute in workspace with JIT invariants injected"
                    },
                    "verify": {
                        "type": "boolean",
                        "description": "Whether to run deterministic physical verification via k0maru-verifier (default: true)"
                    },
                    "red_check": {
                        "type": "boolean",
                        "description": "Whether to enforce Popperian physical falsification (default: false)"
                    },
                    "workspace": {
                        "type": "string",
                        "description": "Optional target workspace directory path"
                    }
                },
                "required": ["task", "domain"]
            }
        }),
        json!({
            "name": "harness_receipt",
            "description": "Inspect and verify a task's cryptographic receipt in 1-line !Σ:RECEIPT|... wire format.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "task_id": {
                        "type": "string",
                        "description": "Task identifier (e.g. TICK-007)"
                    },
                    "dir": {
                        "type": "string",
                        "description": "Optional directory override for receipts (defaults to ~/.k0maru/receipts)"
                    }
                },
                "required": ["task_id"]
            }
        }),
    ]
}

fn format_probe_wire() -> String {
    let report = ComponentProbe::run_full_probe();
    let status = if report.ready { "READY" } else { "DEGRADED" };
    let comp_summary: Vec<String> = report
        .components
        .iter()
        .map(|c| {
            let ok = matches!(c.availability, Availability::Available { .. });
            format!(
                "{}:{}",
                c.name.to_ascii_lowercase(),
                if ok { "OK" } else { "MISSING" }
            )
        })
        .collect();
    let bus_ok = report.buses.iter().all(|b| b.exists && b.writable);
    format!(
        "!Σ:PROBE|status:{}|{}|buses:{}",
        status,
        comp_summary.join("|"),
        if bus_ok { "OK" } else { "DEGRADED" }
    )
}

fn dispatch_tool_call(name: &str, args: &Value) -> Result<String, String> {
    match name {
        "harness_probe" => Ok(format_probe_wire()),
        "harness_jit" => {
            let decision = args
                .get("decision")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing required parameter 'decision'".to_string())?;
            let limit = args
                .get("limit")
                .and_then(|v| v.as_u64())
                .map(|v| v as usize)
                .unwrap_or(3);
            let vault = args
                .get("vault")
                .and_then(|v| v.as_str())
                .map(PathBuf::from);
            let res = jit_activate(decision, limit, vault.as_deref())
                .map_err(|e| format!("harness_jit error: {e}"))?;
            Ok(res.formatted_prompt)
        }
        "harness_record_lesson" => {
            let title = args
                .get("title")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing required parameter 'title'".to_string())?;
            let cause = args
                .get("cause")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing required parameter 'cause'".to_string())?;
            let standard = args
                .get("standard")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing required parameter 'standard'".to_string())?;
            let vault = args
                .get("vault")
                .and_then(|v| v.as_str())
                .map(PathBuf::from);
            let res = record_lesson(vault.as_deref(), title, cause, standard)
                .map_err(|e| format!("harness_record_lesson error: {e}"))?;
            Ok(format!(
                "!Σ:LESSON|{}|count:{}|promoted:{}|path:{}",
                res.title,
                res.count,
                res.promoted,
                res.lessons_path.display()
            ))
        }
        "harness_run" => {
            let task = args
                .get("task")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing required parameter 'task'".to_string())?
                .to_string();
            let domain = args
                .get("domain")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing required parameter 'domain'".to_string())?
                .to_string();
            let exec_cmd = args
                .get("exec_cmd")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let verify = args.get("verify").and_then(|v| v.as_bool()).unwrap_or(true);
            let red_check = args
                .get("red_check")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let workspace = args
                .get("workspace")
                .and_then(|v| v.as_str())
                .map(PathBuf::from);
            let vault = args
                .get("vault")
                .and_then(|v| v.as_str())
                .map(PathBuf::from);

            let opts = PipelineOptions {
                task,
                domain,
                node: None,
                verify,
                red_check,
                workspace,
                exec_cmd,
                vault,
                json: false,
            };
            let res = run_pipeline(&opts).map_err(|e| format!("harness_run error: {e}"))?;
            Ok(format_wire_summary(&res))
        }
        "harness_receipt" => {
            let task_id = args
                .get("task_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing required parameter 'task_id'".to_string())?;
            let dir = args.get("dir").and_then(|v| v.as_str()).map(PathBuf::from);
            inspect_receipt_wire(task_id, dir.as_deref())
                .map_err(|e| format!("harness_receipt error: {e}"))
        }
        other => Err(format!("Unknown tool: {other}")),
    }
}

/// Processes a single JSON-RPC 2.0 request and returns an optional response `Value`
/// (`None` for notifications such as `notifications/initialized`).
pub fn handle_mcp_request(req: &Value) -> Option<Value> {
    let method = req.get("method").and_then(|m| m.as_str()).unwrap_or("");
    let id = req.get("id").cloned();

    if method == "notifications/initialized" || method.starts_with("notifications/") {
        return None;
    }

    let id_val = id.unwrap_or(Value::Null);

    match method {
        "initialize" => Some(json!({
            "jsonrpc": "2.0",
            "id": id_val,
            "result": {
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "tools": {}
                },
                "serverInfo": {
                    "name": "k0maru-harness",
                    "version": "0.1.0"
                }
            }
        })),
        "tools/list" => Some(json!({
            "jsonrpc": "2.0",
            "id": id_val,
            "result": {
                "tools": mcp_tool_definitions()
            }
        })),
        "tools/call" => {
            let params = req.get("params").cloned().unwrap_or_else(|| json!({}));
            let tool_name = params.get("name").and_then(|n| n.as_str()).unwrap_or("");
            let args = params
                .get("arguments")
                .cloned()
                .unwrap_or_else(|| json!({}));

            match dispatch_tool_call(tool_name, &args) {
                Ok(text) => Some(json!({
                    "jsonrpc": "2.0",
                    "id": id_val,
                    "result": {
                        "content": [
                            {
                                "type": "text",
                                "text": text
                            }
                        ],
                        "isError": false
                    }
                })),
                Err(err_msg) => Some(json!({
                    "jsonrpc": "2.0",
                    "id": id_val,
                    "result": {
                        "content": [
                            {
                                "type": "text",
                                "text": err_msg
                            }
                        ],
                        "isError": true
                    }
                })),
            }
        }
        "ping" => Some(json!({
            "jsonrpc": "2.0",
            "id": id_val,
            "result": {}
        })),
        _ => Some(json!({
            "jsonrpc": "2.0",
            "id": id_val,
            "error": {
                "code": -32601,
                "message": format!("Method not found: {}", method)
            }
        })),
    }
}

/// Runs the JSON-RPC 2.0 MCP server over arbitrary `BufRead` / `Write` streams.
pub fn serve_mcp_stdio<R: BufRead, W: Write>(reader: R, mut writer: W) -> Result<(), HarnessError> {
    for line in reader.lines() {
        let line = line?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let parsed: Value = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(e) => {
                let err_resp = json!({
                    "jsonrpc": "2.0",
                    "id": null,
                    "error": {
                        "code": -32700,
                        "message": format!("Parse error: {}", e)
                    }
                });
                writeln!(writer, "{}", err_resp)?;
                writer.flush()?;
                continue;
            }
        };

        if let Some(resp) = handle_mcp_request(&parsed) {
            writeln!(writer, "{}", serde_json::to_string(&resp)?)?;
            writer.flush()?;
        }
    }
    Ok(())
}

/// Runs the JSON-RPC 2.0 MCP server over standard input and standard output.
pub fn run_mcp_server() -> Result<(), HarnessError> {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    serve_mcp_stdio(stdin.lock(), stdout.lock())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mcp_initialize_and_notifications() {
        let init_req = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {}
        });
        let resp = handle_mcp_request(&init_req).expect("initialize must return response");
        assert_eq!(resp["id"], 1);
        assert_eq!(resp["result"]["protocolVersion"], "2024-11-05");
        assert_eq!(resp["result"]["serverInfo"]["name"], "k0maru-harness");
        assert_eq!(resp["result"]["serverInfo"]["version"], "0.1.0");

        let notif = json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized"
        });
        assert!(handle_mcp_request(&notif).is_none());
    }

    #[test]
    fn test_mcp_tools_list_exposes_5_tools() {
        let list_req = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list"
        });
        let resp = handle_mcp_request(&list_req).expect("tools/list must return response");
        let tools = resp["result"]["tools"]
            .as_array()
            .expect("tools must be array");
        assert_eq!(tools.len(), 5);
        let names: Vec<&str> = tools.iter().filter_map(|t| t["name"].as_str()).collect();
        assert_eq!(
            names,
            vec![
                "harness_probe",
                "harness_jit",
                "harness_record_lesson",
                "harness_run",
                "harness_receipt"
            ]
        );
    }

    #[test]
    fn test_mcp_tools_call_probe_and_receipt() {
        // 1. Test harness_probe via tools/call
        let probe_req = json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": {
                "name": "harness_probe",
                "arguments": {}
            }
        });
        let probe_resp = handle_mcp_request(&probe_req).expect("harness_probe response");
        assert_eq!(probe_resp["result"]["isError"], false);
        let probe_text = probe_resp["result"]["content"][0]["text"]
            .as_str()
            .expect("text content");
        assert!(probe_text.starts_with("!Σ:PROBE|status:"));

        // 2. Test harness_receipt via tools/call using a temporary receipt file
        let tmp = tempfile::tempdir().expect("tempdir");
        let receipt_json = json!({
            "task_id": "TICK-007-TEST",
            "status": "PASS",
            "git_commit": "abcdef1234567890",
            "signature_sha256": "9988776655443322",
            "checks": {
                "test": { "exit_code": 0 },
                "red_check": { "status": "PASS" }
            }
        });
        std::fs::write(
            tmp.path().join("TICK-007-TEST.json"),
            serde_json::to_string(&receipt_json).unwrap(),
        )
        .unwrap();

        let receipt_req = json!({
            "jsonrpc": "2.0",
            "id": 4,
            "method": "tools/call",
            "params": {
                "name": "harness_receipt",
                "arguments": {
                    "task_id": "TICK-007-TEST",
                    "dir": tmp.path().to_string_lossy()
                }
            }
        });
        let receipt_resp = handle_mcp_request(&receipt_req).expect("harness_receipt response");
        assert_eq!(receipt_resp["result"]["isError"], false);
        let receipt_text = receipt_resp["result"]["content"][0]["text"]
            .as_str()
            .expect("text content");
        assert_eq!(
            receipt_text,
            "!Σ:RECEIPT|TICK-007-TEST|status:PASS|red_check:PASS|commit:abcdef123456|sig:998877665544"
        );
    }
}
