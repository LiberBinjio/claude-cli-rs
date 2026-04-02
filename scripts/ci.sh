#!/usr/bin/env bash
# Full CI pipeline: check -> clippy -> test -> release build.
# Usage: ./scripts/ci.sh
# Note: chmod +x scripts/ci.sh if needed.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

echo "=== Claude CLI (Rust) — CI Pipeline ==="
echo "  Project: $PROJECT_ROOT"
echo ""

cd "$PROJECT_ROOT"

NAMES=("cargo check --workspace"
       "cargo clippy (deny warnings)"
       "cargo test --workspace"
       "cargo build --release")

CMDS=("cargo check --workspace"
      "cargo clippy --workspace -- -D warnings"
      "cargo test --workspace"
      "cargo build --release")

TOTAL=${#CMDS[@]}
PASSED=0
FAILED=0
TIMINGS=()
STATUSES=()

CI_START=$(date +%s)

for i in $(seq 0 $((TOTAL - 1))); do
    NUM=$((i + 1))
    echo "[$NUM/$TOTAL] ${NAMES[$i]}..."

    START=$(date +%s)
    if eval "${CMDS[$i]}" > /dev/null 2>&1; then
        END=$(date +%s)
        ELAPSED=$((END - START))
        echo "  PASSED (${ELAPSED}s)"
        PASSED=$((PASSED + 1))
        STATUSES+=("PASS")
    else
        END=$(date +%s)
        ELAPSED=$((END - START))
        echo "  FAILED (${ELAPSED}s)"
        FAILED=$((FAILED + 1))
        STATUSES+=("FAIL")
        echo ""
        echo "CI FAILED at stage: ${NAMES[$i]}"
        # Show the error by re-running
        echo "--- Error output ---"
        eval "${CMDS[$i]}" 2>&1 | tail -30 || true
        echo "---"
        TIMINGS+=("$ELAPSED")
        break
    fi
    TIMINGS+=("$ELAPSED")
done

CI_END=$(date +%s)
CI_TOTAL=$((CI_END - CI_START))

# Summary
echo ""
echo "=== CI Summary ==="

for i in $(seq 0 $((${#STATUSES[@]} - 1))); do
    printf "  %-40s %5ss  [%s]\n" "${NAMES[$i]}" "${TIMINGS[$i]}" "${STATUSES[$i]}"
done

# Skipped stages
REACHED=${#STATUSES[@]}
if [ "$REACHED" -lt "$TOTAL" ]; then
    for j in $(seq "$REACHED" $((TOTAL - 1))); do
        printf "  %-40s %5s   [%s]\n" "${NAMES[$j]}" "—" "SKIP"
    done
fi

echo ""
echo "  Total: ${CI_TOTAL}s | Passed: $PASSED/$TOTAL | Failed: $FAILED"

if [ "$FAILED" -gt 0 ]; then
    exit 1
fi
echo ""
echo "  All CI stages passed!"
exit 0
