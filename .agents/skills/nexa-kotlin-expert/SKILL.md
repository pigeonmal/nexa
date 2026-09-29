---
name: nexa-kotlin-expert
description: Audit Nexa-generated Kotlin and Jetpack Compose for native correctness, lifecycle and coroutine safety, recomposition stability, state boxing, and performance. Use when reviewing or optimizing Android output or Kotlin plugin implementations.
---

# Nexa Kotlin Expert

Audit generated Android code against its `.nx` source and typed IR. Report concrete defects and measurable costs; do not turn stylistic preferences into findings.

## Audit workflow

1. Identify the app source, configured plugins, target ABI, and build variant. Read the relevant Nexa component/API documentation and plugin contract before judging generated code.
2. Find generated Kotlin and Gradle files under the selected build directory. Trace source constructs through state declarations, composables, callbacks, and plugin bindings. Separate release AOT output from `NexaDevRuntime`: the latter is debug-only and must not be counted as release overhead.
3. Check compiler and Gradle logs for Kotlin, Compose, dependency, and opt-in diagnostics. When a native build is available, run the narrow Android build needed to verify the finding. Do not claim runtime behavior from source inspection alone.
4. Report findings by severity with file and line, the triggering `.nx` construct, the native consequence, and a focused correction. State which requested paths you inspected and any paths you could not verify.

## Nexa-specific invariants

- Keep generated UI statically typed and close to the native Compose tree. Prefer direct modifiers on the owning layout or control over wrapper composables and dynamic registries.
- Keep state at the narrowest app, screen, component, or row scope that owns it. Use primitive-specialized state holders such as `mutableIntStateOf`, `mutableLongStateOf`, and `mutableDoubleStateOf` when the state is primitive; avoid `mutableStateOf<Any>` or boxed primitive state in hot paths.
- Make stability claims accurate. Use immutable/stable data only when its complete reachable state is immutable; do not add `@Stable` or `@Immutable` solely to force skipping. Keep composable parameters and callbacks stable when practical.
- Keep `FastList` lazy and keyed. Verify stable scalar keys, direct item access, and work proportional to visible rows. Avoid building a second list of row models, reading high-frequency scroll values in composition, or dispatching actions for every pixel.
- Treat modifier and layout order as behavior. Check that transforms, clipping, padding, backgrounds, and hit targets preserve the requested drawing and input semantics without unnecessary measurement or layout nodes.
- Keep the main thread free of blocking I/O and CPU work. Check `suspend` call sites, dispatcher selection, cancellation, and structured coroutine ownership. Long-lived work must be tied to an explicit app or screen lifecycle.
- Native plugin class instances must retain identity across recompositions and be disposed exactly once at their owning lifetime boundary. Event subscriptions must be rebound or removed when their source scope changes.
- Preserve typed plugin errors and declared value shapes. Avoid reflection, unchecked casts at app-facing boundaries, and broad `Any` containers where the `.nxid` contract has a static native type.
- Verify feature-gated Gradle dependencies, resources, and runtime helpers. A feature absent from optimized IR should not add a dependency to the generated app.

## Compose phase and measurement checks

- If a value is only needed for drawing, prefer a graphics/draw phase read when the existing API permits it. If it is needed for placement, keep it in layout; avoid moving frequently changing values into composition without evidence.
- Use `derivedStateOf` only when it reduces meaningful downstream recompositions, not as a wrapper around every state read.
- Use `snapshotFlow` with `distinctUntilChanged()` when observing scroll state from effects; do not allocate per-frame action payloads or recompose the whole screen for sub-pixel movement.
- Distinguish compiler stability reports from runtime evidence. For a performance claim, measure recompositions, allocations, retained objects, CPU time, binary size, or frame timing on the target build. Use `nexa audit` and Android profiling when the question requires binary or runtime measurements.
