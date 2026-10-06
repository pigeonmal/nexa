---
name: nexa-performance
description: "Use when analyzing, profiling, or optimizing Nexa runtime performance, memory allocations, binary sizes, SwiftUI/Compose layout trees, or C++ JNI/bridging overhead."
---

# Nexa Performance

Find real costs with evidence. Do not report a speedup that was never measured.

## Invariants that must hold

| Invariant | Why |
|---|---|
| No `AnyView` in Swift release view trees or virtualized list cells | Type erasure forces a heap allocation per layout pass. Specialize with generic parameters: the generated runtime declares `NexaFastList<RowContent: View, HeaderContent: View>` and `NexaFastSectionedList<RowContent: View, HeaderContent: View>`, with defaulted parameters where a slot is unused (`NexaFastList<_, EmptyView>`). Its use inside debug DevRuntime is a separate concern. |
| No boxed primitive state in Kotlin | `mutableStateOf(0.0)` allocates `MutableState<java.lang.Double>` and boxes on every mutation. Use `mutableIntStateOf`, `mutableLongStateOf`, `mutableDoubleStateOf`. |
| No reflection, string member lookup, or untyped containers | These defeat AOT entirely. |
| `FastList` work proportional to visible rows | Eagerly materializing the full collection or rebuilding on every scroll frame is the defect. |
| Feature-gated native output | A capability absent from optimized IR must not add a dependency, import, or runtime helper to the app. |

## What counts as evidence

Ranked by strength:

1. **Measured on a target build** — allocations, retained objects, recomposition counts, CPU time,
   frame timing, binary size.
2. **Compiler output** — `nexa audit` reachability and generated-dependency reports; generated
   source volume per feature.
3. **Source inspection** — establishes that a pattern *exists*, never that it is slow.

Never infer a speedup from fewer source lines or from native-looking syntax. When you report a
finding, state which of these three produced it.

## Workflow

1. **Locate the output.** `nexa audit App.nx --target ios|android` for capability reachability, or
   read the generated units under the build directory. Separate release AOT output from
   `NexaDevRuntime` — the latter is debug-only and its interpreter infrastructure is not release
   overhead.
2. **Find the hot path.** Name the specific construct: which `.nx` source, which generated
   function, which Compose recomposition scope.
3. **Measure it.** iOS: Instruments *Allocations* and *Time Profiler* while scrolling.
   Android: Android Studio *Memory Profiler* and *Layout Inspector* for recomposition counts.
4. **Fix and re-measure.** A fix without a second measurement is a proposal, not a result.

## Platform-specific traps

**SwiftUI.** `AnyView` around section headers, sticky headers, or rows destroys view identity
across layout passes. User enums inside `Result` must conform to `Error`; generated ones are
`String, Error`-backed so a case name crosses the boundary without runtime type information.

**Compose.** Reading `LazyListState.layoutInfo` inside `snapshotFlow` without filtering emits on
every sub-pixel offset change, allocating `LazyListItemInfo` lists and churning the GC — append
`.distinctUntilChanged()`. Modifier order is behavior, not taste: transforms, clipping, padding,
backgrounds, and hit targets change drawing and input semantics. Use `derivedStateOf` only when it
removes meaningful downstream recomposition, not as a wrapper around every read.

**C++ / JNI.** A native class wrapped as `std::unique_ptr` behind a `Long` handle leaks the C++
heap if `dispose()` is never called. Generated Kotlin wrappers implement `AutoCloseable` with a
`finalize()` safety net for exactly this reason. iOS uses Swift 6 C++ interop with
`SWIFT_SHARED_REFERENCE` to avoid Objective-C boxing.

## Reporting

For each finding: the `.nx` construct that triggers it, the generated consequence, the measured or
inspected cost, and a focused correction. State which paths you inspected and which you could not
verify.
