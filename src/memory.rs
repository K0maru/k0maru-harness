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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LessonRecordResult {
    pub title: String,
    pub count: u32,
    pub promoted: bool,
    pub lessons_path: PathBuf,
}

/// Cleans up raw snippets by collapsing all whitespace and newlines into a single line.
pub fn clean_snippet(raw: &str) -> String {
    raw.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Estimates token count roughly using character length divided by 4.
pub fn estimate_tokens(text: &str) -> usize {
    text.chars().count() / 4
}

/// Returns `true` only if `path` belongs to an architectural invariant, lesson, card, or project spec,
/// rejecting noisy third-party docs (`30_Resources/`), daily notes (`00_Daily/`), and raw logs (`01_AI_Logs/`).
pub fn is_invariant_or_lesson_path(path: &str) -> bool {
    let p = path.trim_start_matches("./").trim_start_matches('/');
    if p.starts_with("30_Resources/")
        || p.starts_with("00_Daily/")
        || p.starts_with("Archive_Legacy/")
        || p.starts_with("99_Attachments/")
        || p.starts_with("01_AI_Logs/")
    {
        return false;
    }

    p == "LESSONS.md"
        || p == "AGENTS.md"
        || p.starts_with("20_Cards/")
        || p.starts_with("10_Projects/")
        || p.contains("ADR")
}

/// Assigns a priority tier to a vault path so distilled lessons and ADRs always rank above general notes:
/// - Tier 0: `LESSONS.md`, `AGENTS.md`, or paths containing `ADR`
/// - Tier 1: `20_Cards/`
/// - Tier 2: `10_Projects/`
/// - Tier 3: everything else
pub fn priority_tier(path: &str) -> u8 {
    let p = path.trim_start_matches("./").trim_start_matches('/');
    if p == "LESSONS.md" || p == "AGENTS.md" || p.contains("ADR") {
        0
    } else if p.starts_with("20_Cards/") {
        1
    } else if p.starts_with("10_Projects/") {
        2
    } else {
        3
    }
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

fn extract_keywords(decision_point: &str) -> Vec<String> {
    let mut kws = Vec::new();
    for token in decision_point.split(|c: char| {
        c.is_whitespace() || matches!(c, ',' | ';' | ':' | '(' | ')' | '[' | ']' | '"' | '\'')
    }) {
        let trimmed = token.trim().to_lowercase();
        if trimmed.chars().count() >= 2 && !kws.contains(&trimmed) {
            kws.push(trimmed.clone());
        }
        for sub in trimmed.split(['/', '-', '_']) {
            let sub_trimmed = sub.trim().to_lowercase();
            if sub_trimmed.chars().count() >= 2 && !kws.contains(&sub_trimmed) {
                kws.push(sub_trimmed);
            }
        }
    }
    kws
}

/// Directly searches `<vault>/LESSONS.md` for `### ` sections under `## 🛑` or `## 🏆`
/// matching any keyword (len >= 2) from `decision_point`.
pub fn search_local_lessons(vault: &Path, decision_point: &str) -> Vec<JitHit> {
    let lessons_path = vault.join("LESSONS.md");
    let content = match std::fs::read_to_string(&lessons_path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    let keywords = extract_keywords(decision_point);
    if keywords.is_empty() {
        return Vec::new();
    }

    let mut hits = Vec::new();
    let mut in_target_h2 = false;
    let mut current_title: Option<String> = None;
    let mut current_lines: Vec<String> = Vec::new();

    let flush_section = |title: &str, lines: &[String], hits_out: &mut Vec<JitHit>| {
        let body = lines.join("\n");
        let combined_lower = format!("{}\n{}", title.to_lowercase(), body.to_lowercase());
        if keywords.iter().any(|kw| combined_lower.contains(kw)) {
            let done_right_line = lines
                .iter()
                .find(|l| l.contains("做对的标准") || l.to_lowercase().contains("done right"))
                .cloned()
                .unwrap_or_else(|| clean_snippet(&body));
            let snippet = clean_snippet(done_right_line.trim_start_matches('-').trim());
            hits_out.push(JitHit {
                path: "LESSONS.md".to_string(),
                title: title.to_string(),
                score: 1.0,
                snippet,
            });
        }
    };

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("## ") && !trimmed.starts_with("### ") {
            if let Some(ref title) = current_title.take() {
                flush_section(title, &current_lines, &mut hits);
                current_lines.clear();
            }
            in_target_h2 = trimmed.contains('🛑')
                || trimmed.contains('🏆')
                || trimmed.contains("Active Lessons")
                || trimmed.contains("Promoted");
        } else if in_target_h2 && trimmed.starts_with("### ") {
            if let Some(ref title) = current_title.take() {
                flush_section(title, &current_lines, &mut hits);
                current_lines.clear();
            }
            let heading = trimmed.trim_start_matches("###").trim().to_string();
            current_title = Some(heading);
        } else if current_title.is_some() && trimmed != "---" {
            current_lines.push(trimmed.to_string());
        }
    }

    if let Some(ref title) = current_title.take() {
        flush_section(title, &current_lines, &mut hits);
    }

    hits
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
/// Queries local `LESSONS.md` first, then `k0maru-agent-memory` (`k0maru search --limit 15`),
/// filters hits via `is_invariant_or_lesson_path`, sorts by `priority_tier`, deduplicates,
/// and truncates to `limit`.
pub fn jit_activate(
    decision_point: &str,
    limit: usize,
    vault: Option<&Path>,
) -> Result<JitActivationResult, HarnessError> {
    let vault_path = resolve_vault(vault);

    let mut combined_hits = search_local_lessons(&vault_path, decision_point);

    let k0maru_bin = crate::probe::find_binary("k0maru");
    if let Some(bin) = k0maru_bin {
        let search_limit = std::cmp::max(15, limit);
        let output = Command::new(&bin)
            .arg("search")
            .arg(decision_point)
            .arg("-v")
            .arg(&vault_path)
            .arg("--limit")
            .arg(search_limit.to_string())
            .arg("--json")
            .output();

        if let Ok(out) = output {
            if out.status.success() {
                let raw_hits =
                    serde_json::from_slice::<Vec<JitHit>>(&out.stdout).unwrap_or_default();
                for hit in raw_hits {
                    if is_invariant_or_lesson_path(&hit.path) {
                        combined_hits.push(hit);
                    }
                }
            }
        }
    }

    // Sort by priority_tier preserving score order within each tier
    combined_hits.sort_by(|a, b| {
        priority_tier(&a.path)
            .cmp(&priority_tier(&b.path))
            .then_with(|| {
                b.score
                    .partial_cmp(&a.score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    });

    // Deduplicate by (path, title)
    let mut hits = Vec::new();
    for hit in combined_hits {
        let norm_path = hit
            .path
            .trim_start_matches("./")
            .trim_start_matches('/')
            .to_string();
        if !hits.iter().any(|existing: &JitHit| {
            let ex_path = existing
                .path
                .trim_start_matches("./")
                .trim_start_matches('/');
            ex_path == norm_path && existing.title == hit.title
        }) {
            hits.push(hit);
        }
    }

    hits.truncate(limit);

    let formatted_prompt = format_jit_prompt(decision_point, &hits);
    let token_estimate = estimate_tokens(&formatted_prompt);

    Ok(JitActivationResult {
        decision_point: decision_point.to_string(),
        hits,
        formatted_prompt,
        token_estimate,
    })
}

fn strip_heading_number(heading: &str) -> &str {
    let trimmed = heading.trim();
    if let Some(dot_idx) = trimmed.find('.') {
        let prefix = &trimmed[..dot_idx];
        if !prefix.is_empty() && prefix.chars().all(|c| c.is_ascii_digit()) {
            return trimmed[dot_idx + 1..].trim();
        }
    }
    trimmed
}

fn parse_seen_count(line: &str) -> Option<u32> {
    let lower = line.to_lowercase();
    let pos = lower.find("seen ")?;
    let after = &line[pos + 5..];
    let digits: String = after
        .chars()
        .skip_while(|c| c.is_whitespace())
        .take_while(|c| c.is_ascii_digit())
        .collect();
    digits.parse::<u32>().ok()
}

/// Records a lesson in `<vault>/LESSONS.md` and advances the Seen-3x state machine:
/// - New lesson -> `Seen 1× (YYYY-MM-DD)` under `## 🛑 活跃动态教训 (Active Lessons)`
/// - Existing lesson -> increments `Seen N×` to `Seen (N+1)×`
/// - When `count >= 3` -> marks `promoted = true`, appends ` **[PROMOTED TO LIVING ADR]**`
///   and records a summary entry under `## 🏆 已转正硬化规则 (Promoted Living ADRs)`.
pub fn record_lesson(
    vault: Option<&Path>,
    title: &str,
    cause: &str,
    done_right: &str,
) -> Result<LessonRecordResult, HarnessError> {
    let vault_path = resolve_vault(vault);
    if !vault_path.exists() {
        std::fs::create_dir_all(&vault_path)?;
    }
    let lessons_path = vault_path.join("LESSONS.md");
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();

    let initial_content = if lessons_path.exists() {
        std::fs::read_to_string(&lessons_path)?
    } else {
        "# 📓 LESSONS.md — 动态错题本与事不过三 (Seen 3×) 规则演化\n\n---\n\n## 🛑 活跃动态教训 (Active Lessons)\n\n## 🏆 已转正硬化规则 (Promoted Living ADRs)\n"
            .to_string()
    };

    let clean_query_title = strip_heading_number(title).trim();
    let norm_query = clean_query_title.to_lowercase();

    let mut lines: Vec<String> = initial_content.lines().map(|s| s.to_string()).collect();

    // Locate matching `### ` heading if it exists
    let mut matched_h3_idx: Option<usize> = None;
    let mut matched_heading_clean = clean_query_title.to_string();
    let mut max_section_num: u32 = 0;

    for (idx, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with("### ") {
            let raw_heading = trimmed.trim_start_matches("###").trim();
            if let Some(dot_idx) = raw_heading.find('.') {
                if let Ok(num) = raw_heading[..dot_idx].trim().parse::<u32>() {
                    if num > max_section_num {
                        max_section_num = num;
                    }
                }
            }
            let stripped = strip_heading_number(raw_heading);
            let stripped_lower = stripped.to_lowercase();
            let raw_lower = raw_heading.to_lowercase();
            if !norm_query.is_empty()
                && (stripped_lower.contains(&norm_query)
                    || norm_query.contains(&stripped_lower)
                    || raw_lower.contains(&norm_query))
            {
                matched_h3_idx = Some(idx);
                matched_heading_clean = stripped.to_string();
            }
        }
    }

    if let Some(start_idx) = matched_h3_idx {
        // Find end of this `### ` section
        let mut end_idx = lines.len();
        for (idx, line) in lines.iter().enumerate().skip(start_idx + 1) {
            let trimmed = line.trim();
            if trimmed.starts_with("### ") || trimmed.starts_with("## ") {
                end_idx = idx;
                break;
            }
        }

        // Find `Seen N×` line in [start_idx + 1 .. end_idx]
        let mut seen_line_idx: Option<usize> = None;
        let mut current_count: u32 = 1;
        for (idx, line) in lines.iter().enumerate().take(end_idx).skip(start_idx + 1) {
            if let Some(c) = parse_seen_count(line) {
                seen_line_idx = Some(idx);
                current_count = c;
                break;
            }
        }

        let new_count = current_count + 1;
        let promoted = new_count >= 3;
        let promo_badge = if promoted {
            " **[PROMOTED TO LIVING ADR]**"
        } else {
            ""
        };
        let updated_seen_line = format!(
            "- **复现记录**：Seen {}× ({}).{}",
            new_count, today, promo_badge
        );

        if let Some(s_idx) = seen_line_idx {
            lines[s_idx] = updated_seen_line;
        } else {
            lines.insert(end_idx, updated_seen_line);
        }

        if promoted {
            // Also append a promoted summary bullet under `## 🏆` if not already present
            let promo_bullet = format!(
                "- **[PROMOTED TO LIVING ADR ({})]** **{}** — {} (Seen {}×)",
                today, matched_heading_clean, done_right, new_count
            );
            let already_listed = lines.iter().any(|l| {
                l.contains("[PROMOTED TO LIVING ADR") && l.contains(&matched_heading_clean)
            });
            if !already_listed {
                // Remove placeholder if present
                lines.retain(|l| !l.contains("*(当前暂无升格条目"));
                if let Some(h2_promo_idx) = lines.iter().position(|l| {
                    l.trim().starts_with("## 🏆") || l.contains("Promoted Living ADRs")
                }) {
                    let mut insert_pos = lines.len();
                    for (idx, line) in lines.iter().enumerate().skip(h2_promo_idx + 1) {
                        if line.trim().starts_with("## ") {
                            insert_pos = idx;
                            break;
                        }
                    }
                    lines.insert(insert_pos, promo_bullet);
                } else {
                    lines.push(String::new());
                    lines.push("## 🏆 已转正硬化规则 (Promoted Living ADRs)".to_string());
                    lines.push(promo_bullet);
                }
            }
        }

        let final_text = format!("{}\n", lines.join("\n").trim_end());
        std::fs::write(&lessons_path, final_text)?;

        Ok(LessonRecordResult {
            title: matched_heading_clean,
            count: new_count,
            promoted,
            lessons_path,
        })
    } else {
        let next_num = max_section_num + 1;
        let new_section = vec![
            format!("### {}. {}", next_num, clean_query_title),
            format!("- **事故与成因**：{}", cause),
            format!("- **做对的标准 (Done right)**：{}", done_right),
            format!("- **复现记录**：Seen 1× ({}).", today),
            String::new(),
            "---".to_string(),
            String::new(),
        ];

        // Insert before `## 🏆` if present, otherwise at the end of the file
        if let Some(h2_promo_idx) = lines
            .iter()
            .position(|l| l.trim().starts_with("## 🏆") || l.contains("Promoted Living ADRs"))
        {
            for (offset, new_line) in new_section.into_iter().enumerate() {
                lines.insert(h2_promo_idx + offset, new_line);
            }
        } else {
            if !lines
                .iter()
                .any(|l| l.trim().starts_with("## 🛑") || l.contains("Active Lessons"))
            {
                lines.push(String::new());
                lines.push("## 🛑 活跃动态教训 (Active Lessons)".to_string());
                lines.push(String::new());
            }
            lines.extend(new_section);
        }

        let final_text = format!("{}\n", lines.join("\n").trim_end());
        std::fs::write(&lessons_path, final_text)?;

        Ok(LessonRecordResult {
            title: clean_query_title.to_string(),
            count: 1,
            promoted: false,
            lessons_path,
        })
    }
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
    fn test_is_invariant_or_lesson_path_filtering() {
        // Must reject third-party docs, daily logs, raw AI logs, archives, attachments
        assert!(!is_invariant_or_lesson_path(
            "30_Resources/lancedb-docs/guide.md"
        ));
        assert!(!is_invariant_or_lesson_path("00_Daily/2026/2026-10-13.md"));
        assert!(!is_invariant_or_lesson_path(
            "01_AI_Logs/2026-10-13-debug.md"
        ));
        assert!(!is_invariant_or_lesson_path("Archive_Legacy/old.md"));
        assert!(!is_invariant_or_lesson_path("99_Attachments/diagram.png"));

        // Must accept LESSONS.md, AGENTS.md, 20_Cards/, 10_Projects/, and ADR paths
        assert!(is_invariant_or_lesson_path("LESSONS.md"));
        assert!(is_invariant_or_lesson_path("AGENTS.md"));
        assert!(is_invariant_or_lesson_path("20_Cards/Sqlite-Locking.md"));
        assert!(is_invariant_or_lesson_path("10_Projects/K0maru-Harness.md"));
        assert!(is_invariant_or_lesson_path("docs/ADR-001-Architecture.md"));
    }

    #[test]
    fn test_priority_tier_ordering() {
        assert_eq!(priority_tier("LESSONS.md"), 0);
        assert_eq!(priority_tier("AGENTS.md"), 0);
        assert_eq!(priority_tier("10_Projects/ADR-002.md"), 0);
        assert_eq!(priority_tier("20_Cards/Card.md"), 1);
        assert_eq!(priority_tier("10_Projects/Project.md"), 2);
        assert_eq!(priority_tier("other/file.md"), 3);
    }

    #[test]
    fn test_record_lesson_seen_3x_promotion_and_local_search() {
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let vault = temp_dir.path();

        // 1st occurrence -> Seen 1×, not promoted
        let r1 = record_lesson(
            Some(vault),
            "SQLite 并发写锁缺失",
            "多线程直接写库导致 SQLITE_BUSY",
            "必须启用 WAL 模式与 busy_timeout",
        )
        .expect("1st record_lesson should succeed");
        assert_eq!(r1.count, 1);
        assert!(!r1.promoted);

        let content1 = std::fs::read_to_string(&r1.lessons_path).unwrap();
        assert!(content1.contains("### 1. SQLite 并发写锁缺失"));
        assert!(content1.contains("Seen 1×"));

        // Local search should immediately find the lesson in Tier 0
        let local_hits = search_local_lessons(vault, "sqlite 并发");
        assert_eq!(local_hits.len(), 1);
        assert_eq!(local_hits[0].path, "LESSONS.md");
        assert!(local_hits[0].title.contains("SQLite 并发写锁缺失"));
        assert!(local_hits[0].snippet.contains("必须启用 WAL 模式"));

        // 2nd occurrence -> Seen 2×, not promoted
        let r2 = record_lesson(
            Some(vault),
            "SQLite 并发写锁缺失",
            "多线程直接写库导致 SQLITE_BUSY",
            "必须启用 WAL 模式与 busy_timeout",
        )
        .expect("2nd record_lesson should succeed");
        assert_eq!(r2.count, 2);
        assert!(!r2.promoted);
        let content2 = std::fs::read_to_string(&r2.lessons_path).unwrap();
        assert!(content2.contains("Seen 2×"));

        // 3rd occurrence -> Seen 3×, promoted to Living ADR!
        let r3 = record_lesson(
            Some(vault),
            "SQLite 并发写锁缺失",
            "多线程直接写库导致 SQLITE_BUSY",
            "必须启用 WAL 模式与 busy_timeout",
        )
        .expect("3rd record_lesson should succeed");
        assert_eq!(r3.count, 3);
        assert!(r3.promoted);
        let content3 = std::fs::read_to_string(&r3.lessons_path).unwrap();
        assert!(content3.contains("Seen 3×"));
        assert!(content3.contains("**[PROMOTED TO LIVING ADR]**"));
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
