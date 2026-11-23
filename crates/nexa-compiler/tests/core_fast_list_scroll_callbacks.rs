//! FastList scroll notifications must avoid re-entering native view updates.

use std::fs;

use nexa_codegen::Backend;
use nexa_compiler::{Target, compile_file_with_warnings_for_target};
use nexa_testkit::TestProject;

#[test]
fn ios_end_reached_callbacks_fire_again_after_leaving_the_end() {
    let runtime = include_str!("../../../runtime/ios/NexaDevRenderer.swift");
    let state = include_str!("../../../runtime/ios/NexaDevState.swift");
    let native = include_str!("../../nexa-backend-swift/src/generator/components/list_runtime.rs");
    assert!(!runtime.contains("NexaDevListPositionPreference"));
    assert!(!runtime.contains("NexaDevPageSnapList"));
    assert_eq!(
        native
            .matches("self.rowCount == count,\n                      self.isAtEnd,\n                      self.endReachedGeneration == generation")
            .count(),
        3,
        "shared UIKit list callbacks must discard stale edge notifications on every list layout"
    );
    assert!(
        !runtime.contains("store.claimFastListEndReached(")
            && !runtime.contains("store.updateFastListEndReachedCount(")
            && !state.contains("fastListEndReachedKeys"),
        "DevRuntime must rely on native edge tracking so a later end crossing can fire again"
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
    assert!(runtime.contains("onEndReached: onEndReachedActions.map { actions in"));
    assert!(runtime.contains("store.perform(actions, scope: scope, locals: callbackLocals)"));
}

#[test]
fn android_dev_runtime_keeps_scroll_observer_stable_across_recomposition() {
    let runtime = include_str!("../../../runtime/android/NexaDevRenderer.kt");
    assert!(
        runtime.contains("import androidx.compose.runtime.rememberUpdatedState")
            && runtime.contains("val currentPosition = rememberUpdatedState(onPosition)")
            && runtime.contains("val currentEndReached = rememberUpdatedState(onEndReached)"),
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
        runtime.contains("LaunchedEffect(listState, settledOnly)")
            && runtime.contains("position != lastVisiblePosition")
            && runtime.contains("currentPosition.value(position)")
            && runtime.contains("if (onEndReached != null && count > 0)")
            && runtime.contains("}.distinctUntilChanged().collect { reachedEnd ->")
            && runtime
                .contains("var wasAtEnd by rememberSaveable(count) { mutableStateOf(false) }")
            && runtime.contains("if (!wasAtEnd) currentEndReached.value()")
            && runtime.contains("wasAtEnd = false")
            && !runtime.contains("}.first { it }")
            && !runtime.contains("repeat(callbackCount) {"),
        "position observations report real transitions and end-reached fires once per visible edge"
    );
    assert!(runtime.contains("store.setScrollPositionFromNative("));
    assert!(runtime.contains("LaunchedEffect(listState, scrollPositionCommandRevision, count"));
    assert!(!runtime.contains("nativeScrollPositionEcho"));
    let state = include_str!("../../../runtime/android/NexaDevState.kt");
    assert!(state.contains("fun stateCommandRevision(name: String, scope: String = \"app\"): Int"));
    assert!(state.contains("expectedCommandRevision: Int"));
    assert!(state.contains("(stateCommandRevisions[target] ?: 0) != expectedCommandRevision"));
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
        swift.contains("page == rowCount - 1 && !callbackState.isAtEnd"),
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
            .count()
            + swift
                .matches("guard !isApplyingProgrammaticScrollPosition,")
                .count(),
        3,
        "vertical, horizontal, and grid delegates must ignore transient rows during controlled scrolls\n{swift}"
    );
    assert_eq!(
        swift
            .matches("isApplyingProgrammaticScrollPosition = true")
            .count(),
        3,
        "bound-index scrolling on each UIKit axis must bracket delegate callbacks\n{swift}"
    );
    assert_eq!(
        swift
            .matches("isApplyingProgrammaticScrollPosition = false")
            .count(),
        6,
        "each UIKit axis must clear its position guard after the synchronous scroll\n{swift}"
    );
    assert!(
        !swift.contains("isIgnoringProgrammaticScrollCallbacks"),
        "a position restore must not suppress a later ComponentRef scroll\n{swift}"
    );
    assert_eq!(
        swift
            .matches("coordinator.scrollCallbackGeneration += 1")
            .count(),
        6,
        "each UIKit list axis must invalidate deferred callbacks on a new command and when dismantled\n{swift}"
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
        swift.contains("reportProgrammaticScrollPosition(visiblePosition)"),
        "{swift}"
    );
    assert_eq!(
        swift
            .matches("reportProgrammaticScrollPosition(position)")
            .count(),
        5,
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
        swift.contains("DispatchQueue.main.asyncAfter(deadline: .now() + 0.45, execute: restore)"),
        "restoration must wait for the pager's selection animation to finish\n{swift}"
    );
    assert!(
        restoration.contains("self.restoreRevision == revision")
            && restoration.contains("tableView.dataSource === self")
            && restoration.contains("self.applyScrollPosition(to: tableView)"),
        "a delayed restore must still match the currently selected page revision before moving the list\n{swift}"
    );
    assert!(swift.contains("coordinator.cancelPendingScrollPositionRestore()"));
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
        swift.contains("return visibleRows.lazy.map(\\.row).min()")
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
        kotlin.contains("}.collect { firstVisiblePosition ->"),
        "{kotlin}"
    );
    assert!(
        kotlin.contains("snapshotFlow { leadingVisibleDataItemIndex() }")
            && kotlin.contains("firstVisiblePosition != lastVisiblePosition")
            && kotlin.contains("rememberUpdatedState {")
            && !kotlin.contains("nexaScrollCallbackCount"),
        "Android AOT should deliver observed row transitions through the latest closure"
    );
    assert!(kotlin.contains("nexaReportedScrollPosition"), "{kotlin}");
    assert!(
        kotlin.contains("if (nexaReportedScrollPosition"),
        "{kotlin}"
    );
    assert!(kotlin.contains("> 0 && target != ("), "{kotlin}");
    assert!(kotlin.contains("= -1"), "{kotlin}");
    assert!(
        kotlin.contains("}.distinctUntilChanged().collect { reachedEnd ->"),
        "{kotlin}"
    );
    assert!(
        !kotlin.contains("}.first { it }"),
        "end-reached observation must stay active so leaving and reaching the end again is observable\n{kotlin}"
    );
    assert!(kotlin.contains("nexaListEndIsVisible("), "{kotlin}");
    assert!(kotlin.contains("nexaEndReached"), "{kotlin}");
    assert!(
        kotlin.contains("fun leadingVisibleDataItemIndex(): Int"),
        "Android callbacks should calculate their leading item independently of a pinned header item\n{kotlin}"
    );
    assert!(
        kotlin.contains("hasStickyHeader && offset < stickyEnd && end > stickyStart"),
        "Android callbacks should skip rows obscured by the pinned header\n{kotlin}"
    );
    assert!(
        kotlin.contains("end <= viewportStartOffset || offset >= viewportEndOffset"),
        "Android position callbacks must ignore lazy-list items retained just outside the viewport\n{kotlin}"
    );
    assert!(
        kotlin.contains("val logicalPosition = firstVisiblePosition")
            && !kotlin.contains("reverseLayout && index > leadingIndex"),
        "reverse Android callbacks should report the leading source row at the bottom\n{kotlin}"
    );
    assert!(
        kotlin.contains("layoutInfo.visibleItemsInfo, nexaListCount1 + 0 - 1, false,")
            && kotlin
                .contains("if (end <= viewportStartOffset || start >= viewportEndOffset) continue"),
        "Android end detection without a sticky header should still require the final item to intersect the viewport\n{kotlin}"
    );
    assert!(
        kotlin.contains("layoutInfo.visibleItemsInfo, nexaListCount0 + 1 - 1, true,")
            && kotlin.contains(
                "if (end <= headerStart || start >= headerStart + itemSize(header)) return true"
            ),
        "Android end detection should only count an in-viewport final row that is not obscured by the pinned header\n{kotlin}"
    );
    assert!(
        kotlin.contains("nexaWasAtEndnexaListState0 by rememberSaveable(nexaListCount0) { mutableStateOf(false) }"),
        "the edge latch must survive pager page disposal and re-arm after leaving the end\n{kotlin}"
    );
    assert!(
        kotlin.contains(
            "if (!nexaWasAtEndnexaListState0) nexaEndReachedActionnexaListState0.value()"
        ) && kotlin.contains("nexaWasAtEndnexaListState0 = false"),
        "the saved edge latch should suppress repeated end callbacks while visible and reset after leaving\n{kotlin}"
    );
    assert!(kotlin.contains("import androidx.compose.runtime.saveable.rememberSaveable"));
    assert!(kotlin.contains("leadingVisibleDataItemIndex()"), "{kotlin}");
    assert!(
        kotlin.contains("(firstVisiblePosition - 1).coerceAtLeast(0)"),
        "the generated scrollPosition should remove the sticky-header slot after choosing an unobscured item\n{kotlin}"
    );
}

#[test]
fn fast_list_begin_drag_reports_only_user_drags_on_both_targets() {
    let project = TestProject::new("nexa-fast-list-begin-drag");
    let entry = project.join("App.nx");
    fs::write(
        &entry,
        r#"
app FastListBeginDrag {
    state dragCount: Int32 = 0
    body {
        FastList(count: 8) { index in
            Text("Timed lyric row \(index)")
        }.onScrollBeginDrag {
            dragCount = dragCount + 1
        }
    }
}
"#,
    )
    .expect("test source should be written");

    let swift_module = compile_file_with_warnings_for_target(&entry, Target::Swift)
        .expect("vertical FastList drag callbacks should compile for Swift")
        .module;
    let swift = nexa_backend_swift::SwiftBackend.generate(&swift_module);
    assert!(swift.contains("onScrollBeginDrag: {"), "{swift}");
    assert!(swift.contains("onScrollBeginDrag?()"), "{swift}");
    assert!(
        swift.contains("func scrollViewWillBeginDragging"),
        "{swift}"
    );

    let kotlin_module = compile_file_with_warnings_for_target(&entry, Target::Kotlin)
        .expect("vertical FastList drag callbacks should compile for Kotlin")
        .module;
    let kotlin = nexa_backend_kotlin::KotlinBackend.generate(&kotlin_module);
    assert!(
        kotlin.contains("import androidx.compose.foundation.interaction.collectIsDraggedAsState"),
        "{kotlin}"
    );
    assert!(
        kotlin.contains("interactionSource.collectIsDraggedAsState()"),
        "{kotlin}"
    );
    assert!(
        kotlin.contains("LaunchedEffect(nexaListIsDragged"),
        "{kotlin}"
    );
    let list_state = kotlin
        .find("val nexaListState0 = rememberLazyListState()")
        .expect("the vertical FastList state should be declared");
    let drag_observer = kotlin
        .find("nexaListState0.interactionSource.collectIsDraggedAsState()")
        .expect("the drag observer should use the declared list state");
    assert!(
        list_state < drag_observer,
        "AOT must declare the Compose list state before collecting drag interactions\n{kotlin}"
    );

    let android_dev_runtime = include_str!("../../../runtime/android/NexaDevRenderer.kt");
    assert!(
        android_dev_runtime.contains("listState.interactionSource.collectIsDraggedAsState()")
            && android_dev_runtime.contains("store.perform(onScrollBeginDrag, scope, locals)"),
        "Android DevRuntime should emit the same user-drag action from the list interaction source"
    );
    let ios_dev_runtime = include_str!("../../../runtime/ios/NexaDevRenderer.swift");
    assert!(
        ios_dev_runtime.contains("onScrollBeginDrag: onScrollBeginDrag")
            && ios_dev_runtime.contains("onScrollBeginDrag: onScrollBeginDragActions.map"),
        "iOS DevRuntime should emit one drag-start event per user gesture"
    );
}
