use clap::{Parser, Subcommand};
use colored::*;
use k0maru_harness::probe::{Availability, ComponentProbe};

#[derive(Parser, Debug)]
#[command(
    name = "k0maru",
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

        /// Run deterministic physical verification gate
        #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
        verify: bool,

        /// Disable physical verification gate
        #[arg(long)]
        no_verify: bool,

        /// Emit machine-readable JSON output
        #[arg(long)]
        json: bool,
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
            verify,
            no_verify,
            json,
        } => {
            let effective_verify = verify && !no_verify;
            let opts = k0maru_harness::pipeline::PipelineOptions {
                task: task.clone(),
                domain: domain.clone(),
                node: node.clone(),
                verify: effective_verify,
                workspace,
                json,
            };

            match k0maru_harness::pipeline::run_pipeline(&opts) {
                Ok(res) => {
                    if json {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&res)
                                .expect("Failed to serialize PipelineResult")
                        );
                    } else {
                        println!();
                        println!(
                            "{}",
                            "🚀 [K0maru-Harness] Task Pipeline Execution".bold().cyan()
                        );
                        println!(
                            "{}",
                            "═══════════════════════════════════════════════════════════════════"
                                .cyan()
                        );
                        println!("  Task ID:      {}", res.task_id.bold());
                        println!("  Task:         {}", task);
                        println!("  Domain:       {}", domain);
                        println!(
                            "  Node:         {}",
                            node.as_deref().unwrap_or("localhost (local)")
                        );
                        println!(
                            "  Workspace:    {}",
                            opts.workspace
                                .as_deref()
                                .map(|p| p.display().to_string())
                                .unwrap_or_else(|| ".".to_string())
                        );
                        println!();

                        let probe_badge = if res.probe_passed {
                            "READY".green().bold()
                        } else {
                            "DEGRADED".yellow().bold()
                        };
                        println!(
                            "  🔍 Phase 1: Component Readiness Probe -> [{}]",
                            probe_badge
                        );

                        println!(
                            "  🧠 Phase 2: JIT Invariant Activation  -> Activated ({} lines)",
                            res.jit_invariants.lines().count()
                        );

                        if let Some(tup) = &res.tuple_id {
                            println!(
                                "  🚌 Phase 3: Linda Space Task Ready     -> [{}] ({})",
                                "EMITTED".green().bold(),
                                tup.cyan()
                            );
                        } else {
                            println!(
                                "  🚌 Phase 3: Linda Space Task Ready     -> [{}]",
                                "SKIPPED / ROUTER OFFLINE".dimmed()
                            );
                        }

                        if effective_verify {
                            if res.verified {
                                let summary = res.receipt_summary.as_deref().unwrap_or("Passed");
                                println!(
                                    "  🛡️  Phase 4: Physical Verification     -> [{}] ({})",
                                    "VERIFIED".green().bold(),
                                    summary
                                );
                            } else {
                                let summary = res.receipt_summary.as_deref().unwrap_or("Failed");
                                println!(
                                    "  🛡️  Phase 4: Physical Verification     -> [{}] ({})",
                                    "FAILED".red().bold(),
                                    summary
                                );
                            }
                        } else {
                            println!(
                                "  🛡️  Phase 4: Physical Verification     -> [{}]",
                                "DISABLED (--no-verify)".dimmed()
                            );
                        }

                        println!(
                            "{}",
                            "═══════════════════════════════════════════════════════════════════"
                                .cyan()
                        );
                        println!();

                        if effective_verify && !res.verified {
                            eprintln!("{}", "❌ Physical verification gate failed!".red().bold());
                            std::process::exit(1);
                        }
                    }
                }
                Err(err) => {
                    eprintln!("Error executing pipeline: {}", err);
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
            println!(
                "{}",
                "ℹ️  Phase 1 active: Reactive claiming engine ready in k0maru-router.".yellow()
            );
        }
        Commands::E2e { task_id } => {
            println!(
                "{}",
                "🧪 [K0maru-Harness] E2E Pipeline Verification"
                    .bold()
                    .cyan()
            );
            println!("  Task ID:  {}", task_id);
            let report = ComponentProbe::run_full_probe();
            if report.ready {
                println!(
                    "{}",
                    "✅ All components ready for E2E orchestration.".green()
                );
            } else {
                eprintln!("{}", "❌ Harness components not fully ready.".red());
                std::process::exit(1);
            }
        }
    }
}
