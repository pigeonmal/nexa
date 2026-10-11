---
name: nexa-kotlin-expert
description: Use independent Kotlin and Compose expert skills to review or change Nexa-generated Kotlin and Jetpack Compose, with Nexa AOT, concurrency, lifecycle, and iOS parity constraints.
---

# Nexa Kotlin and Compose Expert

Use Ivan Morgillo's independently maintained Compose Agent as the primary authoring and review
guide: [`upstream/compose-agent/SKILL.md`](upstream/compose-agent/SKILL.md). It includes dedicated
Kotlin, coroutine/Flow, lifecycle, Compose effect, and performance references. Load only the topics
relevant to the change.

For deeper Compose UI, design-to-code, or runtime implementation questions, consult Adit Lal's
[`upstream/compose-expert/SKILL.md`](upstream/compose-expert/SKILL.md) and its relevant reference.
It includes AndroidX and Compose Multiplatform source receipts. Do not load both full workflows for
every task: Compose Agent is the primary guide; Compose Expert supplies targeted second-source and
implementation evidence.

## Quick start

```text
Trace the requested UI from .nx through typed IR to Compose, apply the relevant upstream guidance,
then compare the matching SwiftUI output and report what was verified.
```

## Nexa workflow

1. Read the `.nx` source and typed IR, then trace the affected construct into generated Kotlin. For
   built-in components, inspect the Kotlin backend; for native plugin behavior, inspect its `.nxid`
   contract and Kotlin implementation.
2. Inspect the generated Gradle configuration and actual Kotlin, Android Gradle Plugin, Compose,
   and SDK versions. Treat the vendored source snapshots as explanatory evidence, not as the
   project's dependency versions; verify newer or version-sensitive API claims against the active
   build and corresponding official source.
3. Distinguish release AOT output from debug-only `NexaDevRuntime` behavior. Do not claim runtime
   behavior from generated source alone.
4. For every shared UI change, trace the same `.nx`/IR behavior through SwiftUI and compare the
   rendered layout, interaction, sizing, spacing, and colors. Treat platform-only behavior as an
   explicit exception with a concrete native reason.

## Nexa invariants

- Keep generated UI statically typed and close to the native Compose tree. Keep `FastList` lazy and
  keyed, with work proportional to visible rows.
- Use primitive-specialized state holders for primitive state. Add stability annotations only when
  the complete reachable state actually meets their contract; do not force compiler skipping by
  assertion.
- Keep main-thread work non-blocking. Review coroutine ownership, lifecycle, cancellation, and
  dispatcher selection; tie long-lived work to an explicit app or screen lifetime.
- Preserve native plugin identity across recomposition and dispose it once at its owning boundary.
  Remove or replace subscriptions when their source scope changes.
- Fix generated behavior in the `.nx` source, typed IR, or Kotlin backend as appropriate. Do not
  edit transient generated files to mask a compiler defect.
- Nexa output is not a conventional hand-authored Android app. Apply upstream Compose advice about
  ViewModels, Hilt, Material 3, resources, navigation, scaffolding, or component architecture only
  when those APIs and structures exist in the generated host or are part of the requested change.
  Treat the upstream optional component-clarification prompt as a design aid; do not stop to ask it
  when the request and existing Nexa component contract already make the implementation clear.
- Use the upstream review guidance, but report only defects supported by source, compiler/build
  diagnostics, or a runtime check. State what was and was not verified.

## Upstream provenance

See [`UPSTREAM.md`](UPSTREAM.md) for the source repository, pinned revision, and licenses.
