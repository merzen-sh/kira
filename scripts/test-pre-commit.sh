#!/usr/bin/env bash
# Focused tests for .githooks/pre-commit.
#
# Covers staged-only, unstaged, partially staged, untracked, and failing
# (formatting/clippy) changes. Every test asserts that the hook leaves the
# index and worktree exactly as it found them.
set -uo pipefail

root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
hook="$root/.githooks/pre-commit"
work=$(mktemp -d "${TMPDIR:-/tmp}/kira-pre-commit.XXXXXX")
passed=0
failed=0

cleanup() {
    rm -rf "$work"
}
trap cleanup EXIT

fail() {
    printf 'not ok - %s\n' "$1" >&2
    return 1
}

pass() {
    printf 'ok - %s\n' "$1"
}

check() {
    local name=$1
    shift

    if "$@"; then
        pass "$name"
        passed=$((passed + 1))
    else
        fail "$name"
        failed=$((failed + 1))
    fi
}

new_repo() {
    repo="$work/$1"
    mkdir -p "$repo/src" "$repo/.githooks"
    cp "$hook" "$repo/.githooks/pre-commit"
    chmod +x "$repo/.githooks/pre-commit"

    git -C "$repo" init -q
    git -C "$repo" config user.email test@example.com
    git -C "$repo" config user.name test
    git -C "$repo" config core.hooksPath .githooks

    printf '[package]\nname = "kira"\nversion = "0.1.0"\nedition = "2024"\n' >"$repo/Cargo.toml"
    cat >"$repo/src/lib.rs" <<'EOF'
#![allow(dead_code)]

pub fn value() -> u32 {
    1
}
EOF
    git -C "$repo" add Cargo.toml src
    git -C "$repo" commit -q --no-verify -m base
}

commit() {
    git -C "$repo" commit -q -m "$1" >/dev/null 2>&1
}

snapshot() {
    {
        git -C "$repo" diff --cached
        git -C "$repo" diff
        git -C "$repo" status --porcelain=v1 -- src Cargo.toml
        sha256sum "$repo/src"/*.rs
    } 2>/dev/null
}

staged_content() {
    git -C "$repo" show HEAD:src/lib.rs
}

test_staged_formatted_change() {
    new_repo staged
    cat >"$repo/src/lib.rs" <<'EOF'
#![allow(dead_code)]

pub fn value() -> u32 {
    2
}
EOF
    git -C "$repo" add src/lib.rs

    check "staged formatted change commits" commit staged
    check "staged change is committed" bash -c "git -C '$repo' show HEAD:src/lib.rs | grep -q '    2'"
    check "nothing left staged" bash -c "git -C '$repo' diff --cached --quiet"
}

test_staged_unformatted_change() {
    new_repo staged-unformatted
    printf 'pub fn value() -> u32 {2}\n' >"$repo/src/lib.rs"
    git -C "$repo" add src/lib.rs
    before=$(snapshot)

    if commit staged-unformatted; then
        fail "unformatted staged change is rejected"
        failed=$((failed + 1))
    else
        pass "unformatted staged change is rejected"
        passed=$((passed + 1))
    fi
    check "failed format check leaves state unchanged" test "$before" = "$(snapshot)"
}

test_unstaged_change_is_preserved() {
    new_repo unstaged
    cat >"$repo/src/lib.rs" <<'EOF'
#![allow(dead_code)]

pub fn value() -> u32 {
    2
}
EOF
    git -C "$repo" add src/lib.rs
    printf '\npub fn extra() {}\n' >>"$repo/src/lib.rs"

    check "staged change commits with unstaged edits" commit unstaged
    check "unstaged edit is not committed" bash -c "! git -C '$repo' show HEAD:src/lib.rs | grep -q 'extra'"
    check "unstaged edit remains in worktree" bash -c "grep -q 'extra' '$repo/src/lib.rs'"
    check "unstaged edit remains unstaged" test "$(git -C "$repo" status --porcelain -- src/lib.rs)" = " M src/lib.rs"
}

test_unformatted_unstaged_change_blocks() {
    new_repo unstaged-unformatted
    cat >"$repo/src/lib.rs" <<'EOF'
#![allow(dead_code)]

pub fn value() -> u32 {
    2
}
EOF
    git -C "$repo" add src/lib.rs
    printf '\npub fn extra(){}\n' >>"$repo/src/lib.rs"
    before=$(snapshot)

    if commit unstaged-unformatted; then
        fail "unformatted unstaged change blocks the commit"
        failed=$((failed + 1))
    else
        pass "unformatted unstaged change blocks the commit"
        passed=$((passed + 1))
    fi
    check "blocked commit leaves state unchanged" test "$before" = "$(snapshot)"
}

test_partially_staged_change() {
    new_repo partial
    cat >"$repo/src/lib.rs" <<'EOF'
#![allow(dead_code)]

pub fn value() -> u32 {
    2
}
EOF
    git -C "$repo" add src/lib.rs
    printf '\npub fn extra() {}\n' >>"$repo/src/lib.rs"

    check "partially staged change commits" commit partial
    check "only staged content is committed" bash -c "! git -C '$repo' show HEAD:src/lib.rs | grep -q 'extra'"
    check "unstaged content is preserved" bash -c "grep -q 'extra' '$repo/src/lib.rs'"
    check "only the unstaged hunk remains" test "$(git -C "$repo" status --porcelain -- src/lib.rs)" = " M src/lib.rs"
    check "nothing extra is staged" bash -c "git -C '$repo' diff --cached --quiet"
}

test_clippy_failure_leaves_state() {
    new_repo clippy
    cat >"$repo/src/lib.rs" <<'EOF'
#![allow(dead_code)]

pub fn value() -> u32 {
    let x = 2;
    x
}
EOF
    git -C "$repo" add src/lib.rs
    before=$(snapshot)

    if commit clippy; then
        fail "clippy failure blocks the commit"
        failed=$((failed + 1))
    else
        pass "clippy failure blocks the commit"
        passed=$((passed + 1))
    fi
    check "clippy failure leaves state unchanged" test "$before" = "$(snapshot)"
}

test_untracked_file_is_untouched() {
    new_repo untracked
    cat >"$repo/src/lib.rs" <<'EOF'
#![allow(dead_code)]

pub fn value() -> u32 {
    2
}
EOF
    git -C "$repo" add src/lib.rs
    printf 'pub fn untracked(){}\n' >"$repo/src/untracked.rs"
    before=$(sha256sum "$repo/src/untracked.rs")

    check "commit succeeds with an untracked file" commit untracked
    check "untracked file is unchanged" test "$before" = "$(sha256sum "$repo/src/untracked.rs")"
    check "untracked file is still untracked" bash -c "git -C '$repo' ls-files --error-unmatch src/untracked.rs >/dev/null 2>&1; test \$? -ne 0"
}

test_staged_formatted_change
test_staged_unformatted_change
test_unstaged_change_is_preserved
test_unformatted_unstaged_change_blocks
test_partially_staged_change
test_clippy_failure_leaves_state
test_untracked_file_is_untouched

printf '\n%d passed, %d failed\n' "$passed" "$failed"
[[ "$failed" -eq 0 ]]
