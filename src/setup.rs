use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use crate::error::HarnessError;
use crate::mcp::mcp_tool_definitions;
use crate::probe::ComponentProbe;

/// Executes deterministic harness setup rooted at an explicit `home_dir`.
///
/// 1. Ensures `<home_dir>/.k0maru/{tuples,receipts,logs}` directories exist.
/// 2. Reads `<home_dir>/.gemini/config/mcp_config.json` (creating parent dirs and JSON object if missing),
///    and idempotently registers/updates `k0maru-harness`, `k0maru-memory`, and `k0maru-router` under
///    `"mcpServers"`, preserving all other existing server entries.
/// 3. Writes the 5 MCP tool schema JSON files into both
///    `<home_dir>/.gemini/antigravity-cli/mcp/k0maru-harness/` and
///    `<home_dir>/.gemini/config/mcp/k0maru-harness/`.
/// 4. Runs `ComponentProbe::run_full_probe()` and returns a single-line K-DSL wire summary:
///    `!Σ:SETUP|mcp_config:UPDATED|schemas:5|dirs:OK|probe:<READY/DEGRADED>`
pub fn run_setup_with_home(
    home_dir: &Path,
    vault_override: Option<&Path>,
) -> Result<String, HarnessError> {
    // 1. Ensure ~/.k0maru/{tuples,receipts,logs} exist
    let k0maru_dir = home_dir.join(".k0maru");
    for sub in ["tuples", "receipts", "logs"] {
        std::fs::create_dir_all(k0maru_dir.join(sub))?;
    }

    // Resolve vault path
    let resolved_vault: PathBuf = if let Some(v) = vault_override {
        v.to_path_buf()
    } else {
        let candidate = home_dir.join("Documents/SecondBrain");
        if candidate.exists() {
            candidate
        } else {
            crate::memory::resolve_vault(None)
        }
    };
    let vault_str = resolved_vault.to_string_lossy().to_string();

    // 2. Idempotently update <home_dir>/.gemini/config/mcp_config.json
    let mcp_config_dir = home_dir.join(".gemini/config");
    std::fs::create_dir_all(&mcp_config_dir)?;
    let mcp_config_path = mcp_config_dir.join("mcp_config.json");

    let mut root: Value = if mcp_config_path.exists() {
        let raw = std::fs::read_to_string(&mcp_config_path)?;
        if raw.trim().is_empty() {
            json!({ "mcpServers": {} })
        } else {
            serde_json::from_str(&raw)?
        }
    } else {
        json!({ "mcpServers": {} })
    };

    if !root.is_object() {
        root = json!({ "mcpServers": {} });
    }
    if !root
        .get("mcpServers")
        .map(|v| v.is_object())
        .unwrap_or(false)
    {
        root["mcpServers"] = json!({});
    }

    if let Some(servers) = root.get_mut("mcpServers").and_then(|v| v.as_object_mut()) {
        servers.insert(
            "k0maru-harness".to_string(),
            json!({
                "command": "k0maru-harness",
                "args": ["mcp"]
            }),
        );
        servers.insert(
            "k0maru-memory".to_string(),
            json!({
                "command": "k0maru",
                "args": ["mcp", "--vault", vault_str]
            }),
        );
        servers.insert(
            "k0maru-router".to_string(),
            json!({
                "command": "k0maru-router",
                "args": ["mcp"]
            }),
        );
    }

    let formatted_config = format!("{}\n", serde_json::to_string_pretty(&root)?);
    std::fs::write(&mcp_config_path, formatted_config)?;

    // 3. Write the 5 MCP tool schema JSON files into both lazy MCP schema directories
    let schema_dirs = [
        home_dir.join(".gemini/antigravity-cli/mcp/k0maru-harness"),
        home_dir.join(".gemini/config/mcp/k0maru-harness"),
    ];
    let defs = mcp_tool_definitions();
    let schema_count = defs.len();

    for dir in &schema_dirs {
        std::fs::create_dir_all(dir)?;
        for tool in &defs {
            let name = tool
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            let description = tool
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let parameters = tool
                .get("inputSchema")
                .cloned()
                .unwrap_or_else(|| json!({ "type": "object", "properties": {}, "required": [] }));
            let agy_schema = json!({
                "name": name,
                "description": description,
                "parameters": parameters
            });
            let file_path = dir.join(format!("{}.json", name));
            std::fs::write(file_path, serde_json::to_string(&agy_schema)?)?;
        }
    }

    // 4. Run full readiness probe and return single-line K-DSL summary
    let probe_report = ComponentProbe::run_full_probe();
    let probe_status = if probe_report.ready {
        "READY"
    } else {
        "DEGRADED"
    };

    Ok(format!(
        "!Σ:SETUP|mcp_config:UPDATED|schemas:{}|dirs:OK|probe:{}",
        schema_count, probe_status
    ))
}

/// Runs deterministic setup using the current user's home directory (`dirs::home_dir()`).
pub fn run_setup(vault_override: Option<&Path>) -> Result<String, HarnessError> {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    run_setup_with_home(&home, vault_override)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_run_setup_with_home_preserves_existing_mcp_servers_and_writes_schemas() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let home = tmp.path();
        let custom_vault = home.join("CustomVault");
        std::fs::create_dir_all(&custom_vault).unwrap();

        // Pre-populate mcp_config.json with existing servers (diana-voice and zhihu)
        let config_dir = home.join(".gemini/config");
        std::fs::create_dir_all(&config_dir).unwrap();
        let initial_cfg = json!({
            "mcpServers": {
                "diana-voice": {
                    "command": "uv",
                    "args": ["run", "diana", "mcp"]
                },
                "zhihu": {
                    "type": "streamableHttp",
                    "url": "https://developer.zhihu.com/api/mcp/v1"
                }
            }
        });
        std::fs::write(
            config_dir.join("mcp_config.json"),
            serde_json::to_string_pretty(&initial_cfg).unwrap(),
        )
        .unwrap();

        let wire = run_setup_with_home(home, Some(&custom_vault)).expect("setup should succeed");
        assert!(wire.starts_with("!Σ:SETUP|mcp_config:UPDATED|schemas:5|dirs:OK|probe:"));

        // Verify ~/.k0maru/{tuples,receipts,logs} exist
        assert!(home.join(".k0maru/tuples").is_dir());
        assert!(home.join(".k0maru/receipts").is_dir());
        assert!(home.join(".k0maru/logs").is_dir());

        // Verify mcp_config.json preserved diana-voice and zhihu and added the 3 K0maru MCP servers
        let updated_raw = std::fs::read_to_string(config_dir.join("mcp_config.json")).unwrap();
        let updated: Value = serde_json::from_str(&updated_raw).unwrap();
        let servers = updated["mcpServers"].as_object().unwrap();
        assert!(servers.contains_key("diana-voice"));
        assert!(servers.contains_key("zhihu"));
        assert_eq!(servers["k0maru-harness"]["command"], "k0maru-harness");
        assert_eq!(servers["k0maru-harness"]["args"], json!(["mcp"]));
        assert_eq!(servers["k0maru-router"]["command"], "k0maru-router");
        assert_eq!(servers["k0maru-router"]["args"], json!(["mcp"]));
        assert_eq!(servers["k0maru-memory"]["command"], "k0maru");
        assert_eq!(
            servers["k0maru-memory"]["args"],
            json!(["mcp", "--vault", custom_vault.to_string_lossy().to_string()])
        );

        // Verify all 5 schema files exist in both directories
        let expected_files = [
            "harness_probe.json",
            "harness_jit.json",
            "harness_record_lesson.json",
            "harness_run.json",
            "harness_receipt.json",
        ];
        for fname in &expected_files {
            let p1 = home
                .join(".gemini/antigravity-cli/mcp/k0maru-harness")
                .join(fname);
            let p2 = home.join(".gemini/config/mcp/k0maru-harness").join(fname);
            assert!(p1.is_file(), "Missing schema file {:?}", p1);
            assert!(p2.is_file(), "Missing schema file {:?}", p2);
            let s1: Value = serde_json::from_str(&std::fs::read_to_string(&p1).unwrap()).unwrap();
            assert!(s1.get("name").is_some());
            assert!(s1.get("parameters").is_some());
        }
    }
}
