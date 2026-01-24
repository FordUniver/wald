#!/usr/bin/env bash
# Tests for 'wald repo list' command

# Source test libraries (run_tests.sh handles this, but allow standalone execution)
if [[ -z "$WALD_BIN" ]]; then
    SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
    source "$SCRIPT_DIR/lib/assertions.sh"
    source "$SCRIPT_DIR/lib/setup.sh"
    source "$SCRIPT_DIR/lib/helpers.sh"
    WALD_BIN="${WALD_BIN:-cargo run --quiet --}"
fi

# ====================================================================================
# Basic list tests
# ====================================================================================

begin_test "wald repo list shows no repos message when empty"
    setup_wald_workspace

    # List on fresh workspace
    _result=$($WALD_BIN repo list 2>&1)
    assert_contains "$_result" "No repositories"

    teardown_wald_workspace
end_test

begin_test "wald repo list shows registered repos"
    setup_wald_workspace

    # Add repos
    $WALD_BIN repo add --no-clone github.com/test/repo1
    $WALD_BIN repo add --no-clone github.com/test/repo2

    # List should show both
    _result=$($WALD_BIN repo list 2>&1)
    assert_contains "$_result" "github.com/test/repo1"
    assert_contains "$_result" "github.com/test/repo2"

    teardown_wald_workspace
end_test

begin_test "wald repo list shows LFS policy"
    setup_wald_workspace

    # Add repo with custom LFS policy
    $WALD_BIN repo add --no-clone --lfs=full github.com/test/repo

    # List should show LFS policy
    _result=$($WALD_BIN repo list 2>&1)
    assert_contains "$_result" "lfs:full"

    teardown_wald_workspace
end_test

begin_test "wald repo list shows depth policy"
    setup_wald_workspace

    # Add repo with custom depth
    $WALD_BIN repo add --no-clone --depth=50 github.com/test/repo

    # List should show depth
    _result=$($WALD_BIN repo list 2>&1)
    assert_contains "$_result" "depth:50"

    teardown_wald_workspace
end_test

begin_test "wald repo list shows cloned status"
    setup_wald_workspace

    # Add repo without cloning
    $WALD_BIN repo add --no-clone github.com/test/not-cloned

    # Create bare repo for another
    create_bare_repo "github.com/test/cloned" with_commits
    add_repo_to_manifest "github.com/test/cloned"

    # List should show "cloned" only for the one with bare repo
    _result=$($WALD_BIN repo list 2>&1)

    # The cloned one should show "cloned"
    # Note: grep for the specific line
    echo "$_result" | grep "github.com/test/cloned" | grep -q "cloned" || \
        _fail "cloned repo should show 'cloned' status"

    teardown_wald_workspace
end_test

begin_test "wald repo list shows aliases"
    setup_wald_workspace

    # Add repo with aliases
    $WALD_BIN repo add --no-clone --alias=dots --alias=df github.com/user/dotfiles

    # List should show aliases
    _result=$($WALD_BIN repo list 2>&1)
    assert_contains "$_result" "aliases"

    teardown_wald_workspace
end_test

begin_test "wald repo list shows upstream"
    setup_wald_workspace

    # Add repo with upstream
    $WALD_BIN repo add --no-clone --upstream=git.zib.de/docker/ais2t git.zib.de/cspiegel/ais2t

    # List should show upstream
    _result=$($WALD_BIN repo list 2>&1)
    assert_contains "$_result" "upstream:git.zib.de/docker/ais2t"

    teardown_wald_workspace
end_test

begin_test "wald repo list --json produces valid JSON"
    setup_wald_workspace

    # Add repos
    $WALD_BIN repo add --no-clone github.com/test/repo1
    $WALD_BIN repo add --no-clone github.com/test/repo2

    # List with JSON output
    _result=$($WALD_BIN repo list --json 2>&1)

    # Should be valid JSON (contains braces and keys)
    assert_contains "$_result" "{"
    assert_contains "$_result" "github.com/test/repo1"
    assert_contains "$_result" "github.com/test/repo2"

    teardown_wald_workspace
end_test

begin_test "wald repo list is sorted alphabetically"
    setup_wald_workspace

    # Add repos in non-alphabetical order
    $WALD_BIN repo add --no-clone github.com/zzz/repo
    $WALD_BIN repo add --no-clone github.com/aaa/repo
    $WALD_BIN repo add --no-clone github.com/mmm/repo

    # List should be sorted
    _result=$($WALD_BIN repo list 2>&1)

    # Extract order of repo names
    _first=$(echo "$_result" | grep -n "github.com" | head -1 | cut -d: -f1)
    _aaa_line=$(echo "$_result" | grep -n "aaa" | cut -d: -f1)
    _mmm_line=$(echo "$_result" | grep -n "mmm" | cut -d: -f1)
    _zzz_line=$(echo "$_result" | grep -n "zzz" | cut -d: -f1)

    # aaa should come before mmm, mmm before zzz
    if [[ "$_aaa_line" -gt "$_mmm_line" ]] || [[ "$_mmm_line" -gt "$_zzz_line" ]]; then
        _fail "repos should be sorted alphabetically"
    fi

    teardown_wald_workspace
end_test

# Print summary if running standalone
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    print_summary
fi
