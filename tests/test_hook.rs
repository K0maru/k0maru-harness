use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::process::Command as StdCommand;

use k0maru_harness::hook::{install_pre_commit_hook, PRE_COMMIT_HOOK_CONTENT};

#[test]
fn test_install_pre_commit_hook_temp_repo() {
    let temp = tempfile::tempdir().unwrap();
    let status = StdCommand::new("git")
        .arg("init")
        .current_dir(temp.path())
        .status()
        .expect("Failed to run git init");
    assert!(status.success());

    let res = install_pre_commit_hook(Some(temp.path()));
    assert!(res.is_ok());

    let hook_path = res.unwrap();
    assert!(hook_path.exists());
    assert_eq!(hook_path, temp.path().join(".git/hooks/pre-commit"));

    let content = fs::read_to_string(&hook_path).unwrap();
    assert_eq!(content, PRE_COMMIT_HOOK_CONTENT);
    assert!(content.contains("cargo fmt --all --check"));
    assert!(content.contains("k0maru guard check --diff"));
    assert!(content.contains("k0maru-harness memory record-lesson"));

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = fs::metadata(&hook_path).unwrap().permissions();
        assert_eq!(
            perms.mode() & 0o111,
            0o111,
            "Hook file should be executable"
        );
    }
}

#[test]
fn test_install_pre_commit_hook_no_git_fails() {
    let temp = tempfile::tempdir().unwrap();
    let res = install_pre_commit_hook(Some(temp.path()));
    assert!(res.is_err());
}

#[test]
fn test_cli_init_hook_command() {
    let temp = tempfile::tempdir().unwrap();
    let status = StdCommand::new("git")
        .arg("init")
        .current_dir(temp.path())
        .status()
        .expect("Failed to run git init");
    assert!(status.success());

    let mut cmd = Command::cargo_bin("k0maru-harness").unwrap();
    cmd.args(["init-hook", "--path", temp.path().to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Pre-commit gate hook installed at:",
        ));

    let hook_path = temp.path().join(".git/hooks/pre-commit");
    assert!(hook_path.exists());
}
