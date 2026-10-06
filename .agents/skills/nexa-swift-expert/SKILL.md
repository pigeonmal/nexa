---
name: nexa-swift-expert
description: Audit Nexa-generated Swift and SwiftUI for native correctness, lifecycle safety, Swift concurrency, memory ownership, and performance. Use when reviewing or optimizing iOS output or Swift plugin implementations.
---

# Nexa Swift Expert

Audit generated iOS code against its `.nx` source and typed IR. Report concrete defects and
measurable costs; do not turn stylistic preferences into findings.

## Workflow

1. Identify the app source, configured plugins, target, and build configuration. Read
   [`docs/syntax-audit.md`](../../../docs/syntax-audit.md) and the plugin's `native.nxid` before
   judging generated code — several findings are really misunderstandings of the contract.
2. Find generated Swift under the selected build directory. Trace each important source construct
   through generated declarations, view bodies, callbacks, and plugin bindings. Separate release
   AOT output from `NexaDevRuntime`: the latter is debug-only and its interpreter infrastructure
   must not leak into release output.
3. Check compiler and build logs for Swift diagnostics. When a native build is available, run the
   narrow simulator or device build needed to verify the finding. **Do not claim runtime behavior
   from source inspection alone.**
4. Report by severity: file and line, the triggering `.nx` construct, the native consequence, and a
   focused correction. State which paths you inspected and which you could not verify.

## Invariants

- Preserve direct, statically typed SwiftUI output. No `AnyView` in generated release view trees or
  virtualized list cells; the runtime declares generic `NexaFastList<RowContent, HeaderContent>` and
  `NexaFastSectionedList<RowContent, HeaderContent>` with defaulted unused slots. Its use inside
  debug DevRuntime is a separate concern.
- Keep state at the narrowest app, screen, component, or row scope that owns it. Native class
  instances must hold stable identity across recomposition and be disposed at the lifetime boundary
  the compiler defines.
- Keep `FastList` work proportional to visible rows: stable scalar identities, direct row
  construction, no eager materialization of the collection, no per-scroll-pixel callback churn.
- Keep view builders and plugin adapters statically typed. No reflection, string-based member
  lookup, broad type erasure, or intermediate wrapper views where a direct SwiftUI modifier or
  generic view expresses the behavior.
- Keep UI work non-blocking. Long I/O and CPU work uses the declared async path; check cancellation
  and actor isolation at lifecycle boundaries.
- Review Swift 6 concurrency at every task and plugin boundary. Values crossing actors must satisfy
  `Sendable` or stay actor-isolated. Do not silence diagnostics with unchecked annotations without
  proving ownership and synchronization.
- Inspect closure ownership. Callbacks, subscriptions, tasks, and plugin event handlers must not
  retain screens or owners indefinitely; teardown must cancel work and detach callbacks.
- Preserve typed native errors and explicit recovery. Never flatten a declared plugin error into a
  string or swallow a failure.
- Verify feature-gated imports, frameworks, and resources. A capability absent from optimized IR
  must not add native dependencies or runtime code.

## Performance evidence

Ranked by strength: measured on a target build (allocations, retained objects, generated cell
count, CPU time, frame timing, binary size) > compiler output (`nexa audit`) > source inspection,
which establishes that a pattern exists but never that it is slow. Do not infer a speedup from
fewer source lines. See the `nexa-performance` skill for the full evidence hierarchy.
