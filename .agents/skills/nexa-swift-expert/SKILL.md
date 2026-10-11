---
name: nexa-swift-expert
description: Use the independently authored SwiftUI and Swift Concurrency expert skills to review or change Nexa-generated Swift, SwiftUI, or Swift plugins, while preserving Nexa AOT and cross-platform parity constraints.
---

# Nexa SwiftUI and Concurrency Expert

This Nexa adapter routes native Swift work to two independently maintained expert skills:

- SwiftUI: [`upstream/swiftui-pro/SKILL.md`](upstream/swiftui-pro/SKILL.md), by Paul Hudson.
- Swift concurrency: [`upstream/swift-concurrency-pro/SKILL.md`](upstream/swift-concurrency-pro/SKILL.md), by Paul Hudson.

Read the relevant upstream skill and only the reference files it routes to. This adapter supplies
the Nexa compiler and parity context those general Swift skills do not have. The SwiftUI source
covers modern SwiftUI, including SwiftUI 6-era APIs, and view/data/navigation/performance/accessibility
patterns. The concurrency source covers Swift 6.2+ isolation, actors, structured tasks, cancellation, async
streams, diagnostics, and testing. Check the active Xcode target before applying newer APIs.

## Quick start

```text
Trace the requested UI from .nx through typed IR to SwiftUI, apply the relevant upstream guidance,
then compare the matching Compose output and report what was verified.
```

## Nexa workflow

1. Read the `.nx` source and typed IR, then trace the affected construct into generated Swift. For
   built-in components, inspect the Swift backend; for native plugin behavior, inspect its `.nxid`
   contract and Swift implementation.
2. Inspect the active Xcode/Swift toolchain, deployment target, and build settings before applying
   version-specific advice. Upstream references may describe newer Swift or iOS releases; use an
   API only when the configured SDK and target support it.
3. Distinguish release AOT output from debug-only `NexaDevRuntime` behavior. Do not claim runtime
   behavior from generated source alone.
4. For every shared UI change, trace the same `.nx`/IR behavior through Compose and compare the
   rendered layout, interaction, sizing, spacing, and colors. Treat platform-only behavior as an
   explicit exception with a concrete native reason.

## Nexa invariants

- Keep release output directly typed in SwiftUI. Do not add `AnyView` to generated view trees or
  virtualized rows; preserve the generic content slots used by Nexa fast lists.
- Keep generated state at the narrowest owning scope and preserve native object identity across
  updates. Cancel tasks and detach callbacks at the lifecycle boundary that owns them.
- Apply Swift 6 concurrency guidance at task and plugin boundaries: respect actor isolation,
  cancellation, `Sendable`, and reentrancy. Do not silence a diagnostic with unchecked annotations
  without proving synchronization and ownership.
- Fix generated behavior in the `.nx` source, typed IR, or Swift backend as appropriate. Do not edit
  transient generated files to mask a compiler defect.
- Use the upstream review format, but report only defects supported by the source, diagnostics, or
  a runtime check. State what was and was not verified.

## Upstream provenance

See [`UPSTREAM.md`](UPSTREAM.md) for source repositories, pinned revisions, and licenses.
