//! Semantic and structural navigation tests for Navigation v2.
//!
//! Validates:
//! 1. Duplicate-screen navigation with typed payloads compile identically across AOT and Dev.
//! 2. Tab-isolated navigation stacks compile with explicit navigator scopes.
//! 3. Nested routes and deep-link payload contracts compile identically across targets.
//! 4. Screen inventory additions preserve route identity.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use nexa_codegen::Backend;
use nexa_compiler::{IncrementalProjectCompiler, Target};
use nexa_testkit::TestProject;

fn compile_both_modes(
    app_source: &str,
    project_name: &str,
) -> (serde_json::Value, serde_json::Value) {
    let project = TestProject::new(project_name);
    let entry = project.write_app(app_source);
    let roots: HashMap<String, std::path::PathBuf> = HashMap::new();

    let mut aot_compiler = IncrementalProjectCompiler::default();
    let aot_res = aot_compiler
        .compile_file_with_warnings_for_targets_and_plugin_roots(
            &entry,
            &[Target::Swift, Target::Kotlin],
            &roots,
        )
        .expect("AOT compilation should succeed");

    let mut dev_compiler = IncrementalProjectCompiler::default();
    let dev_res = dev_compiler
        .compile_dev_runtime_file_with_warnings_for_targets_and_plugin_roots(
            &entry,
            &[Target::Swift, Target::Kotlin],
            &roots,
        )
        .expect("Dev compilation should succeed");

    let aot_ir = serde_json::to_value(&aot_res[0].module).expect("serialize AOT module");
    let dev_ir = serde_json::to_value(&dev_res[0].module).expect("serialize Dev module");

    (aot_ir, dev_ir)
}

#[test]
fn navigation_duplicate_push_fixture_compiles_identically_across_modes() {
    let fixture_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("tests/fixtures/navigation-v2-duplicate-push.nx");
    let source = fs::read_to_string(&fixture_path).expect("read fixture");

    let (aot_ir, dev_ir) = compile_both_modes(&source, "nav-v2-dup-push");
    assert_eq!(
        aot_ir, dev_ir,
        "AOT and Dev canonical IR must be 100% equivalent for duplicate push"
    );
}

#[test]
fn navigation_tab_back_fixture_compiles_identically_across_modes() {
    let fixture_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("tests/fixtures/navigation-v2-tab-back.nx");
    let source = fs::read_to_string(&fixture_path).expect("read fixture");

    let (aot_ir, dev_ir) = compile_both_modes(&source, "nav-v2-tab-back");
    assert_eq!(
        aot_ir, dev_ir,
        "AOT and Dev canonical IR must be 100% equivalent for tab back"
    );
}

#[test]
fn navigation_deep_link_fixture_compiles_identically_across_modes() {
    let fixture_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("tests/fixtures/navigation-v2-deep-link.nx");
    let source = fs::read_to_string(&fixture_path).expect("read fixture");

    let (aot_ir, dev_ir) = compile_both_modes(&source, "nav-v2-deep-link");
    assert_eq!(
        aot_ir, dev_ir,
        "AOT and Dev canonical IR must be 100% equivalent for deep link"
    );
}

#[test]
fn navigation_graph_migration_fixture_compiles_identically_across_modes() {
    let fixture_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("tests/fixtures/navigation-v2-graph-migration.nx");
    let source = fs::read_to_string(&fixture_path).expect("read fixture");

    let (aot_ir, dev_ir) = compile_both_modes(&source, "nav-v2-migration");
    assert_eq!(
        aot_ir, dev_ir,
        "AOT and Dev canonical IR must be 100% equivalent for graph migration"
    );
}

#[test]
fn navigation_graph_contains_typed_routes_and_deterministic_navigators() {
    let fixture_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("tests/fixtures/navigation-v2-tab-back.nx");
    let source = fs::read_to_string(&fixture_path).expect("read fixture");

    let project = TestProject::new("nav-v2-graph-check");
    let entry = project.write_app(&source);
    let roots: HashMap<String, std::path::PathBuf> = HashMap::new();
    let mut compiler = IncrementalProjectCompiler::default();
    let res = compiler
        .compile_file_with_warnings_for_targets_and_plugin_roots(&entry, &[Target::Swift], &roots)
        .expect("compilation should succeed");

    let graph = res[0].module.navigation_graph();
    assert_eq!(graph.routes.len(), 3);
    assert!(
        graph
            .find_route(&nexa_ir::RouteKey::new("TabAHome"))
            .is_some()
    );
    assert!(
        graph
            .find_route(&nexa_ir::RouteKey::new("TabADetails"))
            .is_some()
    );
    assert!(
        graph
            .find_route(&nexa_ir::RouteKey::new("TabBHome"))
            .is_some()
    );

    assert_eq!(graph.navigators.len(), 2);
    assert_eq!(graph.navigators[0].id, nexa_ir::NavigatorId::tab(0));
    assert_eq!(
        graph.navigators[0].root_route,
        nexa_ir::RouteKey::new("TabAHome")
    );
    assert_eq!(graph.navigators[1].id, nexa_ir::NavigatorId::tab(1));
    assert_eq!(
        graph.navigators[1].root_route,
        nexa_ir::RouteKey::new("TabBHome")
    );
}

#[test]
fn nested_navigation_stacks_and_tab_stacks_produce_distinct_scoped_navigators_and_deterministic_dispatch()
 {
    let fixture_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("tests/fixtures/navigation-v2-tab-back.nx");
    let source = fs::read_to_string(&fixture_path).expect("read fixture");

    let project = TestProject::new("nav-v2-scoped-coordinator");
    let entry = project.write_app(&source);
    let roots: HashMap<String, std::path::PathBuf> = HashMap::new();
    let mut compiler = IncrementalProjectCompiler::default();
    let res = compiler
        .compile_file_with_warnings_for_targets_and_plugin_roots(&entry, &[Target::Swift], &roots)
        .expect("compilation should succeed");

    let graph = res[0].module.navigation_graph();
    let mut snapshot = nexa_ir::NavigationSnapshot::from_graph(&graph);

    let tab0 = nexa_ir::NavigatorId::tab(0);
    let tab1 = nexa_ir::NavigatorId::tab(1);

    assert_eq!(snapshot.navigators.get(&tab0).unwrap().entries.len(), 1);
    assert_eq!(snapshot.navigators.get(&tab1).unwrap().entries.len(), 1);

    // Push into Tab 0:
    let mut entry_counter = 0;
    let mut next_entry_id = || {
        entry_counter += 1;
        nexa_ir::EntryId::new(format!("entry_{entry_counter}"))
    };

    let push_transition = nexa_ir::reduce_navigation_command(
        &mut snapshot,
        nexa_ir::NavigationCommand::Push {
            navigator: tab0.clone(),
            route: nexa_ir::RouteKey::new("TabADetails"),
            arguments: vec![],
        },
        &mut next_entry_id,
    )
    .expect("push to tab 0 should succeed");

    assert_eq!(push_transition.affected_navigator, tab0);
    assert_eq!(snapshot.navigators.get(&tab0).unwrap().entries.len(), 2);
    // Crucial: Tab 1 must remain untouched!
    assert_eq!(snapshot.navigators.get(&tab1).unwrap().entries.len(), 1);

    // Pop Tab 0:
    let pop_transition = nexa_ir::reduce_navigation_command(
        &mut snapshot,
        nexa_ir::NavigationCommand::Pop {
            navigator: tab0.clone(),
        },
        &mut next_entry_id,
    )
    .expect("pop from tab 0 should succeed");

    assert_eq!(pop_transition.affected_navigator, tab0);
    assert_eq!(snapshot.navigators.get(&tab0).unwrap().entries.len(), 1);
    assert_eq!(snapshot.navigators.get(&tab1).unwrap().entries.len(), 1);

    // Attempting to pop Tab 0 again should fail with IllegalRootPop, preserving both stacks
    let pop_err = nexa_ir::reduce_navigation_command(
        &mut snapshot,
        nexa_ir::NavigationCommand::Pop {
            navigator: tab0.clone(),
        },
        &mut next_entry_id,
    );
    assert_eq!(pop_err, Err(nexa_ir::NavigationError::IllegalRootPop(tab0)));
    assert_eq!(snapshot.navigators.get(&tab1).unwrap().entries.len(), 1);
}

#[test]
fn nav_t01_duplicate_push_produces_isolated_entry_identities() {
    let app = nexa_ir::NavigatorId::root();
    let mut snapshot = nexa_ir::NavigationSnapshot {
        graph_version: 1,
        active_navigator: app.clone(),
        navigators: [(
            app.clone(),
            nexa_ir::NavigatorStackState {
                id: app.clone(),
                root_route: nexa_ir::RouteKey::new("HomeScreen"),
                entries: vec![nexa_ir::NavigationEntryState {
                    entry_id: nexa_ir::EntryId::new("root_entry"),
                    route: nexa_ir::RouteKey::new("HomeScreen"),
                    arguments: vec![],
                }],
            },
        )]
        .into_iter()
        .collect(),
    };

    let mut counter = 0;
    let mut next_entry_id = || {
        counter += 1;
        nexa_ir::EntryId::new(format!("entry_{counter}"))
    };

    // First push of Details(id=7)
    let t1 = nexa_ir::reduce_navigation_command(
        &mut snapshot,
        nexa_ir::NavigationCommand::Push {
            navigator: app.clone(),
            route: nexa_ir::RouteKey::new("Details"),
            arguments: vec![nexa_ir::Expr::Number {
                raw: "7".to_string(),
                ty: nexa_ir::NumericType::Int32,
            }],
        },
        &mut next_entry_id,
    )
    .expect("first push succeeds");

    // Second push of same Details(id=7)
    let t2 = nexa_ir::reduce_navigation_command(
        &mut snapshot,
        nexa_ir::NavigationCommand::Push {
            navigator: app.clone(),
            route: nexa_ir::RouteKey::new("Details"),
            arguments: vec![nexa_ir::Expr::Number {
                raw: "7".to_string(),
                ty: nexa_ir::NumericType::Int32,
            }],
        },
        &mut next_entry_id,
    )
    .expect("second push succeeds");

    let entry1_id = snapshot.navigators.get(&app).unwrap().entries[1]
        .entry_id
        .clone();
    let entry2_id = snapshot.navigators.get(&app).unwrap().entries[2]
        .entry_id
        .clone();
    assert_eq!(snapshot.navigators.get(&app).unwrap().entries.len(), 3);
    assert_ne!(
        entry1_id, entry2_id,
        "Duplicate pushes must have distinct EntryIds"
    );
    assert_eq!(t1.affected_navigator, app);
    assert_eq!(t2.affected_navigator, app);

    // Pop once removes only top entry (entry 2)
    let pop_t = nexa_ir::reduce_navigation_command(
        &mut snapshot,
        nexa_ir::NavigationCommand::Pop {
            navigator: app.clone(),
        },
        &mut next_entry_id,
    )
    .expect("pop succeeds");

    let entries_after_pop = &snapshot.navigators.get(&app).unwrap().entries;
    assert_eq!(entries_after_pop.len(), 2);
    assert_eq!(entries_after_pop[1].entry_id, entry1_id);
    assert_eq!(pop_t.affected_navigator, app);
}

#[test]
fn nav_t03_tab_histories_survive_switching_and_back_only_affects_active_tab() {
    let tab0 = nexa_ir::NavigatorId::tab(0);
    let tab1 = nexa_ir::NavigatorId::tab(1);

    let mut snapshot = nexa_ir::NavigationSnapshot {
        graph_version: 1,
        active_navigator: tab0.clone(),
        navigators: [
            (
                tab0.clone(),
                nexa_ir::NavigatorStackState {
                    id: tab0.clone(),
                    root_route: nexa_ir::RouteKey::new("Tab0Home"),
                    entries: vec![nexa_ir::NavigationEntryState {
                        entry_id: nexa_ir::EntryId::new("tab0_root"),
                        route: nexa_ir::RouteKey::new("Tab0Home"),
                        arguments: vec![],
                    }],
                },
            ),
            (
                tab1.clone(),
                nexa_ir::NavigatorStackState {
                    id: tab1.clone(),
                    root_route: nexa_ir::RouteKey::new("Tab1Home"),
                    entries: vec![nexa_ir::NavigationEntryState {
                        entry_id: nexa_ir::EntryId::new("tab1_root"),
                        route: nexa_ir::RouteKey::new("Tab1Home"),
                        arguments: vec![],
                    }],
                },
            ),
        ]
        .into_iter()
        .collect(),
    };

    let mut counter = 0;
    let mut next_entry_id = || {
        counter += 1;
        nexa_ir::EntryId::new(format!("entry_{counter}"))
    };

    // Push 2 screens into Tab 0 (depth 3)
    for i in 1..=2 {
        nexa_ir::reduce_navigation_command(
            &mut snapshot,
            nexa_ir::NavigationCommand::Push {
                navigator: tab0.clone(),
                route: nexa_ir::RouteKey::new(format!("Tab0Screen{i}")),
                arguments: vec![],
            },
            &mut next_entry_id,
        )
        .unwrap();
    }
    assert_eq!(snapshot.navigators.get(&tab0).unwrap().entries.len(), 3);
    assert_eq!(snapshot.navigators.get(&tab1).unwrap().entries.len(), 1);

    // Switch to Tab 1
    nexa_ir::reduce_navigation_command(
        &mut snapshot,
        nexa_ir::NavigationCommand::SelectTab {
            navigator: tab1.clone(),
        },
        &mut next_entry_id,
    )
    .unwrap();
    assert_eq!(snapshot.active_navigator, tab1);

    // Push 1 screen into Tab 1 (depth 2)
    nexa_ir::reduce_navigation_command(
        &mut snapshot,
        nexa_ir::NavigationCommand::Push {
            navigator: tab1.clone(),
            route: nexa_ir::RouteKey::new("Tab1Details"),
            arguments: vec![],
        },
        &mut next_entry_id,
    )
    .unwrap();
    assert_eq!(snapshot.navigators.get(&tab1).unwrap().entries.len(), 2);
    // Tab 0 must still be at depth 3!
    assert_eq!(snapshot.navigators.get(&tab0).unwrap().entries.len(), 3);

    // Pop Tab 1
    nexa_ir::reduce_navigation_command(
        &mut snapshot,
        nexa_ir::NavigationCommand::Pop {
            navigator: tab1.clone(),
        },
        &mut next_entry_id,
    )
    .unwrap();
    assert_eq!(snapshot.navigators.get(&tab1).unwrap().entries.len(), 1);
    assert_eq!(snapshot.navigators.get(&tab0).unwrap().entries.len(), 3);

    // Switch back to Tab 0 and verify depth 3 preserved
    nexa_ir::reduce_navigation_command(
        &mut snapshot,
        nexa_ir::NavigationCommand::SelectTab {
            navigator: tab0.clone(),
        },
        &mut next_entry_id,
    )
    .unwrap();
    assert_eq!(snapshot.active_navigator, tab0);
    assert_eq!(snapshot.navigators.get(&tab0).unwrap().entries.len(), 3);

    // Pop Tab 0 -> depth becomes 2
    nexa_ir::reduce_navigation_command(
        &mut snapshot,
        nexa_ir::NavigationCommand::Pop {
            navigator: tab0.clone(),
        },
        &mut next_entry_id,
    )
    .unwrap();
    assert_eq!(snapshot.navigators.get(&tab0).unwrap().entries.len(), 2);
}

#[test]
fn nav_t05_mutation_test_double_pop_and_wrong_tab_dispatch_rejected() {
    let tab0 = nexa_ir::NavigatorId::tab(0);
    let mut snapshot = nexa_ir::NavigationSnapshot {
        graph_version: 1,
        active_navigator: tab0.clone(),
        navigators: [(
            tab0.clone(),
            nexa_ir::NavigatorStackState {
                id: tab0.clone(),
                root_route: nexa_ir::RouteKey::new("Tab0Home"),
                entries: vec![nexa_ir::NavigationEntryState {
                    entry_id: nexa_ir::EntryId::new("tab0_root"),
                    route: nexa_ir::RouteKey::new("Tab0Home"),
                    arguments: vec![],
                }],
            },
        )]
        .into_iter()
        .collect(),
    };

    let mut next_entry_id = || nexa_ir::EntryId::new("id");

    // Root pop mutation fails
    let err = nexa_ir::reduce_navigation_command(
        &mut snapshot,
        nexa_ir::NavigationCommand::Pop {
            navigator: tab0.clone(),
        },
        &mut next_entry_id,
    );
    assert_eq!(err, Err(nexa_ir::NavigationError::IllegalRootPop(tab0)));

    // Unknown navigator mutation fails
    let unknown = nexa_ir::NavigatorId::tab(99);
    let err2 = nexa_ir::reduce_navigation_command(
        &mut snapshot,
        nexa_ir::NavigationCommand::Pop {
            navigator: unknown.clone(),
        },
        &mut next_entry_id,
    );
    assert_eq!(
        err2,
        Err(nexa_ir::NavigationError::NavigatorNotFound(unknown))
    );
}

#[test]
fn nav_t02_home_to_details_a_details_b_back_twice_preserves_root() {
    let nav = nexa_ir::NavigatorId::root();
    let mut snapshot = nexa_ir::NavigationSnapshot {
        graph_version: 1,
        active_navigator: nav.clone(),
        navigators: [(
            nav.clone(),
            nexa_ir::NavigatorStackState {
                id: nav.clone(),
                root_route: nexa_ir::RouteKey::new("Home"),
                entries: vec![nexa_ir::NavigationEntryState {
                    entry_id: nexa_ir::EntryId::new("home_root"),
                    route: nexa_ir::RouteKey::new("Home"),
                    arguments: vec![],
                }],
            },
        )]
        .into_iter()
        .collect(),
    };

    let mut counter = 0;
    let mut next_entry_id = || {
        counter += 1;
        nexa_ir::EntryId::new(format!("entry_{counter}"))
    };

    // Home -> Details A
    nexa_ir::reduce_navigation_command(
        &mut snapshot,
        nexa_ir::NavigationCommand::Push {
            navigator: nav.clone(),
            route: nexa_ir::RouteKey::new("DetailsA"),
            arguments: vec![],
        },
        &mut next_entry_id,
    )
    .unwrap();

    // Details A -> Details B
    nexa_ir::reduce_navigation_command(
        &mut snapshot,
        nexa_ir::NavigationCommand::Push {
            navigator: nav.clone(),
            route: nexa_ir::RouteKey::new("DetailsB"),
            arguments: vec![],
        },
        &mut next_entry_id,
    )
    .unwrap();

    let stack = snapshot.navigators.get(&nav).unwrap();
    assert_eq!(stack.entries.len(), 3);
    assert_eq!(
        stack.entries.last().unwrap().route,
        nexa_ir::RouteKey::new("DetailsB")
    );

    // Back once -> Details A
    let t1 = nexa_ir::reduce_navigation_command(
        &mut snapshot,
        nexa_ir::NavigationCommand::Pop {
            navigator: nav.clone(),
        },
        &mut next_entry_id,
    )
    .unwrap();
    assert_eq!(t1.new_top, Some(nexa_ir::EntryId::new("entry_1")));
    assert_eq!(snapshot.navigators.get(&nav).unwrap().entries.len(), 2);

    // Back twice -> Home (Root)
    let t2 = nexa_ir::reduce_navigation_command(
        &mut snapshot,
        nexa_ir::NavigationCommand::Pop {
            navigator: nav.clone(),
        },
        &mut next_entry_id,
    )
    .unwrap();
    assert_eq!(t2.new_top, Some(nexa_ir::EntryId::new("home_root")));
    assert_eq!(snapshot.navigators.get(&nav).unwrap().entries.len(), 1);

    // Third pop must be rejected (cannot drop root)
    let t3_err = nexa_ir::reduce_navigation_command(
        &mut snapshot,
        nexa_ir::NavigationCommand::Pop {
            navigator: nav.clone(),
        },
        &mut next_entry_id,
    );
    assert_eq!(t3_err, Err(nexa_ir::NavigationError::IllegalRootPop(nav)));
}

#[test]
fn nav_t04_and_t08_reset_and_pop_to_conformance() {
    let nav = nexa_ir::NavigatorId::root();
    let mut snapshot = nexa_ir::NavigationSnapshot {
        graph_version: 1,
        active_navigator: nav.clone(),
        navigators: [(
            nav.clone(),
            nexa_ir::NavigatorStackState {
                id: nav.clone(),
                root_route: nexa_ir::RouteKey::new("Home"),
                entries: vec![nexa_ir::NavigationEntryState {
                    entry_id: nexa_ir::EntryId::new("root_entry"),
                    route: nexa_ir::RouteKey::new("Home"),
                    arguments: vec![],
                }],
            },
        )]
        .into_iter()
        .collect(),
    };

    let mut counter = 0;
    let mut next_entry_id = || {
        counter += 1;
        nexa_ir::EntryId::new(format!("e_{counter}"))
    };

    // Push 3 screens
    for name in &["Screen1", "Screen2", "Screen3"] {
        nexa_ir::reduce_navigation_command(
            &mut snapshot,
            nexa_ir::NavigationCommand::Push {
                navigator: nav.clone(),
                route: nexa_ir::RouteKey::new(*name),
                arguments: vec![],
            },
            &mut next_entry_id,
        )
        .unwrap();
    }
    assert_eq!(snapshot.navigators.get(&nav).unwrap().entries.len(), 4);

    // PopTo Screen1 (entry "e_1", non-inclusive) -> stack becomes [root, Screen1]
    nexa_ir::reduce_navigation_command(
        &mut snapshot,
        nexa_ir::NavigationCommand::PopTo {
            navigator: nav.clone(),
            entry: nexa_ir::EntryId::new("e_1"),
            inclusive: false,
        },
        &mut next_entry_id,
    )
    .unwrap();
    assert_eq!(snapshot.navigators.get(&nav).unwrap().entries.len(), 2);
    assert_eq!(
        snapshot
            .navigators
            .get(&nav)
            .unwrap()
            .entries
            .last()
            .unwrap()
            .entry_id,
        nexa_ir::EntryId::new("e_1")
    );

    // PopTo invalid entry must fail with EntryNotFound (NAV-T16)
    let missing_entry = nexa_ir::EntryId::new("non_existent");
    let err = nexa_ir::reduce_navigation_command(
        &mut snapshot,
        nexa_ir::NavigationCommand::PopTo {
            navigator: nav.clone(),
            entry: missing_entry.clone(),
            inclusive: false,
        },
        &mut next_entry_id,
    );
    assert_eq!(
        err,
        Err(nexa_ir::NavigationError::EntryNotFound(missing_entry))
    );

    // Reset stack (NAV-T08) -> replaces entire stack with single new root
    let reset_transition = nexa_ir::reduce_navigation_command(
        &mut snapshot,
        nexa_ir::NavigationCommand::Reset {
            navigator: nav.clone(),
            route: nexa_ir::RouteKey::new("NewLanding"),
            arguments: vec![],
        },
        &mut next_entry_id,
    )
    .unwrap();
    assert_eq!(snapshot.navigators.get(&nav).unwrap().entries.len(), 1);
    assert_eq!(
        snapshot.navigators.get(&nav).unwrap().entries[0].route,
        nexa_ir::RouteKey::new("NewLanding")
    );
    assert_eq!(reset_transition.affected_navigator, nav);
}

#[test]
fn navigation_commands_generate_valid_native_swift_and_compose_output() {
    let module = nexa_ir::Module {
        widgets: vec![],
        app_name: "NavCmdApp".to_owned(),
        plugins: vec![],
        plugin_assets: vec![],
        enums: vec![],
        structs: vec![],
        functions: vec![],
        background_tasks: vec![],
        globals: vec![],
        states: vec![],
        screens: vec![
            nexa_ir::Screen {
                id: nexa_ir::ScreenId(0),
                name: "Home".to_owned(),
                parameters: vec![],
                states: vec![],
                body: vec![nexa_ir::Node::Button {
                    label: nexa_ir::Expr::String("GoDetails".to_owned()),
                    icon: None,
                    loading: None,
                    disabled: None,
                    style: None,
                    size: None,
                    shape: None,
                    tint: None,
                    glass: false,
                    accessibility_label: None,
                    actions: vec![
                        nexa_ir::Action::NavigationCommand(Box::new(
                            nexa_ir::NavigationCommand::Push {
                                navigator: nexa_ir::NavigatorId::new("app"),
                                route: nexa_ir::RouteKey::new("Details"),
                                arguments: vec![nexa_ir::Expr::String("item_123".to_owned())],
                            },
                        )),
                        nexa_ir::Action::NavigationCommand(Box::new(
                            nexa_ir::NavigationCommand::Pop {
                                navigator: nexa_ir::NavigatorId::new("app"),
                            },
                        )),
                        nexa_ir::Action::NavigationCommand(Box::new(
                            nexa_ir::NavigationCommand::PopTo {
                                navigator: nexa_ir::NavigatorId::new("app"),
                                entry: nexa_ir::EntryId::new("entry_abc"),
                                inclusive: false,
                            },
                        )),
                        nexa_ir::Action::NavigationCommand(Box::new(
                            nexa_ir::NavigationCommand::ReplaceTop {
                                navigator: nexa_ir::NavigatorId::new("app"),
                                route: nexa_ir::RouteKey::new("Details"),
                                arguments: vec![nexa_ir::Expr::String("item_456".to_owned())],
                            },
                        )),
                        nexa_ir::Action::NavigationCommand(Box::new(
                            nexa_ir::NavigationCommand::Reset {
                                navigator: nexa_ir::NavigatorId::new("app"),
                                route: nexa_ir::RouteKey::new("Details"),
                                arguments: vec![],
                            },
                        )),
                        nexa_ir::Action::NavigationCommand(Box::new(
                            nexa_ir::NavigationCommand::SelectTab {
                                navigator: nexa_ir::NavigatorId::new("tab_profile"),
                            },
                        )),
                        nexa_ir::Action::NavigationCommand(Box::new(
                            nexa_ir::NavigationCommand::OpenExternalRoute {
                                uri: "https://example.com/item".to_owned(),
                            },
                        )),
                    ],
                }],
                status_bar: None,
                on_appear: None,
                on_appear_async: false,
                on_disappear: None,
            },
            nexa_ir::Screen {
                id: nexa_ir::ScreenId(1),
                name: "Details".to_owned(),
                parameters: vec![nexa_ir::FunctionParameter {
                    name: "itemId".to_owned(),
                    ty: nexa_ir::Type::String,
                }],
                states: vec![],
                body: vec![],
                status_bar: None,
                on_appear: None,
                on_appear_async: false,
                on_disappear: None,
            },
        ],
        components: vec![],
        body: vec![nexa_ir::Node::NavigationStack {
            root: nexa_ir::ScreenId(0),
            arguments: vec![],
            show_navigation_bar: false,
            title_mode: nexa_ir::NavigationTitleMode::Inline,
            title_alignment: nexa_ir::NavigationTitleAlignment::Automatic,
        }],
        status_bar: None,
        direction: None,
        on_appear: None,
        on_appear_async: false,
        on_disappear: None,
        on_active: None,
        on_inactive: None,
        on_background: None,
    };

    let swift_code = nexa_backend_swift::SwiftBackend.generate(&module);
    assert!(swift_code.contains(
        "nexaNavigationPath.wrappedValue.append(NexaNavigationRoute.screen1(UUID(), \"item_123\"))"
    ));
    assert!(swift_code.contains("if !nexaNavigationPath.wrappedValue.isEmpty { nexaNavigationPath.wrappedValue.removeLast() }"));
    assert!(swift_code.contains("firstIndex(where: { \"\\($0.identity)\" == \"entry_abc\" })"));
    assert!(swift_code.contains("var identity: UUID"));
    assert!(swift_code.contains("nexaNavigationPath.wrappedValue.removeAll()"));
    assert!(swift_code.contains("NotificationCenter.default.post(name: NSNotification.Name(\"NexaSelectTab\"), object: \"tab_profile\")"));
    assert!(swift_code.contains(
        "if let url = URL(string: \"https://example.com/item\") { UIApplication.shared.open(url) }"
    ));

    let kotlin_code = nexa_backend_kotlin::KotlinBackend.generate(&module);
    assert!(kotlin_code.contains(
        "navController.navigate(\"nexa_screen_1\" + \"/\" + Uri.encode(\"item_123\".toString()))"
    ));
    assert!(kotlin_code.contains("navController.popBackStack()"));
    assert!(kotlin_code.contains("navController.popBackStack"));
    assert!(kotlin_code.contains("navController.navigate(\"nexa_screen_1\") { popUpTo(0) }"));
    assert!(kotlin_code.contains("NexaRuntime.selectTab(\"tab_profile\")"));
    assert!(kotlin_code.contains("navController.context.startActivity"));
}
