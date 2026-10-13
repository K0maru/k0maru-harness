# TICK-002: Implement Pipeline Runner (k0maru-harness run) and Git Hook Installer (init-hook)

## 📌 Context & Motivation
Following the anti-compaction defense architecture:
1. **Control Inversion**: Instead of hoping the agent remembers to run JIT activation and verification, `k0maru-harness run` orchestrates the lifecycle: probes ecosystem, pre-activates JIT invariants for the task (<180 tokens), emits the ready tuple to Linda space, and triggers the physical verifier.
2. **Deterministic Pre-Commit Gate**: `k0maru-harness init-hook` installs a `.git/hooks/pre-commit` script that runs `cargo fmt` and `k0maru guard check --diff`, aborting invalid commits before they happen.

## 🎯 Acceptance Criteria (DoD)
1. **Module `src/pipeline.rs`**:
   - `PipelineOptions`: `task`, `domain`, `node`, `verify`, `workspace`, `json`.
   - `PipelineResult`: `task_id`, `probe_passed`, `jit_invariants`, `tuple_id`, `verified`, `receipt_summary`.
   - `pub fn run_pipeline(opts: &PipelineOptions) -> Result<PipelineResult, HarnessError>`:
     - Probes component availability.
     - Performs JIT invariant activation via `crate::memory::jit_activate(&opts.task, 3, None)`.
     - Emits `TASK_READY` Linda tuple via `k0maru-router tuple out` (or gracefully falls back if router unavailable).
     - If `verify`, runs `k0maru-verifier verify` with `--task-id` and checks exit status.
2. **Hook Installer `src/hook.rs`**:
   - `pub fn install_pre_commit_hook(target_dir: Option<&Path>) -> Result<PathBuf, HarnessError>`:
     - Locates `.git/hooks` in target directory (or current directory).
     - Writes executable `pre-commit` script with format check and `k0maru guard check --diff`.
3. **CLI Integration (`src/main.rs`)**:
   - Connect `Commands::Run` to `pipeline::run_pipeline`.
   - Add `Commands::InitHook { path: Option<PathBuf> }`.
4. **Tests & Quality Gates**:
   - Unit and integration tests in `tests/test_pipeline.rs` and `tests/test_hook.rs`.
   - `cargo fmt --all --check` passes.
   - `cargo clippy --all-targets -- -D warnings` passes.
   - `cargo test` passes 100%.
   - `k0maru guard check --diff` passes with 0 violations.
