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
        }.swipeActions {
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
            Target::Swift => "NexaFastListReorderableRow",
            Target::Kotlin => "awaitEachGesture {",
            Target::All => unreachable!("test targets are concrete platforms"),
        };
        assert!(native.contains(expected), "{native}");
        if target == Target::Kotlin {
            assert!(native.contains("val down = awaitFirstDown("), "{native}");
            assert!(native.contains("NexaFastListSwipePrimitive("), "{native}");
            assert!(native.contains("PointerEventPass.Initial"), "{native}");
            assert!(
                native.contains("withTimeoutOrNull(viewConfiguration.longPressTimeoutMillis)"),
                "{native}"
            );
            assert!(native.contains("change.consume()"), "{native}");
            let pointer_input = native
                .find(".pointerInput(")
                .expect("reorder item handles drag");
            let drag_layer = native
                .find(".graphicsLayer {")
                .expect("reorder item follows the finger");
            let z_index = native
                .find(".zIndex(if (isDragging) 1f else 0f)")
                .expect("the lazy item root raises the dragged row above siblings");
            let native_row = native
                .find("NexaNativeListRowPrimitive(position = nexa_index")
                .expect("native row remains inside the elevated lazy item");
            assert!(pointer_input < drag_layer && drag_layer < z_index && z_index < native_row);
            assert!(native.contains("finally {"), "{native}");
            assert!(native.contains("draggedCenter"), "{native}");
            assert!(
                native.contains("for (visibleIndex in visibleItems.indices)"),
                "{native}"
            );
            assert!(native.contains("targetIndex = item.index - 0"), "{native}");
            assert!(!native.contains("asSequence()"), "{native}");
            assert!(native.contains("nexa_to: Int = targetIndex"), "{native}");
            assert!(
                native.contains("val corner = androidx.compose.ui.unit.Dp(0.toFloat())"),
                "{native}"
            );
            assert!(native.contains("Box(modifier = Modifier.fillMaxWidth()"));
        } else {
            assert!(
                native
                    .contains("NexaNativeListPrimitive(scrollEnabled: true, reorderEnabled: true)"),
                "native list must install the no-icon iOS reorder session when onMove is present: {native}"
            );
            assert!(
                native.contains("LongPressGesture(minimumDuration: 0.35)"),
                "{native}"
            );
            assert!(native.contains("NexaFastListRowFramesKey.self"), "{native}");
            assert!(
                native.contains("session.update(to: drag.location"),
                "{native}"
            );
            assert!(!native.contains(".onDrag {"), "{native}");
            assert!(!native.contains(".onDrop("), "{native}");
            assert!(native.contains("NexaFastListReorderSession"), "{native}");
            assert!(native.contains("reorderEnabled: nexa_canMove"), "{native}");
            assert!(!native.contains("\\.editMode"), "{native}");
        }
        if target == Target::Kotlin {
            assert!(native.contains("if (nexa_canMove)"), "{native}");
        }
        if target == Target::Kotlin {
            let reorder_branch = native
                .find("if (nexa_canMove) {")
                .expect("enabled reordering branch should be generated");
            let reorder_branch_end = reorder_branch
                + native[reorder_branch..]
                    .find("} else {")
                    .expect("disabled mode branch should follow reordering")
                + "} else {".len();
            let swipe_action = native
                .find("NexaFastListSwipePrimitive(")
                .expect("swipe actions remain available when reordering is disabled");
            assert!(
                swipe_action > reorder_branch_end,
                "Android swipe actions must be outside the active reorder branch: {native}"
            );
        }
    }
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
            assert!(
                native.contains(".animateItem()"),
                "stable keys animate reordered rows: {native}"
            );
            let drag_layer = native
                .find(".graphicsLayer {")
                .expect("drag transform is generated");
            let z_index = native
                .find(".zIndex(if (isDragging) 1f else 0f)")
                .expect("the lazy item root raises the dragged row above siblings");
            let native_row = native
                .find("NexaNativeListRowPrimitive(position = nexa_index")
                .expect("native row remains inside the elevated lazy item");
            assert!(drag_layer < z_index && z_index < native_row, "{native}");
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
                }.swipeActions {
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
                assert!(native.contains("UILongPressGestureRecognizer("));
                assert!(native.contains("rowTouchRecognizer.minimumPressDuration = 0"));
                assert!(native.contains("rowTouchRecognizer.cancelsTouchesInView = false"));
                assert!(native.contains("require(toFail: rowTouchRecognizer)"));
                assert!(
                    native.contains("shouldRecognizeSimultaneouslyWith otherGestureRecognizer")
                );
                assert!(native.contains("reorderEnabled: nexa_canMove"));
                assert!(native.contains("LongPressGesture(minimumDuration: 0.35)"));
                assert!(native.contains(".sequenced(before: DragGesture(minimumDistance: 0"));
                assert!(!native.contains("\\.editMode"));
                assert!(native.contains(".swipeActions(edge: .trailing, allowsFullSwipe: false)"));
            }
            Target::Kotlin => {
                assert!(native.contains("userScrollEnabled = !nexaSwipeRowGestureActive.value"));
                assert!(native.contains(
                    "LocalNexaFastListSwipePagerLock provides nexaSwipeRowGestureActive"
                ));
                assert!(native.contains("resetKey = ") && native.contains(".size"));
                assert!(native.contains("androidx.compose.runtime.key(resetKey)"));
                assert!(
                    native.contains("androidx.compose.runtime.LaunchedEffect(state, resetKey)")
                );
                assert!(native.contains("state.reset()"));
                assert!(native.contains("pagerGestureLock.value = true"));
                assert!(native.contains("pagerGestureLock.value = false"));
                let reorder_branch = native
                    .find("if (nexa_canMove) {")
                    .expect("enabled reordering branch should be generated");
                let reorder_branch_end = reorder_branch
                    + native[reorder_branch..]
                        .find("} else {")
                        .expect("swipe mode should follow reordering")
                    + "} else {".len();
                let swipe_action = native
                    .find("NexaFastListSwipePrimitive(")
                    .expect("swipe actions should remain available with reordering off");
                assert!(
                    swipe_action > reorder_branch_end,
                    "swipe actions must be rendered only outside the active reorder branch: {native}"
                );
            }
            Target::All => unreachable!("test targets are concrete platforms"),
        }
    }
}

#[test]
fn fast_list_scroll_to_index_modifier_uses_the_native_position_binding_on_both_platforms() {
    let project = TestProject::new("nexa-fast-list-scroll-to-index-modifier");
    let entry = project.join("App.nx");
    fs::write(
        &entry,
        r#"
app ScrollToExample {
    state selectedRow: Int32 = 0
    body {
        FastList(count: 24) { index in
            Text(index)
        }.scrollToIndex(index: selectedRow)
    }
}
"#,
    )
    .expect("test source should be written");

    for target in [Target::Swift, Target::Kotlin] {
        let module = compile_file_with_warnings_for_target(&entry, target)
            .expect("FastList scrollToIndex should compile with a mutable Int32 position")
            .module;
        let Node::FastList { plan } = &module.body[0] else {
            panic!("expected FastList node");
        };
        assert_eq!(plan.scroll_position(), Some("selectedRow"));

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
                native.contains("initialFirstVisibleItemIndex = nexa_selectedRow.coerceAtLeast(0)"),
                "{native}"
            ),
            Target::All => unreachable!("test targets are concrete platforms"),
        }
    }
}

#[test]
fn fast_list_masonry_and_scroll_commands_generate_native_lazy_lists() {
    let project = TestProject::new("nexa-fast-list-masonry-scroll-commands");
    let entry = project.join("App.nx");
    fs::write(
        &entry,
        r#"
app MasonryScrollExample {
    state topCommand: Int32 = 0
    state endCommand: Int32 = 0
    state contentOffset: Float64 = 0
    body {
        FastList(count: 24, axis: Masonry(2), spacing: 12) { index in
            Text(index)
        }
            .scrollToTop(trigger: topCommand)
            .scrollToEnd(trigger: endCommand)
            .scrollToOffset(offset: contentOffset)
    }
}
"#,
    )
    .expect("test source should be written");

    for target in [Target::Swift, Target::Kotlin] {
        let module = compile_file_with_warnings_for_target(&entry, target)
            .expect("masonry and scroll commands should compile")
            .module;
        let native = match target {
            Target::Swift => nexa_backend_swift::SwiftBackend.generate(&module),
            Target::Kotlin => nexa_backend_kotlin::KotlinBackend.generate(&module),
            Target::All => unreachable!("test targets are concrete platforms"),
        };

        match target {
            Target::Swift => {
                assert!(native.contains("masonry: true"), "{native}");
                assert!(native.contains("NexaFastListCommands("), "{native}");
                assert!(native.contains("top: nexa_topCommand"), "{native}");
                assert!(native.contains("end: nexa_endCommand"), "{native}");
                assert!(
                    native.contains("offset: Double(nexa_contentOffset)"),
                    "{native}"
                );
                assert!(native.contains("NexaFastMasonryLayout"), "{native}");
            }
            Target::Kotlin => {
                assert!(native.contains("LazyVerticalStaggeredGrid"), "{native}");
                assert!(native.contains("StaggeredGridCells.Fixed(2)"), "{native}");
                assert!(native.contains("scrollToItem"), "{native}");
                assert!(native.contains("nexaLastScrollCommand"), "{native}");
                assert!(native.contains("nexaLastScrollOffset"), "{native}");
                assert!(native.contains("items as staggeredItems"), "{native}");
                assert!(native.contains("staggeredItems("), "{native}");
                assert!(
                    native.contains("import androidx.compose.ui.platform.LocalDensity"),
                    "{native}"
                );
            }
            Target::All => unreachable!("test targets are concrete platforms"),
        }
    }
}
