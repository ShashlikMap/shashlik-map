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
is still true, write a pointer (`undocumented — see Overlay.kt:97`) rather than a
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

## Known blind spot

`uniffi.ffi_run` is excluded from `shared.api`, so changes *inside* `Point`,
`Color`, `ShapeType` or `ShashlikMapApi` do not show up in the dump diff even
though they break consumers. Section 3 of the scan script diffs
`ffi-run/src/lib.rs` as a substitute — read it. This blind spot closes once those
four types are wrapped in hand-written Kotlin.

## Current known-broken items

Verify these each run; do not silently drop them.

- **Changing `ShashlikShape` `anchor` recreates the shape.** The `updateShape`
  call is commented out (`Overlay.kt:128`), and `DisposableEffect` keys on
  `anchor`, so the shape is destroyed and recreated. Keep it under *Temporary,
  non-blocking* in `README_API.md`, not *Broken or disabled*: the cost is small,
  it is temporary, and consumer agents previously read the stronger wording as a
  blocker and refused to use anchored shapes.
- **No iOS artifact.** iOS targets are commented out (`shared/build.gradle.kts:55`).
  The published library is Android-only, `arm64-v8a` only.
