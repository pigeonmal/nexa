//! FastList scroll notifications must avoid re-entering native view updates.

use std::fs;

use nexa_codegen::Backend;
use nexa_compiler::{Target, compile_file_with_warnings_for_target};
use nexa_testkit::TestProject;

#[test]
fn ios_dev_runtime_reserves_end_reached_before_deferring_actions() {
    let runtime = include_str!("../../../runtime/ios/NexaDevRenderer.swift");
    let state = include_str!("../../../runtime/ios/NexaDevState.swift");
    let observer = runtime
        .split(".onPreferenceChange(NexaDevListPositionPreference.self)")
        .nth(1)
        .expect("iOS DevRuntime should observe visible FastList rows")
        .split("\n    }\n\n    private func scheduleProgrammaticScroll")
        .next()
        .expect("the list preference observer should end before scroll scheduling");

    assert!(
        runtime.contains("private final class NexaDevListEndReachedGate")
            && runtime.contains("func claim(count: Int) -> Bool")
            && runtime.contains("func complete(count: Int) -> Bool")
            && runtime.contains("func cancel(count: Int)")
            && runtime.contains("pendingCount = count")
            && runtime.contains("lastReachedCount = count"),
        "the iOS DevRuntime should reserve end callbacks before dispatch and commit or cancel the reservation afterward"
    );
    assert!(
        runtime.contains("@State private var endReachedGate = NexaDevListEndReachedGate()")
            && runtime.contains("endReachedGate.reset()")
            && runtime.contains("private var gatedOnEndReached: ((Int) -> Void)?")
            && runtime.matches("onEndReached: gatedOnEndReached").count() >= 5,
        "native and SwiftUI list callbacks should share a per-view once-per-count gate"
    );
    assert!(
        runtime.contains(
            "store.updateFastListEndReachedCount(identity: identity, scope: scope, count: count)"
        ) && runtime.contains("store.claimFastListEndReached(")
            && state.contains("private var fastListEndReachedCounts: [String: Int] = [:]")
            && state.contains("private var fastListEndReachedKeys = Set<String>()")
            && state.contains(
                "func updateFastListEndReachedCount(identity: String, scope: String, count: Int)"
            )
            && state.contains(
                "func claimFastListEndReached(identity: String, scope: String, count: Int) -> Bool"
            ),
        "DevRuntime must keep its once-per-count reservation in the persistent state store so native list recreation cannot duplicate callbacks"
    );
    assert!(
        runtime.contains("@Environment(\\.nexaDevNodeIdentity) private var nodeIdentity")
            && runtime.contains("identity: \"\\(nodeIdentity)/\\(index)\"")
            && runtime.contains("identity: \"\\(identity)/children/\\(index)\""),
        "the persistent callback reservation key should use a stable tree path to distinguish separate FastLists"
    );
    let scroll_action = runtime
        .split("onScroll: onScrollActions.map { actions in")
        .nth(1)
        .expect("the iOS DevRuntime should bind FastList scroll actions")
        .split("onEndReached: onEndReachedActions.map { actions in")
        .next()
        .expect("the scroll action binding should end before end-reached binding");
    assert!(
        scroll_action.contains(
            "store.stateCommandRevision($0, scope: scope) == scrollPositionCommandRevision"
        ),
        "queued iOS DevRuntime scroll actions must be discarded when a newer explicit scroll-position command supersedes them"
    );
    let claim = observer
        .find("let shouldReportEndReached = isAtEnd && endReachedGate.claim(count: count)")
        .expect("the observer should reserve the callback before deferring work");
    let dispatch = observer[claim..]
        .find("DispatchQueue.main.async {")
        .expect("the reserved notification should be followed by deferred SwiftUI updates");
    assert!(
        dispatch > 0,
        "end-reached must be claimed synchronously before enqueueing deferred actions"
    );
    assert!(
        observer.contains(
            "if shouldReportEndReached && endReachedGate.complete(count: count) {\n                    onEndReached?(visibleIndex)\n                }"
        )
            && !observer.contains("didReachEnd"),
        "the deferred callback should only run for a committed reservation and must not rely on deferred @State deduplication"
    );
    assert!(
        observer.contains("if shouldReportEndReached { endReachedGate.cancel(count: count) }")
            && observer.contains("shouldReportEndReached && endReachedGate.complete(count: count)"),
        "a superseded preference update must release its end-reached reservation, while a current update commits it exactly once"
    );
    assert!(
        runtime.contains("@State private var suppressesProgrammaticScrollCallbacks = false")
            && runtime.contains("suppressesProgrammaticScrollCallbacks = true")
            && runtime.contains("let controlledPositionRevision: Int")
            && runtime.contains(".onChange(of: controlledPositionRevision)")
            && runtime.contains("observedControlledPositionRevision != revision")
            && runtime
                .contains("let capturedControlledPositionRevision = controlledPositionRevision")
            && runtime.contains(
                "observedControlledPositionRevision != capturedControlledPositionRevision"
            )
            && runtime
                .contains("reachedProgrammaticTarget || !suppressesProgrammaticScrollCallbacks")
            && runtime.contains(
                "pendingProgrammaticScrollPosition != nil || suppressesProgrammaticScrollCallbacks"
            ),
        "controlled scrolls should report the settled target once and ignore stale intermediate visible-row preferences until a user drag"
    );
}

#[test]
fn android_dev_runtime_keeps_scroll_observer_stable_across_recomposition() {
    let runtime = include_str!("../../../runtime/android/NexaDevRenderer.kt");
    assert!(
        runtime.contains("import androidx.compose.runtime.rememberUpdatedState")
            && runtime.contains("val currentObserveScroll = rememberUpdatedState(observeScroll)"),
        "the Android DevRuntime should publish the latest callback without replacing its list observer"
    );
    assert_eq!(
        runtime
            .matches("LaunchedEffect(listState, scrollPositionName, onScroll, onEndReached, count)")
            .count(),
        0,
        "callback identities must not restart snapshotFlow and replay the current row as a scroll"
    );
    assert!(
        runtime.contains("LaunchedEffect(listState, count)")
            && runtime.contains("currentObserveScroll.value(snapshot.first, snapshot.second, count, snapshot.third, isScrollInProgress)")
            && runtime.contains("currentObserveScroll.value(\n                                            nexaDevLeadingVisibleDataItemIndex(")
            && runtime.contains("listState.isScrollInProgress to Triple(")
            && runtime.contains("val callbackCount = if (isScrollInProgress || previousScrollWasInProgress)")
            && runtime.contains("repeat(callbackCount) { callbackIndex ->")
            && runtime.contains("scrollWasInProgress[0] = isScrollInProgress"),
        "Android grid and list observers should synthesize callbacks for each crossed row while remaining scoped to list/count changes"
    );
}

#[test]
fn scroll_callbacks_are_deferred_in_swift_and_collected_from_scroll_state_in_kotlin() {
    let project = TestProject::new("nexa-fast-list-scroll-callbacks");
    let entry = project.join("App.nx");
    fs::write(
        &entry,
        r#"
app ScrollCallbacks {
    state scrollPosition: Int32 = 0
    state reversePosition: Int32 = 0
    state scrollCount: Int32 = 0
    state endCount: Int32 = 0
    body {
        Column {
            FastList(count: 24, scrollPosition: scrollPosition) { index in
                Text("Vertical row \(index)")
            }.onScroll {
                scrollCount += 1
            }.onEndReached {
                endCount += 1
            }.stickyHeader {
                Text("Pinned list header")
            }
            FastList(count: 24, scrollPosition: reversePosition, reverseLayout: true) { index in
                Text("Reverse row \(index)")
            }.onScroll {
                scrollCount += 1
            }.onEndReached {
                endCount += 1
            }
            FastList(count: 24, axis: Horizontal, rowHeight: 48) { index in
                Text("Horizontal row \(index)")
            }.onScroll {
                scrollCount += 1
            }.onEndReached {
                endCount += 1
            }
            FastList(count: 24, axis: Grid(2), rowHeight: 48) { index in
                Text("Grid row \(index)")
            }.onScroll {
                scrollCount += 1
            }.onEndReached {
                endCount += 1
            }
            FastList(count: 4, pageSnap: true) { index in
                Text("Page \(index)")
            }.onScroll {
                scrollCount += 1
            }.onEndReached {
                endCount += 1
            }
        }
    }
}
"#,
    )
    .expect("test source should be written");

    let swift_module = compile_file_with_warnings_for_target(&entry, Target::Swift)
        .expect("FastList callbacks should compile for Swift")
        .module;
    let swift = nexa_backend_swift::SwiftBackend.generate(&swift_module);
    assert!(
        swift.contains("NexaSwiftUIPageSnapCallbackState"),
        "{swift}"
    );
    assert!(
        swift.contains("callbackState.lastEndReachedCount != rowCount"),
        "{swift}"
    );
    assert!(
        swift.contains(".onAppear { reportSettledPage(selectedPage) }"),
        "{swift}"
    );
    assert!(
        swift.contains("scrollPositionEchoes.consume(scrollPosition)"),
        "{swift}"
    );
    assert!(
        swift.contains("scrollPositionEchoes.record(position)"),
        "{swift}"
    );
    assert_eq!(
        swift
            .matches("guard self.lastReportedScrollPosition == position else { return }")
            .count(),
        3,
        "vertical, horizontal, and grid UIKit delegate callbacks must discard stale queued positions before invoking actions\n{swift}"
    );
    assert_eq!(
        swift
            .matches("private func reportProgrammaticScrollPosition(_ position: Int)")
            .count(),
        3,
        "each UIKit list axis must report a visible-row change after a bound programmatic scroll\n{swift}"
    );
    assert_eq!(
        swift
            .matches("if isApplyingProgrammaticScrollPosition { return }")
            .count(),
        3,
        "vertical, horizontal, and grid delegates must ignore transient rows during controlled scrolls\n{swift}"
    );
    assert_eq!(
        swift
            .matches("isApplyingProgrammaticScrollPosition = true")
            .count(),
        3,
        "each controlled UIKit scroll must bracket its delegate callbacks before reporting the settled target\n{swift}"
    );
    assert_eq!(
        swift
            .matches("isIgnoringProgrammaticScrollCallbacks = true")
            .count(),
        3,
        "each UIKit list axis must keep delayed native callbacks from duplicating a controlled scroll notification\n{swift}"
    );
    assert_eq!(
        swift
            .matches("!isIgnoringProgrammaticScrollCallbacks")
            .count(),
        3,
        "every UIKit axis must exclude delayed controlled-scroll events from its scroll callback path\n{swift}"
    );
    assert!(
        swift
            .matches("isIgnoringProgrammaticScrollCallbacks = false")
            .count()
            >= 3,
        "each UIKit list axis must resume native scroll notifications when the user starts dragging\n{swift}"
    );
    assert_eq!(
        swift.matches("scrollCallbackGeneration += 1").count(),
        3,
        "each UIKit list axis must invalidate deferred callbacks when the controlled position changes\n{swift}"
    );
    assert_eq!(
        swift
            .matches("self.scrollCallbackGeneration == callbackGeneration")
            .count(),
        6,
        "delegate and settled-position callbacks on each UIKit axis must discard notifications queued before a controlled position change\n{swift}"
    );
    assert_eq!(
        swift
            .matches(
                "self.scrollCallbackGeneration == callbackGeneration,\n                      self.lastReportedScrollPosition == position else { return }"
            )
            .count(),
        3,
        "each UIKit programmatic callback must discard stale generations and positions before invoking actions\n{swift}"
    );
    assert!(
        swift.contains("reportProgrammaticScrollPosition(logicalPosition)"),
        "{swift}"
    );
    assert_eq!(
        swift
            .matches("reportProgrammaticScrollPosition(position)")
            .count(),
        2,
        "{swift}"
    );
    assert!(
        swift.contains(
            "let isScrollPositionEcho = previousScrollPosition != scrollPosition\n            && coordinator.scrollPositionEchoes.consume(scrollPosition)"
        ),
        "vertical UIKit lists must consume their own scroll-position updates instead of scrolling back to the reported row\n{swift}"
    );
    assert!(
        swift.contains(
            "if !isScrollPositionEcho {\n            coordinator.reconfigureVisibleRows(in: tableView)"
        ),
        "vertical UIKit lists must not reconfigure hosted cells for their own scroll-position state echo\n{swift}"
    );
    assert!(
        swift.contains(
            "guard force || (!tableView.isTracking && !tableView.isDragging && !tableView.isDecelerating) else {"
        ),
        "controlled scroll-position updates must not interrupt an active UIKit drag\n{swift}"
    );
    assert!(
        swift.contains("restoreRevision: Int = 0"),
        "the native list primitive must accept an internal page re-entry revision without changing the Nexa API\n{swift}"
    );
    assert!(
        swift.contains("private var didReportAttachedViewport = false")
            && swift
                .contains("let firstAttachedLayout = window != nil && !didReportAttachedViewport"),
        "UIKit lists must detect a newly attached page even when its bounds did not change\n{swift}"
    );
    let viewport_restore = swift
        .split("tableView.onViewportSizeChanged = {")
        .nth(1)
        .expect("vertical list should observe its attached viewport")
        .split("tableView.isPagingEnabled")
        .next()
        .expect("viewport callback should end before list configuration");
    assert!(
        viewport_restore.contains("guard tableView.window != nil, !pageSnap else { return }")
            && viewport_restore.contains("!tableView.isTracking")
            && viewport_restore.contains("!tableView.isDragging")
            && viewport_restore.contains("!tableView.isDecelerating")
            && viewport_restore.contains("coordinator.applyScrollPosition(to: tableView)")
            && !viewport_restore.contains("force: true")
            && viewport_restore.contains("coordinator.seedInitialScrollPosition(in: tableView)"),
        "a newly attached list must restore only while idle and seed its controlled position after it has a real viewport\n{swift}"
    );
    assert!(
        swift.contains("coordinator.restoreBoundScrollPositionIfNeeded(\n                in: tableView,\n                revision: restoreRevision\n            )"),
        "vertical UIKit lists must reconcile the viewport on an explicit pager re-entry when the bound position is unchanged\n{swift}"
    );
    let restoration = swift
        .split("func restoreBoundScrollPositionIfNeeded(in tableView: UITableView, revision: Int)")
        .nth(1)
        .expect("vertical list should implement explicit page re-entry restoration")
        .split("\n        func ")
        .next()
        .expect("restoration method should be followed by another coordinator method");
    assert!(
        restoration.contains("self.scrollPosition == scrollPosition"),
        "a delayed restoration must honor the latest controlled scroll-position binding\n{swift}"
    );
    assert!(
        !restoration.contains("lastReportedScrollPosition == Int(scrollPosition)"),
        "restoration must not be blocked by a stale UIKit callback position after the page became visible\n{swift}"
    );
    assert!(
        swift.contains("DispatchQueue.main.asyncAfter(deadline: .now() + 0.45)"),
        "restoration must wait for the pager's selection animation to finish\n{swift}"
    );
    assert!(
        swift.contains("self.restoreRevision == revision")
            && swift.contains("self.applyScrollPosition(to: tableView, force: true)"),
        "a delayed restore must still match the currently selected page revision before moving the list\n{swift}"
    );
    assert!(
        swift.contains("private var positions: [Int] = []"),
        "{swift}"
    );
    assert!(
        swift.contains("positions.lastIndex(of: position)"),
        "scroll-position echoes must survive intermediate SwiftUI updates until the matching state arrives\n{swift}"
    );
    assert!(
        swift.contains("positions.removeSubrange(0...echoIndex)"),
        "coalesced scroll-position echoes should be consumed through the latest matching state\n{swift}"
    );
    assert!(
        swift.contains("self.lastReportedScrollPosition == position"),
        "{swift}"
    );
    assert!(
        swift.contains("self.scrollPosition != Int32(position)"),
        "{swift}"
    );
    assert!(
        swift.contains("let positions = visibleRows.map(\\.row)")
            && swift.contains("return unobscuredPositions.min()"),
        "UIKit visible rows should be normalized and rows covered by a pinned header excluded before deriving callback positions\n{swift}"
    );
    assert!(
        swift.contains("if !isScrollPositionEcho { coordinator.applyScrollPosition"),
        "{swift}"
    );
    assert!(swift.contains("scrollPosition ?? 0"), "{swift}");
    assert!(
        swift.contains(
            "if !pageSnap && !reverseLayout {\n                coordinator.applyScrollPosition(to: tableView, force: true)\n            }\n            coordinator.seedInitialScrollPosition(in: tableView)"
        ),
        "vertical UIKit lists must restore a controlled position after the containing page gets its final geometry\n{swift}"
    );
    assert!(
        swift.contains("if pageSnap {\n                reportSettledPage(in: tableView)"),
        "{swift}"
    );
    assert!(swift.contains("scrollCallback?()"), "{swift}");
    assert!(swift.contains("scrollCallback?(page)"), "{swift}");
    assert!(swift.contains("$0.row >= rowCount - 1"), "{swift}");
    assert!(
        !swift.contains("else if page < rowCount - 1"),
        "endReached must not re-arm when paging away from the end; it is keyed to item count\n{swift}"
    );

    let scroll_delegate_positions = swift
        .match_indices("func scrollViewDidScroll(")
        .map(|(start, _)| start)
        .collect::<Vec<_>>();
    assert!(
        scroll_delegate_positions.len() >= 3,
        "expected vertical, horizontal, and grid UIKit scroll delegates; got {} in {swift}",
        scroll_delegate_positions.len()
    );
    for start in scroll_delegate_positions {
        let tail = &swift[start..];
        let method_end = tail
            .find("\n        func ")
            .expect("scroll delegate should be followed by another method");
        let method = &tail[..method_end];
        let dispatch = method
            .find("DispatchQueue.main.async {")
            .expect("UIKit delegate callbacks should be deferred until after the delegate returns");
        let before_dispatch = &method[..dispatch];
        assert!(
            !before_dispatch.contains("onScrollPositionChanged?("),
            "{method}"
        );
        assert!(!before_dispatch.contains("onScroll?()"), "{method}");
        assert!(!before_dispatch.contains("onEndReached()"), "{method}");
    }

    let kotlin_module = compile_file_with_warnings_for_target(&entry, Target::Kotlin)
        .expect("FastList callbacks should compile for Kotlin")
        .module;
    let kotlin = nexa_backend_kotlin::KotlinBackend.generate(&kotlin_module);
    assert!(kotlin.contains("snapshotFlow {"), "{kotlin}");
    assert!(
        kotlin.contains("distinctUntilChanged().collect { firstVisiblePosition ->"),
        "{kotlin}"
    );
    assert!(
        kotlin.contains("snapshotFlow { nexaListState2.isScrollInProgress to leadingVisibleDataItemIndex() }")
            && kotlin.contains("val previousScrollWasInProgress = nexaScrollWasInProgressnexaListState2[0]")
            && kotlin.contains("kotlin.math.abs(firstVisiblePosition - nexaPreviousScrollPositionnexaListState2).coerceAtLeast(1)")
            && kotlin.contains("repeat(nexaScrollCallbackCountnexaListState2) {")
            && kotlin.contains("nexaScrollWasInProgressnexaListState2[0] = isScrollInProgress"),
        "Android AOT should synthesize one callback for each leading row crossed during a user scroll\n{kotlin}"
    );
    assert!(kotlin.contains("nexaHasInitialScrollPosition"), "{kotlin}");
    assert!(kotlin.contains("nexaReportedScrollPosition"), "{kotlin}");
    assert!(
        kotlin.contains("if (nexaReportedScrollPosition"),
        "{kotlin}"
    );
    assert!(kotlin.contains("else if (target != "), "{kotlin}");
    assert!(kotlin.contains("= -1"), "{kotlin}");
    assert!(kotlin.contains("finalItemVisible"), "{kotlin}");
    assert!(kotlin.contains("nexaEndReached"), "{kotlin}");
    assert!(
        kotlin.contains("fun leadingVisibleDataItemIndex(): Int"),
        "Android callbacks should calculate their leading item independently of a pinned header item\n{kotlin}"
    );
    assert!(
        kotlin.contains("item.offset + item.size > stickyHeader.offset"),
        "Android callbacks should skip rows obscured by the pinned header\n{kotlin}"
    );
    assert!(
        kotlin.contains("visibleItems = layoutInfo.visibleItemsInfo.filter { it.offset + it.size > layoutInfo.viewportStartOffset && it.offset < layoutInfo.viewportEndOffset }"),
        "Android position callbacks must ignore lazy-list items retained just outside the viewport\n{kotlin}"
    );
    assert!(
        kotlin.contains("item.index > leadingIndex"),
        "reverse Android callbacks should choose the last in-viewport layout item, after filtering prefetched rows\n{kotlin}"
    );
    assert!(
        kotlin.contains("item.index >= nexaListCount1 + 0 - 1 && item.offset < layoutInfo.viewportEndOffset && item.offset + item.size > layoutInfo.viewportStartOffset"),
        "Android end detection without a sticky header should still require the final item to intersect the viewport\n{kotlin}"
    );
    assert!(
        kotlin.contains("item.index >= nexaListCount0 + 1 - 1 && item.offset < layoutInfo.viewportEndOffset && item.offset + item.size > layoutInfo.viewportStartOffset && (stickyHeader == null"),
        "Android end detection should only count an in-viewport final row that is not obscured by the pinned header\n{kotlin}"
    );
    assert!(
        kotlin.contains("by rememberSaveable(nexaListCount0) { mutableIntStateOf(-1) }"),
        "the end-reached marker must survive lazy pager page disposal/recreation\n{kotlin}"
    );
    assert!(
        kotlin.contains("import androidx.compose.runtime.saveable.rememberSaveable"),
        "the saved end-reached marker must use the Compose saveable state holder\n{kotlin}"
    );
    assert!(kotlin.contains("leadingVisibleDataItemIndex()"), "{kotlin}");
    assert!(
        kotlin.contains("(firstVisiblePosition - 1).coerceAtLeast(0)"),
        "the generated scrollPosition should remove the sticky-header slot after choosing an unobscured item\n{kotlin}"
    );
}
