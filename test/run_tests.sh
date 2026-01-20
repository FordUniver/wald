#!/usr/bin/env bash
# Test runner for wald integration tests
# Executes test cases and outputs TAP (Test Anything Protocol) format
#
# Usage:
#   ./run_tests.sh [wald-binary] [test_file...]
#
# Examples:
#   ./run_tests.sh                                    # Run all tests with cargo
#   ./run_tests.sh target/release/wald                # Use built binary
#   ./run_tests.sh "cargo run --" cases/test_plant.sh # Run specific test

# Note: We use -uo pipefail but NOT -e because:
# - Assertions return non-zero on failure (expected behavior)
# - We want to continue running tests after failures
# - Each test file handles its own error reporting via TAP
set -uo pipefail

# ====================================================================================
# Configuration
# ====================================================================================

# Get script directory (test/)
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Default wald binary (use cargo run in development)
WALD_BIN="${1:-cargo run --quiet --}"

# If WALD_BIN is a file path (not a command), make it absolute
# This ensures it works after tests cd to temp directories
if [[ -f "$WALD_BIN" ]]; then
    WALD_BIN="$(cd "$(dirname "$WALD_BIN")" && pwd)/$(basename "$WALD_BIN")"
fi

# Test files to run (default: all test_*.sh in cases/)
shift 2>/dev/null || true
if [[ $# -gt 0 ]]; then
    TEST_FILES=("$@")
else
    TEST_FILES=()
    while IFS= read -r -d '' file; do
        TEST_FILES+=("$file")
    done < <(find "$SCRIPT_DIR/cases" -name 'test_*.sh' -type f -print0 2>/dev/null | sort -z)
fi

# Debug mode (set DEBUG=1 to see detailed output)
DEBUG="${DEBUG:-0}"

# ====================================================================================
# Load Test Libraries
# ====================================================================================

# Source assertion library
# shellcheck source=test/lib/assertions.sh
source "$SCRIPT_DIR/lib/assertions.sh"

# Source setup library
# shellcheck source=test/lib/setup.sh
source "$SCRIPT_DIR/lib/setup.sh"

# Source helpers library
# shellcheck source=test/lib/helpers.sh
source "$SCRIPT_DIR/lib/helpers.sh"

# ====================================================================================
# Test Execution
# ====================================================================================

# Global counters (across all test files)
TOTAL_PASSED=0
TOTAL_FAILED=0
FAILED_TESTS=()

# Execute a single test file and parse its TAP output
run_test_file() {
    local test_file="$1"
    local test_name
    test_name="$(basename "$test_file" .sh)"

    if [[ $DEBUG -eq 1 ]]; then
        echo "# Running $test_name" >&2
    fi

    # Export WALD_BIN for test scripts
    export WALD_BIN

    # Run test file in subshell, capture TAP output
    # shellcheck disable=SC1090
    local output
    output=$(
        # Reset counters in subshell
        _TEST_PASSED=0
        _TEST_FAILED=0
        _TEST_CURRENT=""
        _CURRENT_TEST_FAILED=""
        _DIAGNOSTIC_OUTPUT=""

        source "$test_file"

        # Ensure summary is printed
        if [[ "${BASH_SOURCE[0]}" != "${0}" ]]; then
            print_summary 2>/dev/null || true
        fi
    ) || true  # Don't fail on non-zero exit (expected for failing tests)

    # Replay output for user (with renumbered test numbers)
    local file_passed=0
    local file_failed=0

    while IFS= read -r line; do
        if [[ "$line" =~ ^ok\ [0-9]+\ -\ (.*)$ ]]; then
            ((file_passed++)) || true
            local test_num=$((TOTAL_PASSED + TOTAL_FAILED + file_passed + file_failed))
            echo "ok $test_num - ${BASH_REMATCH[1]}"
        elif [[ "$line" =~ ^not\ ok\ [0-9]+\ -\ (.*)$ ]]; then
            ((file_failed++)) || true
            local test_num=$((TOTAL_PASSED + TOTAL_FAILED + file_passed + file_failed))
            echo "not ok $test_num - ${BASH_REMATCH[1]}"
        elif [[ "$line" =~ ^[[:space:]]*# ]]; then
            # Diagnostic line, pass through
            echo "$line"
        elif [[ "$line" =~ ^1\.\. ]]; then
            # Skip per-file plan line (we'll emit global plan at end)
            :
        elif [[ -n "$line" ]]; then
            # Other output (e.g., summary messages), show as comment
            echo "# $line"
        fi
    done <<< "$output"

    # Update global counters
    TOTAL_PASSED=$((TOTAL_PASSED + file_passed))
    TOTAL_FAILED=$((TOTAL_FAILED + file_failed))

    if [[ $file_failed -gt 0 ]]; then
        FAILED_TESTS+=("$test_name")
    fi
}

# ====================================================================================
# Main Execution
# ====================================================================================

main() {
    # Check if wald binary is available (skip actual execution check since it might not be built yet)
    if [[ "$WALD_BIN" == "cargo run"* ]]; then
        # Verify Cargo.toml exists
        if [[ ! -f "$SCRIPT_DIR/../Cargo.toml" ]]; then
            echo "Error: Cargo.toml not found. Run from test/ directory or specify wald binary." >&2
            exit 1
        fi
    elif [[ "$WALD_BIN" != *"cargo"* && ! -x "$WALD_BIN" ]]; then
        echo "Error: wald binary not found or not executable: $WALD_BIN" >&2
        exit 1
    fi

    # Print header
    if [[ $DEBUG -eq 1 ]]; then
        echo "# wald integration tests" >&2
        echo "# Binary: $WALD_BIN" >&2
        echo "# Test files: ${#TEST_FILES[@]}" >&2
        echo "" >&2
    fi

    # Check if we have any test files
    if [[ ${#TEST_FILES[@]} -eq 0 ]]; then
        echo "# No test files found in $SCRIPT_DIR/cases/" >&2
        echo "1..0 # SKIP no tests found"
        exit 0
    fi

    # Run each test file
    for test_file in "${TEST_FILES[@]}"; do
        if [[ ! -f "$test_file" ]]; then
            echo "# Warning: test file not found: $test_file" >&2
            continue
        fi

        run_test_file "$test_file"
    done

    # Print TAP summary
    echo ""
    echo "1..$((TOTAL_PASSED + TOTAL_FAILED))"

    # Print final summary
    if [[ $TOTAL_FAILED -eq 0 ]]; then
        echo "# All $((TOTAL_PASSED)) tests passed"
        exit 0
    else
        echo "# $TOTAL_FAILED of $((TOTAL_PASSED + TOTAL_FAILED)) tests failed"
        echo "# Failed test files:"
        for failed in "${FAILED_TESTS[@]}"; do
            echo "#   - $failed"
        done
        exit 1
    fi
}

# ====================================================================================
# Entry Point
# ====================================================================================

# Handle script being sourced vs executed
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    main "$@"
fi
