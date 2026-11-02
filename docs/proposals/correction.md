# Nexa — Independent implementation verification

**Audit date:** 2026-10-31<br>
**Related claim:** agent's “24 PRs / Navigation v2 / five Mazica fixes complete” report (provided separately)<br>
**Decision:** **NOT VERIFIED FOR RELEASE. Substantial implementation work is present, but several concrete mismatches remain.**

> **Evidence convention:** `CONFIRMED` means directly visible in the uploaded source, not necessarily reproduced on a device. `TESTED` means executed in this review environment. `RISK` means a potentially consequential runtime outcome that requires focused reproduction. Line numbers refer to the files in the uploaded updated archive. This is a source audit, **not** a substitute for iOS/Android device validation.

## 2. Executive findings

| ID | Priority | Status | Finding |
|---|---|---|---|
| NAV-1 | **P0** | CONFIRMED | The canonical navigation reducer is exercised by Rust tests, but **not connected to the Swift/Kotlin AOT generators or DevRuntime navigation interpreters**. |
| NAV-2 | **P0** | CONFIRMED | The native AOT implementations of new `NavigationCommand` operations use incompatible route representations; some commands can be silently ignored. |
| NAV-3 | **P0** | CONFIRMED | Android DevRuntime's new typed `Push` drops route arguments, ignores the command's navigator, and lacks `PopTo`; iOS DevRuntime also lacks `PopTo`. |
| PATCH-1 | **P0** | CONFIRMED | The Kotlin candidate patch engine rejects legitimate `set` operations on newly added object properties; the Rust producer can emit those patches. |
| PATCH-2 | **P1** | CONFIRMED | Android `commitPreparedPatch` changes translations before candidate installation is known to succeed; **complete rollback is not guaranteed**. |
| NAV-4 | **P1** | CONFIRMED | Both DevRuntimes retain fallback selection of arbitrary/latest registered navigation stacks, contrary to the Navigation v2 explicit-scope promise. |
| NAV-5 | **P1** | CONFIRMED | Android deep-link argument failure behavior differs from iOS, and warm deep-link delivery is not demonstrated by an `onNewIntent` source path. |
| VIS-1 | **P1** | CONFIRMED | iOS development navigation and root view inject hard-coded black/#0F0F0F backgrounds, risking fresh AOT/Dev mismatch in light apps. |
| QA-1 | **P1** | CONFIRMED | Navigation compilation tests mainly compare serialized IR and a separate reducer; they do not execute generated commands on the four native paths. |
| QA-2 | **P1** | CONFIRMED | Frame-fence waiting is opt-in (`NEXA_DEV_FRAME_FENCE`, default 0); CI parity invocation does not enable it. |
| PERF-1 | **P1** | CONFIRMED | Added benchmark budgets compare compiler/codegen timing and *generated source bytes*, not installed binary size, UI input latency, GC, FPS or runtime memory. |
| APP-1 | **P2** | NOT VERIFIED | Five Mazica-specific fixes are not provable without the Mazica working tree and device artifacts. |

## 3. Source findings and exact repairs

### NAV-1 — The proposed canonical reducer is not the native execution contract (**P0**)

**Evidence**

- `crates/nexa-ir/src/navigation.rs:373-561` implements `reduce_navigation_command` and the types `NavigationSnapshot`, `NavigatorId`, `EntryId`.
- The only call sites of `reduce_navigation_command` in the submitted Rust/Swift/Kotlin source are the Rust tests in `crates/nexa-ir/src/navigation.rs` and `crates/nexa-compiler/tests/navigation_semantics.rs`. The native generators and development runtimes do not consume its output or a generated transition contract.
- AOT still performs native path mutations in `crates/nexa-backend-swift/src/generator/components/controls.rs:1379-1416` and direct `NavHostController` operations in `crates/nexa-backend-kotlin/src/generator/components/controls.rs:1474-1511`. DevRuntime actions independently interpret JSON.

**Why it matters:** Passing reducer unit tests proves a model's behavior, not the equivalence of actual app execution paths.

**Change**

1. Define a versioned, serializable `NavigationOperation`/`NavigationTransition` contract with *resolved* `navigatorId`, `entryId`, route key, arguments and resulting stack mutations. Keep the pure Rust reducer as the reference semantics in tests.
2. Generate platform-native coordinators whose transition logic is verified against that contract. Production code can specialize static routes; it must not embed an interpreter merely to achieve parity.
3. Add executable conformance tests to run the same transitions through Swift and Kotlin coordinator reference implementations, plus both DevRuntime dispatch paths; compare canonical stack snapshots, entry IDs and state lifecycle traces.
4. Add an explicit compile-time or runtime error for unsupported commands, never a wildcard no-op.

**Acceptance:** Duplicate push, nested stacks, `PopTo` inclusive/exclusive, `ReplaceTop`, `Reset`, `SelectTab`, restoration and deep links produce equivalent state transitions under the four paths.

### NAV-2 — AOT `NavigationCommand` code generation is not route-safe (**P0**)

**Evidence**

- Swift route cases are generated as `navigation_case_name(screen.id)` with a required leading UUID in `crates/nexa-backend-swift/src/generator/engine/types.rs:282-306`, and the normal route helper emits `NexaNavigationRoute.<case>(UUID(), ...)` in `crates/nexa-backend-swift/src/generator/components/navigation.rs:576-581`.
- Yet `Action::NavigationCommand::Push` emits `.\{route\}` or `.\{route\}(args)` using the *string route key* and **no UUID** in `crates/nexa-backend-swift/src/generator/components/controls.rs:1379-1387`. The same mismatch affects `Reset` and `ReplaceTop` (`:1395-1413`). This would be invalid or inconsistent with the generated route enum when this IR branch is used.
- Kotlin registers routes with `navigation_route_name(screen.id)` in `crates/nexa-backend-kotlin/src/generator/components/navigation.rs:970-979`. Its `NavigationCommand` handler navigates to `route.as_str()` in `crates/nexa-backend-kotlin/src/generator/components/controls.rs:1474-1509`, without resolving the route key to that route pattern. The handler builds multiple argument expressions with `join(", ")` (`:1477`), which is not the same as the route path encoding used by the ordinary navigation helper (`navigation.rs:981-1005`).
- Swift AOT ignores `PopTo`, `SelectTab` and `OpenExternalRoute` by its `_ => {}` branch (`controls.rs:1415`). Kotlin does the same (`controls.rs:1510`).

**Change**

1. Resolve every `RouteKey` to a known `ScreenId` and its typed parameter schema during semantic lowering. Error on missing/ambiguous routes.
2. Pass the resolved route to the **existing tested AOT route constructors**, including UUID/entry identity on Swift and URI encoding/type-aware path generation on Kotlin.
3. Implement or explicitly reject all command variants; replace `_ => {}` with exhaustive handling.
4. Add a fixture that constructs a real `Action::NavigationCommand` for each variant and **compiles the emitted Swift and Kotlin**, not just Rust IR serialization.

**Acceptance:** Generated route code compiles and commands target the correct route and navigator, with parameters preserved, on both native platforms.

### NAV-3 — Development `NavigationCommand` handlers lose semantics (**P0**)

**Evidence**

- `runtime/android/NexaDevActions.kt:64-95` reads `Push.route` but calls `encodeScreenRoute(route, JSONObject(), ...)` (**drops arguments**), reads no command-level navigator, and handles only `Push`, `Pop`, `Reset`, `ReplaceTop` before returning `true` for other commands.
- `runtime/ios/NexaDevActions.swift:168-213` and `:797-840` handle the same subset; neither handles `PopTo`. Both select the navigator from action locals rather than the command's explicit `navigator` property. The asynchronous and synchronous implementations are duplicated.
- The Rust enum declares `Push`, `Pop`, `PopTo`, `ReplaceTop`, `Reset`, `SelectTab`, and `OpenExternalRoute` in `crates/nexa-ir/src/navigation.rs:257-287`.

**Change**

1. Create one platform-local command decoder per OS, shared by synchronous and asynchronous action execution.
2. Validate `navigator`, `route`, `arguments`, `entry`, `inclusive` against the active graph before mutation.
3. Evaluate typed arguments using the target screen's parameter schema; do not substitute empty JSON.
4. Implement `PopTo`, `SelectTab`, external-route handling or produce a precise unsupported-command diagnostic.
5. Add behavioral tests for commands with nonempty arguments, explicitly routed commands across two tabs, and error/no-mutation outcomes.

**Acceptance:** Every command variant is either correctly executed on all four paths or rejected during compilation, not silently consumed.

### PATCH-1 — Rust and Android disagree on legal JSON patch operations (**P0**)

**Evidence**

- The Rust patch producer `crates/nexa-dev-ir/src/lib.rs:282-300` emits `set` for a property that did not previously exist. Its reference applier supports object insertion (`:385-390`), appending to arrays, and other operations (`:391-420`).
- The Android candidate engine checks `val child = node.opt(key) ?: return Step.Invalid` **before** its leaf `Set` branch in `runtime/android/NexaDevPatch.kt:155-168`. Therefore setting `/body/0/Text/new_property` is rejected if `new_property` is absent, even though the Rust producer can emit such a patch.
- Android also rejects all array-element removals and arrays at `idx == length` (`NexaDevPatch.kt:178-190`), whereas the Rust reference applier supports them. The normal diff producer often replaces length-changed arrays wholesale, but the two patch implementations are still not a complete shared contract.

**Change**

1. Separate parent traversal from target-leaf access. For object `Set`, allow an absent leaf; require present targets for operations that need them, as dictated by the shared protocol.
2. Decide the normative contract for array append/remove, root replacement and empty pointer segments; implement identically or reject such operations at the **server** and always emit an explicitly supported replacement form.
3. Add golden operation vectors produced by the Rust patch engine and consumed by the Android JVM engine and iOS implementation. Include **new optional field**, removed field, escaped key, array length change, function body, presentation field and malformed operations.
4. Add a real hot-reload test changing `.nx` from default property omission to an explicitly set property; assert incremental patch applies without requesting a full module.

**Acceptance:** Every patch emitted by the server is accepted by each compatible client, or the server deliberately falls back to full module before transmission. No silently incompatible operations.

### PATCH-2 — Android patch installation is not fully transactional (**P1**)

**Evidence**

- `runtime/android/NexaDevState.kt:773-803` correctly checks object identity and runs on main thread, then calls `installTranslations(translations)` at **line 777 before** attempting `installPatchedFunctions` or `installPatchedScreens`. If installation returns `false` or throws, the function returns `false` but translations may already be changed.
- `installPatchedFunctions` updates a live registry (`:805-810`). `installPatchedScreens` commits `values` and `typeSignatures` in a sequence (`:840-845`); failed or exception-throwing commits are not covered by a general rollback journal.
- `snapshotModule()` reads `currentModule` on the WebSocket worker (`:756`), but the field is ordinary private state (`NexaDevState.kt:207`), not visibly protected by a lock/volatile atomic publication. The **thread-visibility risk** deserves verification even if it has not manifested.

**Change**

1. Build and validate a complete `PreparedInstallation` containing candidate module, translations, function registry, state changes and diagnostics with **no live mutations**.
2. Commit the prepared values in a single main-thread transaction after all validation passes. On unexpected exceptions, retain/restore previous state and do not advance revision or acknowledge.
3. Publish the installed module reference with a defined cross-thread memory visibility mechanism (e.g., guarded immutable snapshot or volatile reference), keeping all Compose state writes on the main thread.
4. Add failure injection tests at each commit boundary and assert exact equality of old module, translations, functions, states and revision after rejection.

**Acceptance:** A rejected patch has *zero* observable side effects; a successful patch changes all intended subsystems exactly once.

### NAV-4 — Fallback navigation targeting still exists (**P1**)

**Evidence**

- `runtime/ios/NexaDevState.swift:717-734` ends `resolveNavigatorId` with `navigationPaths.keys.first ?? "app"`, after several heuristic attempts.
- `runtime/android/NexaDevState.kt:237-265` falls back to `currentNavController` and `controllerOrder.lastOrNull()`, after looking for scoped controller identities.
- These fallbacks contradict the architecture document's statement that every command has an authoritative navigator and there is no heuristic selection.

**Change:** Make resolved navigator identity mandatory at action dispatch. Preserve lexical navigator provenance through closures, plugin callbacks and async tasks. When a scope is missing, return an explicit error rather than choosing another tab. Add a cross-tab delayed-callback test and a nested-stack collision test.

### NAV-5 — Deep links are not yet a single cross-platform contract (**P1**)

**Evidence**

- Android DevRuntime `runtime/android/NexaDevNavigation.kt:183-202` silently converts invalid `Bool`, `Int` and `Double` input to `false`, `0`, `0.0`. iOS DevRuntime `runtime/ios/NexaDevNavigation.swift:81-93` rejects invalid values. Thus the same malformed URL can open a route on Android and do nothing on iOS.
- Kotlin AOT `navigation.rs:731-815` and DevRuntime `NexaDevNavigation.kt:169-203` derive deep links from `Activity.intent.data` in composition/`LaunchedEffect`. This alone does not prove that a **new warm `onNewIntent` event** is reactively delivered when the Activity stays alive. The iOS path uses `onOpenURL`.
- The Rust `OpenExternalRoute` reducer at `crates/nexa-ir/src/navigation.rs:538-559` performs simplistic string splitting, doesn't coerce/validate parameters and does not check the route against the graph.

**Change:** Centralize a URL parsing and typed argument policy, including encoding, missing/invalid arguments, duplicate deep links, cold vs warm launch, and tab selection. Inject Android new-intent events through a reactive native bridge. Test good/bad URLs in all four modes and check stack outcomes.

### VIS-1 — Fixed dark backgrounds in iOS DevRuntime (**P1**)

**Evidence**

- `runtime/ios/NexaDevRenderer.swift:3894-3907` inserts a hard-coded `Color(red:15/255, green:15/255, blue:15/255)` for navigation stacks without a native bar, and also for the no-root-toolbar branch.
- `runtime/ios/NexaDevRuntime.swift:349` sets a black root background. The Swift AOT navigation generator in `crates/nexa-backend-swift/src/generator/components/navigation.rs:125-215` does not emit an equivalent universal hard-coded dark background.

**Change:** Respect the app appearance and authored background. Do not hard-code a Mazica-specific color into the generic Nexa renderer. Derive window/container background from canonical component defaults or native appearance contract. Add dark, light, system and explicit-color tests, including pushed screens and safe areas.

### QA-1 — Current tests cannot substantiate full navigation parity (**P1**)

**Evidence**

- `crates/nexa-compiler/tests/navigation_semantics.rs:16-39` compares serialized AOT/Dev IR. This is valuable, but doesn't execute native navigation handlers.
- Reducer tests test `nexa_ir::reduce_navigation_command`, which is not linked to the native dispatch implementations. The same repository has multiple independently coded navigation paths, including duplicate iOS synchronous/asynchronous actions.
- `crates/nexa-compiler/tests/action_parity_gate.rs:92-101` accepts action support when the action name occurs anywhere in the source file. A file containing the string `"NavigationCommand"` passes even if some variants are swallowed.

**Change:** Replace name-presence checks with generated exhaustive variant visitor registration and execution-level tests. For each navigation variant and parameter combination, produce native trace assertions, not only string checks.

### QA-2 — Frame fence is optional in the parity job (**P1**)

**Evidence:** `tests/hot-reload-aot-parity.sh:639-641,651-653` waits for the new frame fence only if `${NEXA_DEV_FRAME_FENCE:-0} == 1`. The CI parity invocations in `.github/workflows/ci.yml:184-188` and `:300-304` do not set it.

**Change:** Enable the fence in CI by default, match the exact acknowledged revision and frame token, fail on timeout/mismatch, and verify the signal corresponds to rendering of the committed tree (not merely a composition observer that may fire before the screenshot is settled). Keep a documented opt-out only for diagnostics.

### PERF-1 — The performance claim is not established (**P1**)

**Evidence:** `tests/benchmark-suite.py:48-113` compares compiler time, codegen time and `generatedSourceBytes`. No evidence in these measurements establishes "60/120 fps", lack of GC stutter, input-to-photon latency, memory usage, or executable binary size. The new Python unit tests validate the comparison function using synthetic benchmarks; they are not device measurements.

**Change:** Add release AOT and DevRuntime device benchmarks: tap-to-frame latency (median/p95/p99), janky frames, GC pauses, steady memory, startup and native artifact size. Regressions should be measured against a pinned device/OS/toolchain baseline. Retain codegen benchmarks as a separate useful metric.

## 4. What genuinely improved

These changes are supported by the uploaded source, although some still require device verification:

1. Android patch **preparation** is now copy-on-write in `runtime/android/NexaDevPatch.kt` rather than mutating the installed object immediately, and the protocol moves commit to the main thread in `runtime/android/NexaDevProtocol.kt:194-234`.
2. `runtime/android/NexaDevState.kt:766-775` rejects commits if the installed JSON reference no longer matches the prepared base — a useful stale-candidate guard.
3. `runtime/android/NexaDevActions.kt` no longer increments `moduleRevision` on every ordinary state assignment; `runtime/android/NexaDevState.kt:879-886` relies on `mutableStateMapOf` keyed state updates. This is promising for Android responsiveness, but doesn't establish latency measurements.
4. The compiler now has differential IR tests, more realistic navigation fixtures, a patch/full test and stronger Python screenshot comparison checks.
5. Swift AOT route types now carry UUID route identity (`engine/types.rs:282-306`), a good foundation for repeated screen pushes with separate state.
6. Per-entry scope strings exist in both development navigators (`runtime/android/NexaDevNavigation.kt:249-255`; `runtime/ios/NexaDevRenderer.swift:3857-3863`), although complete lifecycle conformance requires device testing.
7. The Python test suite genuinely passes all 26 discovered tests.

**Do not revert this progress wholesale.** Repair the remaining contract gaps incrementally, with regression tests that fail before the fix and pass after it.

## 5. Verification of the five Mazica claims

| Reported fix | Framework source evidence | Verification status |
|---|---|---|
| iOS custom back / swipe | SwiftUI gesture probe and native-path logic are present | Code present; actual edge-swipe completion, nested stacks and cancellation **not device verified** |
| Hide iOS native tab bar | `NexaHideTabBarModifier` is present in Swift AOT and DevRuntime | Code present; OS/device matrix behavior **not verified** |
| Localization immediate refresh | Android reactive language state and invalidation are present | Framework paths changed; Mazica `App.nx` and both device results **not supplied** |
| PlayerSettingsScreen dark background | Generic Nexa DevRuntime hard-codes dark backgrounds | Mazica screen edit **not supplied**; generic regression risk identified |
| Android click responsiveness | Full-root `moduleRevision` increments removed from routine actions | Good structural fix; 60/120 FPS and GC claims **not benchmarked** |

## 6. Required corrective PR sequence (send this to the coding agent)

### Blocker PR A — Match the patch producer and candidate engine

**Files:** `runtime/android/NexaDevPatch.kt`, `runtime/android/tests/NexaDevPatchTest.kt`, `crates/nexa-dev-ir/src/lib.rs`, `crates/nexa-dev-ir/tests/patch_vs_full_module.rs`.

- Add object-property insertion, including optional fields that the Rust diff emits.
- Normalize/remap array and root operations, or constrain the producer's output to a formally documented common subset.
- Test server-produced JSON vectors against Kotlin and iOS.
- **Gate:** 100% of emitted patch operations apply to the correct prior module and produce the full-module result, with no forced full reload for supported property additions.

### Blocker PR B — Make Android module commits fully atomic

**Files:** `runtime/android/NexaDevState.kt`, `runtime/android/NexaDevProtocol.kt`, plus state/translation tests.

- Precompute all affected registries and state changes; atomically install translations and module.
- Define a synchronization boundary for worker snapshot publication.
- Inject failures; assert previous revision, translations, live state and registries unchanged.
- **Gate:** No partial state changes on rejection or mid-install failure.

### Blocker PR C — Implement NavigationCommand on all paths

**Files:** `crates/nexa-backend-swift/src/generator/components/controls.rs`, `crates/nexa-backend-kotlin/src/generator/components/controls.rs`, `runtime/ios/NexaDevActions.swift`, `runtime/android/NexaDevActions.kt`, `crates/nexa-compiler/tests/navigation_semantics.rs`.

- Resolve typed route keys to screen declarations and use each platform's canonical native route constructor.
- Implement every declared command or explicitly reject unsupported commands at semantic validation.
- Use specified navigator IDs, typed argument encoding and target entry identity.
- Consolidate iOS async/sync command interpretation.
- **Gate:** generated Swift/Xcode and Kotlin/Gradle builds plus native sequence tests for *all* variants.

### Blocker PR D — Remove implicit stack fallback and align navigation state

**Files:** `runtime/ios/NexaDevState.swift`, `runtime/android/NexaDevState.kt`, `runtime/{ios,android}/NexaDevNavigation.*`, AOT navigation coordinators.

- Propagate navigator provenance through closures, async tasks and native callbacks.
- Remove `navigationPaths.keys.first`, `currentNavController` and `controllerOrder.lastOrNull` as fallback execution choices.
- Reconcile the root entry and pushed-entry scope policies across AOT/Dev for each platform.
- **Gate:** two tabs with identical destinations + delayed callbacks cannot ever cross-navigate or share state.

### PR E — Deep links, theme, safe areas

**Files:** `crates/nexa-ir/src/navigation.rs`, both AOT navigation generators, both DevRuntime nav files, `runtime/ios/NexaDevRenderer.swift`.

- Shared typed URL parsing rules; explicit error behavior; warm intent delivery on Android.
- Remove generic hard-coded dark background, keep Mazica's appearance in Mazica only.
- **Gate:** deep-link and light/dark screenshots match their intended same-platform AOT/Dev baseline.

### PR F — Enforce real end-to-end proof and performance

**Files:** `.github/workflows/ci.yml`, `tests/hot-reload-aot-parity.sh`, `tests/benchmark-suite.py`, conformance tests.

- Enable revision-matched frame-fence waiting on CI.
- Add native command execution sequences to the parity suite.
- Add runtime responsiveness/FPS/memory benchmarks; retain compiler metrics.
- Attach screenshots, navigation snapshots, accessibility trees, perf traces and build logs for failures.
- **Gate:** all AOT-vs-Dev navigation sequences pass on iOS and Android and performance budgets pass on pinned devices.

## 7. Minimum mandatory test scenarios

| Case | Starting state | Action | Expected invariant |
|---|---|---|---|
| N-01 | Root | Push `Details(id=1)` twice | Two distinct entry IDs; independent state |
| N-02 | Tab A + Tab B | Push same route in both | Independent navigator and state scope |
| N-03 | Tab A callback pending | Switch to Tab B; callback fires | Command targets captured Tab A, not current Tab B |
| N-04 | Root → A → B → C | PopTo(A, inclusive=false) | Root → A; B/C state destroyed |
| N-05 | Root → A → B → C | PopTo(A, inclusive=true) | Root only; correct errors on root removal |
| N-06 | Root → A | ReplaceTop(B, args) | Root → B with typed arguments; old A disposed |
| N-07 | Root → A | Reset(B) | Documented root semantics identical in AOT/Dev |
| N-08 | Tab A/B with history | SelectTab(B), back | Only selected stack affected |
| N-09 | Existing list | Hot reload adds optional presentation prop | Patch accepted, no full reload, correct rendering |
| N-10 | Existing state | Invalid late patch operation | No changes to JSON, translations, values, registry, revision |
| N-11 | Light app, hidden nav bar | Push, swipe back | Background and system chrome match native AOT/Dev |
| N-12 | Screen with int parameter | URL with invalid int | Same explicit rejection on all paths |
| N-13 | Running Android Activity | New deep link intent | New navigation transition exactly once |
| N-14 | AOT/Dev in same environment | Nav hot reload + screenshot | Revision-matched post-render snapshot, not previous frame |
| N-15 | Rapid taps, 1k list rows | State mutation | No full-root recomposition; frame-time and GC budgets pass |

## 8. Claims not independently proven by this audit

- `cargo check --workspace --all-targets`, `clippy -D warnings`, `cargo test --workspace`, and ignored collision test: **reported by agent, not executable here**.
- Android JVM patch engine tests: **present in repository, not executed here**.
- iOS interactive gesture success on real devices, OS-version-specific tab-bar chrome, screenshots, localization, and Mazica click FPS: **not reproducible from framework ZIP alone**.
- “Zero `AnyView` in AOT” is a separate code generation property; it does not imply equivalence. The iOS **DevRuntime** explicitly still uses `AnyView`, as expected for a dynamic renderer.
- “100% AOT/Dev parity” and “Navigation v2 complete” **are not supported** by the native command paths found in this archive.

## 9. Final decision

**Accept the updated source as real progress, not as completed navigation/parity verification.** Keep the new patch architecture, identities, testing infrastructure and fine-grained Compose reactivity. Before calling the rewrite complete, fix **NAV-2, NAV-3 and PATCH-1**, make installation atomic, remove implicit stack selection, and run genuine native AOT/Dev differential scenarios. Do not trade away direct SwiftUI/Compose codegen to achieve those fixes.

**Recommended owner workflow:** one corrective PR at a time, with a failing regression fixture recorded *before* the patch; attach generated native code and actual device runs to the PR. A source-string parity gate or a Rust-only reducer test cannot substitute for executing the generated native program.
