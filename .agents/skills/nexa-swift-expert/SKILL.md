---
name: nexa-swift-expert
description: Audit Nexa-generated Swift and SwiftUI for native correctness, lifecycle safety, Swift concurrency, memory ownership, and performance. Use when reviewing or optimizing iOS output or Swift plugin implementations.
---

# Nexa Swift Expert

Audit the generated iOS implementation against its `.nx` source and typed IR. Report concrete defects and measurable costs; do not turn stylistic preferences into findings.

## Audit workflow

1. Identify the app source, configured plugins, target, and build configuration. Read the relevant Nexa component/API documentation and plugin contract before judging generated code.
2. Find generated Swift under the selected build directory. Trace each important source construct through generated declarations, view bodies, callbacks, and plugin bindings. Separate release AOT output from `NexaDevRuntime`: the latter is debug-only and may use interpreter infrastructure that must not leak into release output.
3. Check the generated project and compiler logs for Swift diagnostics. When a native build is available, run the narrow iOS simulator or device build needed to verify the finding. Do not claim runtime behavior from source inspection alone.
4. Report findings by severity with file and line, the triggering `.nx` construct, the native consequence, and a focused correction. State which requested paths you inspected and any paths you could not verify.

## Nexa-specific invariants

- Preserve direct, statically typed SwiftUI output. Do not add `AnyView` to generated release view trees or virtualized list cells. Treat its use inside the debug DevRuntime as a separate concern.
- Keep state at the narrowest app, screen, component, or row scope that owns it. Native class instances must keep stable identity across SwiftUI recomposition and be disposed at the lifetime boundary defined by the compiler.
- Keep `FastList` work proportional to visible rows. Verify stable scalar identities, direct row construction, and no eager materialization of the full collection or per-scroll callback churn.
- Keep view builders and plugin adapters statically typed. Avoid reflection, string-based member lookup, broad type erasure, and intermediate wrapper views when a direct SwiftUI modifier or generic view can express the behavior.
- Keep UI work non-blocking. Long-running I/O and CPU work must use the declared async path; check cancellation and actor isolation at lifecycle boundaries.
- Review Swift 6 concurrency at every task or plugin boundary. Values crossing actors must satisfy the required `Sendable` rules or remain actor-isolated. Do not silence diagnostics with unchecked annotations without proving ownership and synchronization.
- Inspect closure ownership. UI callbacks, subscriptions, tasks, and plugin event handlers must not retain screens or owners indefinitely; teardown must cancel work and detach callbacks where the contract requires it.
- Preserve typed native errors and explicit recovery. Do not flatten declared plugin errors into strings or silently swallow failures.
- Check feature-gated imports, frameworks, resources, and helper code. A feature absent from the optimized IR should not add native dependencies or runtime code to the app.

## Performance evidence

Prefer generated-source inspection and compiler/build logs for ordinary audits. For a performance claim, identify the hot path and compare an observable quantity such as allocations, retained objects, generated cell count, CPU time, binary size, or frame timing. Do not infer a speedup from fewer source lines or from native-looking syntax alone. Use `nexa audit` and native profiling when the question requires binary or runtime measurements.
