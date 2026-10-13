# TICK-001: Implement JIT Experience & Invariant Activation (memory jit)

## 📌 Context & Motivation
Following the ExpActivator (arXiv:2609.33304) thesis ("Relevance does not imply applicability"), coding agents suffer from massive context pollution when static documentation or long logs are dumped into prompts.
`k0maru-harness` needs a Just-In-Time (JIT) experience activation subcommand that queries `k0maru-agent-memory` at specific decision points (e.g. "git branch strategy", "locking convention", "sqlite pragma") and produces a compact (<150 tokens) prompt injection block.

## 🎯 Acceptance Criteria (DoD)
1. **Module `src/memory.rs`**:
   - `JitHit` struct with `path`, `title`, `score`, `snippet` (derives `Serialize, Deserialize, Debug, Clone`).
   - `JitActivationResult` struct with `decision_point`, `hits`, `formatted_prompt`, `token_estimate` (derives `Serialize, Deserialize, Debug, Clone`).
   - `pub fn jit_activate(decision_point: &str, limit: usize, vault: Option<&Path>) -> Result<JitActivationResult, HarnessError>`:
     - Uses `crate::probe::find_binary("k0maru")` to locate the binary.
     - Resolves vault: explicit `vault` -> env var `SECOND_BRAIN_VAULT` -> `~/Documents/SecondBrain` if exists -> default `.`
     - Executes `k0maru search <decision_point> -v <vault> --limit <limit> --json`.
     - Parses stdout into `Vec<JitHit>`.
     - Generates `formatted_prompt` in high-density Markdown:
       ```markdown
       ### 🧠 [JIT Invariant Activation: <decision_point>]
       - [[<path>]] (<title>): <clean 1-line snippet>
       ```
     - Provides graceful fallback if `k0maru` binary is not found or execution yields no results.
2. **CLI Integration**:
   - Command: `k0maru-harness memory jit --decision "<query>" [--limit <N>] [--vault <PATH>] [--json]`
   - Both human-readable formatted output and `--json` serialization.
3. **Tests & Quality**:
   - Unit tests in `src/memory.rs` or `tests/test_memory.rs` verifying prompt generation, token calculation, and fallback when binary is absent.
   - `cargo fmt --check` passes with zero changes.
   - `cargo clippy --all-targets -- -D warnings` passes with zero warnings.
   - `cargo test` passes 100%.
   - `k0maru guard check --diff` passes with zero violations.
