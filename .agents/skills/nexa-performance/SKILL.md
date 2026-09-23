---
name: nexa-performance
description: "Use when analyzing, profiling, or optimizing Nexa runtime performance, memory allocations, binary sizes, SwiftUI/Compose layout trees, or C++ JNI/bridging overhead."
---

# Nexa Performance & Zero-Cost Architecture

This skill guides the design, profiling, and optimization of Nexa applications and compiler backend targets to achieve maximum execution speed and minimum memory/binary overhead.

## Performance Guarantees

Nexa is engineered to generate native code that meets or beats hand-written Swift and Kotlin. Every compiler pass and backend generator must preserve these guarantees:

### 1. Zero Type Erasure in UI Hierarchies
- **Swift**: Never use `AnyView`. Specialize view builders and container components (such as `NexaFastList` and `NexaFastSectionedList`) with generic parameters (e.g. `HeaderContent: View = EmptyView`).
- **Compose**: Avoid wrapper composables that introduce intermediate layout nodes. Ensure modifiers are chained directly on layout primitives.

### 2. Allocation-Free State & Mutability
- Avoid object boxing for primitives. In Kotlin, use `mutableDoubleStateOf` (not `mutableStateOf<Double>`), `mutableIntStateOf`, and `mutableLongStateOf`.
- Keep state variables local to the screen or component requiring recomposition to minimize invalidation scopes.

### 3. Recomposition & Diffing Churn Prevention
- When observing scroll states or layout metrics in Jetpack Compose, always filter with `.distinctUntilChanged()` to prevent allocations and recompositions during sub-pixel movements.
- In virtualized lists, provide stable, unique keys (`key: item.id`) so platform diffing engines (SwiftUI `ForEach(..., id: \.self)` / Compose `items(..., key = { ... })`) can reuse view instances without churn.

### 4. Zero-Copy Plugin & C++ Interop
- In plugin IDL contracts, prefer contiguous primitives, structs, and binary buffer views over nested dynamic collections.
- Ensure all native C++ handles on Android are bound to `AutoCloseable` with `finalize()` safety nets to prevent native heap leakage.
- On iOS, utilize Swift 6 / C++ direct interop with `SWIFT_SHARED_REFERENCE` for zero-overhead pointer passing without Objective-C boxing.

---

## Profiling & Benchmarking Workflow

When auditing performance:
1. **Source Inspection**: Run `nexa audit` to inspect generated capabilities, native code volume, and dead code removal.
2. **Binary Measurement**: Build release packages with `nexa build ios --release` and `nexa build android --release`. Measure IPA/APK size, stripped binary symbols, and dynamic library overhead.
3. **Allocation Profiling**:
   - iOS: Profile in Xcode Instruments using **Allocations** and **Time Profiler**. Inspect memory spikes during list scrolling.
   - Android: Profile in Android Studio using **Memory Profiler** and **Layout Inspector** to verify recomposition counts and zero Double boxing.
