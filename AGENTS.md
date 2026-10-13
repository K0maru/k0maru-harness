# AGENTS.md — k0maru-harness Engineering & Workflow Constitution

This document defines the authoritative architecture, engineering discipline, and collaboration standards for AI agents (Antigravity, Claude Code, Cursor, etc.) developing `k0maru-harness`.

---

## 🧭 1. Context Pointers & System Glossary

Before modifying core orchestration logic or CLI commands, consult these primary sources:
- **Architecture Log**: `SecondBrain: 01_AI_Logs/2026-10-13-k0maru-harness-meta-orchestration-and-jit-architecture.md`
- **Project Blueprint**: `SecondBrain: 10_Projects/K0maru-Harness.md`
- **Paper Motivations**:
  - `arXiv:2610.13361` (AgentSWE: Anti-specification drift and anti-hollow runtime)
  - `arXiv:2609.33304` (ExpActivator: JIT selective experience activation)

---

## 🎯 2. Delivery Loop & Quality Gates

Follow the strict specification-first delivery pipeline. Zero vibe coding.

1. **Iron Invariant: No Git Submodules**:
   - Submodules introduce detached HEAD issues, pointer merge hell, and multi-agent workspace friction.
   - `k0maru-harness` coordinates external binaries via POSIX CLI execution, exit status codes, and the Linda tuple bus (`~/.k0maru/tuples/`).
2. **Iron Invariant: Zero Monolithic Linkage**:
   - Do NOT statically link or include sibling crates as Cargo path dependencies unless explicitly approved by the Tech Lead.
   - The harness is a porcelain supervisor over separate micro-kernel binaries.
3. **Master-Worker Orchestration**:
   - Master agent acts as Tech Lead/Auditor: designs specs, splits tickets, audits PRs, and verifies acceptance.
   - Worker agents operate in clean contexts, follow TDD, and implement one ticket at a time.
4. **Mandatory Quality Gates**:
   - Formatter: `cargo fmt --check`
   - Linter: `cargo clippy --all-targets -- -D warnings` (Zero warnings permitted)
   - Tests: `cargo test`
   - Living ADR Guard: `k0maru guard check --diff` (Zero violations permitted)

---

## 🌿 3. Git Branching Discipline

1. **`main` (Production Release Trunk)**:
   - Absolutely stable. Direct push strictly forbidden after initial setup.
   - Accepts milestone PRs from `dev` via standard merge commits (`--no-ff`).
2. **`dev` (Integration Trunk)**:
   - Active integration target for all features and fixes.
   - Feature branches merge into `dev` via Squash & Merge (`feat(scope): description`).
3. **Feature Branches (`feat/*`, `fix/*`)**:
   - Created from `dev`. Isolated TDD implementation.

---

## 📐 4. Testing Seams & Probing

- All binary probing logic must support fallback resolution (`$PATH` -> `~/.local/bin` -> `~/.cargo/bin`).
- Unit and integration tests must run without requiring mock binaries if testing fallback paths.
- System integration tests should gracefully skip or report warnings if external pillar binaries are not present on the test runner.
