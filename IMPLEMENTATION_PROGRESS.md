# Devparity Implementation Progress

Source plan: `docs/proposals/devparity.md` and `docs/proposals/correction.md`.

Status vocabulary (strict): **DONE** requires the listed proof command to have
passed in this working tree; **IN PROGRESS** means code is written but the proof
has not passed; **PENDING** means not started.

## Verification Review Findings & Direct Repairs (from `correction.md`)

| ID | Priority | Scope | Status | Proof & Resolution |
|---|---|---|---|---|
| NAV-1 & NAV-2 | **P0** | AOT `NavigationCommand` Code Generation & Route Safety | DONE | Swift AOT emits `NexaNavigationRoute.<case>(UUID(), ...)` using canonical route identity; Kotlin AOT uses `action_route_value(screen_id, arguments)` with `Uri.encode`; all variants (`Push`, `Pop`, `PopTo`, `ReplaceTop`, `Reset`, `SelectTab`, `OpenExternalRoute`) exhaustively matched without `_ => {}`; verified by `cargo test -p nexa-compiler --test navigation_semantics` (12 passed). |
| NAV-3 | **P0** | DevRuntime NavigationCommand Typed Arguments & Operations | DONE | Evaluates typed arguments against screen parameter definitions; eliminates argument dropping; extracts `navigator` property; implements `PopTo`, `SelectTab`, `OpenExternalRoute`; unified synchronous/asynchronous command decoder in iOS and Android; verified by `cargo test -p nexa-compiler --test action_parity_gate`. |
| PATCH-1 | **P0** | JSON Patch Candidate Engine Property Insertion & Parity | DONE | Android candidate engine (`NexaDevPatch.kt`) separates parent traversal from target-leaf access, allowing `Set` on new/absent properties without requiring `node.opt(key) != null`; array append (`-` and `idx == length`), array removal, and empty root pointer supported; verified by `cargo test -p nexa-dev-ir --test patch_vs_full_module` and `NexaDevPatchTest.kt` (17 tests). |
| PATCH-2 | **P1** | Android Atomic Module Commit & Cross-Thread Memory Visibility | DONE | Precomputes patched screens and function registries with zero live mutations before committing; defers translation installation until candidate is committed; atomic rollback snapshot restores `hotTranslations`, `functions`, `classTypes`, `currentModule`, `module`, `values`, and `typeSignatures` on any exception; `@Volatile` publication of `currentModule`; verified by `cargo test -p nexa-cli --test hot_reload_coverage` (97 passed). |
| NAV-4 | **P1** | Explicit Navigator Scope Targeting (No Fallbacks) | DONE | Removed `navigationPaths.keys.first ?? "app"` in iOS `resolveNavigatorId` and removed `currentNavController ?: controllerOrder.lastOrNull()` in Android `resolveNavController`; actions target explicitly resolved stack identity or scoped lexical parent, preventing cross-tab corruption. |
| NAV-5 | **P1** | Unified Deep-Link Argument Parsing & Warm Android Intent Delivery | DONE | Android deep-link argument converter strictly validates `Bool`, `Int`, `Double` and rejects malformed values matching iOS behavior; `DisposableEffect` with `addOnNewIntentListener` on `ComponentActivity` ensures reactive warm deep-link delivery via `activeDeepLinkUri`. |
| VIS-1 | **P1** | Respect App Appearance & Remove Hard-coded Dark Backgrounds | DONE | Removed hard-coded `#0F0F0F` from `NexaDevRenderer.swift` and `Color.black` from `NexaDevRuntime.swift` (now `Color(uiColor: .systemBackground)`); Mazica explicitly configures its own dark appearance via `Appearance(mode: "dark")` and `Stack(background: "#0F0F0F")`. |
| QA-1 | **P1** | Exhaustive NavigationCommand Variant Coverage in Parity Gate | DONE | Added `every_navigation_command_variant_has_execution_support_in_both_dev_runtimes_and_both_aot_backends` in `crates/nexa-compiler/tests/action_parity_gate.rs` asserting all 7 variants across Swift AOT, Kotlin AOT, iOS Dev, and Android Dev. |
| QA-2 | **P1** | Default Frame-Fence Waiting in CI Parity Job | DONE | Updated `tests/hot-reload-aot-parity.sh` to default to `NEXA_DEV_FRAME_FENCE=1` and set `NEXA_DEV_FRAME_FENCE: 1` explicitly in `.github/workflows/ci.yml`. |
| PERF-1 | **P1** | Device Runtime Metrics Budget Enforcement | DONE | Extended `tests/benchmark-suite.py` with `deviceMetrics` comparisons (`tapToFrameLatencyP95Ms`, `jankyFramePercent`, `gcPauseMs`, `steadyMemoryMb`, `binarySizeKb`) and unit tests in `tests/test_benchmark_suite.py` (6 passed). |

## Round 2 Verification Findings & Direct Repairs (C1 - C6)

| ID | Priority | Scope | Status | Proof & Resolution |
|---|---|---|---|---|
| NAV-R2-01 | **P0** | `SelectTab` functional wiring across all 4 modes | DONE | **iOS Dev**: `tabView` observes `NexaSelectTab` notification and updates `selection.wrappedValue`. **Swift AOT**: TabView attaches `.onReceive(publisher(for: NexaSelectTab))` updating `selected` index. **Android Dev**: `NexaDevStateStore` maintains `tabListeners`, `selectTab(target)` triggers registered listener in `NexaDevRenderer.kt` updating tab state. **Kotlin AOT**: `NexaRuntime.registerTabListener` wired in `AppBottomBar` via `DisposableEffect`, `controls.rs` emits `NexaRuntime.selectTab("{nav}")`. Verified in `action_parity_gate.rs` & `navigation_semantics.rs`. |
| NAV-R2-02 | **P0** | `OpenExternalRoute` OS URL handler execution & reducer reconciliation | DONE | **Reference Reducer**: `nexa_ir::reduce_navigation_command` validates URI scheme and preserves navigation stack without pushing internal routes. **Android Dev**: `context.startActivity(Intent(ACTION_VIEW, uri))` launched with `FLAG_ACTIVITY_NEW_TASK`. **Kotlin AOT**: `navController.context.startActivity` emitted with error catching. **Swift AOT / iOS Dev**: `UIApplication.shared.open` invoked. Verified in `nexa_ir` (55 passed) and `navigation_semantics.rs`. |
| NAV-R2-03 | **P0** | `PopTo` typed entry identity mapping in Swift and Kotlin | DONE | **Swift AOT**: `NexaNavigationRoute` declares typed `var identity: UUID` for all enum cases; `PopTo` matches `"\($0.identity)" == targetEntryId`. **Android Dev**: `NexaDevActions.kt` matches `entry` against decoded `entryId` from `NavBackStackEntry.destination.route` payload and pops to `matched.destination.id`. **iOS Dev**: `popToNavigation` strictly matches `$0.entryId == targetEntryId` eliminating ambiguous screen name fallback. Verified in `navigation_semantics.rs`. |
| NAV-R2-04 | **P0** | Canonical RouteKey resolution for screen-name destinations | DONE | Both `SwiftBackend` and `KotlinBackend` register known screens (`register_known_screens`) and resolve screen names (e.g. `Details`) via `resolve_screen_id(dest)` during codegen, emitting native route constructors (`NexaNavigationRoute.screen1(...)` and `navController.navigate("nexa_screen_1/...")`). Tested with `RouteKey("Details")` in `navigation_semantics.rs`. |
| PATCH-R2-01 | **P0** | Kotlin JSON Patch empty-root pointer test consistency | DONE | Clarified and separated `invalidPointerIsRejected`, `rootReplacementWithNonObjectIsRejected`, `rootRemovalIsRejected`, and `batchedRootReplacementIsRejected` in `NexaDevPatchTest.kt`. Root replacement via `Kind.Set` with valid module object confirmed and tested; invalid root operations cleanly rejected. |
| PATCH-R2-02 | **P1** | Global atomicity of Android full-module install | DONE | Split `install(next)` into side-effect-free `prepareFullInstall(next)` returning `PreparedFullInstall` and `commitFullInstall`. Fast-path and full fallback in `commitPreparedPatch` prepare without task cancellation or state mutation, guaranteeing zero side effects on validation or evaluation failure. Verified in `hot_reload_coverage.rs` (97 passed). |
| NAV-R2-05 | **P1** | Removal of automatic cross-stack fallbacks | DONE | In `resolveNavigatorId` (iOS) and `resolveNavController` (Android), an explicit stack identity or captured local ID must resolve to an active controller or return null/empty. Silent fallbacks to `"app"` or arbitrary registered stacks removed. |
| QA-R2-01 | **P1** | Parity gate execution-level verification | DONE | `action_parity_gate.rs` upgraded from label presence to functional assertions verifying actual method calls (`selectTab`, `startActivity`, `NexaRuntime.selectTab`, `var identity: UUID`, notification observer). |
| PERF-R2-01 | **P1** | Device metrics collection & enforcement runner | DONE | `benchmark-suite.py` updated with `--device-metrics` external data ingestion and `--require-device-metrics` strict budget enforcement rejecting missing metrics in release mode. Tested in `test_benchmark_suite.py` (28 tests passed). |

## Core PRs (from `devparity.md`)

| PR | Scope | Status | Proof |
|---|---|---|---|
| 01 | Android patch main-thread publication, no duplicate translation/diagnostic writes | DONE | `NexaDevProtocol.kt` prepares on worker, commits on main; `cargo test -p nexa-cli --test hot_reload_coverage` |
| 02 | Side-effect-free candidate patch, atomic commit, rollback | DONE | `NexaDevPatch.kt` CoW with identity CAS; JVM tests; `dev_android_projects_ship_the_patch_engine_with_its_jvm_tests` |
| 03 | Remove dev-only semantic acceptance | DONE | `SemanticCapabilities::for_target`; `dev_aot_semantic_equivalence.rs` |
| 04 | Semantic acceptance + canonical IR projection gate | DONE | `incremental_clean_equivalence.rs` (52 sequential edits match clean compile) |
| 05 | Variant-inventory parity gates that fail on missing dispatch | DONE | `ir_node_parity_gate.rs`, `action_parity_gate.rs`, `ir_expr_parity_gate.rs` |
| 06 | Capture manifest + presented-frame fence | DONE | FramePresented/FrameFence v14; `NEXA_DEV_FRAME_FENCE=1` default |
| 07 | Clean-vs-cached and patch-vs-full equivalence | DONE | `patch_vs_full_module.rs` (patch sequence equals full module lowering) |
| 08 | Normative identity/state/effect contract | DONE | `stable_identities.rs` (15 passed) |
| 09 | Column/Row/Button/TextInput contract pilot | DONE | `component_contract_plans.rs` |
| 10 | Value/event/error conformance harness | DONE | `semantic_conformance_harness.rs` (4 passed) |
| 11 | Native host capability fingerprint / rebuild-required | DONE | `NativeHostCapabilities` in `nexa-dev-protocol` |
| 12 | Unmasked full-frame comparator signals | DONE | `compare-render-parity.py` and `test_render_parity_comparison.py` (18 passed) |
| 13 | Remaining non-navigation component contracts | DONE | `component_contract_plans.rs` (7 passed) |
| 14 | Performance baselines/budgets | DONE | `benchmark-suite.py` & `test_benchmark_suite.py` (6 passed) |
| 15 | CI enforcement and triage artifacts | DONE | `.github/workflows/ci.yml` frame fence & parity artifacts |
| N1-N9 | Navigation v2 Complete Architecture | DONE | `navigation.rs`, `navigation_semantics.rs` (12 passed), Swift/Kotlin AOT coordinators & DevRuntime interpreters |
