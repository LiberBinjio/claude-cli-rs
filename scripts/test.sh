#!/usr/bin/env bash
# Full test suite with detailed per-crate reporting for claude-cli-rs.
# Usage: ./scripts/test.sh [--quick] [--show-all] [--stop-on-fail]
# Note: chmod +x scripts/test.sh before first run.
set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

# Parse args
QUICK=false
SHOW_ALL=false
STOP_ON_FAIL=false
for arg in "$@"; do
    case "$arg" in
        --quick) QUICK=true ;;
        --show-all) SHOW_ALL=true ;;
        --stop-on-fail) STOP_ON_FAIL=true ;;
        *) echo "Unknown argument: $arg"; exit 1 ;;
    esac
done

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
CYAN='\033[0;36m'
GRAY='\033[0;90m'
NC='\033[0m' # No Color

pass() { echo -e "  ${GREEN}[PASS]${NC} $1"; }
fail() { echo -e "  ${RED}[FAIL]${NC} $1"; }

TOTAL_STAGES=4
if [ "$QUICK" = true ]; then TOTAL_STAGES=2; fi

OVERALL_PASS=true
STAGE_NAMES=()
STAGE_RESULTS=()
STAGE_TIMES=()

cd "$PROJECT_ROOT"

echo -e "${CYAN}=== Claude CLI (Rust) - Test Suite ===${NC}"

# ── Stage 1: cargo check ──

echo ""
echo -e "${YELLOW}[1/$TOTAL_STAGES] cargo check --workspace${NC}"
START=$(date +%s)
CHECK_OUTPUT=$(cargo check --workspace 2>&1) || true
CHECK_EXIT=${PIPESTATUS[0]:-$?}
END=$(date +%s)
ELAPSED=$((END - START))

ERROR_COUNT=$(echo "$CHECK_OUTPUT" | grep -c '^error' || true)

STAGE_NAMES+=("check")
STAGE_TIMES+=("$ELAPSED")
if [ "$CHECK_EXIT" -eq 0 ] && [ "$ERROR_COUNT" -eq 0 ]; then
    pass "Passed (${ELAPSED}s) - 0 errors"
    STAGE_RESULTS+=(0)
else
    fail "Failed (${ELAPSED}s) - $ERROR_COUNT errors"
    if [ "$SHOW_ALL" = true ]; then echo "$CHECK_OUTPUT"; fi
    STAGE_RESULTS+=(1)
    OVERALL_PASS=false
    if [ "$STOP_ON_FAIL" = true ]; then
        echo -e "\n${RED}Stopped at cargo check${NC}"
        exit 1
    fi
fi

# ── Stage 2: cargo clippy ──

echo ""
echo -e "${YELLOW}[2/$TOTAL_STAGES] cargo clippy --workspace -- -D warnings${NC}"
START=$(date +%s)
CLIPPY_OUTPUT=$(cargo clippy --workspace -- -D warnings 2>&1) || true
CLIPPY_EXIT=${PIPESTATUS[0]:-$?}
END=$(date +%s)
ELAPSED=$((END - START))

STAGE_NAMES+=("clippy")
STAGE_TIMES+=("$ELAPSED")
if [ "$CLIPPY_EXIT" -eq 0 ]; then
    pass "Passed (${ELAPSED}s) - 0 warnings"
    STAGE_RESULTS+=(0)
else
    WARN_COUNT=$(echo "$CLIPPY_OUTPUT" | grep -c '^warning:' || true)
    fail "Failed (${ELAPSED}s) - $WARN_COUNT warnings"
    if [ "$SHOW_ALL" = true ]; then echo "$CLIPPY_OUTPUT"; fi
    STAGE_RESULTS+=(1)
    OVERALL_PASS=false
    if [ "$STOP_ON_FAIL" = true ]; then
        echo -e "\n${RED}Stopped at cargo clippy${NC}"
        exit 1
    fi
fi

if [ "$QUICK" = true ]; then
    echo ""
    echo -e "${GRAY}Quick mode: skipping test stages.${NC}"
else

    # ── Stage 3: cargo test ──

    echo ""
    echo -e "${YELLOW}[3/$TOTAL_STAGES] cargo test --workspace${NC}"
    START=$(date +%s)
    TEST_OUTPUT=$(cargo test --workspace 2>&1) || true
    TEST_EXIT=${PIPESTATUS[0]:-$?}
    END=$(date +%s)
    ELAPSED=$((END - START))

    # Parse per-crate results
    TOTAL_PASSED=0
    TOTAL_FAILED=0
    TOTAL_IGNORED=0

    CURRENT_CRATE="unknown"

    # Collect crate names and results
    declare -A CRATE_PASSED 2>/dev/null || true
    declare -A CRATE_FAILED 2>/dev/null || true
    declare -A CRATE_IGNORED 2>/dev/null || true
    CRATE_ORDER=()
    HAS_ASSOC=true

    # Check if associative arrays are supported (bash 4+)
    if ! declare -A _test_assoc 2>/dev/null; then
        HAS_ASSOC=false
    fi

    FAILED_TESTS=""

    while IFS= read -r line; do
        # Detect crate from "Running" lines
        if echo "$line" | grep -qE 'Running.*crates/([^/]+)/'; then
            CURRENT_CRATE=$(echo "$line" | sed -n 's/.*crates\/\([^/]*\)\/.*/\1/p')
        elif echo "$line" | grep -qE 'Running.*deps/([a-zA-Z0-9_]+)-'; then
            CURRENT_CRATE=$(echo "$line" | sed -n 's/.*deps\/\([a-zA-Z0-9_]*\)-.*/\1/p')
            CURRENT_CRATE=$(echo "$CURRENT_CRATE" | tr '-' '_')
        fi

        # Individual test results
        if echo "$line" | grep -qE '^test .+ \.\.\. (ok|FAILED|ignored)'; then
            TEST_NAME=$(echo "$line" | sed 's/^test \(.*\) \.\.\..*/\1/')
            TEST_RESULT=$(echo "$line" | sed 's/.*\.\.\. //')

            if [ "$SHOW_ALL" = true ]; then
                case "$TEST_RESULT" in
                    ok)      echo -e "    ${GREEN}ok${NC}  $TEST_NAME" ;;
                    FAILED)  echo -e "    ${RED}FAILED${NC}  $TEST_NAME" ;;
                    ignored) echo -e "    ${GRAY}ignored${NC}  $TEST_NAME" ;;
                esac
            fi

            if [ "$TEST_RESULT" = "FAILED" ]; then
                FAILED_TESTS="$FAILED_TESTS    - ${CURRENT_CRATE}::${TEST_NAME}\n"
            fi
        fi

        # Aggregate "test result:" lines
        if echo "$line" | grep -qE '^test result:'; then
            P=$(echo "$line" | grep -oE '[0-9]+ passed' | grep -oE '[0-9]+')
            F=$(echo "$line" | grep -oE '[0-9]+ failed' | grep -oE '[0-9]+')
            I=$(echo "$line" | grep -oE '[0-9]+ ignored' | grep -oE '[0-9]+')
            P=${P:-0}; F=${F:-0}; I=${I:-0}

            TOTAL_PASSED=$((TOTAL_PASSED + P))
            TOTAL_FAILED=$((TOTAL_FAILED + F))
            TOTAL_IGNORED=$((TOTAL_IGNORED + I))

            if [ "$HAS_ASSOC" = true ]; then
                OLD_P=${CRATE_PASSED[$CURRENT_CRATE]:-0}
                OLD_F=${CRATE_FAILED[$CURRENT_CRATE]:-0}
                OLD_I=${CRATE_IGNORED[$CURRENT_CRATE]:-0}
                CRATE_PASSED[$CURRENT_CRATE]=$((OLD_P + P))
                CRATE_FAILED[$CURRENT_CRATE]=$((OLD_F + F))
                CRATE_IGNORED[$CURRENT_CRATE]=$((OLD_I + I))

                # Track ordering
                FOUND=false
                for c in "${CRATE_ORDER[@]}"; do
                    if [ "$c" = "$CURRENT_CRATE" ]; then FOUND=true; break; fi
                done
                if [ "$FOUND" = false ]; then CRATE_ORDER+=("$CURRENT_CRATE"); fi
            fi
        fi
    done <<< "$TEST_OUTPUT"

    # Print per-crate table
    echo ""
    echo -e "  ${CYAN}Per-crate results:${NC}"
    printf "  %-24s %8s %8s %8s\n" "Crate" "Passed" "Failed" "Ignored"
    printf "  %-24s %8s %8s %8s\n" "------------------------" "------" "------" "-------"

    if [ "$HAS_ASSOC" = true ]; then
        for crate in "${CRATE_ORDER[@]}"; do
            P=${CRATE_PASSED[$crate]:-0}
            F=${CRATE_FAILED[$crate]:-0}
            I=${CRATE_IGNORED[$crate]:-0}
            if [ "$F" -gt 0 ]; then
                printf "  ${RED}%-24s %8s %8s %8s${NC}\n" "$crate" "$P" "$F" "$I"
            else
                printf "  ${GREEN}%-24s %8s %8s %8s${NC}\n" "$crate" "$P" "$F" "$I"
            fi
        done
    fi

    printf "  %-24s %8s %8s %8s\n" "========================" "======" "======" "======="
    if [ "$TOTAL_FAILED" -gt 0 ]; then
        printf "  ${RED}%-24s %8s %8s %8s${NC}\n" "TOTAL" "$TOTAL_PASSED" "$TOTAL_FAILED" "$TOTAL_IGNORED"
    else
        printf "  ${GREEN}%-24s %8s %8s %8s${NC}\n" "TOTAL" "$TOTAL_PASSED" "$TOTAL_FAILED" "$TOTAL_IGNORED"
    fi

    STAGE_NAMES+=("test")
    STAGE_TIMES+=("$ELAPSED")
    if [ "$TEST_EXIT" -eq 0 ] && [ "$TOTAL_FAILED" -eq 0 ]; then
        echo ""
        pass "All tests passed (${ELAPSED}s) - $TOTAL_PASSED passed, $TOTAL_FAILED failed, $TOTAL_IGNORED ignored"
        STAGE_RESULTS+=(0)
    else
        echo ""
        fail "Tests failed (${ELAPSED}s) - $TOTAL_PASSED passed, $TOTAL_FAILED failed, $TOTAL_IGNORED ignored"
        if [ -n "$FAILED_TESTS" ]; then
            echo ""
            echo -e "  ${RED}Failed tests:${NC}"
            echo -e "$FAILED_TESTS"
        fi
        STAGE_RESULTS+=(1)
        OVERALL_PASS=false
        if [ "$STOP_ON_FAIL" = true ]; then
            echo -e "\n${RED}Stopped at cargo test${NC}"
            exit 1
        fi
    fi

    # ── Stage 4: cargo run --version ──

    echo ""
    echo -e "${YELLOW}[4/$TOTAL_STAGES] cargo run -- --version${NC}"
    START=$(date +%s)
    VERSION_OUTPUT=$(cargo run -p claude_cli -- --version 2>&1) || true
    VERSION_EXIT=${PIPESTATUS[0]:-$?}
    END=$(date +%s)
    ELAPSED=$((END - START))

    VERSION_LINE=$(echo "$VERSION_OUTPUT" | grep -E '[0-9]+\.[0-9]+\.[0-9]+' | grep -v -E '(Compiling|Downloading|Finished|warning)' | tail -1)
    VERSION_LINE=${VERSION_LINE:-"(no version output)"}

    STAGE_NAMES+=("version")
    STAGE_TIMES+=("$ELAPSED")
    if [ "$VERSION_EXIT" -eq 0 ]; then
        pass "Output: $(echo "$VERSION_LINE" | xargs) (${ELAPSED}s)"
        STAGE_RESULTS+=(0)
    else
        fail "cargo run -- --version failed (${ELAPSED}s)"
        STAGE_RESULTS+=(1)
        OVERALL_PASS=false
    fi
fi

# ── Final Summary ──

echo ""
echo -e "${CYAN}=== Summary ===${NC}"

TOTAL_TIME=0
for i in "${!STAGE_NAMES[@]}"; do
    NAME=${STAGE_NAMES[$i]}
    RESULT=${STAGE_RESULTS[$i]}
    TIME=${STAGE_TIMES[$i]}
    TOTAL_TIME=$((TOTAL_TIME + TIME))

    if [ "$RESULT" -eq 0 ]; then
        printf "  ${GREEN}[PASS]${NC}  %-20s %5ss\n" "$NAME" "$TIME"
    else
        printf "  ${RED}[FAIL]${NC}  %-20s %5ss\n" "$NAME" "$TIME"
    fi
done

echo ""
if [ "$OVERALL_PASS" = true ]; then
    echo -e "${GREEN}=== ALL STAGES PASSED === (total: ${TOTAL_TIME}s)${NC}"
    exit 0
else
    echo -e "${RED}=== SOME STAGES FAILED === (total: ${TOTAL_TIME}s)${NC}"
    exit 1
fi
