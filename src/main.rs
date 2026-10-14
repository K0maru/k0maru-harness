use clap::{Parser, Subcommand};
use colored::*;
use k0maru_harness::probe::{Availability, ComponentProbe};

#[derive(Parser, Debug)]
#[command(
    name = "k0maru-harness",
    author = "K0maru Team",
    version,
    about = "Next-Generation Daemonless Meta-Harness & Orchestrator for Decoupled Agent Pillars",
    long_about = "A unified, daemonless Meta-Harness coordinating task execution across decoupled Unix pillars:\n\
                  • k0maru-agent-router    (L3/L4 Routing & Linda Tuple Space)\n\
                  • k0maru-agent-workspace (L1 Git Worktree Ephemeral Sandboxes)\n\
                  • k0maru-agent-verifier  (L2 Deterministic Physical Oracle)\n\
                  • k0maru-agent-memory    (L0 Cognitive Hub & Living ADR Guard)"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Probe the availability, versions, and paths of the 4 harness pillars and Linda total bus
    #[command(alias = "doctor", alias = "probe")]
    Check {
        /// Emit machine-readable JSON output
        #[arg(long)]
        json: bool,
    },

    /// Query and activate JIT experiences and architectural invariants
    Memory {
        #[command(subcommand)]
        command: MemoryCommands,
    },

    /// Run a task end-to-end through the complete harness pipeline
    Run {
        /// Task description or specification
        task: String,

        /// Target domain identifier (e.g. ROUTER, AUTH-SRV)
        #[arg(short, long, default_value = "DEFAULT")]
        domain: String,

        /// Optional remote node hostname over Tailscale (e.g. company-pc)
        #[arg(long)]
        node: Option<String>,

        /// Target workspace directory (defaults to current directory)
        #[arg(short, long)]
        workspace: Option<std::path::PathBuf>,

        /// Optional shell command to execute in workspace with JIT invariants injected
        #[arg(short, long)]
        exec: Option<String>,

        /// Explicit path to SecondBrain vault root for JIT recall and auto-lesson capture
        #[arg(long)]
        vault: Option<std::path::PathBuf>,

        /// Run deterministic physical verification gate
        #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
        verify: bool,

        /// Enforce Popperian physical falsification (new tests must fail on baseline)
        #[arg(long)]
        red_check: bool,

        /// Disable physical verification gate
        #[arg(long)]
        no_verify: bool,

        /// Emit machine-readable JSON output
        #[arg(long)]
        json: bool,

        /// Emit compact single-line K-DSL wire summary (!Σ:PIPELINE|...)
        #[arg(long)]
        wire: bool,
    },

    /// Inspect a physical verification receipt and emit compact K-DSL wire summary
    Receipt {
        /// Task identifier (e.g. TICK-005)
        task_id: String,

        /// Optional directory override for receipts (defaults to ~/.k0maru/receipts)
        #[arg(long)]
        dir: Option<std::path::PathBuf>,
    },

    /// Install pre-commit physical gate hook into git repository
    #[command(name = "init-hook", alias = "hook")]
    InitHook {
        /// Target repository root directory (defaults to current directory)
        #[arg(short, long)]
        path: Option<std::path::PathBuf>,
    },

    /// Start a reactive worker loop listening on Linda Tuple Space
    Worker {
        /// Target domain to consume tasks for
        #[arg(short, long)]
        domain: String,

        /// Process a single task and exit
        #[arg(long)]
        once: bool,
    },

    /// Run end-to-end integration self-test
    E2e {
        /// Task identifier
        #[arg(long, default_value = "e2e-self-test")]
        task_id: String,
    },
}

#[derive(Subcommand, Debug)]
pub enum MemoryCommands {
    /// Just-in-time invariant and memory activation for a decision point
    Jit {
        /// Decision point query string (e.g. 'git branching', 'sqlite lock')
        #[arg(short, long)]
        decision: String,

        /// Maximum number of hits to retrieve
        #[arg(short, long, default_value_t = 3)]
        limit: usize,

        /// Explicit path to SecondBrain vault root
        #[arg(short, long)]
        vault: Option<std::path::PathBuf>,

        /// Emit machine-readable JSON output
        #[arg(long)]
        json: bool,
    },

    /// Record an operational lesson in LESSONS.md and advance Seen-3x state machine
    RecordLesson {
        /// Lesson title or anti-pattern summary
        #[arg(short, long)]
        title: String,

        /// Incident description and root cause
        #[arg(short, long)]
        cause: String,

        /// Standard for doing it right (Done right)
        #[arg(short, long)]
        standard: String,

        /// Explicit path to SecondBrain vault root
        #[arg(short, long)]
        vault: Option<std::path::PathBuf>,

        /// Emit machine-readable JSON output
        #[arg(long)]
        json: bool,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Check { json } => {
            let report = ComponentProbe::run_full_probe();
            if json {
                println!("{}", serde_json::to_string_pretty(&report).unwrap());
            } else {
                println!();
                println!(
                    "{}",
                    "🔍 [K0maru-Harness] Component & Bus Readiness Probe"
                        .bold()
                        .cyan()
                );
                println!(
                    "{}",
                    "═══════════════════════════════════════════════════════════════════".cyan()
                );
                println!("  Timestamp: {}", report.timestamp);
                println!();
                println!("{}", "📦 Decoupled Micro-Kernels (Pillars):".bold());

                for c in &report.components {
                    let req_badge = if c.required {
                        "[REQUIRED]".red()
                    } else {
                        "[OPTIONAL]".dimmed()
                    };

                    match &c.availability {
                        Availability::Available { version, path } => {
                            println!(
                                "  ✅ {:<10} {:<10} {:<30} -> {}",
                                c.name.green().bold(),
                                req_badge,
                                path.display().to_string().dimmed(),
                                version.cyan()
                            );
                        }
                        Availability::Missing => {
                            let mark = if c.required {
                                "❌".red()
                            } else {
                                "⚠️ ".yellow()
                            };
                            println!(
                                "  {} {:<10} {:<10} (Binary '{}' not found in PATH or ~/.local/bin)",
                                mark,
                                c.name.yellow().bold(),
                                req_badge,
                                c.binary
                            );
                        }
                        Availability::Error(err) => {
                            println!(
                                "  ❌ {:<10} {:<10} Error: {}",
                                c.name.red().bold(),
                                req_badge,
                                err
                            );
                        }
                    }
                }

                println!();
                println!("{}", "🚌 Local Event & Evidence Buses:".bold());
                for b in &report.buses {
                    let status = if b.exists && b.writable {
                        "OK (Writable)".green()
                    } else if b.exists {
                        "RO (Read-Only)".yellow()
                    } else {
                        "MISSING".red()
                    };
                    println!(
                        "  📁 {:<28} -> {:<32} [{}]",
                        b.name.cyan(),
                        b.path.display().to_string().dimmed(),
                        status
                    );
                }

                println!(
                    "{}",
                    "───────────────────────────────────────────────────────────────────".cyan()
                );
                if report.ready {
                    println!(
                        "{}",
                        "🎉 Harness status: READY FOR AUTONOMOUS DISPATCH"
                            .green()
                            .bold()
                    );
                } else {
                    println!(
                        "{}",
                        "⚠️  Harness status: DEGRADED / MISSING REQUIRED PILLARS"
                            .yellow()
                            .bold()
                    );
                }
                println!(
                    "{}",
                    "═══════════════════════════════════════════════════════════════════".cyan()
                );
                println!();
            }
        }
        Commands::Memory { command } => match command {
            MemoryCommands::Jit {
                decision,
                limit,
                vault,
                json,
            } => match k0maru_harness::memory::jit_activate(&decision, limit, vault.as_deref()) {
                Ok(result) => {
                    if json {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&result)
                                .expect("Failed to serialize JIT result")
                        );
                    } else {
                        println!("{}", result.formatted_prompt);
                    }
                }
                Err(err) => {
                    eprintln!("Error activating memory: {}", err);
                    std::process::exit(1);
                }
            },
            MemoryCommands::RecordLesson {
                title,
                cause,
                standard,
                vault,
                json,
            } => match k0maru_harness::memory::record_lesson(
                vault.as_deref(),
                &title,
                &cause,
                &standard,
            ) {
                Ok(result) => {
                    if json {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&result)
                                .expect("Failed to serialize LessonRecordResult")
                        );
                    } else {
                        let status_badge = if result.promoted {
                            "PROMOTED TO LIVING ADR (Seen 3×+)".green().bold()
                        } else {
                            format!("RECORDED (Seen {}×)", result.count).cyan().bold()
                        };
                        println!(
                            "✅ [K0maru-Harness] Lesson '{}' -> {} at {}",
                            result.title,
                            status_badge,
                            result.lessons_path.display()
                        );
                    }
                }
                Err(err) => {
                    eprintln!("Error recording lesson: {}", err);
                    std::process::exit(1);
                }
            },
        },
        Commands::InitHook { path } => {
            match k0maru_harness::hook::install_pre_commit_hook(path.as_deref()) {
                Ok(hook_path) => {
                    println!(
                        "{} Pre-commit gate hook installed at: {}",
                        "✅ [K0maru-Harness]".green().bold(),
                        hook_path.display().to_string().cyan()
                    );
                }
                Err(err) => {
                    eprintln!(
                        "{} Failed to install pre-commit hook: {}",
                        "❌ [K0maru-Harness]".red().bold(),
                        err
                    );
                    std::process::exit(1);
                }
            }
        }
        Commands::Run {
            task,
            domain,
            node,
            workspace,
            exec,
            vault,
            verify,
            red_check,
            no_verify,
            json,
            wire,
        } => {
            let effective_verify = verify && !no_verify;
            let opts = k0maru_harness::pipeline::PipelineOptions {
                task: task.clone(),
                domain: domain.clone(),
                node: node.clone(),
                verify: effective_verify,
                red_check,
                workspace,
                exec_cmd: exec,
                vault,
                json,
            };

            match k0maru_harness::pipeline::run_pipeline(&opts) {
                Ok(res) => {
                    let wire_line = k0maru_harness::pipeline::format_wire_summary(&res);
                    if wire {
                        println!("{}", wire_line);
                    } else if json {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&res)
                                .expect("Failed to serialize PipelineResult")
                        );
                    } else {
                        let probe_badge = if res.probe_passed {
                            "READY".green().bold()
                        } else {
                            "DEGRADED".yellow().bold()
                        };
                        let tuple_badge = res
                            .tuple_id
                            .as_deref()
                            .map(|t| format!("EMITTED ({})", t))
                            .unwrap_or_else(|| "SKIPPED".to_string());
                        let exec_badge = match res.exec_passed {
                            Some(true) => "PASS",
                            Some(false) => "FAIL",
                            None => "SKIP",
                        };
                        let (verify_badge, verify_detail) = if effective_verify {
                            if res.verified {
                                (
                                    "VERIFIED".green().bold(),
                                    res.receipt_summary.as_deref().unwrap_or("Passed"),
                                )
                            } else {
                                (
                                    "FAILED".red().bold(),
                                    res.receipt_summary.as_deref().unwrap_or("Failed"),
                                )
                            }
                        } else {
                            ("DISABLED".dimmed(), "--no-verify")
                        };

                        println!(
                            "{} ({} | {})",
                            "🚀 [K0maru-Harness] Task Pipeline Execution".bold().cyan(),
                            res.task_id.bold(),
                            domain
                        );
                        println!(
                            "  🔍 Phase 1: Component Readiness Probe -> [{}]",
                            probe_badge
                        );
                        println!(
                            "  🧠 Phase 2: JIT Invariant Activation  -> Activated ({} lines)",
                            res.jit_invariants.lines().count()
                        );
                        println!(
                            "  🚌 Phase 3: Linda Space Task Ready     -> [{}] (exec: {})",
                            tuple_badge, exec_badge
                        );
                        println!(
                            "  🛡️  Phase 4: Physical Verification     -> [{}] ({})",
                            verify_badge, verify_detail
                        );
                        println!("{}", wire_line);
                    }

                    if res.exec_passed == Some(false) || (effective_verify && !res.verified) {
                        std::process::exit(1);
                    }
                }
                Err(err) => {
                    eprintln!("Error executing pipeline: {}", err);
                    std::process::exit(1);
                }
            }
        }
        Commands::Receipt { task_id, dir } => {
            match k0maru_harness::pipeline::inspect_receipt_wire(&task_id, dir.as_deref()) {
                Ok(wire) => println!("{}", wire),
                Err(err) => {
                    eprintln!("Error inspecting receipt '{}': {}", task_id, err);
                    std::process::exit(1);
                }
            }
        }
        Commands::Worker { domain, once } => {
            println!(
                "{}",
                "👷 [K0maru-Harness] Autonomous Worker Loop".bold().cyan()
            );
            println!("  Domain:   {}", domain);
            println!("  Once:     {}", once);
            println!();

            let Some(router_bin) = k0maru_harness::probe::find_binary("k0maru-router") else {
                eprintln!(
                    "{}",
                    "❌ Required binary 'k0maru-router' not found in PATH or ~/.local/bin".red()
                );
                std::process::exit(1);
            };

            if once {
                let output = std::process::Command::new(&router_bin)
                    .args([
                        "tuple",
                        "in",
                        "--kind",
                        "TASK_READY",
                        "--domain",
                        &domain,
                        "--json",
                    ])
                    .output();

                match output {
                    Ok(out) if out.status.success() => {
                        let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
                        if stdout.is_empty() || stdout == "null" {
                            println!(
                                "{}",
                                format!(
                                    "ℹ️  No TASK_READY tuple available in domain '{}'.",
                                    domain
                                )
                                .yellow()
                            );
                        } else {
                            println!(
                                "{} Claimed tuple in domain '{}':\n{}",
                                "✅".green(),
                                domain.bold(),
                                stdout
                            );
                        }
                    }
                    Ok(out) => {
                        let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
                        println!(
                            "{}",
                            format!(
                                "ℹ️  No TASK_READY tuple claimed in domain '{}' ({})",
                                domain, stderr
                            )
                            .yellow()
                        );
                    }
                    Err(err) => {
                        eprintln!("❌ Failed to invoke k0maru-router tuple in: {}", err);
                        std::process::exit(1);
                    }
                }
            } else {
                let status = std::process::Command::new(&router_bin)
                    .args(["worker", "--domain", &domain])
                    .status();
                match status {
                    Ok(s) if s.success() => {}
                    Ok(s) => std::process::exit(s.code().unwrap_or(1)),
                    Err(err) => {
                        eprintln!("❌ Failed to execute k0maru-router worker: {}", err);
                        std::process::exit(1);
                    }
                }
            }
        }
        Commands::E2e { task_id } => {
            println!(
                "{}",
                "🧪 [K0maru-Harness] E2E Pipeline Verification"
                    .bold()
                    .cyan()
            );
            println!("  Task ID:  {}", task_id);

            let opts = k0maru_harness::pipeline::PipelineOptions {
                task: format!("E2E self-test ({})", task_id),
                domain: "E2E_SELF_TEST".to_string(),
                node: None,
                verify: false,
                red_check: false,
                workspace: None,
                exec_cmd: Some("echo e2e-self-test-ok".to_string()),
                vault: None,
                json: false,
            };

            match k0maru_harness::pipeline::run_pipeline(&opts) {
                Ok(res) => {
                    // Clean up the emitted TASK_READY tuple via tuple in roundtrip
                    let mut tuple_roundtrip_ok = false;
                    if res.tuple_id.is_some() {
                        if let Some(router_bin) =
                            k0maru_harness::probe::find_binary("k0maru-router")
                        {
                            if let Ok(out) = std::process::Command::new(&router_bin)
                                .args([
                                    "tuple",
                                    "in",
                                    "--kind",
                                    "TASK_READY",
                                    "--domain",
                                    "E2E_SELF_TEST",
                                    "--json",
                                ])
                                .output()
                            {
                                tuple_roundtrip_ok = out.status.success();
                            }
                        }
                    }

                    println!(
                        "  Probe Ready:     {}",
                        if res.probe_passed { "YES" } else { "NO" }
                    );
                    println!(
                        "  JIT Activated:   {} lines",
                        res.jit_invariants.lines().count()
                    );
                    println!("  Exec Passed:     {}", res.exec_passed == Some(true));
                    println!("  Tuple Roundtrip: {}", tuple_roundtrip_ok);

                    if res.probe_passed && res.exec_passed == Some(true) {
                        println!(
                            "{}",
                            "✅ E2E self-test pipeline executed and verified successfully.".green()
                        );
                    } else {
                        eprintln!("{}", "❌ E2E self-test pipeline failed.".red());
                        std::process::exit(1);
                    }
                }
                Err(err) => {
                    eprintln!("❌ E2E self-test failed: {}", err);
                    std::process::exit(1);
                }
            }
        }
    }
}
