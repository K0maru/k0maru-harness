use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::error::HarnessError;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct JitHit {
    #[serde(default)]
    pub path: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub score: f64,
    #[serde(default)]
    pub snippet: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct JitActivationResult {
    pub decision_point: String,
    pub hits: Vec<JitHit>,
    pub formatted_prompt: String,
    pub token_estimate: usize,
}

/// Cleans up raw snippets by collapsing all whitespace and newlines into a single line.
pub fn clean_snippet(raw: &str) -> String {
    raw.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Estimates token count roughly using character length divided by 4.
pub fn estimate_tokens(text: &str) -> usize {
    text.chars().count() / 4
}

/// Resolves the SecondBrain vault path according to priority:
/// 1. Explicit vault path passed as parameter
/// 2. `SECOND_BRAIN_VAULT` environment variable
/// 3. `~/Documents/SecondBrain` if it exists
/// 4. Current directory `.` as fallback
pub fn resolve_vault(vault: Option<&Path>) -> PathBuf {
    if let Some(v) = vault {
        v.to_path_buf()
    } else if let Ok(env_val) = std::env::var("SECOND_BRAIN_VAULT") {
        if !env_val.trim().is_empty() {
            PathBuf::from(env_val)
        } else {
            resolve_default_vault()
        }
    } else {
        resolve_default_vault()
    }
}

fn resolve_default_vault() -> PathBuf {
    if let Some(home) = dirs::home_dir() {
        let sb = home.join("Documents/SecondBrain");
        if sb.exists() {
            return sb;
        }
    }
    PathBuf::from(".")
}

/// Formats retrieved memory hits into a high-density Markdown injection prompt block.
pub fn format_jit_prompt(decision_point: &str, hits: &[JitHit]) -> String {
    let mut lines = Vec::new();
    lines.push(format!(
        "### 🧠 [JIT Invariant Activation: {}]",
        decision_point
    ));

    if hits.is_empty() {
        lines.push("- (No active invariants or memory records found)".to_string());
    } else {
        for hit in hits {
            let clean = clean_snippet(&hit.snippet);
            lines.push(format!("- [[{}]] ({}): {}", hit.path, hit.title, clean));
        }
    }

    lines.join("\n")
}

/// Activates JIT invariants and memory experiences for a specific decision point.
///
/// Queries `k0maru-agent-memory` (`k0maru search`) with limits and formats
/// the results into a compact (<150 tokens) prompt injection block.
///
/// If the `k0maru` binary is not found or fails to execute, returns a graceful fallback
/// `JitActivationResult` with empty hits instead of panicking.
pub fn jit_activate(
    decision_point: &str,
    limit: usize,
    vault: Option<&Path>,
) -> Result<JitActivationResult, HarnessError> {
    let vault_path = resolve_vault(vault);

    let k0maru_bin = crate::probe::find_binary("k0maru");
    let hits = match k0maru_bin {
        Some(bin) => {
            let output = Command::new(&bin)
                .arg("search")
                .arg(decision_point)
                .arg("-v")
                .arg(&vault_path)
                .arg("--limit")
                .arg(limit.to_string())
                .arg("--json")
                .output();

            match output {
                Ok(out) if out.status.success() => {
                    serde_json::from_slice::<Vec<JitHit>>(&out.stdout).unwrap_or_default()
                }
                _ => Vec::new(),
            }
        }
        None => Vec::new(),
    };

    let formatted_prompt = format_jit_prompt(decision_point, &hits);
    let token_estimate = estimate_tokens(&formatted_prompt);

    Ok(JitActivationResult {
        decision_point: decision_point.to_string(),
        hits,
        formatted_prompt,
        token_estimate,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_snippet() {
        let raw = "  # Header \n\n Line 1 \r\n Line 2 \t\t with   spaces  ";
        let cleaned = clean_snippet(raw);
        assert_eq!(cleaned, "# Header Line 1 Line 2 with spaces");
    }

    #[test]
    fn test_estimate_tokens() {
        let text = "12345678";
        assert_eq!(estimate_tokens(text), 2);
        let empty = "";
        assert_eq!(estimate_tokens(empty), 0);
    }

    #[test]
    fn test_format_jit_prompt_with_hits() {
        let hits = vec![
            JitHit {
                path: "10_Projects/Git.md".to_string(),
                title: "Git Rules".to_string(),
                score: 0.95,
                snippet: "Rule 1: Always branch from dev.\nRule 2: No force push.".to_string(),
            },
            JitHit {
                path: "20_Cards/Locking.md".to_string(),
                title: "Locking".to_string(),
                score: 0.85,
                snippet: "Use advisory locks for sqlite.".to_string(),
            },
        ];

        let prompt = format_jit_prompt("git branching", &hits);
        assert!(prompt.starts_with("### 🧠 [JIT Invariant Activation: git branching]"));
        assert!(prompt.contains("- [[10_Projects/Git.md]] (Git Rules): Rule 1: Always branch from dev. Rule 2: No force push."));
        assert!(
            prompt.contains("- [[20_Cards/Locking.md]] (Locking): Use advisory locks for sqlite.")
        );
    }

    #[test]
    fn test_format_jit_prompt_empty_fallback() {
        let prompt = format_jit_prompt("unknown topic", &[]);
        assert!(prompt.starts_with("### 🧠 [JIT Invariant Activation: unknown topic]"));
        assert!(prompt.contains("- (No active invariants or memory records found)"));
    }

    #[test]
    fn test_resolve_vault_explicit() {
        let path = Path::new("/tmp/my_custom_vault");
        let resolved = resolve_vault(Some(path));
        assert_eq!(resolved, PathBuf::from("/tmp/my_custom_vault"));
    }

    #[test]
    fn test_jit_activate_fallback_on_missing_or_empty() {
        // Calling with a query should return Ok even if binary fails or returns empty
        let result = jit_activate("nonexistent_invariant_xyz_12345", 3, None);
        assert!(result.is_ok());
        let res = result.unwrap();
        assert_eq!(res.decision_point, "nonexistent_invariant_xyz_12345");
        assert!(res.formatted_prompt.contains("JIT Invariant Activation"));
    }
}
