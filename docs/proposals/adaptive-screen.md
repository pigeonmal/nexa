# Nexa — Composition-First Adaptive Layout, Safe Areas & Native Scroll

## Role and mission

You are a principal compiler engineer, native UI framework architect, and developer-experience designer with expertise in SwiftUI, Jetpack Compose, declarative UI, window insets, scroll coordination, and performance-sensitive code generation.

Repository: https://github.com/pigeonmal/nexa

Design and, **after architecture approval**, implement a coherent set of capabilities for adaptive mobile layouts in Nexa. The developer should describe *what the interface does*, not calculate operating-system geometry or construct scroll-offset state machines.

**Central principle: composition first, behavior second.** Developers must be free to combine ordinary views using the layout primitives Nexa already provides. Special semantic screen regions must not become a prerequisite for displaying an image, gradient, text, button, floating control, or scrollable list.

This document is a product and engineering **goal specification**, not a pre-approved implementation plan or a proposed language grammar. Discover Nexa's actual language syntax, existing components, compiler architecture, and backend constraints before proposing any public API. Do not fabricate `.nx` examples.

## 1. First inspect the existing repository

Read the relevant source code, docs, tests, and architectural guidelines. Verify, rather than assume:

- Actual `.nx` syntax, parsing, AST, type checking, typed IR, component metadata, and diagnostics.
- Existing layout and composition constructs, including `Column`, `Row`, and `Stack` if they exist in the current checkout.
- Existing screen hosting, navigation, status/system bar handling, window configuration, and view ownership.
- Native SwiftUI and Jetpack Compose generation paths and supported platform versions.
- Scrolling ownership of existing `FastList`, `Form`, `KeyboardAware`, and any other relevant components; whether a general scroll primitive already exists.
- Existing sizing, padding, scroll references, geometry, keyboard, and layout APIs.
- Compatibility guarantees, code-generation golden tests, native tests, profiling or benchmarks.
- Uncommitted changes. Preserve them; do not overwrite or silently reformat unrelated work.

Produce a concise, evidence-based inventory: what exists, what can be reused, missing capabilities, relevant file locations, major risks, and questions that must be resolved. Distinguish observations from design proposals.

## 2. Product philosophy and acceptance criteria

The goal is to make common layouts effortless and complex layouts possible without structural restrictions or hacks.

- **Unrestricted composition:** The existing layout system is the default way to arrange views. The user should not have to classify every child into predefined roles such as Background, Content, Bottom, or Overlay.
- **Optional semantic coordination:** If the application needs coordinated header, scroll, persistent control, or keyboard behavior, provide a concise, discoverable way to express that relationship. It must not monopolize ordinary layout.
- **Edge-to-edge presentation by default:** Visuals may draw behind system bars. Do not confuse drawing edge-to-edge with leaving interactive content obscured.
- **Explicit, reusable control:** Opt into safe positioning, draw-under behavior, scroll-content accommodation, sticky/collapsing behavior, or keyboard avoidance where needed.
- **Sensible defaults, no inscrutable magic:** Automatically infer relationships only when unambiguous. Otherwise demand a small, typed, explicit association and give a useful compiler diagnostic.
- **Native implementation:** Preserve the strengths and platform conventions of SwiftUI and Jetpack Compose without adding a new runtime interpreter or layout engine.
- **Small coherent API:** Prefer orthogonal capabilities that combine well rather than a large collection of screen-specific components and special-case parameters.

A successful design lets developers build (a) a simple image with a gradient/backdrop and button, (b) a safe settings screen, (c) an edge-to-edge feed, (d) a collapsing title, (e) a translucent sticky header, (f) a chat with keyboard-aware composer, and (g) a full-screen viewer with safe controls—all with a consistent mental model and without manually calculating system-bar sizes.

## 3. Explicit non-goals and prohibitions

Do **not**:

- Require a new top-level screen container for normal composition.
- Require fixed named slots such as Background, Header, Content, Bottom, and Overlay, or a mandatory Content region.
- Treat visual stacking, gradients, backdrops, and ordinary floating buttons as special semantic screen regions when existing layout primitives can express them.
- Turn `Column`, `Row`, `Stack`, and every other container into a large collection of safe-area and system-bar knobs.
- Introduce a competing layout engine, JavaScript bridge, runtime UI interpreter, or per-item inset adjustment system.
- Automatically nest another vertical scroll container around components that already own native scrolling.
- Mix fixed design padding with dynamic window insets under the same undocumented meaning.
- Declare scroll ownership ambiguously or silently pick an arbitrary vertical scroll.
- Make all platform behavior pixel-identical at the expense of native ergonomics, accessibility, or performance.
- Add unverified Nexa syntax, fabricated APIs, or placeholder implementations disguised as finished features.

An optional higher-level coordination abstraction is allowed **only** if it demonstrably simplifies common multi-component behavior and stays compatible with unrestricted composition.

## 4. Separate composition from behavior

Define the architecture in two layers:

**Composition:** Existing Nexa layout primitives determine parent-child relationships, visual stacking, alignment, sizing, padding, and placement. An image, an obscuring gradient, and a button should remain an ordinary composable layout, not a prescribed screen schema.

**Behavior:** Small opt-in capabilities determine how composed views interact with system insets, scrolling, pinned elements, translucent overlaps, keyboard movement, and other dynamic native constraints. These capabilities may attach to existing primitives or be expressed by dedicated reusable scopes, as dictated by verified Nexa conventions.

Avoid inventing a new public component for every visual role. Explain why each new concept is essential, why existing primitives are insufficient, and how it composes with the others.

## 5. Edge-to-edge, safe areas, and window insets

Default to edge-to-edge drawing, while permitting safe interactive layout.

The design must distinguish:

- Physical window bounds versus safe drawing and interaction areas.
- Status bar, navigation bar, display cutout, and gesture insets.
- Viewport extent versus scroll content placement.
- Dynamic native inset values versus fixed design spacing.
- Insets provided, applied, and consumed by ancestors or nested components.
- Keyboard/IME insets and focused-input visibility from ordinary system-bar protection.

Required abilities:

- Draw imagery or decoration beneath system bars without pushing every foreground control underneath them.
- Protect any chosen view/subtree on selected edges, independent of whether that subtree is a list.
- Combine fixed top/bottom/horizontal padding *additively* with the remaining relevant dynamic insets.
- Support nested safe-area scopes without double-application; define exact inset-consumption semantics.
- Respond to orientation, resizing, multi-window, bar-visibility changes, and device cutouts.
- Provide low-level reactive access to insets only where advanced composition actually needs it.

Define the default behavior of ordinary content carefully. If safe positioning is implicit in a host or navigation context, explain ownership, disable/override mechanisms, and compatibility with edge-to-edge. Never claim that edge-to-edge alone protects controls.

## 6. Native scrolling, virtualization, and scroll-content accommodation

Inspect scrolling components and respect their existing ownership. A virtualized list, a form that owns native scrolling, or a keyboard-aware scrolling container must not accidentally become embedded in an additional competing scroll owner.

Required cases:

- Ordinary safe scrolling.
- Scrolling with the viewport extending beneath one or more bars or overlapping controls.
- First and last rows fully reachable at scroll boundaries even when intervening rows travel behind translucent UI.
- Fixed padding inside scrolling content, distinct from dynamic inset compensation.
- Full-bleed visual material with independently protected interactive elements.
- Nested and horizontal scrolling, multiple independent vertical scrolls, and tabbed/paged layouts.
- Empty, short, large, and dynamically changing lists.

Define which component owns the viewport, who owns scrolling, and where effective content insets are applied. Preserve stable item identity, virtualization, native gestures, and native scroll physics. Do not use fake list items as inset spacers or observe geometry for every row.

If a developer requests viewport extension behind a bar, make endpoint accessibility automatic **where the relationship and occluding region are known**. For arbitrary custom overlays, give an explicit way to declare or account for the obstruction; do not pretend the compiler can infer geometry it does not own.

## 7. Adaptive headers and scroll-linked behavior

Support fixed, expanded/collapsible, hide/reveal-on-scroll, transparent/translucent, and custom headers without prescribing a fixed screen skeleton. These should be composable behaviors of an existing or suitably introduced header/layout abstraction.

For standard effects:

- No application-level scroll-offset state machine should be required.
- Use native scroll state and animation mechanisms with minimal recomposition/layout work.
- Define activation thresholds, hysteresis, boundary behavior, restoration, nested-scroll behavior, and accessibility/reduced-motion behavior.
- Make scroll association deterministic: infer only when exactly one eligible primary scroll is unambiguous; otherwise use a typed explicit association.
- Do not let a nested horizontal carousel accidentally control a vertical collapsing title.
- Allow future tabbed navigation in which the active scroll controls the header while inactive tabs retain positions.

A collapsing header is an **interaction between composed elements**, not proof that every view on the screen belongs in a fixed role slot.

## 8. Persistent controls, overlays, and keyboard behavior

An ordinary overlay, gradient, or floating button should be possible through normal layout composition. Introduce extra semantics only when an element needs system-aware or scroll-aware behavior.

Support:

- Controls pinned to safe top/bottom locations.
- Controls that visually overlap scrollable content without permanently obscuring its endpoints.
- Translucent/blurred/visual-effect surfaces where supported, with platform-appropriate fallbacks.
- Chat-style input controls and persistent bottom actions.
- Keyboard/IME appearance, dismissal, resizing, interactive movement, hardware/floating keyboards, and focused-input visibility.
- Correct scroll anchoring in chat and other bottom-anchored experiences.
- Safe hit testing, z-order, accessibility, and navigation/modal interactions.

Do not force a persistent control to use a mandatory Bottom slot. Define whether the relationship is expressed by an existing layout capability, a reusable anchor/inset behavior, or a small optional coordination mechanism after inspecting the repo.

Not all bottom controls should move with the keyboard. Defaults and overrides must be explicit and predictable.

## 9. Advanced immersive and multi-pane layouts

Verify that arbitrary compositions remain viable for photo/video viewers, maps, galleries, dashboards, split/multi-pane screens, overlays, translucent headers, nested tabs, and custom navigation.

Allow decorative visuals to cover the physical display while separate controls respect the safe region. Do not make advanced cases require hardcoded pixel offsets or brittle platform-specific hacks. Conversely, do not over-generalize the automatic coordination layer when the developer already has a clear composed layout.

## 10. Public API research and design process

**No illustrative or invented `.nx` syntax in the initial proposal.** First derive syntax and naming from actual repository definitions and examples.

For each proposed capability, provide:

1. Developer need and ordinary use case.
2. Existing primitive(s) reused.
3. New public surface, if any, with a rationale.
4. Default and override semantics.
5. Composition behavior and conflict resolution.
6. Typed compile-time validation or diagnostics.
7. Native backend implementation and compatibility implications.
8. A comparison against a simpler alternative.

Compare at least two realistic approaches: (A) composition-first with independent capabilities and optional coordination, and (B) a higher-level region-coordinating container. Choose the smallest solution consistent with the product goals; do not automatically endorse the older fixed-slot design.

Only once the repo has been inspected may you provide code examples, and those examples must use **verified Nexa syntax** or be clearly labeled as proposed changes to that syntax.

## 11. Compiler and native backend integration

Use Nexa's existing parser, semantic analysis, component metadata, typed IR, code generation, and diagnostics where appropriate.

Separate compile-time policy/relationship resolution from dynamic values supplied at runtime by the OS. Preserve correct scoping, ownership, and consumption of insets. Make incompatible feature combinations explainable at compile time wherever possible.

**SwiftUI:** Investigate safe area and inset APIs, scroll content margins/adjustments, pinned/collapsing header approaches, native scrolling observation, keyboard handling, view specialization, and target-version availability. Avoid pervasive geometry readers, unnecessary wrappers, and hot-path type erasure.

**Jetpack Compose:** Investigate window insets and consumption, edge-to-edge host setup, IME handling, lazy list content padding, nested scroll, pinned/collapsing headers, state and recomposition boundaries, and supported Compose/Android versions. Avoid broad recomposition or per-item inset work.

Define one shared observable contract while allowing distinct idiomatic native implementations. Clearly document unavoidable platform differences.

## 12. Compatibility, migration, and developer experience

Preserve existing Nexa applications and scroll/component contracts wherever feasible. Assess conflicts with existing `KeyboardAware`, `Form`, `FastList`, `StatusBar`, navigation, and sizing behavior based on current source—not their names alone.

For any breaking change, provide a migration strategy, diagnostics, and focused compatibility tests.

Document behavior in terms of outcomes and practical decisions (safe content, scrolling under a bar, a sticky control, a collapsing title), not just native jargon like WindowInsets or safeAreaInset. Developers should be able to choose behavior without knowing underlying OS measurements.

## 13. Verification matrix

Create meaningful compiler, codegen, platform, and integration tests for:

- Ordinary visual composition: image + gradient/backdrop + button, without special regions.
- Safe text/control placement over an edge-to-edge background.
- Fixed design padding added to dynamic safe insets.
- Nested safe scopes and inset consumption.
- Status/navigation bars, notches/cutouts, gesture navigation, landscape and resizing.
- Virtualized lists with viewport extension, correct first/last item visibility, short/empty/large collections, and stable identities.
- Nested vertical/horizontal scrolls, multiple candidate scroll sources, and deterministic typed association.
- Fixed, collapsing, and hide/reveal headers including accessibility and reduced motion.
- Translucent overlap and scroll endpoint compensation.
- Persistent bottom controls, chat scroll anchoring, IME/focus behavior, and keyboard variants.
- Overlays, hit testing, modal/nav stacking, multi-pane and immersive layouts.
- Backward compatibility and generated native code parity.

Run existing test suites; verify the generated SwiftUI and Compose outputs on available targets. State clearly which behaviors were not run or cannot be verified in the environment.

## 14. Performance requirements

No JavaScript bridge, runtime UI interpretation, secondary layout engine, per-row safe area adjustment, artificial list spacer items, or gratuitous wrapper hierarchy.

Preserve virtualization, stable identity, native gesture handling, efficient scroll performance, and local scroll-linked updates. Avoid excessive per-frame allocation, whole-screen recomposition, geometry-measurement loops, and unnecessary layout passes.

Benchmark representative ordinary and advanced screens before/after where tooling permits. Report measured results with target/device configuration and clearly label unmeasured risks.

## 15. Work stages and approval gates

**Stage 1 — Repository audit.** Map the current implementation, uncommitted changes, constraints, and reusable primitives.

**Stage 2 — Architecture decision (approval required).** Present composition-first and region-oriented alternatives, a recommended minimal public API based on verified Nexa syntax, detailed semantics, trade-offs, and design risks. **Do not modify implementation code yet.**

**Stage 3 — Verified implementation plan (approval required).** Save a decision-complete plan with file-level workstreams, dependencies, tests, migration approach, acceptance criteria, and independently verifiable milestones.

**Stage 4 — Core insets and composition.** Implement edge-to-edge, safe positioning, native dynamic inset consumption, and composition compatibility.

**Stage 5 — Scroll integration.** Integrate scroll viewport/content semantics with existing scroll owners and virtualized lists.

**Stage 6 — Adaptive scroll interactions.** Implement scroll association, collapsing/hideable/pinned headers, and overlap coordination.

**Stage 7 — Persistent controls and keyboard.** Implement sticky or anchored controls, keyboard handling, scroll endpoint adjustments, and chat behavior.

**Stage 8 — Advanced composition, hardening, and docs.** Validate immersive and multi-pane compositions, tests, performance, compatibility, migration notes, and realistic verified `.nx` examples.

Deliver in reviewable increments. Do not proceed to implementation before approval of the architecture and plan. Do not claim success for unimplemented stubs or unverified platform behavior.

## 16. Required deliverables

- Repository inventory with relevant file references and existing behavior.
- Comparative architecture analysis and justified decision.
- Minimal, coherent public API based on actual Nexa conventions.
- Formal semantics for insets, scrolling, ownership, coordination, and overrides.
- Typed compiler/backend implementation changes.
- Tests and native verification results.
- Compatibility/migration assessment and updated documentation.
- Performance measurements or reproducible benchmark instructions.
- Clear milestone status, known limitations, and unresolved risks.

## 17. Final architecture self-review

Before approving the design, challenge it with these questions:

- Can I make a picture with a gradient and button using ordinary Nexa composition, with no named screen slots?
- Are safe areas independently applicable without turning every container into a special screen-aware primitive?
- Can content flow under a system bar while all necessary controls remain safely reachable?
- Can a list extend under a translucent control without its endpoints becoming inaccessible?
- Can an ordinary collapsing header connect to one scroll without manual event plumbing?
- What happens with two vertical scroll sources or different tabs?
- Do native scroll owners remain intact, especially `FastList`, `Form`, and `KeyboardAware`?
- Does keyboard handling avoid duplicate scroll and inset logic?
- Can advanced designs opt out of automatic coordination without fighting the framework?
- Is any new public concept redundant, inflexible, or harder to learn than the underlying capability?
- Will the generated UI remain native, accessible, predictable, and performant?

**Final objective:** a minimal, composition-first, fully native adaptive layout system. Let developers freely construct the interface with Nexa's existing primitives; provide powerful, declarative opt-in behavior for safe areas, scrolling, headers, and keyboards. Do not force developers into a fixed set of screen regions, and do not invent syntax before reading Nexa.
