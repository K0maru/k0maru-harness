use std::path::{Path, PathBuf};

use crate::error::HarnessError;

pub const PRE_COMMIT_HOOK_CONTENT: &str = r#"#!/usr/bin/env bash
set -euo pipefail
echo "🛡️  [K0maru-Harness] Running pre-commit physical gate..."
# 1. Format check (Rust repositories only)
if [ -f "Cargo.toml" ]; then
    if ! cargo fmt --all --check >/dev/null 2>&1; then
        echo "❌ [Pre-Commit Denied]: Rust code formatting failed! Run 'cargo fmt'." >&2
        exit 1
    fi
fi
if command -v k0maru >/dev/null 2>&1; then
    if ! k0maru guard check --diff >/dev/null 2>&1; then
        echo "❌ [Pre-Commit Denied]: Living ADR invariant violation! Run 'k0maru-harness memory jit --decision \"<topic>\"'." >&2
        exit 1
    fi
fi
echo "✅ [K0maru-Harness] Pre-commit gate passed."
exit 0
"#;

/// Installs the deterministic physical pre-commit hook into the target git repository.
///
/// Detects `.git` in `target_dir` (or current directory if `None`).
/// Ensures `.git/hooks` exists, writes the `pre-commit` script, and makes it executable.
pub fn install_pre_commit_hook(target_dir: Option<&Path>) -> Result<PathBuf, HarnessError> {
    let root = match target_dir {
        Some(d) => d.to_path_buf(),
        None => std::env::current_dir()?,
    };

    let git_dir = if root.file_name() == Some(std::ffi::OsStr::new(".git")) {
        root
    } else {
        root.join(".git")
    };

    if !git_dir.exists() {
        return Err(HarnessError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("No .git repository found at {}", git_dir.display()),
        )));
    }

    let hooks_dir = git_dir.join("hooks");
    std::fs::create_dir_all(&hooks_dir)?;

    let hook_file = hooks_dir.join("pre-commit");
    std::fs::write(&hook_file, PRE_COMMIT_HOOK_CONTENT)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&hook_file)?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&hook_file, perms)?;
    }

    Ok(hook_file)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_install_pre_commit_hook_in_tempdir() {
        let temp = tempfile::tempdir().unwrap();
        let git_dir = temp.path().join(".git");
        std::fs::create_dir_all(&git_dir).unwrap();

        let res = install_pre_commit_hook(Some(temp.path()));
        assert!(res.is_ok());
        let hook_path = res.unwrap();
        assert!(hook_path.exists());
        assert_eq!(hook_path, git_dir.join("hooks/pre-commit"));

        let content = std::fs::read_to_string(&hook_path).unwrap();
        assert!(content.contains("Running pre-commit physical gate"));
        assert!(content.contains("cargo fmt --all --check"));
        assert!(content.contains("k0maru guard check --diff"));

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&hook_path).unwrap().permissions().mode();
            assert_eq!(mode & 0o111, 0o111);
        }
    }

    #[test]
    fn test_install_pre_commit_hook_missing_git() {
        let temp = tempfile::tempdir().unwrap();
        let res = install_pre_commit_hook(Some(temp.path()));
        assert!(res.is_err());
        match res.unwrap_err() {
            HarnessError::Io(err) => {
                assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
            }
            other => panic!("Expected HarnessError::Io, got {:?}", other),
        }
    }
}
