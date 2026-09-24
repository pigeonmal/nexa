# Nexa Hot Reload Implementation Plan

## Goal

Add real hot reload to `nexa dev` while keeping Nexa's current production model unchanged:

```text
Release:
.nx → typed IR → SwiftUI / Compose → native AOT app

Development:
.nx → typed IR → Dev IR → running native DevRuntime
```

Do **not** use Inject, dylib injection, JVMTI, Android Live Edit, or similar native code-injection systems as the foundation.

Hot reload must be a Nexa feature, implemented at the Nexa IR/runtime level.

The DevRuntime must never be included in release builds.

## Current Nexa architecture to preserve

The existing pipeline is already well suited for this:

```text
nexa-syntax
    ↓
nexa-compiler
    ↓
nexa-ir::Module
    ├── nexa-backend-swift
    └── nexa-backend-kotlin
```

`nexa-compiler/src/project.rs` already loads the entire import graph and resolves Nexa/native plugins before semantic lowering.

`nexa-cli/src/project.rs` already generates full Xcode/Gradle projects, splits generated native code into `nexa-unit:*` units, and uses `write_if_changed`.

The existing cache also fingerprints the source/import/plugin graph.

Do not replace any of this for release builds.

## 1. Add a separate development pipeline

Add:

```text
crates/
  nexa-dev-ir/
  nexa-dev-protocol/
  nexa-dev-server/
```

And native debug runtimes:

```text
runtime/
  ios/NexaDevRuntime.swift
  android/NexaDevRuntime.kt
```

Add:

```bash
nexa dev
nexa dev --ios
nexa dev --android
```

On the first launch, `nexa dev` should:

```text
compile project normally
→ generate native dev shell
→ build Xcode / Gradle app
→ start Nexa dev server
→ launch simulator/emulator
→ connect native runtime to dev server
→ watch .nx files
```

After that, normal `.nx` changes should not require Swift/Kotlin recompilation.

## 2. Introduce stable identities before implementing diffs

The current `nexa_ir::Node`, `Action`, and `State` types have no stable identity.

`ScreenId(pub usize)` is also positional, so it is not suitable as a persistent hot-reload identity.

Create a Dev IR containing stable IDs:

```rust
struct DevNode {
    id: StableId,
    node: DevNodeKind,
}

struct DevState {
    id: StableId,
    name: String,
    ty: Type,
    initial: DevExpr,
}
```

IDs must be deterministic, never random UUIDs.

They should be derived from source identity, declaration identity and structural position, for example:

```text
app/Home/body/Column:0
app/Home/body/Column:0/Text:1
component/ProfileCard/state/expanded
screen/Details/state/item
```

Also preserve source-file/span provenance while lowering to Dev IR.

This will later allow precise diagnostics and IR diffs.

## 3. Start with full Dev IR replacement, not incremental patches

Do not build the diff engine first.

MVP:

```text
file changed
→ parse project
→ semantic/type check
→ produce typed Module
→ lower Module → DevModule
→ serialize DevModule
→ send it to running app
→ replace rendered Dev tree
```

Transport can initially be WebSocket + JSON.

Protocol:

```text
Hello
FullModule
Diagnostics
Reload
Restart
```

Once this works reliably, add:

```text
Patch
StateSync
```

and diff old/new Dev IR on the Rust side.

## 4. Build a native renderer, not a native-code injector

The iOS DevRuntime should translate `DevNode` into SwiftUI.

The Android DevRuntime should translate the same logical `DevNode` into Compose.

For example:

```text
DevNode::Text
DevNode::Button
DevNode::Layout(Column)
DevNode::If
DevNode::Component
```

becomes native SwiftUI/Compose views.

Nexa still uses the platform UI frameworks. It does not introduce a custom rendering engine.

The development renderer may be more dynamic than generated release code.

The release Swift backend must keep its existing concrete `some View` generation and its no-`AnyView` performance philosophy.

## 5. Move development state outside generated view identity

Currently Nexa maps mutable state directly to native state:

```text
Swift → @State
Compose → remember { mutableIntStateOf(...) }, etc.
```

That is correct for generated production applications.

For hot reload, introduce a DevStateStore:

```text
StableStateId → {
    type,
    value
}
```

Example:

```text
Home/count : Int32 = 37
```

If UI code changes, `37` survives.

If:

```nx
state count: Int = 0
```

becomes:

```nx
state count: String = ""
```

only that state is reset because its type is incompatible.

Navigation state should eventually use the same principle.

## 6. Add hot-reloadable logic after UI reload works

A change such as:

```nx
count = count + 1
```

to:

```nx
count = count + 10
```

cannot be hot reloaded if that action has already been compiled into Swift/Kotlin.

Therefore DevRuntime needs to execute Nexa logic itself.

Do not build a complex VM initially.

Reuse the shape of the current typed IR:

```text
Expr
Action
BinaryOp
Call
Assign
If
For
While
CollectionMutation
...
```

and define serializable Dev equivalents.

The native runtimes interpret those typed expressions/actions.

Later, if profiling shows it is useful, this representation can be lowered to compact Nexa dev bytecode.

This interpreter exists **only in debug mode**.

Production continues generating direct Swift/Kotlin.

## 7. Treat native changes differently

Nexa plugins currently affect:

```text
native.nxid
Swift sources
Kotlin sources
C++ sources
XCFrameworks
AARs
Swift packages
Maven dependencies
permissions
entitlements
resources
```

These cannot be treated like ordinary `.nx` UI changes.

Classify changes:

```text
Nexa UI / styles           → hot reload
Nexa state / expressions   → hot reload
Nexa functions/actions     → hot reload once Dev interpreter supports them
incompatible app structure → hot restart
native plugin/interface    → native rebuild
dependencies/permissions   → native rebuild
```

Existing plugin graph fingerprinting in `nexa-cli/src/cache.rs` can help detect the rebuild boundary.

## 8. Implementation order

```text
V0
nexa dev command
native DevRuntime shell
WebSocket protocol
file watcher
full DevModule resend
SwiftUI + Compose dynamic rendering

V1
stable IDs
DevStateStore
preserve state across reloads

V2
Dev Expr/Action interpreter
hot reload event handlers and functions

V3
IR diff + Patch protocol
send only changed nodes/functions

V4
preserve navigation, list position, focus and other runtime state

V5
incremental project compilation instead of recompiling the entire Nexa graph

V5 implementation boundary
reuse parsed ASTs for unchanged source files in a dev session; keep semantic
analysis whole-project until dependency-aware invalidation is proven correct

V5 exit gate
automated hot-reload coverage for every Nexa component and runtime feature on
both the iOS Simulator and Android Emulator
```

## V5 hot-reload test acceptance gate

V5 is not complete when incremental compilation alone works. The release gate
must exercise the complete Nexa component and feature surface through the
running development runtime on **both** iOS Simulator and Android Emulator.
Native project compilation tests and Rust unit tests do not satisfy this gate.

The test inventory must be exhaustive against the public typed IR and compiler
features, rather than a hand-picked set of smoke examples. At minimum, coverage
must account for every `nexa-ir::Node` variant: `StatusBar`, `Direction`,
`OnAppear`, `OnDisappear`, `OnActive`, `OnInactive`, `OnBackground`, `Layout`,
`Text`, `Button`, `TextInput`, `Switch`, `Image`, `Pressable`, `NavigationStack`,
`NavigationLink`, `NavigationBack`, `Link`, `Accessibility`, `KeyboardAware`,
`BottomSheet`, `RefreshControl`, `AppBottomBar`, `FastList`, `If`, `When`,
`Content`, `ComponentCall`, and `NativeComponentCall. It must also cover state
types, expressions, actions, functions, and supported plugin/native-component
boundaries. When a language or IR feature has no DevRuntime implementation,
mark it as pending in the inventory; do not silently omit it or claim complete
coverage.

For each runtime-supported component or feature, automated emulator scenarios
must verify the relevant initial rendering and behavior, then change the `.nx`
source and verify that the running app applies the update without a native
rebuild. The suite must cover additions, edits, removals, and conditional or
structural changes where applicable. It must also verify:

- compatible state survives reload, while incompatible state resets safely;
- navigation, list position, and text-input focus survive compatible reloads;
- updated actions and functions run their new logic after reload;
- diagnostics leave the last-good UI running, and a later valid edit recovers;
- hot restart resets runtime state and reports completion;
- plugin/interface, dependency, permission, and other native-host changes take
  the native rebuild path;
- unauthorized or stale development sessions are rejected.

The inventory and scenario fixtures belong under `tests/` (never under
production `src/`). The checked-in coverage matrix enumerates every public IR
enum and has explicit iOS and Android statuses. Its integration test always
checks that the matrix stays exhaustive. When the matrix is complete, both
emulator jobs must run the test with
`NEXA_REQUIRE_HOT_RELOAD_COVERAGE=1`, which rejects any remaining `pending`
status, and publish actionable logs for failures. A feature may be omitted
from the exercised set only while it is explicitly reported as unsupported by
DevRuntime; V5 cannot be declared complete until every public component and
feature intended to work in `nexa dev` is implemented and passes on both
platforms. Platform-specific rendering assertions are acceptable when the
shared Nexa behavior is equivalent.

The existing starter-app reload smoke test proves only one text patch and one
syntax diagnostic. It is a useful connection check, but it is not evidence that
this V5 gate has passed. The first V5 compiler increment is a session-scoped,
content-checked source AST cache: unchanged reachable files reuse parsed ASTs,
changed files are reparsed, and sources removed from the import graph are
pruned. The complete semantic pass still runs on every update so changed shared
declarations cannot leave dependent code unchecked. Further semantic reuse
requires dependency-aware invalidation and correctness tests before it can
replace this safe whole-project check.

Do not optimize V0 prematurely. Sending an entire screen or even the complete DevModule after each save is acceptable initially.

## Architectural rule

Keep two clearly separate execution modes:

```text
DEV
Nexa source
→ compiler/type checker
→ Dev IR
→ NexaDevRuntime
→ SwiftUI / Compose

RELEASE
Nexa source
→ compiler/type checker
→ current nexa-ir
→ current Swift/Kotlin backends
→ native AOT application
```

The hot reload system must not weaken or complicate Nexa's release output.

`nexa build` / `nexa generate` remain pure native compilation.

`nexa dev` is allowed to use a small Nexa runtime specifically because it is a development-only environment.
