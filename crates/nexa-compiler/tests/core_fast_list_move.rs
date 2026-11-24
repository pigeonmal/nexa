//! Reorder callbacks remain statically typed and select the platform list path automatically.

use std::fs;

use nexa_codegen::Backend;
use nexa_compiler::{Target, compile_file_with_warnings_for_target};
use nexa_ir::Node;
use nexa_testkit::TestProject;

#[test]
fn array_fast_list_move_generates_swiftui_and_compose_reordering() {
    let project = TestProject::new("nexa-fast-list-move");
    let entry = project.join("App.nx");
    fs::write(
        &entry,
        r#"
app ReorderExample {
    state values: Array<Int32> = [1, 2, 3]
    state canMove: Bool = true
    body {
        FastList(values) { value, index in
            Text(value)
        }.onMove(enabled: canMove) { from, to ->
            values.move(from, to)
        }.swipeActions(edge: Leading, allowsFullSwipe: true) {
            Button("Delete") { canMove = false }
        }
    }
}
"#,
    )
    .expect("test source should be written");

    for target in [Target::Swift, Target::Kotlin] {
        let module = compile_file_with_warnings_for_target(&entry, target)
            .expect("FastList move should compile for both targets without a mode flag")
            .module;
        let mut found_reorder = false;
        nexa_ir::walk::walk_ir(
            &module.body,
            &mut |node| {
                found_reorder |=
                    matches!(node, Node::FastList { plan } if plan.on_move().is_some());
            },
            &mut |_| {},
        );
        assert!(
            found_reorder,
            "move callback must remain present in typed IR"
        );

        let native = match target {
            Target::Swift => nexa_backend_swift::SwiftBackend.generate(&module),
            Target::Kotlin => nexa_backend_kotlin::KotlinBackend.generate(&module),
            Target::All => unreachable!("test targets are concrete platforms"),
        };
        let expected = match target {
            Target::Swift => ".onMove { sourceOffsets, insertionIndex in",
            Target::Kotlin => "NexaFastListReorderableItem(",
            Target::All => unreachable!("test targets are concrete platforms"),
        };
        assert!(native.contains(expected), "{native}");
        if target == Target::Kotlin {
            assert!(
                native.contains("detectDragGesturesAfterLongPress("),
                "shared helper uses Compose's long-press drag detector: {native}"
            );
            assert!(
                native.contains("rememberNexaFastListReorderState()"),
                "{native}"
            );
            assert!(
                native.contains("reorderState = nexaFastListReorderState0"),
                "{native}"
            );
            assert!(
                native.contains("onMove = { source, destination ->"),
                "{native}"
            );
            assert!(native.contains("nexa_to: Int = destination"), "{native}");
            assert!(
                !native.contains("withTimeoutOrNull(viewConfiguration.longPressTimeoutMillis)"),
                "{native}"
            );
            assert!(
                !native.contains("for (visibleIndex in visibleItems.indices)"),
                "{native}"
            );
            assert!(native.contains("NexaFastListSwipePrimitive("), "{native}");
            assert!(
                native.contains("val corner = androidx.compose.ui.unit.Dp(0.toFloat())"),
                "{native}"
            );
        } else {
            assert!(
                native.contains(
                    "NexaNativeListPrimitive(scrollEnabled: true, hideHorizontalScrollbar: false, hideVerticalScrollbar: false, rowCount:"
                ),
                "native list uses SwiftUI's native move callback: {native}"
            );
            assert!(
                native.contains(".moveDisabled(!(nexa_canMove))"),
                "{native}"
            );
            assert!(!native.contains("NexaFastListReorder"), "{native}");
            assert!(
                !native.contains("LongPressGesture(minimumDuration: 0.35)"),
                "{native}"
            );
            assert!(!native.contains(".onDrag {"), "{native}");
            assert!(!native.contains(".onDrop("), "{native}");
            assert!(
                native.contains("guard (nexa_canMove) else { return }"),
                "{native}"
            );
        }
        if target == Target::Kotlin {
            let reorder_item = native
                .find("NexaFastListReorderableItem(")
                .expect("the row should remain reorderable when enabled");
            let swipe_action = reorder_item
                + native[reorder_item..]
                    .find("NexaFastListSwipePrimitive(")
                    .expect("swipe actions should render inside the reorderable row");
            assert!(
                swipe_action > reorder_item,
                "Android swipe actions should remain available alongside reordering: {native}"
            );
        }
    }
}

#[test]
fn fast_list_swipe_actions_reject_undeclared_row_bindings() {
    let project = TestProject::new("nexa-fast-list-swipe-scope");
    let entry = project.join("App.nx");
    fs::write(
        &entry,
        r#"
app SwipeScopeExample {
    state values: Array<Int32> = [1, 2, 3]
    body {
        FastList(values) { value, index in
            Text(value)
        }.swipeActions {
            Button("Delete") { values.remove(index) }
        }
    }
}
"#,
    )
    .expect("test source should be written");

    let error = match compile_file_with_warnings_for_target(&entry, Target::Swift) {
        Err(error) => error,
        Ok(_) => panic!("swipe action blocks must declare row bindings they use"),
    };
    assert!(
        error.message.contains("unknown state `index`"),
        "undeclared row bindings should be out of scope: {error}"
    );
}

#[test]
fn filtered_array_subset_move_generates_for_both_native_targets() {
    let project = TestProject::new("nexa-fast-list-move-subset");
    let entry = project.join("App.nx");
    fs::write(
        &entry,
        r#"
struct Task { id: Int32, done: Bool }
app ReorderExample {
    state tasks: Array<Task> = []
    body {
        FastList(tasks.filter { task -> !task.done }, key: .id) { task, index in
            Text(task.id)
        }.onMove { from, to ->
            tasks.moveSubset(from, to, tasks.filter { task -> !task.done })
        }
    }
}
"#,
    )
    .expect("test source should be written");

    for target in [Target::Swift, Target::Kotlin] {
        let module = compile_file_with_warnings_for_target(&entry, target)
            .expect("filtered subset reordering should compile for both targets")
            .module;
        let native = match target {
            Target::Swift => nexa_backend_swift::SwiftBackend.generate(&module),
            Target::Kotlin => nexa_backend_kotlin::KotlinBackend.generate(&module),
            Target::All => unreachable!("test targets are concrete platforms"),
        };
        assert!(native.contains("nexaOriginalSubset"), "{native}");
        assert!(native.contains("nexaReorderedSubset"), "{native}");
        if target == Target::Kotlin {
            let placement_animation = native
                .lines()
                .find(|line| line.contains(".animateItem(placementSpec = if ("))
                .expect("stable keys keep placement animation attached during reorder");
            assert!(
                placement_animation.contains("androidx.compose.animation.core.spring<androidx.compose.ui.unit.IntOffset>() else null)"),
                "stable keys animate after the drag while keeping placement still during the drag: {placement_animation}"
            );
            assert!(native.contains("NexaFastListReorderableItem("), "{native}");
        }
    }
}

#[test]
fn fast_list_swipe_actions_take_gesture_priority_over_parent_page_pager() {
    let project = TestProject::new("nexa-fast-list-pager-swipe");
    let entry = project.join("App.nx");
    fs::write(
        &entry,
        r#"
app ReorderExample {
    state page: Int32 = 0
    state values: Array<Int32> = [1, 2, 3]
    state canMove: Bool = true
    body {
        PagePager(selected: page) {
            Tab(index: 0) {
                FastList(values) { value, index in
                    Text(value)
                }.onMove(enabled: canMove) { from, to ->
                    values.move(from, to)
                }.swipeActions(edge: Leading, allowsFullSwipe: true) {
                    value, index in
                    Button("Delete") { values.remove(index) }
                }
            }
            Tab(index: 1) { Text("Next page") }
        }
    }
}

"#,
    )
    .expect("test source should be written");

    for target in [Target::Swift, Target::Kotlin] {
        let module = compile_file_with_warnings_for_target(&entry, target)
            .expect("a page containing swipe actions should compile for both targets")
            .module;
        let native = match target {
            Target::Swift => nexa_backend_swift::SwiftBackend.generate(&module),
            Target::Kotlin => nexa_backend_kotlin::KotlinBackend.generate(&module),
            Target::All => unreachable!("test targets are concrete platforms"),
        };

        match target {
            Target::Swift => {
                assert!(native.contains("NexaFastListSwipePagerGestureBridge()"));
                assert!(native.contains("rowSwipeRecognizer = UIPanGestureRecognizer()"));
                assert!(native.contains("return abs(translation.x) > abs(translation.y)"));
                assert!(native.contains("rowSwipeRecognizer.cancelsTouchesInView = false"));
                assert!(native.contains("require(toFail: rowSwipeRecognizer)"));
                assert!(
                    native.contains("shouldRecognizeSimultaneouslyWith otherGestureRecognizer")
                );
                assert!(native.contains(".onMove { sourceOffsets, insertionIndex in"));
                assert!(native.contains(".moveDisabled(!(nexa_canMove))"));
                assert!(!native.contains("NexaFastListReorder"));
                assert!(
                    native.contains(".swipeActions(edge: .leading, allowsFullSwipe: true)"),
                    "{native}"
                );
            }
            Target::Kotlin => {
                assert!(native.contains("userScrollEnabled = !nexaSwipeRowGestureActive.value"));
                assert!(native.contains(
                    "LocalNexaFastListSwipePagerLock provides nexaSwipeRowGestureActive"
                ));
                assert!(native.contains("resetKey = ") && native.contains(".size"));
                assert!(native.contains("androidx.compose.runtime.remember(resetKey, edge)"));
                assert!(native.contains("NexaFastListSwipeGroup {"));
                assert!(
                    native.contains("androidx.compose.runtime.rememberUpdatedState(onFullSwipe)")
                );
                assert!(
                    native.contains(
                        "androidx.compose.runtime.LaunchedEffect(state, allowsFullSwipe)"
                    )
                );
                assert!(native.contains("state.animateTo(NexaFastListSwipePosition.Closed)"));
                assert!(native.contains("pagerLock.value = true"));
                assert!(native.contains("pagerLock.value = false"));
                assert!(native.contains("edge = NexaFastListSwipeEdge.Leading"));
                assert!(native.contains(".anchoredDraggable(state,"));
                assert!(native.contains("NexaFastListSwipePosition.Open at"));
                assert!(native.contains("clearAndSetSemantics { }"));
                assert!(!native.contains("SwipeToDismissBox"));
                assert!(native.contains("allowsFullSwipe = true"));
                assert!(native.contains("onClick = onClickAndCloseSwipe"));
                assert!(native.contains("viewConfiguration.touchSlop"));
                let reorder_item = native
                    .find("NexaFastListReorderableItem(")
                    .expect("the row should remain reorderable");
                let swipe_action = reorder_item
                    + native[reorder_item..]
                        .find("NexaFastListSwipePrimitive(")
                        .expect("swipe actions should render inside the reorderable row");
                assert!(
                    swipe_action > reorder_item,
                    "swipe actions should remain available alongside reordering: {native}"
                );
            }
            Target::All => unreachable!("test targets are concrete platforms"),
        }
    }
}

#[test]
fn fast_list_scroll_position_binding_tracks_the_visible_row_on_both_platforms() {
    let project = TestProject::new("nexa-fast-list-scroll-position-binding");
    let entry = project.join("App.nx");
    fs::write(
        &entry,
        r#"
app ScrollToExample {
    state selectedRow: Int32 = 0
    body {
        FastList(count: 24, scrollPosition: selectedRow) { index in
            Text(index)
        }
    }
}
"#,
    )
    .expect("test source should be written");

    for target in [Target::Swift, Target::Kotlin] {
        let module = compile_file_with_warnings_for_target(&entry, target)
            .expect("FastList scrollPosition should compile with a mutable Int32 binding")
            .module;
        let Node::FastList { plan } = &module.body[0] else {
            panic!("expected FastList node");
        };
        assert_eq!(plan.scroll_position(), Some("selectedRow"), "{plan:#?}");

        let native = match target {
            Target::Swift => nexa_backend_swift::SwiftBackend.generate(&module),
            Target::Kotlin => nexa_backend_kotlin::KotlinBackend.generate(&module),
            Target::All => unreachable!("test targets are concrete platforms"),
        };
        assert!(native.contains("selectedRow"), "{native}");
        match target {
            Target::Swift => assert!(
                native.contains("scrollPosition: nexa_selectedRow"),
                "{native}"
            ),
            Target::Kotlin => assert!(
                native.contains("initialFirstVisibleItemIndex = nexa_selectedRow.coerceIn(0,"),
                "{native}"
            ),
            Target::All => unreachable!("test targets are concrete platforms"),
        }
    }
}

#[test]
fn fast_list_masonry_ref_commands_generate_native_lazy_lists() {
    let project = TestProject::new("nexa-fast-list-masonry-ref-commands");
    let entry = project.join("App.nx");
    fs::write(
        &entry,
        r#"
app MasonryScrollExample {
    let listRef = ComponentRef<FastList>()
    body {
        Column {
            Button("Show final card") {
                listRef.scrollToIndex(index: 23)
            }
            FastList(count: 24, axis: Masonry(2), gap: 12, ref: listRef) { index in
                Text(index)
            }
        }
    }
}
"#,
    )
    .expect("test source should be written");

    for target in [Target::Swift, Target::Kotlin] {
        let module = compile_file_with_warnings_for_target(&entry, target)
            .expect("masonry and typed ref commands should compile")
            .module;
        let native = match target {
            Target::Swift => nexa_backend_swift::SwiftBackend.generate(&module),
            Target::Kotlin => nexa_backend_kotlin::KotlinBackend.generate(&module),
            Target::All => unreachable!("test targets are concrete platforms"),
        };

        match target {
            Target::Swift => {
                assert!(native.contains("masonry: true"), "{native}");
                assert!(native.contains("NexaFastMasonryLayout"), "{native}");
                assert!(native.contains("componentRef: nexa_listRef"), "{native}");
                assert!(!native.contains("NexaFastListCommands"), "{native}");
            }
            Target::Kotlin => {
                assert!(native.contains("LazyVerticalStaggeredGrid"), "{native}");
                assert!(native.contains("StaggeredGridCells.Fixed(2)"), "{native}");
                assert!(native.contains("items(\n"), "{native}");
                assert!(!native.contains("staggeredItems"), "{native}");
                assert!(native.contains("nexaFastListRef"), "{native}");
                assert!(!native.contains("nexaLastScrollCommand"), "{native}");
                assert!(!native.contains("nexaLastScrollOffset"), "{native}");
            }
            Target::All => unreachable!("test targets are concrete platforms"),
        }
    }
}

#[test]
fn fast_list_masonry_reorder_keeps_the_native_grid() {
    let source = r#"
app MasonryReorderExample {
    state cards: Array<String> = ["Overview", "Weekly priorities", "Design review"]
    body {
        FastList(cards, key: .self, axis: Masonry(2)) { card, index in
            Text(card)
        }.onMove { from, to ->
            cards.move(from, to)
        }
    }
}
"#;
    let project = TestProject::new("nexa-fast-list-masonry-reorder");
    let entry = project.join("App.nx");
    fs::write(&entry, source).expect("test source should be written");

    for target in [Target::Swift, Target::Kotlin] {
        let module = compile_file_with_warnings_for_target(&entry, target)
            .expect("keyed masonry should support onMove")
            .module;
        match target {
            Target::Swift => {
                let swift = nexa_backend_swift::SwiftBackend.generate(&module);
                assert!(swift.contains("onMove: nexaMasonryMove"));
                assert!(swift.contains("beginInteractiveMovementForItem(at:"));
                assert!(swift.contains("updateInteractiveMovementTargetPosition(location)"));
                assert!(swift.contains("endInteractiveMovement()"));
                assert!(swift.contains("moveItemAt sourceIndexPath"));
                assert!(swift.contains("geometry.moveItem(from: current, to: index)"));
                assert!(!swift.contains("performDropWith coordinator"));
                assert!(!swift.contains("NexaFastMasonryPlaceholderView"));
                assert!(!swift.contains("pendingMasonryMoveFrames"));
                assert!(!swift.contains("UICollectionViewDropDelegate"));
            }
            Target::Kotlin => {
                let kotlin = nexa_backend_kotlin::KotlinBackend.generate(&module);
                assert!(kotlin.contains("NexaFastStaggeredGridReorderableItem("));
                assert!(kotlin.contains("targetUnderDraggedItem(visible, dragged)"));
                assert!(kotlin.contains("currentOnMove.value(from, target)"));
                assert!(kotlin.contains("reorderState.activeIndex.intValue = target"));
                assert!(!kotlin.contains("masonryPreview"));
                assert!(!kotlin.contains("masonryReorderPreview"));
            }
            Target::All => unreachable!("test targets are concrete platforms"),
        }
    }
}
