# Nexa AOT ↔ DevRuntime Parity — Source Audit and Implementable Remediation Plan

**Audit date:** 2026-10-31<br>
**Scope:** `.nx` compiler and typed IR, SwiftUI and Compose AOT backends, both DevRuntime implementations, **complete Navigation v2 redesign**, patch protocol, reload orchestration, test harnesses and CI<br>
**Intended audience:** Nexa maintainers / compiler, iOS and Android engineers<br>
**Status:** Source audit completed, including source-level navigation audit; fixes and full Navigation v2 rewrite proposed but **not implemented** in this report.

> **Evidence distinction.** `VERIFIED` means a code-path property directly visible in this archive, not necessarily an observed device-level symptom. `REPRODUCTION REQUIRED` means a plausible failure mechanism that must be confirmed with a focused runtime test. `ARCHITECTURAL RISK` means a structural weakness, not a confirmed current defect. Do not label a component broken just because its implementation is complex or duplicated.

DO NOT KEEP LEGACY CODE, NEXA IS STILL ALPHA SO REMOVE ALL LEGACY CODE

---

**Navigation expansion (2026-10-31):** Section 9 adds a source-grounded **full Navigation v2 rewrite** with 10 findings (NAV-01–10), nine dependency-ordered navigation PRs (N1–N9), 16 navigation regression scenarios, API migration rules, native performance constraints and release gates. This replaces the previous generic suggestion to merely "extend navigation property contracts".

---

## 0. Outcome, non-negotiable constraints, and exact meaning of parity

**The goal:** For identical `.nx` source, target platform, input/environment and observable event history, AOT and DevRuntime must have equivalent externally observable semantics and native UI output. This applies to a cold start and to ongoing interactions. Hot reload additionally has a **specified state-migration contract**; preserving state across a source edit is *not* equivalent to cold-starting a new binary with different initializers.

**Maintain these existing Nexa constraints (see `AGENTS.md` §§1–3):**

1. Release apps remain statically generated SwiftUI or Jetpack Compose. **No interpreter/JSON renderer in release apps.**
2. **No `AnyView` added to generated hot paths**, no generic view boxing added to AOT, and no reflected/untyped parameter maps added to generated output.
3. Preserve Kotlin primitive state specialization and native list virtualization.
4. Keep incremental compiler caching, change hints and Dev IR patches. Improvements must not introduce full-project recompilation or full JSON re-serialization on every edit.
5. Favor the **existing shared native primitives** on each platform. The code already uses `NexaButtonPrimitive`, `NexaColumnPrimitive`, `NexaRowPrimitive`, `NexaStackPrimitive`, `NexaFastList`, etc. Do **not** rebuild these from scratch.
6. Test parity within the **same** OS/device/configuration first. Separate *AOT-vs-DevRuntime* parity from *iOS-vs-Android* design equivalence. Native OS differences between iOS and Android are not, by themselves, AOT/DevRuntime bugs.

**Formal acceptance contract:**

`Observe(ReleaseLowering(Compile(source, target)), environment, events)`<br>
`== Observe(DevLowering(Compile(source, target)), environment, events)`

Where observation includes specified render geometry, accessibility semantics, text/controls, state changes, navigation, effects, persisted values, native API results and errors. Performance and transient debugging overlays are measured separately and should not be forced equal. For images, use exact matches where feasible and documented tight tolerances for OS pixel-rasterization noise. The two programs need **not** have identical generated instructions, allocation graphs or frame timing.

**AOT performance is an independent release gate:** no feature is allowed to “fix parity” by routing production through the DevRuntime.

---

## 1. What was actually audited, and what could be executed

### 1.1 Source subsystems inspected

| Subsystem | Inspected source | Existing basis |
|---|---|---|
| Project compilation | `crates/nexa-compiler/src/project.rs`, `semantic/mod.rs`, `semantic/generics.rs`, `semantic/custom_components.rs` | Shared typed module, mode-sensitive lowering and incremental dev cache |
| Compiler optimizer | `crates/nexa-compiler/src/optimize.rs`, compiler parity test files | Optimizes both modes; supports cached function optimization and reports |
| Typed IR and Dev IR | `crates/nexa-ir`, `crates/nexa-dev-ir/src/lib.rs` | Native backend consumes IR; Dev IR wraps IR with revisions, JSON patching and identity metadata |
| Protocol / server | `crates/nexa-dev-protocol/src/lib.rs`, `crates/nexa-dev-server/src/lib.rs` | Versioned JSON WebSocket, full modules, patches, acknowledgements, full-module recovery |
| Native dev host generation | `crates/nexa-cli/src/commands.rs`, `crates/nexa-cli/src/project/` | Watch/coalescing, native rebuild classification and background-task rebuild |
| iOS AOT and development | `crates/nexa-backend-swift/src/generator/`, `runtime/ios/NexaDev*.swift` | Existing shared primitives, SwiftUI dynamic renderer and state evaluator |
| Android AOT and development | `crates/nexa-backend-kotlin/src/generator/`, `runtime/android/NexaDev*.kt` | Existing shared primitives, Compose dynamic renderer and state evaluator |
| Parity/CI | `tests/hot-reload-aot-parity.sh`, `tests/compare-render-parity.py`, `tests/test_render_parity_comparison.py`, compiler parity gates, `.github/workflows/ci.yml`, `PARITY.md` | Genuine iOS/Android simulator/emulator parity jobs already exist |

The audit was **not** a measurement of a user's specific mismatching screen: none was provided. A repository-wide static audit can identify causes and implementation weaknesses, but it cannot assign a device-level visual bug to a specific component without matching traces/captures.

### 1.2 Checks actually run

- **PASS:** `python3 -m unittest discover -s tests -p 'test_render_parity_comparison.py' -v` — **10/10 tests passed**. This verifies comparator helper behavior, **not** framework/device parity.
- **PASS:** `python3 tests/render_component_parity_report.py --check` — report generator's consistency check passed.
- **INSPECTED ONLY:** `.github/workflows/ci.yml` has Rust tests and real iOS/Android compile, hot-reload and screenshot parity jobs. Their configured presence does **not** mean these jobs were run in this audit.
- **NOT AVAILABLE HERE:** `cargo`, `rustc`, `xcodebuild`, `adb`, Android emulator and Xcode simulator. Thus Rust compile/tests and actual app/device comparisons were **not** run; do not interpret this report as a production validation certificate.
- The provided ZIP is an archive, not a Git checkout; commit SHA and build provenance were not available for this audit.

### 1.3 Existing strengths that should be retained

- A typed `nexa_ir::Module` already feeds both AOT backends; Dev IR wraps that module (`nexa-dev-ir/src/lib.rs:29-42, 347-438`).
- Optimization already has conservative passes and incremental caching (`semantic/mod.rs:2082-2104`, `optimize.rs:133-140`).
- Native shared primitives are **already in use**: Swift AOT `generator/components/controls.rs:68`, Swift Dev `NexaDevRenderer.swift:2347`; Kotlin AOT `generator/components/controls.rs:403`, Kotlin Dev `NexaDevRenderer.kt:1503`; layouts likewise use shared Row/Column/Stack primitives.
- Both runtime protocols support base revision checks, full-module fallback and acknowledgement (`runtime/ios/NexaDevProtocol.swift:137-182`, `runtime/android/NexaDevProtocol.kt:164-211`).
- Android supports hot-reload state preservation and scoped install paths, and iOS has corresponding logic (`NexaDevState.kt:455-683`, `NexaDevState.swift:407-650`).
- **Both platform CI jobs already run `tests/hot-reload-aot-parity.sh`** (`.github/workflows/ci.yml`, iOS/Android compile jobs). Avoid the mistaken recommendation to “add the first parity test”. Improve coverage, reporting and reliability of the existing tests instead.
- `PARITY.md` reports 41/41 agent-reviewed routes. It explicitly separates those reviews from human approval and exhaustive parity; it is not proof of zero mismatch.

---

## 2. Audit findings, prioritized by directness and impact

### F-01 — Android incremental patches mutate Compose state off the main thread

**Priority:** P0 | **Evidence:** VERIFIED threading inconsistency | **Device symptom:** REPRODUCTION REQUIRED

**Exact code:**

- `runtime/android/NexaDevProtocol.kt:35-52, 140-164`: connection frames are received on a worker thread named `nexa-dev-websocket`; `handle(...)` is called from this loop.
- `runtime/android/NexaDevProtocol.kt:170-176`: a **full module** is installed inside `onMainAndWait { store.installTranslations(...); store.install(module) }`.
- `runtime/android/NexaDevProtocol.kt:179-205`: a **patch** calls `store.applyPatch(patch)`, `store.installTranslations(...)` and `store.publishDiagnostics(...)` **without** `onMainAndWait`.
- `runtime/android/NexaDevState.kt:139-160`: the store owns Compose `mutableStateMapOf` and `mutableStateOf`, and patch installation changes the module and related state.

**Why this matters:** The full and patch paths have different thread-affinity contracts. This can cause unsafe timing/visibility, concurrent reads vs writes or recomposition differences. Compose snapshot state can support certain multithreaded uses, but there is no evident controlled snapshot transaction here; do not claim an observed crash before testing.

**Immediate fix:** Move patch **state publication and installation** to the same main-thread transaction as full installs. Pre-parse/validate/calculate patch operations off-thread, then commit on main; acknowledge only after successful commit. As a low-risk emergency correction, put `store.applyPatch` and the **single** translation/diagnostic update into `onMainAndWait` before optimizing the patch-preparation step.

**Regression cases:** 100 rapid styling-only patches, handler edit during tap, patch while list scrolls, patch during Compose recomposition, patch during navigation, patch while background/foreground changes, and patch that changes a state initializer. Add asserts that native state commits are main-owned and never partially visible.

**Acceptance:** No off-main mutation of observable Compose app state by the WebSocket handler; no patch-vs-full disagreement in event traces; median/p95 patch latency recorded to ensure the correction doesn't degrade hot reload unnecessarily.

### F-02 — Android patch validation does not provide truly atomic rollback

**Priority:** P0 | **Evidence:** VERIFIED mutation strategy | **Corruption potential:** REPRODUCTION REQUIRED

**Exact code:** `runtime/android/NexaDevState.kt:634-684`:

```kotlin
val root = currentModule ?: module ?: return false
// validate each operation against root
for (...) if (!validateOperation(root, operation)) return false
// then mutate that same root as operations are applied
for (...) if (!applyOperation(root, operation)) return false
```

`root` is the currently held mutable `JSONObject`, and `applyOperation` changes it in place. The later `shallowCopy(root)` (`:779-791`) comes **after** mutation; it is not an isolation boundary. Individually valid operations can also invalidate subsequent operations (e.g. remove a parent then set a nested child). Existing server-generated diffs normally avoid contradictory overlapping edits, but error/recovery paths must not rely on that assumption.

**Fix design:**

1. Define a `PreparedPatch` object with `baseRevision`, `newRevision`, validated operations and a **candidate** updated module, leaving the last-good module untouched.
2. Reject duplicate/conflicting path prefixes and invalid `op`, path escaping or array indices; validate candidate shape and supported IR version.
3. Use structural sharing / path-copying for changed JSON branches so a single styling patch does **not** deep-copy the whole module or JSON-serialize it anew.
4. Commit candidate + state migration + presentation revision on main in one logical operation.
5. Keep previous module, state and revision unchanged if preparation or install fails; request full module.
6. Add deterministic failure-injection tests at every stage of patch preparation, application, migration and commit.

**Acceptance:** For *every* rejected patch, a deep snapshot of the previous module, state registry, function registry, translation state and revision remains unchanged. No ack for rejected revision.

### F-03 — The compiler has a real dev-vs-AOT **language acceptance difference**

**Priority:** P0 | **Evidence:** VERIFIED source behavior

**Exact code:**

- `crates/nexa-compiler/src/project.rs:382-431`: normal compile calls `lower_with_project_targets(..., dev_runtime=false)`; dev calls the cached dev lowering path.
- `crates/nexa-compiler/src/semantic/mod.rs:490-515`: the mode Boolean is named `allow_nullable_generic_plugin_reads` and is carried into semantic contexts.
- `crates/nexa-compiler/src/semantic/generics.rs:386-425`: generic read storage validation grants top-level `Optional` allowance based on `registries.allow_nullable_generic_plugin_reads`.
- `crates/nexa-compiler/tests/plugin_generics.rs:625-637`: normal mode explicitly tests that `store.getObject<String?>("maybe")` is **rejected** because the top-level optional cannot distinguish `null` from decode failure.

The DevRuntime allows the top-level optional because it has a tagged dynamic result representation, while AOT's static decoder contract does not. That means certain `.nx` source can be **accepted during development but rejected in production**. This is not just a rendering difference; it violates a strict “dev green => AOT builds” guarantee.

**Decision required:** Either (A) implement an equally unambiguous tagged native read result in generated Swift/Kotlin and allow the syntax in both modes, or (B) reject it in both modes with the same diagnostic. **Recommendation:** first enforce B to restore truthfulness; then add A as a separately tested language feature if wanted. Do **not** silently coerce decode errors to `null` in AOT.

**Implementation:** replace the semantic meaning of the mode flag with a capability check that is **identical for AOT and DevRuntime for any supported source**. Keep debug-specific cache paths, but prevent them from changing source acceptance. Add a target×mode test that compiles the same project through the actual `IncrementalProjectCompiler` APIs and compares `Ok/Err`, normalized diagnostics and canonical IR.

**Acceptance:** No `.nx` program is accepted by `nexa dev` for a target while failing `nexa build` for the same target due to a mode-only language rule (excluding explicit native-host-rebuild requirements, which must be reported).

### F-04 — Android patch translations and diagnostics are applied twice

**Priority:** P1 | **Evidence:** VERIFIED

**Exact code:** `runtime/android/NexaDevProtocol.kt:189-203` installs translations and publishes empty diagnostics inside the successful `if (store.applyPatch(...))`, then repeats both after the success check.

**Fix:** Move the translation and diagnostics updates to one main-thread commit routine and delete the duplicate branch. Add a one-update-per-revision instrumentation assertion; test translation-only and mixed module+translation patches.

**Acceptance:** Exactly one presentation/translation notification per successful committed revision; zero notifications on failed candidate patches.

### F-05 — Static “coverage gates” can pass without proving behavior or even correct dispatch

**Priority:** P1 | **Evidence:** VERIFIED

**Exact code:**

- `crates/nexa-compiler/tests/component_primitive_parity_gate.rs`: searches source text for each primitive name. A comment or unreachable code can satisfy the check.
- `crates/nexa-compiler/tests/action_parity_gate.rs` and `ir_expr_parity_gate.rs`: string occurrence rather than executing actions/expressions with value or side-effect assertions.
- `crates/nexa-compiler/tests/ir_node_parity_gate.rs:101-107`: `ios_has` includes `|| ios_dev.contains("devStatusBar")` **without guarding it for the `StatusBar` variant**. Once that string exists, the test can mark **every** iOS node as supported even if its dispatch is missing. This is a specific false-positive condition, not a hypothetical weakness.

**Fix:**

1. Immediately scope the `devStatusBar` condition to `node == "StatusBar"`. Use a regression fixture that deliberately removes a dispatch in a temporary input and expects a failing gate.
2. Replace hand-maintained string lists with variant inventories from `nexa_ir` or a generated manifest. A coverage test must fail when a new `Node`/`Action`/`Expr` variant is added without a target implementation.
3. Preserve lightweight static tests as **coverage smoke tests only**; add real compiled/conformance evaluation and device assertion for behavior. “Name present” must not be reported as “identical behavior”.

**Acceptance:** Mutation testing can remove or alter a target dispatch and the gate reliably fails; value-level tests catch a behavior change without changing any symbol names.

### F-06 — Identity metadata exists but is not visibly consumed by native renderers

**Priority:** P1 | **Evidence:** VERIFIED code search; **observed state bug:** REPRODUCTION REQUIRED

**Exact code:** `crates/nexa-dev-ir/src/lib.rs:377-423, 561-583` emits stable `identities`, deriving node IDs from **kind-based sibling ordinals**. In inspected native runtime files, `identities` occurs in the schema keys but there is no visible read/lookup of that metadata in `NexaDevRenderer`, `NexaDevState`, or patch application. iOS rendering uses structural child indexes, e.g. `runtime/ios/NexaDevRenderer.swift:1764-1777`; Android uses index-keyed composition in several paths (`runtime/android/NexaDevRenderer.kt:590, 749, 792, 3895, 3911`) and has keyed FastList rows in other paths.

**Risk:** Kind-ordinal identity shifts when a same-kind sibling is inserted ahead of another. Runtime structural indexes and Dev IR-generated IDs are also **different schemes**. Component state can appear correct in existing simple tests while complex reorder/insert/move cases lose or migrate state incorrectly.

**Fix:** Inventory where identity is actually used. Establish a single logical identity contract: explicit developer keys first; otherwise documented structural identity, with stable IDs for named component instances and route entries. Carry the resolved identity to every DevRuntime render path **where it affects state ownership**. Audit AOT `ForEach`/Compose `key` paths against the same list and component identity policy. Do not promise state preservation for an unkeyed reorder that is fundamentally ambiguous: document and warn in the compiler.

**Acceptance:** Tests for insert-before, delete-before, reorder, repeated component calls, nested route instances and keyed/unkeyed lists give specified state retention and lifecycle behavior. If `DevModule.identities` is not needed, remove or repurpose it explicitly rather than presenting unused metadata as a guarantee.

### F-07 — Patch acknowledgement means installation, not guaranteed first rendered frame

**Priority:** P1 | **Evidence:** VERIFIED timing; **symptom:** REPRODUCTION REQUIRED

- iOS `runtime/ios/NexaDevProtocol.swift:169-182` acknowledges after the store/module mutation, not after a SwiftUI frame has been rendered.
- Android `runtime/android/NexaDevProtocol.kt:189-211` acknowledges after applying the patch, not after a Compose render frame.
- Server `crates/nexa-dev-server/src/lib.rs:615-630` logs the acknowledgment as an applied patch.

**Fix:** Keep current `Acknowledge` as a fast **module-commit** signal for latency and transport compatibility. Add a test-only `FramePresented(revision, frameId)` (or equivalent observable render-fence signal) behind a diagnostic capability. Screenshot/UI tests wait for **frame presentation + deterministic stabilization**, not merely a source patch ack or a fixed sleep. Do not hold up ordinary production or debug feedback on test capture synchronization.

**Acceptance:** Parity test never captures an old frame after a new revision was merely acknowledged.

### F-08 — Existing visual comparator contains intentional blind spots

**Priority:** P1 | **Evidence:** VERIFIED, behavior depends on scenario

`tests/compare-render-parity.py:309-342` masks the top/bottom **7%** of every screenshot, increasing the band to **12% or 17%** for aligned scroll captures. It allows small inferred scroll offsets for designated phases (`uniform_scroll_offset`, lines ~75-225). Overlays use static fraction bounds and masks (`OVERLAY_BOUNDS`, `OVERLAY_INTERIOR_BOUNDS`, lines ~36-52 and ~348-411). Thresholds are `MAE <= 1.0` and changed-pixel percentage `<= 0.5` in tested regions. These methods deliberately reduce noisy false positives; they can also hide regressions in an excluded area if relied on alone.

**Fix:** Retain the current comparator as one *visual* signal, but introduce per-scenario annotated masks and **full-frame reference captures** for app-owned header, bottom navigation and overlays. Require accessibility geometry and hit-target checks for masked elements. Treat scroll restoration as its own test where offsets are **not** normalized. Derive overlay bounds from actual native/accessibility geometry when possible, not hard-coded fractions. Save unmasked diff images and a machine-readable list of excluded pixels/reasons.

**Acceptance:** Deliberately move a toolbar/bottom bar in either runtime and a parity test fails even though masked screenshot regions remain similar.

### F-09 — Existing native-host rebuild logic can leave the running app deliberately stale

**Priority:** P1 | **Evidence:** VERIFIED intended workflow, not automatically a bug

`crates/nexa-cli/src/commands.rs:1223-1247` detects changes requiring native rebuild and says **press `b`** rather than treating them as ordinary hot reload. `:2582-2589` classifies config, lockfile, assets and plugin-root changes; background task changes cause an actual rebuild (`:2326-2361`). This is sensible: runtime IR cannot add compiled permissions/plugins/assets to an existing host.

**Parity risk:** A developer may compare the current source to an older native host and see a “mismatch” despite no actual source-level semantic difference.

**Fix:** Make rebuild-required state unavoidable in diagnostics and test runner metadata. Record a native host capability/build fingerprint (plugin contracts, permissions, assets, resource manifests, target minima, background tasks, and native template version). Reject or visibly suspend parity claims when source capability requirements and host fingerprint differ. Optional quality-of-life: enable an explicit auto-rebuild policy, but never quietly continue as if the host can interpret missing native code.

**Acceptance:** No “parity passed” report can be generated against a stale native host; a required rebuild is distinguished from a hot-reload semantic failure.

### F-10 — Four independently maintained IR-to-native adapter paths remain across iOS and Android

**Priority:** P2 architectural | **Evidence:** VERIFIED source structure

The shared native primitives reduce drift, but the **adapter code** translating a `Node` into primitive parameters is still separately implemented in:

- Swift backend: `crates/nexa-backend-swift/src/generator/components/`;
- iOS Dev: `runtime/ios/NexaDevRenderer.swift` (approximately 4,060 lines);
- Kotlin backend: `crates/nexa-backend-kotlin/src/generator/components/`;
- Android Dev: `runtime/android/NexaDevRenderer.kt` (approximately 3,920 lines).

For example, iOS Dev layout style/primitive dispatch is at `NexaDevRenderer.swift:1683-1788`, and Android Dev layout decoding/dispatch at `NexaDevRenderer.kt:1032-1106`. Both use shared native primitives, but normalize style fields, arrangements and constraints in separate code.

**Fix:** Centralize **contract rules and tests**, not all render execution. Introduce a declarative/typed registry of IR-to-native property semantics and defaults. Generate small adapter tables/functions where practical; use golden parameter-plan tests where SwiftUI/Compose syntax must remain specialized. Keep per-platform native primitives and direct AOT emission. Start with one simple and one complex component rather than an expensive rewrite.

**Acceptance:** New or changed IR property requires a single canonical contract entry and compulsory four-path conformance proof, without forcing `AnyView`/dynamic maps into generated release code.

### F-11 — Incremental dev caching is an additional equivalence surface

**Priority:** P1 | **Evidence:** VERIFIED separate path; **incorrect cache result:** NOT DEMONSTRATED

`project.rs:400-427` uses `lower_with_project_targets_and_dev_cache`. `semantic/mod.rs:2082-2104` conditionally runs `optimize_with_cached_function_names`, while the normal path runs `optimize`. `nexa-dev-ir/src/lib.rs:464-533` skips folding unchanged function/screen offsets based on change hints. This is performance-sensitive and intentional; *cache invalidation correctness* must still be tested.

**Fix:** Add deterministic **fresh-compiler vs incremental-compiler** equivalence tests for a scripted sequence of source edits and dependencies (globals, shared components, signature changes, imports, plugin analyzer outcomes and optimization facts). Compare fully normalized IR and emitted Dev IR per revision. Keep the cache, instrument invalidation reasons, and only expand dependencies that tests prove need invalidation.

**Acceptance:** Every revision's cached Dev IR is semantically equivalent to a clean Dev IR build of that exact revision; incremental patch resulting state also equals a clean full-module install when no state preservation is requested.

### F-12 — “Unsupported” dynamic native operations are logged rather than always surfaced as parity failures

**Priority:** P2 | **Evidence:** VERIFIED fallback behavior; reachable scenarios require tests

`runtime/ios/NexaDevActions.swift:246,260,790,804` and `runtime/android/NexaDevActions.kt:166,182,951,967` include “native property write is unsupported” / “native event subscription is unsupported” logging paths. `runtime/ios/NexaDevRenderer.swift:2307` and `runtime/android/NexaDevRenderer.kt:916` log unsupported native components.

These may be intentional for unimplemented plugin calls, but silent log-only behavior is unsafe for a strict parity guarantee.

**Fix:** Introduce structured `UnsupportedNativeCapability` diagnostics that contain target, component/property, contract version and source span. Fail DevRuntime conformance tests on any such event. At ordinary dev-time, display a visible error or rebuild-required instruction rather than rendering a misleading fallback.

**Acceptance:** If AOT supports an operation but DevRuntime cannot execute it, the user gets an actionable failure—not a superficially successful hot reload.

---

## 3. Root-cause map: fix where divergence begins

| Symptom seen by developer | First checks | Likely responsible paths | Preferred class of fix |
|---|---|---|---|
| Dev accepts source, build fails | Compiler mode flag, diagnostics | `project.rs`, `semantic/mod.rs`, `semantic/generics.rs` | Common semantic acceptance policy (F-03) |
| Visual after reload differs from clean Dev launch | Partial patch, stale host, identity | `NexaDevState.*`, `NexaDevProtocol.*`, CLI rebuild | Atomic commit, host version and state contract (F-01/02/06/09) |
| AOT cold start vs Dev cold start geometry | IR value, adapter params, native primitive, environment | AOT `generator/components/*`, both DevRenderers | Adapter contract + geometry traces (F-10) |
| Values/callbacks differ but geometry matches | Expr/action evaluator, native plugin codec | `NexaDevState.*`, `NexaDevActions.*`, backends | Value-level conformance and typed errors (F-03/12) |
| First frame differs then settles | Render fence and animation state | Protocol/renderer/test harness | Presented-frame synchronization (F-07) |
| Screenshots pass, bug visible in header/bar | Masks and coverage | `compare-render-parity.py` | Geometry/hit-target assertions and unmasked diffs (F-08) |
| Bug only after inserts/reorders | Structural vs keyed identity | `nexa-dev-ir`, `NexaDevRenderer.*`, list implementations | Explicit identity contract (F-06) |
| Bug only with many hot edits | Incremental cache or patch revision | Compiler cached path, server, client patch routines | Fresh-vs-cached tests and transactional patches (F-02/11) |

**Use this hierarchy for triage:**

```
.nx source + target + manifest
  -> parse / semantic diagnostics
  -> canonical typed IR
  -> optimized / cached IR
  -> native adapter parameters
  -> native primitive + platform environment
  -> native state/effects/navigation
  -> frame, accessibility, pixel output
```

At the **first** mismatching stage, write a regression and fix that stage. Avoid hard-coded compensating offsets in the renderer.

---

## 4. Detailed change plan and proposed APIs

### W0 — Make the audit reproducible, before refactoring (P0)

**Files to change/add:**

- `tests/parity/README.md` (**new**): describes device/OS/toolchain capture protocol and the distinction between AOT and development host.
- `tests/parity/cases.yaml` or typed Rust equivalent (**new**): fixture inventory with supported target/mode/revision, input events, expected observable contract and masks.
- `tests/parity/manifest.py` (**new**): writes a machine-readable environment/capture manifest.
- `tests/hot-reload-aot-parity.sh` (**modify**): produce artifacts for both targets with source hashes, compiler versions, module revisions, native-host fingerprints, OS/device, accessibility schema, screenshot dimensions and timing.
- `tests/parity/minimize.py` (**later new**): minimize action sequences and source fixtures for failing cases.

**Capture manifest example (test-only):**

```json
{
  "fixture": "screen-navigation-001",
  "platform": "android",
  "mode": "dev_cold",
  "source_sha256": "<source hash>",
  "compiler_build": "<build identifier>",
  "host_capability_hash": "<hash>",
  "module_revision": "<revision>",
  "device": "<device model>",
  "os_version": "<OS version>",
  "ui_locale": "en-US",
  "font_scale": 1.0,
  "density": 1.0,
  "appearance": "light",
  "event_sequence": ["launch", "tap:open", "back"],
  "capture_phase": "presented_stable_frame"
}
```

**First targeted reproduction:** Run the existing parity app unchanged, then run an actual user-reported mismatch in four modes: iOS AOT/iOS Dev, Android AOT/Android Dev. Record at least two frames at settled checkpoints. The matrix is empty until this run happens; **do not invent measurements**.

**Done:** Every failure gives an artifact bundle and one reproducible command; baseline performance from the existing benchmark suite is saved separately.

### W1 — Remove dev-only semantic permissiveness (P0; F-03)

**Edit** `crates/nexa-compiler/src/semantic/mod.rs`, `generics.rs`, `context.rs`, `custom_components.rs`, `crates/nexa-compiler/src/project.rs`:

1. Rename the Boolean from `allow_nullable_generic_plugin_reads` to a **feature/capability decision**, and stop setting it implicitly from “dev_runtime”. Prefer an explicit `SemanticCapabilities` derived from the target's actual native compiler support.
2. For immediate parity, force the unsupported top-level optional generic read to error identically in both modes.
3. Preserve the separate incremental cache use; do **not** remove all dev lowering optimizations just because this one flag changes semantics.
4. If implementing optional generic reads later, introduce a Swift/Kotlin typed decoder result that distinguishes `value`, `null` and `decode_error`; add IR/value-codec tests and then allow the syntax in both modes.
5. Add `crates/nexa-compiler/tests/dev_aot_semantic_equivalence.rs` (**new**). Test identical accepted/rejected result and diagnostics using actual normal and cached dev project entry points for iOS and Android, including optional plugin generics and other target capabilities.

**Suggested test matrix:** simple optional plugin read, generic result, enum error type, compound nullable values, async callbacks, native component refs and external plugin analyzers.

**Do not:** Relax release compiler validation to silence a mismatch if generated native code cannot correctly represent the result.

### W2 — Correct Android patch thread ownership and transactionality (P0; F-01/02/04)

**Edit:** `runtime/android/NexaDevProtocol.kt`, `runtime/android/NexaDevState.kt`; if adding a reusable patch representation, `runtime/android/NexaDevPatch.kt` (**new**).

**Stage A, immediate safe patch:**

```kotlin
// Outline only — integrate with existing socket/error handling.
val committed = AtomicBoolean(false)
onMainAndWait {
    if (store.applyPatch(patch)) {
        if (patch.has("translations")) {
            store.installTranslations(patch.optJSONObject("translations"))
        }
        store.publishDiagnostics(emptyList())
        committed.set(true)
    }
}
if (!committed.get()) {
    requestFullModule(output)
    return
}
currentRevision = revision
acknowledge(output, revision)
```

Remove the duplicate translation/diagnostics updates. This eliminates the inconsistent threading while leaving the existing patch algorithm intact; it does **not** finish atomicity work.

**Stage B, correct and efficient transaction:**

- Introduce `preparePatch(baseSnapshot, patch): Result<PreparedPatch, PatchError>` that is side-effect free and can run on the WebSocket worker.
- Implement JSON Pointer parsing once, reject invalid escape sequences, duplicate/conflicting paths and stale base revision.
- Apply operations to a copy-on-write candidate tree, not the current `JSONObject`; build a complete state migration plan without publishing state.
- Commit on `Dispatchers.Main.immediate` / UI-owned thread in one short operation, with revision CAS/check. On failure, retain old snapshot and request full-module recovery.
- Update translation, diagnostics, functions/screens, state signatures and module pointer together. Publish a single revision invalidation.
- Ack only that committed revision. If the connection drops mid-commit, reconnect and request a full module matching the new session.

**Why not just `JSONObject(root.toString())`?** It is a valid temporary correctness fallback but performs O(total module size) work for tiny patches. Build an immutable/path-copy candidate representation for sustained low-latency hot reload.

**Tests to add:**

- `tests/hot-reload-patch-atomicity-android.sh` (**new**, on emulator): contradictory operations, invalid nested path, stale base revision, type-change migration, rapid mixed patches, disconnect/reconnect, patch while scrolling/typing.
- Kotlin unit tests for patch-preparation and JSON Pointer edge cases.
- A test-only commit observer asserting exactly one revision publication and translation change per successful commit.
- Main-thread assertion in the state commit code under development/test builds.

**Done:** Every rejected patch leaves a byte-equivalent previous logical module and equivalent state, and rapid patches do not corrupt Compose state or lose updates.

### W3 — Establish reliable revision/frame barriers (P1; F-07)

**Edit:** `crates/nexa-dev-protocol/src/lib.rs`, `runtime/ios/NexaDevProtocol.swift`, `runtime/android/NexaDevProtocol.kt`, `runtime/ios/NexaDevRuntime.swift`, `runtime/android/NexaDevRuntime.kt`, test harness.

- Add distinct `module_committed` and **test-only** `frame_presented` concepts. Acknowledgement remains fast and compatible; do not redefine a transport ack as a UI render proof.
- Add `frame_presented` only when diagnostic mode is active; emit after the native rendering framework has produced a frame using the new revision, with revision and monotonically increasing frame ID.
- iOS: use SwiftUI view update coordination plus a native display/layout fence appropriate to the test host. Verify the event truly corresponds to the updated revision, not merely a scheduled run-loop callback.
- Android: use Compose composition/frame scheduling (`withFrameNanos` or equivalent render synchronization) and verify the revision used by that composition.
- The screenshot harness waits on the frame fence and any explicitly required animation settling. Fixed delays remain a timeout fallback, not a correctness guarantee.
- Version the test extension; **no extra production AOT payload**.

**Done:** False parity failures from screenshots captured before actual presentation are eliminated without suppressing real frame differences.

### W4 — Turn compile/cache parity into a formal invariant (P0/P1; F-11)

**Edit:** `crates/nexa-compiler/src/project.rs`, `semantic/mod.rs`, `optimize.rs`, `crates/nexa-dev-ir/src/lib.rs`; add tests.

- Extract a test-visible **semantic projection** from the common `Module`: component properties/defaults, statements, effects, state identity, error/codec requirements, target conditions, and native plugin call signatures. Exclude source byte offsets, debug-only trace metadata and revision sequence tokens.
- Compare normal fresh lowering to fresh dev lowering for the same target. If dev transport needs extra metadata, compare the semantic projection before and after adding it.
- Compare fresh dev lowering to cached dev lowering after every edit in an edit-history generator. Verify the cached `change_hint` matches the actual unchanged sections.
- Add mode equivalence tests for optimizer passes and source forms that previously caused regressions. Introduce optimizer pass toggles **only in testing/diagnostic tooling**, not in normal compiler output.
- Preserve the existing optimization passes and cache but explicitly assert `Observe(IR) == Observe(optimized IR)` under supported source semantics; use golden outputs and property-based expression checks.
- On mismatch, print the earliest IR JSON Pointer and compiler cache hit/invalidation reasoning.

**Proposed test files:**

`crates/nexa-compiler/tests/dev_aot_semantic_equivalence.rs`<br>
`crates/nexa-compiler/tests/incremental_clean_equivalence.rs`<br>
`crates/nexa-compiler/tests/optimizer_observation_equivalence.rs`<br>
`crates/nexa-dev-ir/tests/patch_vs_full_module.rs`

**Done:** No mode acceptance difference, no cache-dependent semantic difference, no patch-vs-full semantic difference for equivalent cold-start input.

### W5 — Define exactly one identity/state/effect contract (P1; F-06)

**Edit:** `crates/nexa-dev-ir/src/lib.rs`, `crates/nexa-ir/src/*` only if needed, `runtime/ios/NexaDevState.swift`, `runtime/android/NexaDevState.kt`, `runtime/ios/NexaDevRenderer.swift`, `runtime/android/NexaDevRenderer.kt`, list/navigation emitters in both AOT backends.

**Normative rules to write first:**

1. **App state:** identity by app instance + declared state name and type signature.
2. **Screen state:** identity by navigation entry instance + screen declaration + state declaration; repeated pushes of the same route must not accidentally share mutable screen state unless the language says otherwise.
3. **Component local state:** identity by component call-instance + state declaration, with explicit stable key support for lists/moves if provided.
4. **List rows:** stable item key is authoritative; index identity is allowed only with documented consequences and compiler warning when stateful descendants are reorderable.
5. **Effects/tasks:** owners have stable IDs; when removed or incompatibly replaced, cleanup/cancel happens exactly once; when handler body is replaced, no stale closure remains.
6. **Hot reload:** preserve compatible state; reject or reset incompatible type changes; do not confuse this with the AOT cold-start contract.

**Implementation:** Determine whether to make `DevModule.identities` normative and consumed or to remove it. If retained, encode the *resolved* keys in a form that native renderers consume, rather than maintaining kind ordinals alongside unrelated runtime indices. Add a debug identity tree for comparison with AOT (generated instrumentation only).

**Test sequences:**

- Insert `Text` before a stateful same-kind sibling; insert `Button` before another button; remove or reorder siblings.
- Push two instances of one screen with different route data; pop and push; restore after reload.
- Reorder keyed FastList and unkeyed FastList; compare documented preservation rules.
- Rename/delete/retype state; change closure body; remove an active task; invalidate a native subscription.
- Initializer changes should **preserve** compatible hot state, but a deliberate **hot restart** should use the new initializer (existing `hot-reload-state-android.sh` demonstrates this distinction).

**Done:** One documented identity model matches generated AOT key/owner semantics and both dynamic renderers, with no unused “stable identity” metadata masquerading as runtime correctness.

### W6 — Consolidate IR-to-native property contracts, not rendering machinery (P1/P2; F-10)

**Edit:** `crates/nexa-ir`, both backend `generator/components/*`, both `NexaDevRenderer.*` and shared native primitive templates. Optionally add `crates/nexa-component-contracts` only after a pilot proves it is valuable.

**Implementation sequence:**

1. Create a **contract inventory** per IR variant, not by UI marketing name. For each property record: IR source, type, absent/null handling, default, evaluation timing, native primitive argument, modifier order, lifecycle meaning, target-specific exceptions, accessibility effect and tests.
2. Pilot **Column/Row** and **Button/TextInput**, because all four call sites and shared primitives are easy to identify. Capture the **normalized native argument plan** before constructing views; assert equivalence between AOT and Dev adapters for target-matched fixtures.
3. Keep AOT code generation direct: a typed Rust `ComponentPlan` / argument struct should guide Swift/Kotlin source emission; do not require a runtime JSON dictionary or erased view factory.
4. In DevRuntime, use generated read/default tables or carefully mirrored adapters validated by contract tests. Avoid injecting dynamic evaluation into existing native primitives.
5. Extend to complex layout (safe area, sizing, geometry, gestures, animation, clipping), navigation, overlays and FastList **in measured risk order**. Reuse existing primitives and respect their platform-specific implementation details.
6. For each new property, enforce contract completeness on all four paths. If one target legitimately cannot implement it, return an explicit unsupported capability/diagnostic rather than silently selecting a different behavior.

**Contract record sketch (not an implementation of current defaults):**

```rust
struct ComponentPropertyContract {
    ir_variant: &'static str,
    property: &'static str,
    missing_value: MissingValuePolicy,
    evaluation: EvaluationTime,
    modifier_stage: ModifierStage,
    supported_targets: TargetMask,
}
```

**Do not assert that a source-level property must look visually identical on iOS and Android.** Require AOT vs Dev to use the same native primitive semantics **on each platform**, and separately document intended cross-platform behavior.

**Done:** Parameter diffs expose the exact first incorrect property and component additions cannot be merged without all target/mode mappings and device coverage.

### W7 — Differential expression/action/native API conformance (P1)

**Edit:** tests around `crates/nexa-compiler/tests/{ir_expr_parity_gate,action_parity_gate,core_api_parity_gate}.rs`, `runtime/ios/NexaDevState.swift`, `runtime/android/NexaDevState.kt`, `runtime/ios/NexaDevActions.swift`, `runtime/android/NexaDevActions.kt`, and AOT expression/action generators.

- Turn “handler name found in a file” into a test of **actual values, state diffs, side effects, errors and ordering**.
- Build a typed corpus of deterministic `.nx` expressions and actions: nulls, optional maps, Unicode, integer/float boundaries, locale/time zone, arrays/maps, closures, loops, `Result`, error paths, async cancellation, animation boundaries, native property and subscription lifetimes.
- For each case run AOT Swift/Kotlin native test harness and DevRuntime on the same platform. Where Rust can evaluate a subset without importing native UI, use it as a *reference oracle* for that subset only.
- Record ordered event traces with stable timestamps replaced by sequence numbers; test asynchronous causal order rather than requiring nondeterministic wall-clock completion order.
- Convert unsupported DevRuntime operations into structured visible failures in dev and strict test failures; preserve test logs and source spans.
- Add generative/property tests to the Rust semantic subset and minimize counterexamples. Random seeds go in artifact manifests.

**Done:** Regression in value semantics or side-effect order fails a test even though all `Node`/`Action` enum names remain present in source.

### W8 — Improve native visual, accessibility and interaction parity (P1; F-08)

**Edit:** `tests/compare-render-parity.py`, `tests/test_render_parity_comparison.py`, `tests/hot-reload-aot-parity.sh`, iOS UI tests, Android UI harness and capture scripts.

**Capture layers per checkpoint:**

1. Canonical semantic IR fingerprint (from compiler).
2. Node parameter plan / state snapshot fingerprint (debug-only).
3. Native geometry tree: logical x/y/width/height, measured content size, clipping, hit box, visible/hidden, layout direction, font metrics, scroll offsets.
4. Accessibility tree: role, label, value, enabled, selected, actions, focus and traversal order. **Do not sort away an order difference when traversal order is itself under test.**
5. Screenshot plus unmasked diff, masked diff, exact environment and exclusion reasons.
6. Event/revision/frame sequence.

**Visual gates:** Keep current MAE threshold as a first baseline; move to per-feature budgets only with measured platform noise. For deterministic geometry use exact logical dimensions or a justified physical-pixel quantization tolerance. Test system-owned regions separately from app-owned header/bottom navigation. For overlays compare actual measured surfaces, handle, top/bottom and scrim. For lists compare viewport-relative row geometry, visible order, scroll anchoring, child hit targets and nested gesture arbitration. For animation, compare start/end state and sampled transition frames with a synchronized clock or controlled progress, not screenshots at arbitrary times.

**Mutation tests of the tests:** Inject a 4dp header offset, change disabled hit area, swap accessibility traversal order, reverse navigation transition, alter scroll restoration and omit one native action. Each must cause a known failed gate, not a masked pass.

**Done:** All significant user-visible parity regressions fail at least one check and produce a readable first-divergence report.

### W9 — Native host/plugin/resource truthfulness (P1; F-09/12)

**Edit:** `crates/nexa-cli/src/commands.rs` native rebuild classifier, project planning templates, `crates/nexa-dev-protocol/src/lib.rs`, native schema files and plugin adapter generation.

- Add `NativeHostCapabilities` fingerprint containing compiler/runtime template versions, plugin contract IDs/signatures, native dependencies, permissions/entitlements, bundled assets/fonts, OS minimums and background tasks. Do not hash the full tree every watcher iteration: compute from already resolved build inputs, refresh on a host rebuild.
- At `nexa dev` startup, bind host fingerprint to the session. On source revisions, compare required capabilities against the host. An old host may accept IR-only edits but **must not claim** source/host parity after a capability-changing edit.
- Retain existing `b` manual rebuild UX where intentional; make the status persistent and detectable by CLI/tests. Optionally support `--auto-rebuild-host=on` as an explicit policy, not a silent behavior change.
- Generate strongly typed AOT plugin adapters as before. The DevRuntime dynamic bridge must follow the same `.nxid` serialization, optional/error, event and cancellation contract. Conformance tests use a fake deterministic native plugin on both modes.
- Treat missing native components and property/event bridge failures as test-fatal diagnostics with actionable recovery.

**Done:** Host/version/plugin/resource mismatches are diagnosed before a misleading hot reload or screenshot comparison.

### W10 — Strengthen CI and make regressions impossible to ignore (P0/P1)

**Edit:** `.github/workflows/ci.yml`, `PARITY.md` source fixture and report script, test harness.

**Existing jobs to KEEP:** Ubuntu Rust checks, iOS SDK compile + iPhone simulator component/state/list/visual parity, Android SDK compile + emulator component/state/list/visual parity. These are already present. Add targeted jobs/steps rather than duplicating them.

**Proposed stages:**

| Gate | On PR | Nightly/weekly | Failure policy |
|---|---|---|---|
| Semantic mode acceptance + clean-vs-cache | All compiler/IR edits | All fixtures | Block merge |
| Static variant inventory / mutation test | All IR/runtime edits | Full corpus | Block merge |
| Patch protocol/atomicity/unit tests | All protocol/runtime edits | Fuzzed sequences | Block merge |
| Targeted iOS/Android AOT-vs-Dev device cases | Files affected by a component | All parameter routes | Block merge if repeatable |
| Full 41-route catalog + interactions | Smoke subset PR | Full on nightly | Block release, triage flaky cases |
| Resource/plugin/host rebuild cases | Relevant edits | Full platform matrix | Block merge on unhandled stale-host case |
| AOT benchmark (startup/frame/memory/size) | Native/backend changes | Controlled lab runs | Block regression beyond baseline CI noise |
| Dev reload latency benchmarks | Dev compiler/runtime edits | Controlled lab runs | Performance budget gate |

The existing `PARITY.md` **agent review** can stay, but it should link to current automated pass/fail evidence and human approval separately. Never substitute review notes for automated proof.

**CI artifacts on failure:** input `.nx`, canonical/optimized/Dev IR, target native argument plan, runtime/host versions, event/state traces, screenshot pair, unmasked diff, masked diff, console output, precise native platform info and minimized reproduction.

---

## 5. Performance constraints, budgets and measurement design

**Important:** This plan does *not* require using one dynamic renderer in AOT. The unification is semantic and at the existing native primitive boundary. **No release-frame patch protocol, interpreter state map, debug geometry walker or JSON parsing.**

### Metrics to baseline before changes

- **AOT iOS / Android:** cold start to first meaningful frame, p50/p95 frame time under native list scroll, steady heap, allocated objects per scroll frame, binary size, generated source size, UI responsiveness under rapid state changes, native component overhead.
- **DevRuntime:** cold full-module install, patch prepare time, patch main-thread commit time, patch→module-ack latency, patch→first-presented-frame latency, incremental compiler latency and reload throughput.
- **Compiler:** clean and warm compile time, cache hit/invalidation count, optimizer pass timings and generated native source size.
- **Methodology:** same device, OS, compiler version, release flags, app fixture and warm-up procedure; >= 10 repetitions for stable microbenchmarks, more if device noise demands. Keep raw samples and noise bounds.

### Proposed initial budgets (not measured facts)

- **AOT:** no statistically meaningful regression from the *measured* baseline. For automated preliminary alerts use p95 frame time > +3%, memory > +3%, binary size > +2% or startup > +3%; adjust based on noise and architecture owner approval. A persistent performance regression cannot be dismissed because parity improved.
- **DevRuntime:** target no increase in patch main-thread commit p95; measure the cost added by transaction preparation. Set an absolute latency SLO only *after* actual existing baseline capture.
- **Compiler:** warm `.nx` source edits should remain incremental; do not force full lowering in the watch loop for parity checks. Put expensive equivalence checks behind CI/test/debug flags.

**Safeguards:**

1. Never insert dynamic contract lookup in AOT component render hot paths.
2. Generate or specialize native code at compile time; typed generated structs/functions are acceptable, runtime maps are not.
3. Put introspection and trace compilation under test/debug flags only.
4. Do not deep-serialize full Dev IR for each patch merely to ensure atomicity; use copy-on-write candidate preparation.
5. Keep native virtualization and keyed list performance.
6. Test AOT runtime performance separately from compiler and DevRuntime overhead so improvements in one dimension do not hide regressions in another.

---

## 6. Concrete implementation backlog (PR-sized, dependency-ordered)

| PR | Priority | Files / owner | Change | Required proof | Depends on |
|---|---|---|---|---|---|
| **01** | **P0** | `runtime/android/NexaDevProtocol.kt` | Apply successful patch updates on main; remove duplicate translation/diagnostic writes | Patch during scrolling + one publish/ack per revision | None |
| **02** | **P0** | `NexaDevState.kt`, protocol | Side-effect-free candidate patch, atomic commit and rollback | Invalid/conflicting path rejection preserves old module/state | 01 |
| **03** | **P0** | `semantic/mod.rs`, `generics.rs`, project | Remove mode-only acceptance of nullable generic plugin reads | Dev and AOT accept/reject same plugin fixture, both targets | None |
| **04** | **P0** | compiler tests | Actual semantic acceptance + IR projection gate | Mode differences fail test even if code symbol names unchanged | 03 |
| **05** | **P1** | `ir_node_parity_gate.rs`, other string gates | Fix unconditional success and generate exhaustive IR inventories | Mutation test removes one dispatch and gate fails | None |
| **06** | **P1** | test shell/Python, protocol test extension | Source/environment manifest and deterministic first-frame fence | Screenshot tests cannot observe stale revision | 01 |
| **07** | **P1** | compiler cache / Dev IR | Clean-vs-cached and full-vs-patch differential fixtures | 50+ edited revision sequences match normalized results | 04, 02 |
| **08** | **P1** | IR/Dev IR + native renderers | Normative keyed/structural state/route identity; consume or retire identity metadata | Insert/reorder/nested navigation state tests | 07 |
| **09** | **P1** | renderer and backend component adapters | Contract-plan pilot for Column/Row/Button/TextInput | Same platform has identical effective parameters and device geometry | 04 |
| **10** | **P1** | runtime actions/state + backend tests | Value, event-order, error and native API conformance harness | Fail on semantic mutation | 04, 05 |
| **11** | **P1** | CLI/host/protocol | Capability fingerprint and rebuild-required hard diagnostics | Mutating plugin/assets cannot falsely pass hot parity | 06 |
| **12** | **P1** | screenshot comparator and device capture | Unmasked full-frame/header/footer checks; real overlay geometry | Mask/offset mutation tests fail | 06, 09 |
| **13** | **P2** | remaining component emitters/DevRenderers | Extend property contracts to sheets, lists, gestures and typography; **navigation system rewrite handled by N1–N9** | Full route/parameter and interaction conformance | 08–12 |
| **14** | **P1** | benchmark suite, CI | AOT native and Dev patch latency baseline + budgets | No accepted regression across stable device runs | 01–13; plus N5–N7 navigation benchmarks |
| **15** | **P0 release** | `.github/workflows/ci.yml`, `PARITY.md` | Enforce gates and attach triage bundle | Deliberate bug is blocked from merge/release | 04–14, N1–N9 |

**Navigation rollout:** PRs **N1–N9** in §9 are a **required parallel track** (not optional future work). The existing PR 13 no longer owns navigation. PR 14/15 release decisions must include N5–N9 performance and conformance.

**Owners:** Compiler lead for PR 03/04/07/10, Android lead for 01/02, iOS and Android leads for 06/08/09/12/13, developer-tools lead for 11, QA/CI lead for 05/12/14/15. Where staff are limited, prioritize PR 01–07 before adding new feature APIs.

**Sizing (planning estimates, not committed dates):** PR 01–05 can be parallelized and should precede broad changes; PR 06–12 are the core parity infrastructure; PR 13 is ongoing non-navigation contract migration; **N1–N9 are additional substantial navigation rewrite work and may increase total duration**; PR 14–15 lock in optimization and release quality. The previously estimated 12–20 weeks for the base parity program should **not** be read as the combined navigation v2 delivery estimate. Size N1–N9 separately after test reproductions and API contract approval.

---

## 7. Test matrix that must be real, not nominal

### 7.1 Three different types of parity tests

**A. Cold-start AOT vs cold-start DevRuntime (same target):** Same source, same native capabilities, clean app data, same environment, same event sequence. Strict equality/justified tolerance applies.

**B. Incremental DevRuntime vs full-module DevRuntime:** Begin with same running state; apply incremental patch vs equivalent full replacement. After applying documented migration, state and rendered output must match. This isolates patch correctness from differences in generated AOT.

**C. Hot-reload state evolution vs newly rebuilt AOT:** This is **not** a naïve snapshot equality test. Define whether AOT is cold-started or restored with equivalent serialized state. Otherwise changed initializers intentionally cause different values after a hot reload and should not be misclassified as a framework defect.

### 7.2 Minimum functional matrix

| Area | Required cases | Observation |
|---|---|---|
| Compiler | Same source passes/fails, optional plugin read, target gates | Result/diagnostic/semantic IR |
| IR cache | Function body and signature edits; shared component/global edits; import, analyzer and enum edits | Fresh vs cached normalizations |
| Protocol | Full, styling patch, function patch, screen patch, invalid JSON pointer, revision mismatch, packet loss/reconnect | Atomic snapshot, ack/recovery |
| Layout | Column/Row/Stack, intrinsic text, padding order, min/max, safe area, nested scroll, RTL | Node parameter plan + native measured frames |
| Text | Omitted styles, explicit font/weight, multiline, truncation, accessibility text scale, locale and Unicode | Glyph/container frame and screenshot |
| Controls | Button, Pressable, TextInput, Switch, Slider, Picker, DatePicker, segmented, form | Actions, accessibility, hit targets, focus and input |
| Lists | FastList modes, keyed rows, insert/reorder, lazy scroll, refresh, page snap, gestures | Visible row order, scroll, retained state, memory/frame time |
| Navigation v2 | Same route pushed twice; typed push/pop/pop-to/replace/reset; tab ownership; split view; native back/interactive cancel; cold/warm deep links; restoration; hot-reload graph migration | Canonical `NavigationSnapshot`, entry-owned state, lifecycle/transition traces, native visual/gesture captures (see §9.5) |
| Overlays | BottomSheet, dialog, confirmation, scrim/drag/back, keyboard | Sheet frame, animation phase, callback sequence |
| Lifecycle | appear/disappear, onActive/onInactive, foreground task cancel, async completion | Ordered events and state |
| Plugin/resources | Generic codec, null/error, native event subscription, new resource/permission/asset | Values/errors + host capability check |
| Device configuration | Light/dark, small/large font, RTL, rotated viewport, OS-supported native control variants | Same-target AOT vs Dev geometry and semantics |

### 7.3 Environment independence

- Do not use iOS screenshot pixel thresholds to judge Android or vice versa.
- Pin emulator/simulator images in CI for reproducible baseline, then test additional supported OS versions nightly.
- Record actual app viewport bounds; don't infer from physical screenshot width/height alone.
- Disable debug performance overlays when capturing parity, or explicitly exclude only those own debug overlays.
- Control time, network and randomness with deterministic adapters/test fixtures, not production hacks.
- Make sure AOT and Dev builds come from the **same source archive/commit** and target configuration. Stale CLI/host is a test configuration failure, not an unexplained parity failure.

---

## 8. Example acceptance scenarios with expected results

### Scenario S1 — Dev compiler must not accept a program rejected by AOT

**Input:** plugin `getObject<String?>("maybe")` fixture from `plugin_generics.rs:630-631`.<br>
**Before:** mode-driven code permits top-level optional generic reads in dev lowering, while AOT validation rejects.<br>
**After first fix:** both reject with equivalent diagnostic.<br>
**Later feature extension:** both accept only if typed native result codec unambiguously represents `null` vs decoding failure.<br>
**Pass:** identical compiler acceptance semantics for iOS and Android; no relaxation to `null` fallback.

### Scenario S2 — Invalid patch cannot poison last-good Android state

**Setup:** valid module revision `r1`, known state `count=7`.<br>
**Action:** send a patch with `base_revision=r1`, operations that each validate against original object but conflict sequentially (remove `/status_bar`, then set `/status_bar/background` (with `status_bar` initially an object)), or one invalid replacement type.<br>
**Expected:** reject without altering `r1`, its state, its translation registry, or any composable presentation; request full module; no ack of `r2`.<br>
**Pass:** all snapshots equal before/after rejection and subsequent valid revision succeeds.

### Scenario S3 — Rapid patch while Compose is reading state

**Setup:** FastList scroll with stateful rows, text input editing, navigation transition.<br>
**Action:** apply 100 distinct valid IR patches while user events continue.<br>
**Expected:** sequential committed revisions; no off-main publication of Compose state; no dropped events, no stale closure, no duplicate translation invalidation.<br>
**Pass:** final view/state equals a full replacement with the same migrated starting state; p95 commit time within dev performance budget.

### Scenario S4 — UI screenshot after frame, not just ack

**Setup:** theme/color patch changes a large visible surface.<br>
**Action:** send patch and capture immediately after `Acknowledge`, then after the test-only presented-frame barrier.<br>
**Expected:** only the second capture is used for parity verdict.<br>
**Pass:** no stale-frame false positive/negative; test times out clearly if rendering doesn't reach that revision.

### Scenario S5 — Explicitly keyed reorder preserves the right row's state

**Setup:** three item rows, each with local state, and stable item keys.<br>
**Action:** reorder rows, insert before first item, then hot reload a handler.<br>
**Expected:** item state and action ownership stay with the logical keyed item; disposed item effects run once.<br>
**Pass:** AOT and DevRuntime behavior follows the same identity contract. Repeat with unkeyed rows to validate its *documented* semantics.

### Scenario S6 — Existing comparator cannot hide app-owned bottom bar mismatch

**Setup:** known bottom bar in AOT and Dev.<br>
**Action:** mutate only one adapter's bar height/position by a visible amount while keeping a screenshot mask.<br>
**Expected:** native geometry/hit-target comparison fails, and unmasked diff identifies the bar region.<br>
**Pass:** CI blocks the regression despite the existing generic 7% screen-edge mask.

---

## 9. Navigation v2 — source-audited, complete navigation-system rewrite

**Priority: P0 product/architecture workstream, P0 release gate.** Navigation is not a minor component-property cleanup. Replace the current ad hoc cross-mode navigation implementation with a defined **route graph, state machine, entry identity and platform navigation adapters**, while keeping production AOT **direct native SwiftUI / Jetpack Compose**. This section supplements—and intentionally supersedes the navigation-specific portions of W5/W6 and PR 13 above. It is a proposed redesign; no rewrite has been implemented or device-validated by this audit.

### 9.1 Concrete navigation audit: current behavior and weaknesses

These findings are based on the archive, with code positions relative to the audited snapshot. `VERIFIED` describes a source-code property; device symptoms still require reproduction.

| ID / severity | Evidence and precise location | Consequence / corrective direction |
|---|---|---|
| **NAV-01 / P0 — screen state is keyed by screen name in both DevRuntimes** | iOS `runtime/ios/NexaDevRenderer.swift:3998-4045` uses `scope = "screen/\(name)"` when rendering any destination; state values resolve through `runtime/ios/NexaDevState.swift` using that scope. Android `runtime/android/NexaDevNavigation.kt:174-189` builds `scope = "screen/${screen.optString("name")}"` and `runtime/android/NexaDevState.kt:750-770` uses the same keys. | Two pushed instances of the same screen can share state/effect/parameter namespaces in DevRuntime. Contrast: Android AOT `crates/nexa-backend-kotlin/src/generator/components/navigation.rs:890-932` stores generated screen state per `NavBackStackEntry.viewModelStore`; Swift AOT `.../navigation.rs:130-165` carries a distinct UUID route identity. **Fix: entry-owned state scopes everywhere.** |
| **NAV-02 / P0 — stack selection uses ambiguous fallback** | iOS `runtime/ios/NexaDevState.swift:701-754` chooses a stack by `locals["__nexa_navigation_stack_identity"]`, otherwise `navigationPaths.keys.first` or `"app"`; Back chooses first non-empty/first path. Android `runtime/android/NexaDevState.kt:166-206` keeps a `currentNavController` and falls back by iterating registered controllers in reverse order; actions `runtime/android/NexaDevActions.kt:31-59` may fall back to this global. | A push/back triggered by a detached task, tab, nested stack or stale callback can affect the wrong stack. **Fix: explicit `NavigatorId` captured with the event/effect owner; no unordered/global fallback.** |
| **NAV-03 / P0 — route payloads are not authoritative entry identities in iOS Dev** | `runtime/ios/NexaDevNavigation.swift:31-40` builds token from `screen + canonicalJSON(values)`, storing arguments in `routeArguments[token]`. `runtime/ios/NexaDevState.swift:683-698` generates a UUID only when duplicate tokens are found on assignment. `screenDidDisappear` supports route-aware cleanup at `:930-976`, but the screen render call at `runtime/ios/NexaDevRenderer.swift:4043` passes only `scope`, leaving default route identity `"root"`. | Duplicate route instances are awkward, arguments are stored in an auxiliary growing map, and route-aware cleanup is not reached from this observed call path. **Fix: allocate an `EntryId` at push time; store payload on the entry and pass entry identity into render/appear/disappear.** |
| **NAV-04 / P1 — Android hot reload may reset the entire nav host** | `runtime/android/NexaDevState.kt:575-585` increments `navigationEpoch` when the ordered screen-name signature changes. `:218-225` increments it on locale override, and `:629` on hot restart. `runtime/android/NexaDevNavigation.kt:123-125` puts `rememberNavController()` beneath `key(store.navigationEpoch)`. | Editing the screen inventory can recreate the native controller and lose the back stack; locale change also forces recreation. Some resets are intentional for hot restart, but the *graph-compatible reload* behavior needs a stronger contract. **Fix: reconcile route entries against the new graph; recreate controller only for explicitly incompatible host/graph migrations or intentional restart.** |
| **NAV-05 / P1 — deep-link behavior is independently implemented and visibly non-equivalent** | iOS AOT `crates/nexa-backend-swift/src/generator/components/navigation.rs:236-329` parses `onOpenURL` and **clears** the navigation path before appending the destination. Android AOT `crates/nexa-backend-kotlin/src/generator/components/navigation.rs:711-795` parses the activity's intent URI in a `LaunchedEffect` and **navigates/pushes**. Searching both DevRuntime runtime trees found no matching inbound native deep-link handler; AOT has separate `tests/deep-link-smoke.sh`. | Opening the same route URI while already navigated need not yield the same stack on iOS and Android; AOT and Dev host coverage also differs. **Fix: one route parsing/dispatch policy and a declared policy for external link stack replacement vs push.** Test cold-start, warm-intent delivery and DevRuntime. |
| **NAV-06 / P1 — iOS Dev back gestures are layered** | `runtime/ios/NexaDevRenderer.swift:3860-3950` constructs `NavigationStack`, attaches a `simultaneousGesture(DragGesture)` to destinations and the entire stack, and also applies `NexaDevNavigationBackSwipeModifier` backed by a UIKit `interactivePopGestureRecognizer` delegate (`:3702-3830`). Swift AOT has a separate gesture-probe implementation in `crates/nexa-backend-swift/src/generator/engine/types.rs:338-416`. | Multiple gesture paths and duplicate delegate/probe logic can conflict with system interactive-pop transitions, scroll gestures and RTL. **Fix: one native gesture coordinator per platform/stack, disabled or adjusted for custom chrome; test actual interactive cancellation.** Do **not** replace SwiftUI interactive navigation globally with a manual DragGesture. |
| **NAV-07 / P1 — screen lifecycle is name-scoped, not entry-scoped** | iOS `runtime/ios/NexaDevRenderer.swift:4000-4045` attaches `.onAppear`/`.onDisappear` to name-scoped screen rendering. Android `runtime/android/NexaDevNavigation.kt:262-303` tracks `LifecycleOwner` and uses `scope = "screen/${screen.optString("name")}"`; `activeScreenParameters` is name-scoped. | Rapid push/pop, duplicate screen instances, in-flight async effects and tab switching can invalidate each other's ownership. **Fix: entry-specific scope and explicit foreground/visible/resumed/cancelled lifecycle events.** |
| **NAV-08 / P1 — there is no common navigation command or transition model** | IR `crates/nexa-ir/src/lib.rs:1072-1102, 2223-2231` has `NavigationStack`, `NavigationLink`, `NavigationBack`, `NavigationPush`, `NavigationSplitView` but no explicit replace, pop-to, reset, result or transition command. The Swift backend directly appends to a `Binding<[Route]>`; Kotlin directly calls `navController.navigate`, with independently defined 350 ms slides (`crates/nexa-backend-kotlin/.../navigation.rs:237-274`, `runtime/android/NexaDevNavigation.kt:135-163`). | The existing navigation API cannot describe richer, deterministic stack operations, and independently coded transitions encourage drift. **Fix: typed `NavigationCommand` + `NavigationTransitionPolicy` as canonical semantics.** |
| **NAV-09 / P1 — host model and tabs are implicitly coupled** | Compiler `crates/nexa-compiler/src/semantic/mod.rs:1273-1283` requires top-level `NavigationStack` in each applicable app-body branch; `semantic/components.rs:3030-3096` allows stacks only at app-body or an `AppBottomBar` tab root. Both AOT bottom-bar emitters have their own navigation chrome logic (`.../components/bottom_bar.rs`). | Difficult to extend to independent flows, deep links into nested tabs, adaptive split layouts, modal destinations or custom shells without more special cases. **Fix: define `Navigator` scopes and graph ownership explicitly; retain existing syntax via lowering adapters.** |
| **NAV-10 / P1 — the existing test matrix is too narrow for a rewrite** | `tests/hot-reload-aot-parity.sh:21-25` has only four navigation-only phases (`navigation`, `navigation-nested`, `navigation-pop`, `navigation-root-restored`). `crates/nexa-cli/tests/fixtures/dev_navigation_reload.nx` contains a simple Home→Details→Back flow. | These are useful smoke tests, **not** coverage of repeated destinations, per-entry state, back cancellation, tabs, restoration, deep links, concurrent actions or hot reload. **Fix: add command/state-machine tests, cross-mode event traces and device flows before switching implementation.** |

**Audit qualification:** NAV-01/03/04/05/06/07/08/09/10 are confirmed structural/source properties, not a claim that all listed user-facing symptoms were reproduced. NAV-02's fallback is definitely in source; which user sequences trigger a wrong-stack result must be tested. Before removing native behavior, establish reproductions and device baselines. The current system already supports meaningful navigation paths—avoid a blanket assertion that every operation is broken.

### 9.2 Product definition: what the replacement must support

Minimum v2 contract, independent of whether the public `.nx` spelling changes immediately:

1. **Typed route declarations and payloads:** compile-time known destination, required/optional fields, schema validation, stable route key independent of declaration order; errors for missing or invalid parameters.
2. **One authoritative navigation tree:** independent stack per navigator/tab, plus overlays/sheets modeled explicitly; no global current stack guesswork.
3. **Entry-based state:** each push creates an `EntryId`; navigating to the same screen with equal arguments creates a new entry unless a deliberate single-top/replace policy is requested.
4. **Atomic commands:** `push`, `back/pop`, `popTo`, `replace`, `reset`, `selectTab`, and `openExternalRoute`; specify success/failure and no-op behavior. Modal presentation should use a separate, typed presentation command if added.
5. **Deterministic host reconciliation:** hot reload preserves graph-compatible entries and state; incompatible destinations and changed route schemas get an explicit migration or safe, diagnosable removal/reset.
6. **Native behavior:** maintain SwiftUI `NavigationStack` and Compose Navigation for production; native accessibility, titles/toolbars, system bars, gestures, predictive/back behavior and platform animations remain first class.
7. **Deep links and restoration:** validated inbound URL decoding, cold and warm intent handling, tab selection, nested-route reconstruction, navigation save/restore with versioned payloads and privacy-safe persistence.
8. **Tab/adaptive semantics:** each tab owns its history, switching tabs preserves it; tablet/split view has defined selection and back behavior; dialog/sheet ownership cannot be confused with stack history.
9. **Cancellable navigation transitions:** back gestures and interactive cancellation must not dispose state until the pop is committed. Rapid repeated actions must not create out-of-order entries.
10. **Parity/observability:** identical logical `NavigationSnapshot` after each command across AOT/DevRuntime on the same platform; visual comparisons made at matched transition progress, never arbitrary wall-clock frames.

**Scope boundary:** This is a navigation rewrite, **not** a request to rewrite layout primitives, all view generators, or native UI frameworks. Avoid introducing a generic interpreted router into AOT. Existing `.nx` navigation syntax remains supported through a compatibility lowering until the v2 API is finalized.

### 9.3 Target architecture — one semantic router, four efficient adapters

```text
.nx Screen / NavigationStack / Link / actions / tabs / deep-link config
                             │
                             ▼
             Common semantic validation & route graph
           (stable RouteKey, payload types, graph topology)
                             │
                      Canonical Navigation IR
        (NavigatorId, EntryId, Command, Policy, Snapshot)
                             │
                ┌────────────┴────────────┐
                ▼                         ▼
          AOT lowering                Dev lowering
       static route enums/       compact/versioned nav IR
       functions + adapters       + typed payload checks
          │          │              │             │
        Swift       Kotlin          Swift         Kotlin
          │          │              │             │
          └─────┬────┘              └──────┬──────┘
                ▼                          ▼
         native SwiftUI / Compose navigation components
                └──────────────┬───────────┘
                               ▼
           **same logical snapshot + lifecycle trace**
```

**Design choice:** The canonical router is a *pure, testable state machine and codegen contract*, not a mandatory runtime interpreter. The Rust semantic model supplies types/commands/transitions; optimized Swift/Kotlin output compiles typed operations directly, while DevRuntime decodes equivalent commands and updates its versioned state store. Per-platform native coordinators should be reused by AOT and Dev where that does not compromise static specialization.

**Proposed Rust IR/types (new; adapt naming to existing code):**

```rust
struct RouteDecl {
    key: RouteKey,                 // stable key, never positional ScreenId index
    screen: ScreenId,              // current IR index retained as internal reference
    payload: Vec<TypedRouteField>,
    schema_version: u32,
}
struct NavigatorDecl {
    id: NavigatorId,               // stable per app/tab/declared owner
    root: RouteKey,
    presentation: NavigatorPresentation, // stack, adaptive split, tab owner
    chrome: NavigationChromePolicy,
    transitions: NavigationTransitionPolicy,
}
struct NavigationEntry {
    entry_id: EntryId,             // unique per push; not payload hash
    route: RouteKey,
    payload: TypedRoutePayload,
}
struct NavigationSnapshot {
    graph_version: u32,
    active_navigator: NavigatorId,
    stacks: Map<NavigatorId, Vec<NavigationEntry>>,
    presented: Vec<PresentationEntry>,
}
enum NavigationCommand {
    Push { navigator: NavigatorId, route: RouteKey, payload: TypedRoutePayload },
    Pop { navigator: NavigatorId },
    PopTo { navigator: NavigatorId, entry: EntryId, inclusive: bool },
    ReplaceTop { navigator: NavigatorId, route: RouteKey, payload: TypedRoutePayload },
    Reset { navigator: NavigatorId, root: NavigationEntry },
    SelectTab { navigator: NavigatorId },
    OpenExternalRoute { uri: ValidatedUri },
}
```

`Map`, `TypedRoutePayload`, `ValidatedUri` and policy types above are **design pseudotypes**, not currently defined Rust APIs. Use existing small enums/structs and `serde` compatibility policy rather than copying this literally into `nexa-ir` without RFC review.

**Reducer invariants to write as tests before codegen:**

- `push` increases the addressed stack's length by exactly 1 and leaves other navigators untouched; a new `EntryId` is always generated or supplied by the platform action source.
- `pop` removes at most 1 entry and never removes the root; a `back` from a modal follows modal precedence, then active stack, then platform exit policy.
- `popTo` finds a specific existing entry (not just a screen name); missing entry returns a typed error/no-op according to an explicit policy.
- `replaceTop` changes the addressed top entry without mutating unrelated entries.
- `reset` and deep link have an explicitly configured behavior: replace current stack, replace the active tab stack, or reconstruct a typed parent chain; **do not let iOS clear and Android push silently**.
- Native user back, app `NavigationBack`, system/predictive back and deep links all enter the **same command path**; command commit owns route-state destruction and lifecycle changes.
- Duplicated screen names/payloads are legal on separate entries; no duplicate-token canonicalization needed.
- Route keys are stable under screen declaration reordering, unrelated edits and hot reload. Collisions/renames are explicit compiler diagnostics or migration rules.
- `onAppear/onDisappear` are defined by entry visibility/foreground policy. Their invocation count cannot depend accidentally on host recomposition, screenshot timing or animation frame count.
- Unhandled/missing destinations must produce typed navigation diagnostics in Dev and compilation errors in AOT when statically resolvable; never silently render an empty destination.

### 9.4 Exact code change plan, in implementation order

**N1 — Navigation contract RFC + failing reproductions (P0; no production rewrite yet)**

**Change/add:** `docs/architecture/navigation-v2.md` (new), `tests/fixtures/navigation-v2-*.nx` (new), `crates/nexa-compiler/tests/navigation_semantics.rs` (new), `tests/hot-reload-aot-parity.sh` (extend).

- Document navigation entry identity, command ordering, tab ownership, back precedence, deep link policies, state restoration and effects. Specify platform differences intentionally rather than hiding them.
- Write minimum failing tests for: push same screen twice with distinct counter state; push from nested tab then Back; hot reload after adding an unrelated screen; and warm deep link while stack depth > 1. Capture an initial AOT vs Dev trace, not only screenshots.
- Snapshot existing native back gestures and transitions on real/simulator iOS and Android so the rewrite does not regress native interactions. Include reduced motion, RTL and low-end Android device.
- **Exit:** architecture and deterministic tests approved; at least one real failing navigation parity case captured before implementation.

**N2 — Canonical graph, stable route keys, typed command semantics (P0)**

**Change:** `crates/nexa-ir/src/lib.rs`, `crates/nexa-ir/src/walk.rs`, `crates/nexa-ir/src/facts.rs`, `crates/nexa-syntax/src/ast.rs`/`parser.rs` only for new syntax, `crates/nexa-compiler/src/semantic/components.rs:3030-3170`, `semantic/mod.rs:1273-1283`, `crates/nexa-dev-ir/src/lib.rs`; add `crates/nexa-navigation` pure Rust crate **only if** it avoids cyclic dependencies (else a submodule in existing compiler/IR).

- Add `NavigationGraph`/`RouteDecl` and stable `RouteKey`; continue storing positional `ScreenId` internally only as a resolved reference, not a persisted route identity. Determine key strategy from declared stable name/optional explicit key, with collision validation and compatibility mapping.
- Add typed commands in IR; lower existing `NavigationStack`, `NavigationLink`, `NavigationBack`, `NavigationPush` into those command semantics. Preserve old syntax. Typed route payload checks must be identical in AOT and dev compile paths.
- Separate `NavigatorId` (which stack) from `EntryId` (which pushed instance). Assign `NavigatorId` deterministically to app root and tab roots, not runtime dictionary order or a global controller.
- Add a pure `reduce(snapshot, command) -> Result<Transition, NavigationError>` reference model. Keep transition policy declarative; emitted native code does not need to call Rust.
- Add compiler diagnostics for route ambiguity, unsupported nesting, invalid payloads, illegal root pops and ambiguous navigation from detached/background owners. Validate graph consistency on hot-reload changes.
- **Exit:** compiler model/property tests assert stack invariants, duplicate entries and correct graph validation; optimized normal compilation still uses existing native hosts.

**N3 — Per-entry DevRuntime state and lifecycle (P0; fixes NAV-01/03/07)**

**Change:** `runtime/ios/NexaDevState.swift:407-976`, `runtime/ios/NexaDevNavigation.swift:1-41`, `runtime/ios/NexaDevRenderer.swift:3831-4050`; `runtime/android/NexaDevState.kt:139-225,455-770`, `runtime/android/NexaDevNavigation.kt:1-303`, `runtime/android/NexaDevActions.kt:31-80`.

- Replace `screen/<name>/state/<field>` ownership with `navigator/<navigatorId>/entry/<entryId>/screen/<routeKey>/state/<field>` (exact string encoding may vary; choose a typed key struct where practical). **Do not migrate app-wide state to route-owned storage.**
- Make state lazy **per mounted/created route entry**, so a screen definition does not initialize one global set of values shared by all pushes. State signatures and initializer templates remain keyed by route declaration/schema.
- iOS: allocate identity before creating route, store values with the entry, pass `EntryId` to `renderScreen` and its `.onAppear/.onDisappear`; remove payload-hash route identity/auxiliary value dictionary as the authoritative owner. Gate cleanup by committed pop or explicit entry retirement, not `onDisappear` alone (covered screens may remain on stack).
- Android: attach an entry-specific scope to the actual NavHost back-stack entry. Keep parameter evaluation and subscriptions owned by that entry. Ensure `onPause/onResume` and Compose recomposition do not re-run route initialization; distinguish visible vs removed entries.
- Migrate compatible entry state on a Dev IR reload; cancel effects exactly once for removed entries; disallow stale async completion from reviving removed entry state. For breaking route schema changes, produce a migration result and reset/remove only affected entries.
- **Exit:** two `Details(id: 7)` entries own separate state; popping one cleans only its resources; returning to a covered entry preserves its state on both platforms and both modes.

**N4 — Scoped navigation coordinator and one dispatch path (P0; fixes NAV-02/08)**

**Change:** `runtime/ios/NexaDevState.swift`, `runtime/ios/NexaDevActions.swift`, `runtime/ios/NexaDevRenderer.swift`, `runtime/android/NexaDevState.kt`, `runtime/android/NexaDevActions.kt`, `runtime/android/NexaDevRenderer.kt`, `runtime/android/NexaDevNavigation.kt`.

- Introduce a scoped `NavigationContext { navigatorId, originEntryId, graphVersion }` captured in every handler, effect and navigation-owned task. Propagate it through buttons, toolbars, custom components, plugin callbacks and asynchronous action contexts.
- Replace `navigationPaths.keys.first`, `"app"` fallback, `currentNavController` fallback and scanning controller lists with explicit resolution. If a callback has no valid navigation owner, return a structured error rather than navigating an arbitrary stack.
- Route *all* navigation entry points (links, action statements, Back buttons, system gestures, deep links) through one command dispatcher. Apply commands on the platform UI thread and publish a single logical snapshot revision.
- Let native controllers remain the actual presentation mechanism, but reconcile them from commands and report native completion/cancellation. Guard against duplicate events from rapid taps or concurrent async actions. Never do work proportional to the entire navigation graph on every tap.
- **Exit:** nested stacks/tabs/actions always target the owning navigator, regardless of registration order or component recomposition; no cross-tab back leakage.

**N5 — Swift AOT adapter + shared SwiftUI native coordinator (P0/P1)**

**Change:** `crates/nexa-backend-swift/src/generator/components/navigation.rs:1-629`, `crates/nexa-backend-swift/src/generator/engine/types.rs:282-416`, `crates/nexa-backend-swift/src/generator/components/bottom_bar.rs`, `crates/nexa-backend-swift/src/generator/mod.rs` native primitive templates, `runtime/ios/NexaDevNavigation.swift`, `runtime/ios/NexaDevRenderer.swift`. Optionally extract an existing shared Swift source module included in both the generated app and dev host.

- Emit typed native routes with `RouteKey`, per-push `EntryId` and compile-time payload types. Keep `NavigationStack(path:)` and native destination builders; no runtime JSON lookup in generated screen views.
- Implement an AOT-specializable `NexaNavigationCoordinator` that owns scoped path mutations, back semantics and route lifecycle. DevRuntime uses the same SwiftUI navigation chrome/back coordination where it can; dynamic evaluation remains dev-only.
- Replace separate/custom Dev gesture layers (`simultaneousGesture` + swipe probes) with one iOS native back integration. Preserve interactive back, swipe cancellation, scroll gesture priority and RTL. Avoid global changes to UIKit gesture delegates that leak outside the current host.
- Align title/toolbar ownership and safe-area policy for root vs pushed screen, custom header and tab nesting; do not universally force `.ignoresSafeArea` or hide the native bar to manufacture parity.
- **Exit:** generated release code retains static enums and native view types; navigation UX/back transitions match Dev on a shared device with no extra `AnyView` in AOT.

**N6 — Kotlin AOT adapter + shared Compose native coordinator (P0/P1)**

**Change:** `crates/nexa-backend-kotlin/src/generator/components/navigation.rs:1-1009`, `crates/nexa-backend-kotlin/src/generator/components/bottom_bar.rs`, `crates/nexa-backend-kotlin/src/generator/mod.rs`, `runtime/android/NexaDevNavigation.kt`, `runtime/android/NexaDevState.kt`.

- Keep `NavHostController`, `NavHost` and generated typed destination payload decoding. Separate logical route and entry identity from the serialized Android route-pattern string/Base64 transport used in DevRuntime.
- Bind each generated screen state ViewModel to its `NavBackStackEntry` and match the logical `EntryId` test trace. Do not introduce a universal DevRuntime-owned route-state map into release output.
- Use the navigation coordinator to enforce single-top/replace/pop-to/reset semantics with appropriate `NavOptions`. One authoritative back dispatch should coordinate `NavHostController`, system back and supported predictive-back APIs; avoid duplicate `BackHandler` consumption.
- Avoid blanket controller recreation via `navigationEpoch` for additive/compatible graph hot reload. Keep nav state on tab switching and use `SavedStateHandle`/native saved state for appropriate recovery.
- **Exit:** route stack operations and entry ownership equal Dev; generated code retains native Compose navigation and primitive state specialization.

**N7 — Deep links, state restoration, tabs/split, transitions (P1; NAV-04/05/09)**

**Change:** `crates/nexa-cli/src/config.rs`, native host templates under `crates/nexa-cli/src/project/`, both AOT navigation emitters, `runtime/ios/NexaDevNavigation.swift`, `runtime/android/NexaDevNavigation.kt`, both `bottom_bar.rs` emitters, `crates/nexa-ir/src/lib.rs`.

- Put URL parsing in a versioned *canonical route specification*: scheme/host allowlist, path patterns, percent decoding, query parameter rules, scalar conversions, default/optional fields, invalid-route policy and **cold/warm** delivery semantics. Generate Swift and Kotlin decoders from one manifest; add equivalent inbound processing to the DevRuntime native host where supported.
- Choose an explicit default for external links, e.g. `replace active flow with validated parent chain + destination`, then implement **that same semantic result** on iOS AOT, Android AOT and both Dev hosts. Preserve optional app-specific `push` policy when declared. Do not implicitly adopt one platform's current behavior.
- Handle iOS `onOpenURL`/scene delivery and Android Activity initial intent/**new intent** correctly, including duplicate delivery and already-running app cases. Verify against host manifest filters/associated domains; do not claim universal links work without proper registration.
- Preserve each tab's independent stack and state. Define back behavior when a modal is presented and when an active tab is at its root. Define adaptive split selection and compact/wide size-class changes. Add explicit handling of destinations that are removed/renamed during a hot reload.
- Specify native transition style, reduced-motion and RTL behavior, interaction cancellation and chrome system insets. Require logical end-state parity, not identical pixel timelines across iOS and Android.
- Implement **versioned restoration snapshots** for appropriate navigation state; do not serialize sensitive or unbounded payloads into plain preferences. Unknown/old routes restore to an explicitly documented safe root.
- **Exit:** same input link/environment yields the same logical nav tree in AOT and Dev for each platform; warm delivery works; tab history and state restore correctly.

**N8 — Public `.nx` navigation API v2 and compatibility migration (P1)**

**Change:** `crates/nexa-syntax/src/{ast,parser,catalog}.rs`, `crates/nexa-compiler/src/semantic/{components,mod}.rs`, `crates/nexa-ir/src/lib.rs`, `crates/nexa-lsp/src/completions.rs`, `crates/nexa-cli/src/docs.rs`, examples under `examples/`, tests for old/new syntax.

- Design the **minimal** new API only after N1/N2 establish semantics; do not start with syntax and retrofit behavior. Candidate capabilities: `Navigator`, typed `route(...)` declarations, `navigate(to:)`, `replace(to:)`, `popTo(...)`, `reset(to:)`, `NavigationBack`, tab ownership and explicitly declared deep-link mapping. These spellings are *proposals*, not implemented syntax.
- Continue accepting existing `NavigationStack(root:)`, `NavigationLink(destination:)`, `NavigationBack(...)` and `navigate(...)`/push expressions (where currently supported) by lowering into v2 commands. Provide warnings only for ambiguous behavior, such as implicit stack selection from a non-screen scope.
- Define API version and semantic migration plan: `navigationSemantics: v1|v2` (exact configuration syntax to be decided), old projects opt into v2 until proven compatible; new projects use v2 once stable. Avoid changing behavior without a version boundary.
- Add diagnostics with source spans and actionable suggestions for unsupported nesting, missing route params, unknown/ambiguous navigator and unreachable destinations. Update LSP, docs, templates, examples and route slug/deep-link docs.
- **Exit:** every current navigation `.nx` fixture still compiles in compatibility mode; v2 syntax is typed and tested; migration examples cover tabs, nested stacks, custom bars and deep links.

**N9 — Formal conformance + performance + rollout (P0 release gate)**

**Change:** `tests/hot-reload-aot-parity.sh`, `tests/fixtures/render_parity.nx`, `crates/nexa-cli/tests/fixtures/dev_navigation_reload.nx`, `tests/deep-link-smoke.sh`, `crates/nexa-cli/tests/deep_links.rs`, `crates/nexa-backend-swift/src/generator/mod.rs` tests, `crates/nexa-backend-kotlin/src/generator/mod.rs` tests, `.github/workflows/ci.yml`, benchmarks.

- Add pure reducer/property tests and fixture-driven command-trace comparison using a canonical `NavigationSnapshot` JSON schema: navigator and entry IDs normalized/mapped, route payloads typed, active navigator, current destination, modal queue, lifecycle events, transition result and state ownership. AOT instrumentation is test-only.
- Run an actual four-mode **same-platform** navigation suite (iOS AOT/Dev, Android AOT/Dev) with source and native host fingerprints. Compare normalized entry trees and state across each step; compare UI accessibility, hit targets, title/chrome/safe-area geometry and matched transition frames.
- Add mutation tests that deliberately route Back to the wrong tab, share state across duplicate entries, dispatch a deep link as push instead of reset, lose history on hot reload and double-pop on gesture. Every mutation must fail a specific gate.
- Performance guardrails: AOT native-first startup no regression outside baseline noise, route push/pop p50/p95 latency, memory retained per 100 pushes/pops, steady heap after disposal, frame time during gesture transitions, generated code/binary size, compile time, Dev patch-to-frame latency. Require `no interpreter / no JSON navigation dispatch / no route-name linear search on AOT hot paths`.
- Rollout through internal fixture → examples → dogfood apps → opt-in projects → default v2; keep a reversible versioned compatibility mode for at least one published migration window. Require device-test signoff and zero untriaged P0/P1 navigation parity defects before making v2 default.
- **Exit:** navigation contract coverage is mandatory on PRs touching routing/state or native navigation; all nine N-workstreams have linked test evidence and performance baseline comparisons.

### 9.5 Required navigation differential test suite (concrete scenarios)

Each scenario must record `sourceHash`, `graphVersion`, `hostFingerprint`, `platform`, `OS`, `mode`, `revision`, `navigatorId`, `entryId`, and a normalized pre/post snapshot. IDs may differ at the byte level across binaries; compare a **deterministic correspondence mapping based on command sequence**, not literal UUID equality.

| Case | Sequence | Expected invariant |
|---|---|---|
| NAV-T01 | Push `Details(id=7)` twice; type different text in each; Back; forward again | Same route+payload may have two distinct entries; state is not shared; each entry's effects are separate |
| NAV-T02 | Home → Details A → Details B → Back twice | Pop removes only top entry and never drops root; exact lifecycle sequence |
| NAV-T03 | Tab A depth 3 → Tab B depth 2 → Tab A | Histories, scroll and state survive tab switch; Back affects active tab only |
| NAV-T04 | Push from nested screen toolbar, async action and component callback | Each command addresses originating `NavigatorId`; no global-controller fallback |
| NAV-T05 | Back gesture starts, crosses partial distance, cancels; repeat and complete | Cancel preserves stack/state; commit pops exactly once; accessibility/back targets remain usable |
| NAV-T06 | Hot reload adds/removes unrelated route, changes screen body | Compatible active entry IDs and state survive; no wholesale nav controller recreation |
| NAV-T07 | Hot reload removes active route or changes required parameter type | Explicit migration diagnostic; safe pop/reset only affected entries, no empty destination or crash |
| NAV-T08 | Deep-link to `Details(id=7)` on cold launch and warm process from depth 3 | Same documented reset/push/parent-chain policy on both native targets and Dev hosts |
| NAV-T09 | Re-deliver same deep link on Android via new intent; iOS scene open URL | No accidental duplicate route from one delivery event; separate user opens follow policy |
| NAV-T10 | Process death / scene discard, restore v2 graph; then code update | Valid entry/selected tab restored; incompatible old schema falls back safely |
| NAV-T11 | Enter split detail, rotate compact↔wide, select another item, Back | Selection and back semantics stable across size classes; no leaked nested stack |
| NAV-T12 | Modal presented over nested route; system back, app Back, background tap | Modal precedence is explicit and matches native behavior; underlying stack preserved |
| NAV-T13 | Rapid double-tap and simultaneous async push/back | Deterministic serialized command sequence; no duplicate committed callback/pops |
| NAV-T14 | 100 pushes/pops and tab switches; enable reduced motion/RTL | No retained removed entry tasks/state; gestures respect direction/preferences; no AOT perf regression |
| NAV-T15 | Nested navigation inside custom `showNavigationBar: false` shell | Header/safe-area, hit targets and gesture behavior correct; no phantom native bar |
| NAV-T16 | Invalid/unknown URL route, malformed number, percent encoding, disallowed host | Typed rejection, no navigation mutation or crash; Dev and AOT failure policy matches |

For scenarios that require new functionality absent from v1, test the **v2 contract** rather than falsely labelling old behavior a v1 parity failure. For existing behaviors, first preserve a v1 regression fixture, then add the v2 expected-output fixture.

### 9.6 Navigation rollout, dependencies and milestones

| Navigation PR | Prerequisites | Deliverable | Merge gate |
|---|---|---|---|
| **N1** | Existing PR 04/05 optionally parallel | RFC, minimal reproductions, snapshot schema | Repro and agreed contract |
| **N2** | N1, original PR 04 | Stable graph/commands/reducer and IR semantics | Compiler/unit/property tests |
| **N3** | N2, original PR 01/02/08 | Per-entry Dev state/lifecycle | Same-screen duplicate push tests |
| **N4** | N2, N3 | Scoped context/dispatcher | Nested stack and tab command traces |
| **N5** | N2–N4 | Swift native AOT/Dev adapter | iOS device parity + performance |
| **N6** | N2–N4 | Kotlin native AOT/Dev adapter | Android device parity + performance |
| **N7** | N5, N6, original PR 11 | Deep links, restoration, tabs, split and transitions | Cold/warm/restoration device tests |
| **N8** | N2, N5, N6 | Optional public syntax, compatibility compiler, docs | Existing `.nx` compatibility tests |
| **N9** | N1–N8, original PR 06/12/14 | Enforced CI, release/migration/perf gates | Zero P0/P1 unresolved parity failures |

**Suggested sequencing:** N1 and test baselining can begin alongside the existing early parity correctness PRs. N2 is the critical contract boundary; N3/N4 follow it, while platform leads then work N5/N6 in parallel. N7/N8 can overlap once adapters stabilize. N9 is a release blocker, not a cleanup ticket. Avoid setting a fixed completion date without reproduced failing-case inventory and device benchmark data.

**Acceptance definition for “navigation rewritten”:** No path uses ambiguous global stack selection; state is owned by `EntryId`; supported push/pop/replace/pop-to/reset/tab/deep-link commands have one normative state-transition contract; current native platform navigation remains in AOT; compatibility syntax is covered; semantic and visual same-platform AOT/Dev test suites and native performance gates pass. Nothing about this requires forcing iOS and Android to have the same OS-native animations or visual chrome.

---

## 10. Failure triage workflow for every new mismatch

1. **Capture exact source revision**, target, mode, host build/capability fingerprint, device, OS, appearance and event sequence.
2. **Reproduce in cold-start AOT vs cold-start Dev**. Then test **full Dev module vs incremental patch**. This immediately separates compiler/rendering bugs from patch/state bugs.
3. **Compare semantic compile outcome and normalized IR.** If different, fix semantic lowering or caching first; don't patch the UI renderer.
4. **Compare native argument plans.** If different, fix the adapter/default contract and add the property conformance case.
5. **Compare state/effect/event traces.** If different, fix lifecycle, expression/action execution or identity.
6. **Compare native measured geometry and accessibility** at the same presented revision. If different with equal plans, inspect shared native primitive, OS environment and modifier/layout order.
7. **Compare unmasked and justified-masked screenshots.** If semantics/geometry match but raster differs, inspect dynamic type, font resource, OS widget style and animation phase.
8. **Fix smallest shared responsible layer, write a regression, attach before/after artifacts**, and rerun AOT performance benchmarks if native backend/generated output changes.
9. **Update source fixture** `tests/fixtures/component-parity-status.json` where applicable, regenerate `PARITY.md`, and keep human approval separate.
10. **Never approve unexplained non-equivalence** by enlarging an image mask, raising a tolerance, or adding a one-screen positional hack.

Proposed CLI UX (not present today):

```sh
# Run on an environment with the actual Nexa binary + Apple/Android toolchains.
nexa parity capture --target ios --project ./MyApp --case navigation-push
nexa parity compare --aot ./captures/ios/aot --dev ./captures/ios/dev
nexa parity diagnose --case navigation-push --first-divergence
```

Implement this CLI wrapper only *after* the capture and test-kit data formats are stable. Initially extend `tests/hot-reload-aot-parity.sh` instead of building a new command prematurely.

---

## 11. Release definition of done

A Nexa release can claim **strong AOT/DevRuntime parity** only when:

- [ ] **No mode-only language acceptance:** the exact same target/source is accepted or rejected by AOT and DevRuntime (F-03).
- [ ] **No unsafe patch publication:** Android patch commits use a controlled UI thread/snapshot transaction and rejected patches leave the last-good module intact (F-01/02).
- [ ] **Same effective native component contract** for every supported IR variant/property in both modes of each platform (F-10).
- [ ] **State/effect/lifecycle identity is normative, documented and tested**, including keyed/unkeyed lists, navigation instances and hot-reload migration (F-06).
- [ ] **Navigation v2 architecture is implemented and parity-gated** (NAV-01–10, N1–N9): entry-owned screen state, explicitly scoped navigator, canonical typed commands, one back path per platform, cold/warm deep links, graph-compatible hot reload, tabs/restoration, legacy syntax adapter, device-tested UX and native AOT performance. A generic navigation name-check or four-screen smoke test does not satisfy this gate.
- [ ] **No silent unsupported native action/property/plugin behavior** in parity tests (F-12).
- [ ] **Automated value-level and device-level conformance** exists; source-name checks are only smoke coverage and mutation tests verify test effectiveness (F-05/08).
- [ ] **Visual captures correspond to a presented revision** and app-owned masked regions are tested independently (F-07/08).
- [ ] **Host resources/capabilities are verified**, and a rebuild-required source change cannot be reported as a successful IR hot reload (F-09).
- [ ] **Fresh vs cached, patch vs full** differential tests pass for realistic edit sequences (F-11).
- [ ] **AOT release output remains direct native code** with no interpreter, reflection/boxed hot paths and no unapproved startup, memory, binary or frame-time regression.
- [ ] **iOS and Android parity gates are required by CI**, all repeatable failures are blocked or explicitly fixed, and source/device/test provenance is retained.

**“Forever” is enforced by constraints and gates, not an untestable promise that no device/OS update can ever introduce a difference.** It is achievable as a policy: each supported feature has one normative contract; every target/mode must conform; source changes that break the contract fail tests; OS/framework shifts trigger renewed device validation. Any remaining unknown case is tracked as **not yet verified**, never declared identical by assumption.

---

## 12. Recommended start — first eight engineering actions

1. **Fix Android patch main-thread publication and duplicate translation updates** (`NexaDevProtocol.kt`) and add an immediate concurrency smoke case.
2. **Add candidate/commit rollback to Android patch application** (`NexaDevState.kt`), with invalid-operation and conflicting-path tests.
3. **Remove dev-only nullable generic plugin-read permissiveness** or implement correct AOT typed decoding; gate acceptance parity for both targets.
4. **Fix the unconditional iOS IR node gate condition** and add mutation tests proving coverage tests actually fail.
5. **Add clean-vs-cached Dev IR differential tests** for realistically edited `.nx` projects before expanding the cache code.
6. **Add capture manifests and first-presented-frame synchronization** to existing simulator/emulator parity tests.
7. **Use a real failing screen to compare normalized IR → native argument plan → native geometry → pixels**, and migrate the relevant component adapter contract; do not start with a broad native renderer rewrite.
8. **In parallel, start navigation N1**: reproduce duplicate-screen state ownership, nested-stack back and hot-reload route change. Approve the route/entry/command contract before touching both native navigation generators; see §9.

**Why this order:** Items 1–4 address evidenced correctness risks with limited scope. Items 5–6 make future diagnoses trustworthy. Item 7 then attacks component mismatches while preserving Nexa's performance and existing native primitive investment.

---

## 13. Source index (re-check after file edits)

The following references correspond to the supplied ZIP at the time of audit. Approximate ranges marked `~` should be verified before editing.

| Reference | Relevant source and responsibility |
|---|---|
| `crates/nexa-compiler/src/project.rs:265-431` | Dev/AOT compilation mode, cached lowering selection |
| `crates/nexa-compiler/src/semantic/mod.rs:490-515, 2082-2104` | Nullable generic read mode argument, optimizer choice |
| `crates/nexa-compiler/src/semantic/generics.rs:386-425` | Storable generic read type constraints |
| `crates/nexa-compiler/tests/plugin_generics.rs:625-637` | Normal-mode optional generic read rejection test |
| `crates/nexa-dev-ir/src/lib.rs:29-56, 79-202, 371-438, 464-583` | Dev module, patches, identities, hint-based stripping |
| `crates/nexa-dev-protocol/src/lib.rs:25-88` | WebSocket message types and revisions |
| `crates/nexa-dev-server/src/lib.rs:195-317, 615-659` | Publish/diff and acknowledgement/recovery |
| `runtime/android/NexaDevProtocol.kt:35-52, 140-211, 238-255` | Worker socket thread and inconsistent full/patch UI thread handling |
| `runtime/android/NexaDevState.kt:139-160, 455-683, 779-839` | Mutable Compose store, state install, in-place patch, JSON Pointer |
| `runtime/ios/NexaDevProtocol.swift:130-182, 232-309` | Main-actor full/patch install, patch copy and ack |
| `runtime/ios/NexaDevState.swift:407-650` | iOS patched and full module installation/state compatibility |
| `runtime/ios/NexaDevRenderer.swift:1683-1788, 2309-2365` | Dynamic native layout and button dispatch |
| `runtime/android/NexaDevRenderer.kt:1032-1106, 1490-1530` | Dynamic native layout and button dispatch |
| `crates/nexa-backend-swift/src/generator/components/{layout,controls}.rs` | Direct SwiftUI native primitive codegen |
| `crates/nexa-backend-kotlin/src/generator/components/{layout,controls}.rs` | Direct Compose native primitive codegen |
| `crates/nexa-ir/src/lib.rs:176-189, 1072-1102, 2223-2231` | Current screen declaration, navigation nodes and actions; v2 graph/command extension points |
| `crates/nexa-compiler/src/semantic/components.rs:3030-3170`, `semantic/mod.rs:1273-1283` | Current navigation grammar/validation and top-level navigation-root restrictions |
| `crates/nexa-backend-swift/src/generator/components/navigation.rs:1-629`, `engine/types.rs:282-416` | Swift native stack route/codegen, deep-link dispatch and gesture probe |
| `crates/nexa-backend-kotlin/src/generator/components/navigation.rs:1-1009` | Compose NavHost, route patterns, entry ViewModels, transitions and deep links |
| `runtime/ios/NexaDevNavigation.swift:1-41`, `NexaDevState.swift:680-754, 930-976` | Dev route token/parameter map, ambiguous stack selection and route cleanup |
| `runtime/ios/NexaDevRenderer.swift:3616-3668, 3702-4050` | Dev navigation node dispatch, overlapping swipe mechanisms, name-scoped route rendering |
| `runtime/android/NexaDevNavigation.kt:1-303`, `NexaDevState.kt:166-225, 575-585, 750-770` | Dev NavHost, scope, controller fallback and epoch-induced resets |
| `runtime/android/NexaDevActions.kt:31-80`, `NexaDevRenderer.kt:2217-2273` | Dev action/link/back dispatch and global fallback |
| `tests/deep-link-smoke.sh`, `crates/nexa-cli/tests/deep_links.rs`, `tests/hot-reload-aot-parity.sh:21-25` | Existing AOT deep-link smoke and limited navigation-only parity phases |
| `crates/nexa-cli/src/commands.rs:1223-1285, 2326-2361, 2582-2589` | Hot reload, rebuild and capability-changing files |
| `tests/compare-render-parity.py:25-52, ~75-225, 309-411` | Scroll alignment, masks, overlays, screenshot thresholds |
| `tests/hot-reload-aot-parity.sh` | Actual same-platform paired capture orchestration |
| `tests/test_render_parity_comparison.py` | Comparator unit tests (10/10 passed in this audit) |
| `crates/nexa-compiler/tests/{component_primitive_parity_gate,ir_node_parity_gate,ir_expr_parity_gate,action_parity_gate}.rs` | Source-string coverage gates, not semantic or rendering equivalence |
| `.github/workflows/ci.yml` | Existing Rust, simulator and emulator parity CI |
| `tests/fixtures/component-parity-status.json`, `PARITY.md` | Review data; human approval separate |
| `AGENTS.md`, `.agents/skills/nexa-ui-parity/SKILL.md` | Repository invariants and parity review protocol |

**End of report.**
