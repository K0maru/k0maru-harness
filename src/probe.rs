use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Availability {
    Available { version: String, path: PathBuf },
    Missing,
    Error(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentStatus {
    pub name: String,
    pub role: String,
    pub binary: String,
    pub required: bool,
    pub availability: Availability,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BusStatus {
    pub name: String,
    pub path: PathBuf,
    pub exists: bool,
    pub writable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarnessHealthReport {
    pub timestamp: String,
    pub components: Vec<ComponentStatus>,
    pub buses: Vec<BusStatus>,
    pub ready: bool,
}

pub struct ComponentProbe;

pub fn find_binary(binary: &str) -> Option<PathBuf> {
    ComponentProbe::find_binary(binary)
}

impl ComponentProbe {
    pub fn find_binary(binary: &str) -> Option<PathBuf> {
        if let Some(paths) = std::env::var_os("PATH") {
            for path in std::env::split_paths(&paths) {
                let candidate = path.join(binary);
                if candidate.is_file() {
                    return Some(candidate);
                }
            }
        }
        if let Some(home) = dirs::home_dir() {
            let local_bin = home.join(".local/bin").join(binary);
            if local_bin.is_file() {
                return Some(local_bin);
            }
            let cargo_bin = home.join(".cargo/bin").join(binary);
            if cargo_bin.is_file() {
                return Some(cargo_bin);
            }
        }
        None
    }

    pub fn probe_binary(name: &str, role: &str, binary: &str, required: bool) -> ComponentStatus {
        let path = Self::find_binary(binary);

        let availability = match path {
            Some(p) => {
                let ver_output = Command::new(&p).arg("--version").output();
                match ver_output {
                    Ok(out) => {
                        let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
                        let version = if stdout.is_empty() {
                            String::from_utf8_lossy(&out.stderr).trim().to_string()
                        } else {
                            stdout
                        };
                        Availability::Available {
                            version: if version.is_empty() {
                                "unknown".to_string()
                            } else {
                                version
                            },
                            path: p,
                        }
                    }
                    Err(e) => Availability::Error(e.to_string()),
                }
            }
            None => Availability::Missing,
        };

        ComponentStatus {
            name: name.to_string(),
            role: role.to_string(),
            binary: binary.to_string(),
            required,
            availability,
        }
    }

    pub fn check_directory(name: &str, dir: &Path) -> BusStatus {
        let exists = dir.exists();
        let writable = if exists {
            let test_file = dir.join(".probe_write_test");
            if std::fs::write(&test_file, b"ok").is_ok() {
                let _ = std::fs::remove_file(&test_file);
                true
            } else {
                false
            }
        } else {
            false
        };

        BusStatus {
            name: name.to_string(),
            path: dir.to_path_buf(),
            exists,
            writable,
        }
    }

    pub fn run_full_probe() -> HarnessHealthReport {
        let components = vec![
            Self::probe_binary(
                "Router",
                "L3/L4 Task Routing & Linda Bus",
                "k0maru-router",
                true,
            ),
            Self::probe_binary(
                "Workspace",
                "L1 Worktree Sandbox Lifecycle",
                "k0maru-workspace",
                false,
            ),
            Self::probe_binary(
                "Verifier",
                "L2 Deterministic Physical Oracle",
                "k0maru-verifier",
                false,
            ),
            Self::probe_binary(
                "Memory",
                "L0 Cognitive Hub & Living ADR Guard",
                "k0maru",
                false,
            ),
        ];

        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        let tuple_dir = home.join(".k0maru/tuples");
        let receipt_dir = home.join(".k0maru/receipts");

        // Ensure directories exist or can be created
        let _ = std::fs::create_dir_all(&tuple_dir);
        let _ = std::fs::create_dir_all(&receipt_dir);

        let buses = vec![
            Self::check_directory("Linda Tuple Space", &tuple_dir),
            Self::check_directory("Execution Receipts Ledger", &receipt_dir),
        ];

        let ready = components
            .iter()
            .all(|c| !c.required || matches!(c.availability, Availability::Available { .. }))
            && buses.iter().all(|b| b.exists && b.writable);

        HarnessHealthReport {
            timestamp: chrono::Utc::now().to_rfc3339(),
            components,
            buses,
            ready,
        }
    }
}
