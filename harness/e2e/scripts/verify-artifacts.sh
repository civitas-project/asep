#!/usr/bin/env bash
# verify-artifacts.sh <run-dir>
#
# Deterministic gate for an ASEP e2e run directory (P0-P7, see RUNBOOK.md):
#   1. every required artifact exists (manifest.json, 01-prd.md .. 07-ci.log)
#   2. test hash lock: sha256 of the dummy project's test files matches
#      03-hash.txt, unless a controller ruling artifact exists
#   3. re-run `cargo test` in the dummy project
#   4. re-run `asep check <run-dir> --rigor T4`
#
# Writes <run-dir>/verify.json with fields pass, artifacts, hash_lock_ok,
# test_rerun, check_rerun, reasons. Exits 0 iff pass == true, else 1.
#
# The judge's verdict is only a claim; these re-runs are authoritative.
#
# Test files are regular files under <run-dir> matching '*test*.rs'.
# The controller ruling artifact is <run-dir>/controller-ruling.json;
# its presence overrides the hash lock.

set -euo pipefail

if [ "$#" -ne 1 ]; then
    echo "usage: $0 <run-dir>" >&2
    exit 2
fi

run_dir="$(cd "$1" && pwd)"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

# --- 1. required artifacts -------------------------------------------------
artifacts=(
    "manifest.json"
    "01-prd.md" "01-judge.json"
    "02-tdd.md" "02-judge.json"
    "03-tests.diff" "03-red.log" "03-hash.txt"
    "04-impl.diff"
    "05-findings.md" "05-loom.log"
    "06-optimization.md"
    "07-judge.json" "07-check.log" "07-ci.log"
)

: > "$work/artifacts.txt"
for name in "${artifacts[@]}"; do
    if [ -f "$run_dir/$name" ]; then
        printf '%s true\n' "$name" >> "$work/artifacts.txt"
    else
        printf '%s false\n' "$name" >> "$work/artifacts.txt"
    fi
done

# --- 2. test hash lock -----------------------------------------------------
hash_lock_ok=false
hash_lock_reason="not evaluated"

if [ -f "$run_dir/controller-ruling.json" ]; then
    hash_lock_ok=true
    hash_lock_reason="controller ruling overrides hash lock"
elif [ -f "$run_dir/03-hash.txt" ]; then
    mapfile -t test_files < <(find "$run_dir" -type f -name '*test*.rs' | sort)
    if [ "${#test_files[@]}" -eq 0 ]; then
        hash_lock_reason="no dummy test files found to lock"
    else
        listing="$work/hash-listing.txt"
        : > "$listing"
        for f in "${test_files[@]}"; do
            rel="${f#$run_dir/}"
            hex="$(sha256sum "$f" | awk '{print $1}')"
            printf '%s  %s\n' "$hex" "$rel" >> "$listing"
        done
        computed="$(LC_ALL=C sort "$listing" | sha256sum | awk '{print $1}')"
        stored="$(tr -d ' \t\r\n' < "$run_dir/03-hash.txt" | awk '{print $1}')"
        if [ -n "$stored" ] && [ "$computed" = "$stored" ]; then
            hash_lock_ok=true
            hash_lock_reason="hash lock matches 03-hash.txt"
        else
            hash_lock_reason="hash lock failed: 03-hash.txt does not match current test files"
        fi
    fi
else
    hash_lock_reason="03-hash.txt missing"
fi

printf '{"ok": %s, "reason": "%s"}\n' "$hash_lock_ok" "$hash_lock_reason" > "$work/hash_lock.json"

# --- 3. re-run cargo test in the dummy project -----------------------------
test_rerun_code=0
( cd "$run_dir" && cargo test ) > "$work/test_rerun.out" 2>&1 || test_rerun_code=$?

# --- 4. re-run asep check --rigor T4 ---------------------------------------
check_rerun_code=0
( "$(python3 - "$run_dir/manifest.json" <<'PY'
import json, sys
try:
    b = json.load(open(sys.argv[1])).get("asep_binary") or ""
except Exception:
    b = ""
import os
print(b if b and os.path.isfile(b) and os.access(b, os.X_OK) else "asep")
PY
)" check "$run_dir" --rigor T4 ) > "$work/check_rerun.out" 2>&1 || check_rerun_code=$?

# --- 5. assemble verify.json -----------------------------------------------
verdict="$(python3 - "$run_dir" "$work" "$test_rerun_code" "$check_rerun_code" <<'PY'
import json
import os
import sys

run_dir, work = sys.argv[1], sys.argv[2]
test_rerun_code = int(sys.argv[3])
check_rerun_code = int(sys.argv[4])
limit = 200000


def read_output(path):
    try:
        with open(path, "rb") as f:
            data = f.read()
    except OSError:
        return ""
    if len(data) > limit:
        data = data[:limit] + b"\n... [output truncated] ...\n"
    return data.decode("utf-8", errors="replace")


artifacts = {}
missing = []
with open(os.path.join(work, "artifacts.txt"), encoding="utf-8") as f:
    for line in f:
        line = line.rstrip("\n")
        if not line:
            continue
        name, present = line.split(" ", 1)
        artifacts[name] = present == "true"
        if present == "false":
            missing.append(name)

with open(os.path.join(work, "hash_lock.json"), encoding="utf-8") as f:
    hash_lock = json.load(f)

reasons = []
if missing:
    reasons.append("missing artifacts: " + ", ".join(missing))
if not hash_lock["ok"]:
    reasons.append("hash lock: " + hash_lock["reason"])
if test_rerun_code != 0:
    reasons.append("cargo test re-run failed (exit %d)" % test_rerun_code)
if check_rerun_code != 0:
    reasons.append("asep check re-run failed (exit %d)" % check_rerun_code)

pass_ = (
    not missing
    and hash_lock["ok"]
    and test_rerun_code == 0
    and check_rerun_code == 0
)

asep_binary = "asep"
try:
    with open(os.path.join(run_dir, "manifest.json"), encoding="utf-8") as f:
        b = json.load(f).get("asep_binary") or ""
    if b and os.path.isfile(b) and os.access(b, os.X_OK):
        asep_binary = b
except Exception:
    pass

verify = {
    "pass": pass_,
    "artifacts": artifacts,
    "hash_lock_ok": hash_lock["ok"],
    "test_rerun": {
        "command": ["cargo", "test"],
        "cwd": run_dir,
        "exit_code": test_rerun_code,
        "passed": test_rerun_code == 0,
        "output": read_output(os.path.join(work, "test_rerun.out")),
    },
    "check_rerun": {
        "command": [asep_binary, "check", run_dir, "--rigor", "T4"],
        "exit_code": check_rerun_code,
        "passed": check_rerun_code == 0,
        "output": read_output(os.path.join(work, "check_rerun.out")),
    },
    "reasons": reasons,
}

with open(os.path.join(run_dir, "verify.json"), "w", encoding="utf-8") as f:
    json.dump(verify, f, indent=2)
    f.write("\n")

print("PASS" if pass_ else "FAIL")
PY
)"

if [ "$verdict" = "PASS" ]; then
    exit 0
else
    exit 1
fi
