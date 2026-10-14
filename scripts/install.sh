#!/usr/bin/env bash
set -euo pipefail

# K0maru-Harness One-Command Provisioner (Local & Remote Tailscale/SSH)
# Usage:
#   ./scripts/install.sh                       # Provision local workstation
#   ./scripts/install.sh --remote company-pc   # Sync & provision remote host

REMOTE_HOST=""
WORKSPACE_ROOT="${WORKSPACE_ROOT:-$HOME/workspace}"
VAULT_PATH="${SECOND_BRAIN_VAULT:-$HOME/Documents/SecondBrain}"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --remote)
      REMOTE_HOST="${2:-}"
      if [[ -z "$REMOTE_HOST" ]]; then
        echo "Error: --remote requires an SSH host argument (e.g. --remote company-pc)" >&2
        exit 1
      fi
      shift 2
      ;;
    --workspace)
      WORKSPACE_ROOT="${2:-}"
      shift 2
      ;;
    --vault)
      VAULT_PATH="${2:-}"
      shift 2
      ;;
    -h|--help)
      echo "Usage: $0 [--remote <ssh-host>] [--workspace <dir>] [--vault <dir>]"
      exit 0
      ;;
    *)
      echo "Unknown argument: $1" >&2
      exit 1
      ;;
  esac
done

install_binary() {
  local src_bin="$1"
  local bin_name="$2"
  mkdir -p "$HOME/.local/bin" "$HOME/.cargo/bin"
  rm -f "$HOME/.local/bin/$bin_name" "$HOME/.cargo/bin/$bin_name"
  install -m 755 "$src_bin" "$HOME/.local/bin/$bin_name"
  install -m 755 "$src_bin" "$HOME/.cargo/bin/$bin_name"
  if [[ "$(uname -s)" == "Darwin" ]]; then
    codesign -s - -f "$HOME/.local/bin/$bin_name" >/dev/null 2>&1 || true
    codesign -s - -f "$HOME/.cargo/bin/$bin_name" >/dev/null 2>&1 || true
  fi
}

provision_local() {
  export PATH="$HOME/.cargo/bin:$HOME/.local/bin:$PATH"
  echo "=== [1/4] Building & Installing K0maru Micro-Kernels ==="
  local pairs=(
    "k0maru-harness:k0maru-harness"
    "k0maru-agent-verifier:k0maru-verifier"
    "k0maru-agent-router:k0maru-router"
    "k0maru-agent-workspace:k0maru-workspace"
    "k0maru-agent-memory:k0maru"
  )

  for entry in "${pairs[@]}"; do
    local repo="${entry%%:*}"
    local bin="${entry##*:}"
    local repo_dir="$WORKSPACE_ROOT/$repo"
    if [[ -d "$repo_dir" ]]; then
      echo "  -> Building $repo ($bin)..."
      if (cd "$repo_dir" && cargo build --release); then
        install_binary "$repo_dir/target/release/$bin" "$bin"
      elif command -v "$bin" >/dev/null 2>&1; then
        echo "  [WARN] Build skipped/fell back to existing $bin in PATH"
      else
        echo "  [ERROR] Failed to build $repo and no existing binary found" >&2
        exit 1
      fi
      (cd "$repo_dir" && k0maru-harness init-hook >/dev/null 2>&1 || true)
    fi
  done

  echo "=== [2/5] Synchronizing Agent Skills (k0maru-skills) ==="
  if [[ -x "$WORKSPACE_ROOT/k0maru-skills/scripts/sync.sh" ]]; then
    "$WORKSPACE_ROOT/k0maru-skills/scripts/sync.sh" >/dev/null
  fi

  echo "=== [3/5] Running Deterministic Harness Setup (MCP & Schemas) ==="
  k0maru-harness setup --vault "$VAULT_PATH"

  echo "=== [4/5] Synchronizing SecondBrain Cognitive Vault ==="
  if [[ -d "$VAULT_PATH" ]] && command -v k0maru >/dev/null 2>&1; then
    k0maru sync -v "$VAULT_PATH" || true
    (cd "$VAULT_PATH" && k0maru-harness init-hook >/dev/null 2>&1 || true)
  fi

  echo "=== [5/5] Running Final Readiness Probe ==="
  k0maru-harness probe
}

if [[ -z "$REMOTE_HOST" ]]; then
  provision_local
else
  echo "=== Syncing K0maru Repositories, Skills, Vault & Agy Customizations to $REMOTE_HOST ==="
  REMOTE_HOME="$(ssh "$REMOTE_HOST" 'printf %s "$HOME"')"
  ssh "$REMOTE_HOST" "mkdir -p '$REMOTE_HOME/workspace' '$REMOTE_HOME/Documents' '$REMOTE_HOME/.gemini/config/skills' '$REMOTE_HOME/.gemini/antigravity-cli/skills'"

  for repo in k0maru-harness k0maru-agent-verifier k0maru-agent-router k0maru-agent-workspace k0maru-agent-memory k0maru-skills; do
    if [[ -d "$WORKSPACE_ROOT/$repo" ]]; then
      echo "  -> Syncing $repo to $REMOTE_HOST..."
      rsync -az --exclude 'target' "$WORKSPACE_ROOT/$repo/" "$REMOTE_HOST:$REMOTE_HOME/workspace/$repo/"
    fi
  done

  if [[ -d "$VAULT_PATH" ]]; then
    echo "  -> Syncing SecondBrain vault to $REMOTE_HOST..."
    rsync -az --exclude '.git/objects/pack/tmp*' "$VAULT_PATH/" "$REMOTE_HOST:$REMOTE_HOME/workspace/SecondBrain/"
    ssh "$REMOTE_HOST" "ln -sfn '$REMOTE_HOME/workspace/SecondBrain' '$REMOTE_HOME/Documents/SecondBrain'"
  fi

  if [[ -f "$HOME/.gemini/config/GEMINI.md" ]]; then
    rsync -az "$HOME/.gemini/config/GEMINI.md" "$REMOTE_HOST:$REMOTE_HOME/.gemini/config/GEMINI.md"
    rsync -az "$HOME/.gemini/config/GEMINI.md" "$REMOTE_HOST:$REMOTE_HOME/.gemini/GEMINI.md"
  fi

  echo "=== Executing Remote Provisioning on $REMOTE_HOST ==="
  ssh "$REMOTE_HOST" "bash -lc 'cd ~/workspace/k0maru-harness && ./scripts/install.sh'"
fi
