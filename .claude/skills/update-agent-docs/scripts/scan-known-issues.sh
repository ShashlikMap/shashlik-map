#!/usr/bin/env bash
# Collects candidates for the "Known broken" and "Not supported yet" sections of
# README_API.md. Everything printed is a CANDIDATE, not a finding: a human or an
# agent still has to decide whether it affects SDK consumers.
#
# Usage: scripts/scan-known-issues.sh [since-ref]

set -uo pipefail
cd "$(git rev-parse --show-toplevel)" || exit 1

SCRIPT_DIR=".claude/skills/update-agent-docs/scripts"
TRACKED="$SCRIPT_DIR/tracked-items.tsv"
DOCS="kmp/shared/README_API.md README.md llms.txt"

# Diff anchor: explicit arg > latest mapshared-* tag > merge-base with main.
# The merge-base fallback only covers this branch, not everything since the
# last release — the output says so, so nobody mistakes it for a release diff.
SINCE="${1:-}"
SINCE_LABEL="$SINCE (explicit)"
if [ -z "$SINCE" ]; then
  SINCE="$(git tag -l 'mapshared-*' --sort=-v:refname | head -1)"
  SINCE_LABEL="$SINCE (last release tag)"
fi
if [ -z "$SINCE" ]; then
  SINCE="$(git merge-base HEAD main 2>/dev/null || true)"
  SINCE_LABEL="merge-base with main ${SINCE:0:7} — NO mapshared-* tag yet, so this covers this branch only, not everything since the last release"
fi

# Prints "<line>" for the first fixed-string match of $2 in file $1, or nothing.
anchor_line() { grep -nF -- "$2" "$1" 2>/dev/null | head -1 | cut -d: -f1; }

echo "## Known-issue candidates"
echo
echo "Diff anchor: ${SINCE_LABEL:-none}"
echo

echo "### 0. Tracked items (from tracked-items.tsv)"
echo
echo "MISSING means the pattern is gone: the issue was likely fixed or rewritten."
echo "Investigate, then update the docs, tracked-items.tsv and SKILL.md together."
echo
grep -v '^#' "$TRACKED" | grep -v '^\s*$' | while IFS=$'\t' read -r id file pattern; do
  line="$(anchor_line "$file" "$pattern")"
  if [ -n "$line" ]; then
    echo "- $id: \`$(basename "$file"):$line\` ($file)"
  else
    echo "- $id: **MISSING** — pattern not found in $file"
  fi
done
echo

echo "### 1. TODO / FIXME in the published module and its FFI"
grep -rn "TODO\|FIXME" kmp/shared/src ffi-run/src 2>/dev/null || echo "_none_"
echo

echo "### 2. Commented-out code (disabled features, dead call sites)"
grep -rnE '^\s*//\s*(LaunchedEffect|fun |listOf\(|ios[A-Za-z]+\(|api\.)' \
  kmp/shared/src kmp/shared/build.gradle.kts 2>/dev/null || echo "_none_"
echo

echo "### 3. FFI surface changes (blind spot: uniffi is excluded from shared.api)"
echo
echo "Changes to \`Point\`, \`Color\`, \`ShapeType\` and \`ShashlikMapApi\` break consumers"
echo "but never appear in the shared.api diff. Read the declarations below, not just"
echo "the file names."
echo
if [ -n "$SINCE" ]; then
  # Declaration-level lines only: a full diff of lib.rs is mostly bodies.
  git diff "$SINCE" -- ffi-run/src/lib.rs 2>/dev/null \
    | grep -E '^[+-]\s*(pub (fn|struct|enum)|fn |#\[uniffi|    (pub )?fn )' \
    || echo "_no FFI declaration changes_"
else
  echo "_no diff anchor_"
fi
echo

echo "### 4. Public API changes"
if [ -n "$SINCE" ]; then
  git diff "$SINCE" -- kmp/shared/api/shared.api 2>/dev/null | grep -E '^[+-]\s*public' || echo "_no public API changes_"
else
  echo "_no diff anchor_"
fi
echo

echo "### 5. file:line citations in the docs"
echo
echo "STALE = the cited line is not where a tracked anchor in that file now sits;"
echo "the suggested line is the nearest anchor. UNTRACKED = no anchor for that file"
echo "matches; check the printed code by hand and consider adding an anchor."
echo
# shellcheck disable=SC2086
grep -noE '`[A-Za-z0-9_./-]+\.(kt|kts|rs|toml):[0-9]+`' $DOCS 2>/dev/null \
  | tr -d '`' | while IFS=: read -r doc docline cfile cline; do
  # Citations use either a repo path or a bare file name; resolve both.
  if [ -f "$cfile" ]; then path="$cfile"
  else path="$(git ls-files "*/$(basename "$cfile")" | grep -E '^(kmp/shared|ffi-run)/' | head -1)"; fi
  if [ -z "$path" ]; then
    echo "- $doc:$docline cites \`$cfile:$cline\` — **UNRESOLVED** file"
    continue
  fi
  best=""; bestdist=""
  while IFS=$'\t' read -r id tfile pattern; do
    [ "$tfile" = "$path" ] || continue
    l="$(anchor_line "$tfile" "$pattern")"; [ -n "$l" ] || continue
    d=$(( l > cline ? l - cline : cline - l ))
    if [ -z "$bestdist" ] || [ "$d" -lt "$bestdist" ]; then best="$l"; bestdist="$d"; fi
  done < <(grep -v '^#' "$TRACKED" | grep -v '^\s*$')
  code="$(sed -n "${cline}p" "$path" | sed 's/^[[:space:]]*//')"
  if [ -z "$best" ]; then
    echo "- $doc:$docline \`$cfile:$cline\` **UNTRACKED** → \`$code\`"
  elif [ "$bestdist" -eq 0 ]; then
    echo "- $doc:$docline \`$cfile:$cline\` OK"
  else
    echo "- $doc:$docline \`$cfile:$cline\` **STALE** → nearest anchor is line $best (cited line now: \`$code\`)"
  fi
done
echo

echo "### 6. Engine behaviour changes (map/src; invisible to sections 3 and 4)"
echo
echo "The renderer in map/ changes what overlays *do* without touching any"
echo "signature. Read the overlay diffs: scaling and validation live there."
echo
if [ -n "$SINCE" ]; then
  git diff --stat "$SINCE" -- map/src 2>/dev/null | grep -v 'changed,' || echo "_no engine changes_"
else
  echo "_no diff anchor_"
fi
