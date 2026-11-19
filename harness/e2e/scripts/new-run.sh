#!/usr/bin/env bash
# new-run.sh <run-id>
#
# Scaffold an ASEP e2e run directory and write manifest.json (P0 bootstrap).
#
#   new-run.sh <run-id>
#
# RUN_ROOT overrides the default run root (/tmp/opencode/asep-e2e) for tests.
#
set -euo pipefail

if [ "$#" -ne 1 ]; then
    echo "usage: $0 <run-id>" >&2
    exit 2
fi

run_id="$1"
run_root="${RUN_ROOT:-/tmp/opencode/asep-e2e}"
run_dir="$run_root/$run_id"

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$script_dir/../../.." && pwd)"

mkdir -p "$run_dir"

rustc_version="$(rustc -V)"
asep_commit="$(git -C "$repo_root" rev-parse --short HEAD)"
asep_binary="$repo_root/target/debug/asep"
# Deterministic 32-bit seed derived from the run-id hash.
seed="$(python3 -c 'import hashlib, sys; print(int(hashlib.sha256(sys.argv[1].encode()).hexdigest()[:8], 16))' "$run_id")"
created_at="$(date -u +%Y-%m-%dT%H:%M:%SZ)"

python3 - "$run_dir" "$run_id" "$seed" "$rustc_version" "$asep_commit" "$asep_binary" "$created_at" <<'PY'
import json
import os
import sys

run_dir, run_id, seed, rustc_version, asep_commit, asep_binary, created_at = sys.argv[1:8]
manifest = {
    "schema_version": 1,
    "run_id": run_id,
    "seed": int(seed),
    "rustc_version": rustc_version,
    "asep_commit": asep_commit,
    "asep_binary": asep_binary,
    "created_at": created_at,
}
with open(os.path.join(run_dir, "manifest.json"), "w", encoding="utf-8") as f:
    json.dump(manifest, f, indent=2)
    f.write("\n")
PY

echo "$run_dir"
