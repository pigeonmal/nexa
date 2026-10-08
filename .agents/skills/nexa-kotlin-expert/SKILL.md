---
name: nexa-kotlin-expert
description: Audit Nexa-generated Kotlin and Jetpack Compose for native correctness, lifecycle and coroutine safety, recomposition stability, state boxing, and performance. Use when reviewing or optimizing Android output or Kotlin plugin implementations.
---

# Nexa Kotlin Expert

Audit generated Android code against its `.nx` source and typed IR. Report concrete defects and
measurable costs; do not turn stylistic preferences into findings.

## Workflow

1. Identify the app source, configured plugins, target ABI, and build variant. Read
   [`docs/syntax-audit.md`](../../../docs/syntax-audit.md) and the plugin's `native.nxid` first.
2. Find generated Kotlin and Gradle files under the selected build directory. Trace source constructs
   through state declarations, composables, callbacks, and plugin bindings. Separate release AOT
   output from `NexaDevRuntime` — the latter is debug-only and is not release overhead.
3. Check compiler and Gradle logs for Kotlin, Compose, dependency, and opt-in diagnostics. When a
   native build is available, run the narrow Android build needed to verify the finding. **Do not
   claim runtime behavior from source inspection alone.**
4. Report by severity: file and line, the triggering `.nx` construct, the native consequence, and a
   focused correction. State which paths you inspected and which you could not verify.

## Invariants

- Keep generated UI statically typed and close to the native Compose tree. Prefer direct modifiers
  on the owning layout or control over wrapper composables and dynamic registries.
- Keep state at the narrowest owning scope. Use primitive-specialized holders — `mutableIntStateOf`,
  `mutableLongStateOf`, `mutableDoubleStateOf` — for primitive state; never `mutableStateOf<Any>` or a
  boxed primitive in a hot path.
- Make stability claims accurate. Use immutable/stable data only when its complete reachable state
  is immutable. Do not add `@Stable` or `@Immutable` solely to force skipping; keep composable
  parameters and callbacks stable where practical.
- Keep `FastList` lazy and keyed: stable scalar keys, direct item access, work proportional to
  visible rows. No second list of row models, no high-frequency scroll reads in composition, no
  action dispatch per pixel.
- Treat modifier and layout order as behavior. Transforms, clipping, padding, backgrounds, and hit
  targets must preserve the requested drawing and input semantics without adding measurement or
  layout nodes.
- Keep the main thread free of blocking I/O and CPU work. Check `suspend` call sites, dispatcher
  selection, cancellation, and structured coroutine ownership; long-lived work must be tied to an
  explicit app or screen lifecycle.
- Native plugin class instances must retain identity across recompositions and be disposed exactly
  once at their owning boundary. Event subscriptions must be rebound or removed when their source
  scope changes.
- Preserve typed plugin errors and declared value shapes. No reflection, no unchecked casts at
  app-facing boundaries, no broad `Any` containers where the `.nxid` contract has a static type.
- Verify feature-gated Gradle dependencies, resources, and runtime helpers. A capability absent
  from optimized IR must not add a dependency to the generated app.
- Treat iOS parity as the default for every shared component change: compare the same `.nx` and IR
  path in SwiftUI, then align Compose colors, metrics, spacing, sizing, and interaction behavior.
  Record any platform-only behavior as an intentional exception with a specific native reason.

## Compose phase and measurement

- Read a value in the `draw` phase when only drawing needs it, `layout` when placement does. Do not
  move frequently changing values into composition without evidence.
- Use `derivedStateOf` only when it removes meaningful downstream recomposition.
- Use `snapshotFlow` with `distinctUntilChanged()` when observing scroll state from effects. Do not
  allocate per-frame action payloads or recompose the whole screen for sub-pixel movement.
- Distinguish compiler stability reports from runtime evidence. For a performance claim, measure
  recompositions, allocations, retained objects, CPU time, binary size, or frame timing on the
  target build. See the `nexa-performance` skill for the evidence hierarchy.
