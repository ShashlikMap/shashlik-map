---
name: update-agent-docs
description: Refresh the Shashlik Map SDK's agent-facing docs (README_API.md, FACTS.md, shared.api) so they match the code. Use when preparing a mapshared release, after changing the public API of kmp/shared or ffi-run, or when the published docs are suspected to be out of date.
---

# Update agent docs

Keeps `kmp/shared/README_API.md`, the root `README.md` integration section
and `llms.txt` truthful for the SDK's consumers — most of whom
are coding agents that cannot tell a stale doc from a current one.

## The rule that matters

The document has two kinds of content and they are maintained differently:

- **Inside `<!-- BEGIN GENERATED: x -->` / `<!-- END GENERATED: x -->`** — written
  by Gradle. Never hand-edit, never let a model rewrite it. Run the task instead.
- **Everything else** — hand-written prose. Only this needs judgement.

This split exists because the file previously acquired four false statements in
two days when it was authored freely. Keep the split.

Second rule: **gaps are fine, lies are not.** If you are unsure whether something
is still true, write a pointer (`undocumented — see Overlay.kt:NNN`, with the real line) rather than a
guess. You only owe maintenance on what you assert.

## Workflow

**Run every command below from the repository root.** They are written so no step
changes your working directory — an earlier version used `cd kmp`, which left the
shell in `kmp/` and made the Step 2 paths silently resolve to nothing.

```bash
cd "$(git rev-parse --show-toplevel)"
```

### 1. Regenerate
```bash
./kmp/gradlew -p kmp :shared:agentDocs
```
Rewrites every generated region: three in `README_API.md`, the dependency
block in the root `README.md`, plus `FACTS.md`, `shared.api` and `llms.txt`. No env
vars or JDK flags needed — `rust-toolchain.toml` and `jvmToolchain(17)` handle it.
It is idempotent; running twice changes nothing.

### 2. See what actually changed
```bash
git diff kmp/shared/api/shared.api kmp/shared/agent/FACTS.md
bash .claude/skills/update-agent-docs/scripts/scan-known-issues.sh
```
The dump diff is the authority on API change — commit messages are not. A
signature gaining a mangled suffix (`ShashlikShape-Bx497Mc`) means a value-class
parameter changed type; that is a breaking change even though the name is intact.
So does a suffix that merely *changes* while the JVM descriptor stays identical
(`ConvexPolygon-jHzyhOc` → `-Bx497Mc` when `radius` went from `Dp` to `Float`).
For every breaking change, add a one-line "Changed in X.Y.Z" note next to the
parameter with the old and new form. Consumer agents trained on older docs keep
writing the old form otherwise.

Read the scan's **Diff anchor** line first. Until a `mapshared-*` tag exists it
falls back to the merge-base with `main`, so sections 3 and 4 then cover this
branch only. Say so in your report. Do not present that as "since last release".

### 3. Reconcile the prose
Work the generated **inventory** list as a checklist:
- A name in the inventory with no section here → the doc is incomplete, add one.
- A section describing something not in the inventory → **inconclusive, investigate;
  do not delete on this basis alone.** The inventory is not an exhaustive list of
  supported API. It omits everything in `uniffi.ffi_run` (`Point`, `Color`,
  `ShapeType`, `ShashlikMapApi`), which is excluded from `shared.api`. Confirm
  against `ffi-run/src/lib.rs` and the Kotlin sources before removing anything.
- A signature in the prose that disagrees with `shared.api` → the prose is wrong.

Note that Kotlin extension properties appear in `shared.api` as JVM accessors
(`getWidth` for `ShapeType.Line.width`). The inventory lists them under
**Extension properties** under their Kotlin name; never document them as functions.

Then fold the scan output into the **Known limitations** section of
`README_API.md` (subsections: *Broken or disabled*, *Not supported yet*). Only
include things a consumer would hit; internal TODOs stay out. **If that section
does not exist, create it** — a previous run found it missing and nearly dropped
the scan results on the floor.

### 4. Verify before finishing
- Every Kotlin snippet uses names that exist — checked against the inventory *and*
  the `uniffi.ffi_run` types, which the inventory does not list. A name missing from
  the inventory is not by itself proof the snippet is wrong.
- No snippet imports `uniffi.ffi_run.Color` (overlays take Compose `Color`).
- The version stamp matches `version` in `kmp/shared/build.gradle.kts`.
- The root `README.md` usage example still compiles against the inventory. It sits
  *outside* the generated marker, so the task will not fix it for you — the
  `ShashlikMap { _, _ -> }` form was wrong there for months.
- Hand-written version mentions outside the markers (the rustls "X.Y.Z pins" line
  in both READMEs, "Verified against X.Y.Z" in Known limitations) name the current
  version. `grep -n '<previous version>'` over the three docs. Re-check the rustls
  pin itself in `Cargo.lock` before bumping the sentence.
- Scan section 5 shows every `file:line` citation as OK. Fix each **STALE** one
  to the suggested line. For **UNTRACKED**, add an anchor to `tracked-items.tsv`
  (see Step 5) rather than leaving an unchecked citation.
- Re-run Step 1: it must produce no further diff.

### 5. Maintain this skill

This skill is part of the deliverable. Before finishing, fix anything in it that
this run proved wrong. Edit `SKILL.md` and `scripts/` in the same change as the
docs. Do not just mention it in your report.

- **Never write a line number into this file.** Line numbers rot on every edit to
  the source. That happened here: `Overlay.kt:128` went stale while the docs moved
  on to `:130`. Refer to code by symbol or pattern. Anchors the docs cite by line
  go in `scripts/tracked-items.tsv` (`id<TAB>file<TAB>fixed-string pattern`). The
  scan resolves them each run.
- **A tracked anchor reported MISSING**: the issue was probably fixed. Confirm in
  the code, then update the doc entry, the TSV row and the matching bullet under
  *Current known-broken items* together.
- **A new consumer-facing limitation you added to the docs**: add its bullet under
  *Current known-broken items*. If the docs cite it by line, add a TSV row too.
- **A command, path or assumption in this file that failed or misled you**: fix it
  and add a short *why* ("a previous run …"), so a later run does not revert it.
- **Keep the same bar as the docs.** Gaps are fine, lies are not. Only record what
  you verified this run.

## Known blind spot

`uniffi.ffi_run` is excluded from `shared.api`, so changes *inside* `Point`,
`Color`, `ShapeType` or `ShashlikMapApi` do not show up in the dump diff even
though they break consumers. Section 3 of the scan script diffs
`ffi-run/src/lib.rs` as a substitute — read it. This blind spot closes once those
four types are wrapped in hand-written Kotlin.

## Current known-broken items

Verify these each run; do not silently drop them. Scan section 0 reports each
anchor's current line, or MISSING if the pattern is gone.

- **Changing `ShashlikShape` `anchor` recreates the shape.** The `updateShape`
  call in `Overlay.kt` is commented out (anchor `anchor-recreates-shape`), and `DisposableEffect` keys on
  `anchor`, so the shape is destroyed and recreated. Keep it under *Temporary,
  non-blocking* in `README_API.md`, not *Broken or disabled*: the cost is small,
  it is temporary, and consumer agents previously read the stronger wording as a
  blocker and refused to use anchored shapes.
- **`InternalShashlikMapApi` is maintainer-only.** Keep the "never use or suggest"
  rule in `README_API.md` §4, the demo module note, and the `llms.txt` template.
  Never add opt-in instructions or examples that use `ShashlikMapApiHolder`: a consumer
  agent once proposed the opt-in as the recommended approach while planning.
- **No iOS artifact.** iOS targets are commented out in `kmp/shared/build.gradle.kts`
  (anchor `ios-targets-disabled`).
  The published library is Android-only, `arm64-v8a` only.
- **Location permission revocation is not handled.** `SimpleLocationManager.start()`
  is `@SuppressLint("MissingPermission")` with a FIXME above it (anchor
  `permission-revocation`). Keep under *Broken or disabled*.
- **`LineShape` fails silently** with fewer than two distinct points: the
  `if (!isValid)` early return in `Overlay.kt` (anchor `lineshape-silent-fail`).
  Keep under *Not supported yet*.

Every row in `tracked-items.tsv` should have a bullet here. A previous run found
two anchors tracked in the TSV with no bullet, so nothing told a later run to
keep their doc entries.
