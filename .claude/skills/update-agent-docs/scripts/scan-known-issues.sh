#!/usr/bin/env bash
# Collects candidates for the "Known broken" and "Not supported yet" sections of
# README_API.md. Everything printed is a CANDIDATE, not a finding: a human or an
# agent still has to decide whether it affects SDK consumers.
#
# Usage: scripts/scan-known-issues.sh [since-ref]

set -uo pipefail
cd "$(git rev-parse --show-toplevel)" || exit 1

SINCE="${1:-$(git tag -l 'mapshared-*' --sort=-v:refname | head -1)}"

echo "## Known-issue candidates"
echo

echo "### 1. TODO / FIXME in the published module and its FFI"
grep -rn "TODO\|FIXME" kmp/shared/src ffi-run/src 2>/dev/null || echo "_none_"
echo

echo "### 2. Commented-out code (disabled features, dead call sites)"
grep -rnE '^\s*//\s*(LaunchedEffect|fun |listOf\(|ios[A-Za-z]+\(|api\.)' \
  kmp/shared/src kmp/shared/build.gradle.kts 2>/dev/null || echo "_none_"
echo

echo "### 3. FFI surface changes (blind spot: uniffi is excluded from shared.api)"
if [ -n "$SINCE" ]; then
  git diff --stat "$SINCE" -- ffi-run/src/lib.rs 2>/dev/null || echo "_ref '$SINCE' not usable_"
else
  echo "_no mapshared-* tag yet; tag a release so this diff has an anchor_"
fi
echo

echo "### 4. Public API changes since last release"
if [ -n "$SINCE" ]; then
  git diff "$SINCE" -- kmp/shared/api/shared.api 2>/dev/null | grep -E '^[+-]\s*public' || echo "_no public API changes_"
else
  echo "_no mapshared-* tag yet_"
fi
