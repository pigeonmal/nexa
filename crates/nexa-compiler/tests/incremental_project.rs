use std::collections::HashMap;

use nexa_compiler::{IncrementalProjectCompiler, Target};
use nexa_testkit::TestProject;

#[test]
fn config_icon_aliases_resolve_in_component_icon_parameters_on_both_targets() {
    let project = TestProject::new("nexa-project-custom-icon-font");
    project.write("assets/fonts/Mazicon.ttf", b"test fixture font bytes");
    project.write_config(
        r#"config {
            assets {
                iconFonts { Mazicon: "assets/fonts/Mazicon.ttf" },
                icons { mazica_play: ["Mazicon", 61696] }
            }
        }"#,
    );
    let entry = project.write_app(
        r##"component PlayButton() {
            body { Button("Play", icon: "mazica_play") {} }
        }
        app Demo {
            state selected: String = "First"
            state selectedTab: Int32 = 0
            body {
                PlayButton()
                Picker(items: ["First"], selected: selected, icon: "mazica_play", itemIcons: ["mazica_play"])
                ContentUnavailable(title: "Empty", icon: "mazica_play")
                Icon(system: "mazica_play", description: "Play", size: 24, tint: "#FFFFFF")
                Pressable() { Text("Row") }
                    .onTap { }
                    .contextMenu { Button("Play", icon: "mazica_play") {} }
                AppBottomBar(selected: selectedTab) {
                    Tab(index: 0, label: "Play", icon: "mazica_play") { Text("Main") }
                }
            }
        }"##,
    );

    for target in [Target::Swift, Target::Kotlin] {
        let module = nexa_compiler::compile_file_for_target(&entry, target)
            .expect("project icon aliases should resolve during semantic lowering");
        let expected =
            nexa_ir::SystemIcon::font_glyph("Mazicon", 61696).expect("valid fixture glyph");
        let play_button = module
            .components
            .iter()
            .find(|component| component.name == "PlayButton")
            .expect("component lowered");
        assert!(matches!(
            play_button.body.as_slice(),
            [nexa_ir::Node::Button { icon: Some(icon), .. }] if *icon == expected
        ));
        assert!(module.body.iter().any(|node| matches!(
            node,
            nexa_ir::Node::Picker {
                icon: Some(icon),
                item_icons,
                ..
            } if *icon == expected && item_icons.as_slice() == std::slice::from_ref(&expected)
        )));
        assert!(module.body.iter().any(|node| matches!(
            node,
            nexa_ir::Node::ContentUnavailable { icon, .. } if *icon == expected
        )));
        assert!(module.body.iter().any(|node| matches!(
            node,
            nexa_ir::Node::SystemIcon { icon, .. } if *icon == expected
        )));
        assert!(module.body.iter().any(|node| matches!(
            node,
            nexa_ir::Node::Pressable { context_menu, .. }
                if matches!(context_menu.as_slice(), [nexa_ir::Node::Button { icon: Some(icon), .. }] if *icon == expected)
        )));
        assert!(matches!(
            module.body.iter().find(|node| matches!(node, nexa_ir::Node::AppBottomBar { .. })),
            Some(nexa_ir::Node::AppBottomBar { tabs, .. })
                if matches!(tabs.as_slice(), [tab] if tab.icon.as_ref() == Some(&expected))
        ));
        match target {
            Target::Swift => {
                let generated = nexa_backend_swift::SwiftBackend.generate_units(&module);
                assert!(generated.units.iter().any(|unit| {
                    unit.contents
                        .contains("NexaIconSpec(glyph: Int32(61696), fontFamily: \"Mazicon\")")
                }));
            }
            Target::Kotlin => {
                let generated =
                    nexa_codegen::Backend::generate(&nexa_backend_kotlin::KotlinBackend, &module);
                assert!(
                    generated
                        .contains("NexaIconSpec(glyph = 61696, fontResource = R.font.mazicon)")
                );
            }
            Target::All => unreachable!("test compiles one target at a time"),
        }
    }
}

#[test]
fn imported_file_scope_values_are_shared_with_persistence_functions_on_both_targets() {
    let project = TestProject::new("nexa-imported-file-scope-values");
    let entry = project.join("App.nx");
    project.write(
        "App.nx",
        "import \"storage/Comments.nx\"\napp Demo { body { Text(loadCommentCount()) } }\n",
    );
    project.write(
        "storage/Comments.nx",
        "let commentCount: Int32 = 1\nfn loadCommentCount() -> Int32 { return commentCount }\n",
    );

    for target in [Target::Swift, Target::Kotlin] {
        let module = nexa_compiler::compile_file_for_target(&entry, target)
            .expect("compile imported persistence module");
        assert_eq!(module.globals.len(), 1);
        assert_eq!(module.globals[0].name, "commentCount");
        assert!(!module.globals[0].mutable);
        let function = module
            .functions
            .iter()
            .find(|function| function.name == "loadCommentCount")
            .expect("imported persistence helper is available");
        assert!(matches!(
            function.body,
            nexa_ir::Expr::State(ref name, nexa_ir::Type::Numeric(nexa_ir::NumericType::Int32))
                if name == "commentCount"
        ));
    }
}

#[test]
fn unchanged_project_reuses_parsed_imports_and_reparses_only_changed_source() {
    let project = TestProject::new("nexa-incremental-project");
    let entry = project.join("App.nx");
    project.write(
        "App.nx",
        "import \"Card.nx\"\napp Demo { body { Card() } }\n",
    );
    project.write("Card.nx", "component Card() { body { Text(\"First\") } }\n");
    let mut compiler = IncrementalProjectCompiler::default();
    let plugins = HashMap::new();

    compiler
        .compile_file_with_warnings_for_targets_and_plugin_roots(&entry, &[Target::Swift], &plugins)
        .expect("compile initial project");
    assert_eq!(
        compiler.last_compile_stats(),
        nexa_compiler::ProjectCompileStats {
            parsed_source_files: 2,
            reused_source_files: 0,
        }
    );

    compiler
        .compile_file_with_warnings_for_targets_and_plugin_roots(&entry, &[Target::Swift], &plugins)
        .expect("compile unchanged project");
    assert_eq!(
        compiler.last_compile_stats(),
        nexa_compiler::ProjectCompileStats {
            parsed_source_files: 0,
            reused_source_files: 2,
        }
    );

    project.write(
        "Card.nx",
        "component Card() { body { Text(\"Updated\") } }\n",
    );
    compiler
        .compile_file_with_warnings_for_targets_and_plugin_roots(&entry, &[Target::Swift], &plugins)
        .expect("compile changed imported component");
    assert_eq!(
        compiler.last_compile_stats(),
        nexa_compiler::ProjectCompileStats {
            parsed_source_files: 1,
            reused_source_files: 1,
        }
    );
}

#[test]
fn removed_imports_are_pruned_from_incremental_source_cache() {
    let project = TestProject::new("nexa-incremental-project-prune");
    let entry = project.join("App.nx");
    project.write(
        "App.nx",
        "import \"Card.nx\"\napp Demo { body { Card() } }\n",
    );
    project.write("Card.nx", "component Card() { body { Text(\"Card\") } }\n");
    let mut compiler = IncrementalProjectCompiler::default();
    let plugins = HashMap::new();
    compiler
        .compile_file_with_warnings_for_targets_and_plugin_roots(&entry, &[Target::Swift], &plugins)
        .expect("compile imported project");

    project.write("App.nx", "app Demo { body { Text(\"No import\") } }\n");
    compiler
        .compile_file_with_warnings_for_targets_and_plugin_roots(&entry, &[Target::Swift], &plugins)
        .expect("compile project after removing import");
    assert_eq!(compiler.last_compile_stats().parsed_source_files, 1);

    project.write(
        "App.nx",
        "import \"Card.nx\"\napp Demo { body { Card() } }\n",
    );
    compiler
        .compile_file_with_warnings_for_targets_and_plugin_roots(&entry, &[Target::Swift], &plugins)
        .expect("compile project after restoring import");
    assert_eq!(
        compiler.last_compile_stats(),
        nexa_compiler::ProjectCompileStats {
            parsed_source_files: 2,
            reused_source_files: 0,
        }
    );
}

#[test]
fn imported_screen_modules_join_the_app_navigation_graph() {
    let project = TestProject::new("nexa-imported-screens");
    let entry = project.join("App.nx");
    project.write(
        "App.nx",
        "import \"screens/Home.nx\"\nimport \"screens/Details.nx\"\napp Demo { body { NavigationStack(root: Home) } }\n",
    );
    project.write(
        "screens/Home.nx",
        "screen Home { Column { Text(\"Home\") NavigationLink(destination: Details(42)) { Text(\"Open details\") } Button(\"Open with action\") { Navigation.push(Details(42)) } } }\n",
    );
    project.write(
        "screens/Details.nx",
        "screen Details(orderID: Int32) { Column { Text(orderID) Button(\"Return\") { Navigation.back() } } }\n",
    );

    let module = nexa_compiler::compile_file_for_target(&entry, Target::Swift)
        .expect("compile app with imported screens and typed push arguments");

    assert_eq!(
        module
            .screens
            .iter()
            .map(|screen| screen.name.as_str())
            .collect::<Vec<_>>(),
        ["Home", "Details"]
    );
    assert!(matches!(
        module.screens[0].body.first(),
        Some(nexa_ir::Node::Layout { children, .. })
            if children.iter().any(|node| matches!(
                node,
                nexa_ir::Node::NavigationLink {
                    destination: nexa_ir::ScreenId(1),
                    ..
                }
            ))
    ));
    assert!(matches!(
        module.screens[0].body.first(),
        Some(nexa_ir::Node::Layout { children, .. })
            if children.iter().any(|node| matches!(node, nexa_ir::Node::Button { actions, .. }
                if matches!(actions.as_slice(), [nexa_ir::Action::NavigationPush { destination: nexa_ir::ScreenId(1), arguments }]
                    if matches!(arguments.as_slice(), [nexa_ir::Expr::Number { raw, .. }] if raw == "42"))))
    ));
    assert!(matches!(
        module.screens[1].body.first(),
        Some(nexa_ir::Node::Layout { children, .. })
            if children.iter().any(|node| matches!(node, nexa_ir::Node::Button { actions, .. }
                if matches!(actions.as_slice(), [nexa_ir::Action::NavigationBack])))
    ));
}

#[test]
fn imported_network_failures_keep_their_nexa_source_location() {
    let project = TestProject::new("nexa-imported-network-source-location");
    let entry = project.join("App.nx");
    project.write(
        "App.nx",
        "import \"screens/Details.nx\"\napp Demo { body { NavigationStack(root: Details) } }\n",
    );
    project.write(
        "screens/Details.nx",
        "screen Details { state statusCode: Int32 = 0 OnAppear async { try { statusCode = (await Network.fetch(url: \"https://example.com\")).statusCode } catch { statusCode = 599 } } Text(statusCode) }\n",
    );

    let module = nexa_compiler::compile_file_for_target(&entry, Target::Swift)
        .expect("compile imported screen with an async request");
    let details = module
        .screens
        .iter()
        .find(|screen| screen.name == "Details")
        .expect("imported screen is available");
    let actions = details
        .on_appear
        .as_ref()
        .expect("screen keeps its async appear action");
    let mut location = None;
    nexa_ir::walk::walk_actions(actions, &mut |expression| {
        if let nexa_ir::Expr::Located {
            source_file,
            source_span,
            expression,
        } = expression
        {
            let mut contains_network_fetch = false;
            nexa_ir::walk::walk_expression(expression, &mut |nested| {
                contains_network_fetch |= matches!(nested, nexa_ir::Expr::NetworkFetch(_));
            });
            if contains_network_fetch {
                location = Some((source_file.clone(), (source_span.line, source_span.column)));
            }
        }
    });
    let (source_file, source_span) = location.expect("request is present in typed IR");
    assert!(source_file.ends_with("screens/Details.nx"));
    assert!(source_span.0 == 1 && source_span.1 > 1);
}

#[test]
fn hot_reload_can_add_imported_screens_and_tab_components() {
    let project = TestProject::new("nexa-hot-reload-modular-screens-tabs");
    let entry = project.join("App.nx");
    project.write("App.nx", "app Demo { body { Text(\"Initial\") } }\n");
    let mut compiler = IncrementalProjectCompiler::default();
    let targets = [Target::Swift, Target::Kotlin];
    let plugins = HashMap::new();

    compiler
        .compile_file_with_warnings_for_targets_and_plugin_roots(&entry, &targets, &plugins)
        .expect("compile initial app before adding modular sources");
    assert_eq!(compiler.last_compile_stats().parsed_source_files, 1);

    project.write(
        "App.nx",
        "import \"screens/Home.nx\"\nimport \"screens/Details.nx\"\nimport \"tabs/HomeTab.nx\"\nimport \"tabs/ProfileTab.nx\"\napp Demo { state selectedTab: Int32 = 0 state visits: Int32 = 0 body { NavigationStack(root: Home) } }\n",
    );
    project.write(
        "screens/Home.nx",
        "screen Home { Column { AppBottomBar(selected: selectedTab) { Tab(index: 0, label: \"Home\") { HomeTab() } Tab(index: 1, label: \"Profile\") { ProfileTab() } } NavigationLink(destination: Details) { Text(\"Open details\") } } }\n",
    );
    project.write(
        "screens/Details.nx",
        "screen Details { Column { Text(\"Visits\") Text(visits) } }\n",
    );
    project.write(
        "tabs/HomeTab.nx",
        "component HomeTab() { body { Text(\"Home tab from another file\") } }\n",
    );
    project.write(
        "tabs/ProfileTab.nx",
        "component ProfileTab() { body { Text(\"Profile tab from another file\") } }\n",
    );

    let compilations = compiler
        .compile_file_with_warnings_for_targets_and_plugin_roots(&entry, &targets, &plugins)
        .expect("hot reload app after adding imported screen and tab files");
    assert_eq!(compiler.last_compile_stats().parsed_source_files, 5);
    assert_eq!(compiler.last_compile_stats().reused_source_files, 0);
    assert_eq!(compilations.len(), 2);

    for compilation in compilations {
        let module = compilation.module;
        assert_eq!(
            module
                .screens
                .iter()
                .map(|screen| screen.name.as_str())
                .collect::<Vec<_>>(),
            ["Home", "Details"]
        );
        assert_eq!(
            module
                .components
                .iter()
                .map(|component| component.name.as_str())
                .collect::<Vec<_>>(),
            ["HomeTab", "ProfileTab"]
        );
        assert!(nexa_ir::walk::any_node(
            &module.screens[0].body,
            |node| matches!(
                node,
                nexa_ir::Node::AppBottomBar { tabs, .. }
                    if tabs.iter().flat_map(|tab| &tab.children).any(|child| matches!(
                        child,
                        nexa_ir::Node::ComponentCall { name, .. }
                            if name == "HomeTab" || name == "ProfileTab"
                    ))
            )
        ));
        assert!(nexa_ir::walk::any_node(
            &module.screens[0].body,
            |node| matches!(
                node,
                nexa_ir::Node::NavigationLink {
                    destination: nexa_ir::ScreenId(1),
                    ..
                }
            )
        ));
    }
}

#[test]
fn imported_screen_diagnostics_keep_their_source_file() {
    let project = TestProject::new("nexa-imported-screen-diagnostics");
    let entry = project.join("App.nx");
    project.write(
        "App.nx",
        "import \"screens/Home.nx\"\napp Demo { body { NavigationStack(root: Home) } }\n",
    );
    project.write("screens/Home.nx", "screen Home { Text(missingValue) }\n");

    let error = nexa_compiler::compile_file_for_target(&entry, Target::Swift)
        .expect_err("unknown screen value should fail semantic analysis");

    assert!(
        error
            .file
            .as_deref()
            .is_some_and(|file| file.ends_with("screens/Home.nx")),
        "unexpected diagnostic source: {:?}",
        error.file
    );
}
