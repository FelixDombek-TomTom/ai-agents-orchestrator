#!/usr/bin/env bash
# Update AI Agents Orchestrator in place: fetch upstream, merge, rebuild.
# The release binary path is stable, so the desktop launcher keeps working —
# no reinstall needed.
#
#   bash scripts/update.sh            # pull your fork (origin) + merge upstream, rebuild
#   bash scripts/update.sh --no-upstream   # only pull origin, skip the upstream merge
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$HERE"

source "$HOME/.cargo/env" 2>/dev/null || true

WITH_UPSTREAM=1
[ "${1:-}" = "--no-upstream" ] && WITH_UPSTREAM=0

branch="$(git rev-parse --abbrev-ref HEAD)"
echo "→ on branch '$branch'"

echo "→ pulling origin/$branch"
git pull --ff-only origin "$branch" || {
  echo "  (non-fast-forward; resolve manually) " >&2; exit 1; }

if [ "$WITH_UPSTREAM" -eq 1 ] && git remote | grep -qx upstream; then
  echo "→ fetching upstream"
  git fetch upstream
  # Merge upstream's default branch (master) into the current branch.
  echo "→ merging upstream/master"
  git merge --no-edit upstream/master
fi

echo "→ building (release binary, no bundle)"
cargo tauri build --no-bundle

echo "✓ updated. Launch from your menu (AI Agents Orchestrator) or:"
echo "    $HERE/src-tauri/target/release/app"
