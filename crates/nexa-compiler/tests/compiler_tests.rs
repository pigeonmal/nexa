use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use nexa_ir::{
    Expr, Node, Type,
    walk::{walk_actions, walk_ir},
};

use nexa_compiler::{Target, compile, compile_file_with_warnings_for_target};

static TEMP_PROJECT_COUNTER: AtomicU64 = AtomicU64::new(0);

struct TempProject(PathBuf);

impl TempProject {
    fn new() -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after Unix epoch")
            .as_nanos();
        let sequence = TEMP_PROJECT_COUNTER.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "nexa-plugin-pruning-{}-{nonce}-{sequence}",
            std::process::id()
        ));
        fs::create_dir_all(&root).expect("temporary project should be created");
        Self(root)
    }
}

impl Drop for TempProject {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn unused_native_plugins_are_pruned_from_both_target_modules() {
    let project = TempProject::new();
    for (directory, id, service) in [
        ("used", "dev.example.used", "Used"),
        ("unused", "dev.example.unused", "Unused"),
    ] {
        let plugin = project.0.join(directory);
        fs::create_dir_all(&plugin).expect("plugin directory should be created");
        fs::write(
                plugin.join("plugin.config.nx"),
                format!(
                    "plugin {{ schema: 2 id: \"{id}\" version: \"1.0.0\" sources {{ native: \"native.nxid\" }} }}\n"
                ),
            )
            .expect("plugin manifest should be written");
        fs::write(
            plugin.join("native.nxid"),
            format!("service {service} {{ fn ping() }}\n"),
        )
        .expect("plugin contract should be written");
    }
    let entry = project.0.join("main.nx");
    fs::write(
            &entry,
            "plugin \"used\" as Used\nplugin \"unused\" as Unused\napp Demo { body { Button(\"Ping\") { Used.ping() } } }\n",
        )
        .expect("app source should be written");

    for target in [Target::Swift, Target::Kotlin] {
        let module = compile_file_with_warnings_for_target(&entry, target)
            .expect("app with both valid plugin contracts should compile")
            .module;
        assert_eq!(
            module
                .plugins
                .iter()
                .map(|plugin| plugin.namespace.as_str())
                .collect::<Vec<_>>(),
            ["Used"],
            "unused plugin declarations must not reach native project generation"
        );
    }
}

#[test]
fn nested_borrowed_components_keep_the_owner_instance_live() {
    let project = TempProject::new();
    let plugin = project.0.join("resource");
    fs::create_dir_all(&plugin).expect("plugin directory should be created");
    fs::write(
            plugin.join("plugin.config.nx"),
            "plugin { schema: 2 id: \"dev.example.resource\" version: \"1.0.0\" sources { native: \"native.nxid\" } }\n",
        )
        .expect("plugin manifest should be written");
    fs::write(
        plugin.join("native.nxid"),
        "native class Resource { init() fn play() fn dispose() }\n",
    )
    .expect("native contract should be written");

    let entry = project.0.join("main.nx");
    fs::write(
        &entry,
        r#"
            plugin "resource" as ResourcePlugin

            component Outer(resource: ResourcePlugin.Resource) {
                body { Inner(resource: resource) }
            }

            component Inner(resource: ResourcePlugin.Resource) {
                body { Button("Use") { resource.play() } }
            }

            app Demo {
                let resource = ResourcePlugin.Resource()
                body {
                    Button("Dispose") { resource.dispose() }
                    Outer(resource: resource)
                }
            }
            "#,
    )
    .expect("app source should be written");

    for target in [Target::Swift, Target::Kotlin] {
        let result = compile_file_with_warnings_for_target(&entry, target);
        let Err(error) = result else {
            panic!(
                "a parent callback cannot dispose an object held by nested views for {target:?}"
            );
        };
        assert!(
            error
                .to_string()
                .contains("is disposed in one callback and used in another"),
            "unexpected diagnostic: {error}"
        );
    }
}

#[test]
fn plugin_package_calls_lower_to_typed_instances_and_qualified_components() {
    let entry =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/plugins/video-player-demo.nx");

    for target in [Target::Swift, Target::Kotlin] {
        let module = compile_file_with_warnings_for_target(&entry, target)
            .expect("VideoPlayer example should lower for each target")
            .module;

        assert_eq!(module.plugins.len(), 1);
        assert_eq!(module.plugins[0].namespace, "VideoPlayer");
        assert_eq!(module.components.len(), 2);
        assert!(module.components.iter().all(|component| {
            component.parameters.iter().any(|parameter| {
                parameter.name == "player"
                    && matches!(
                        &parameter.ty,
                        Type::Plugin { namespace, name }
                            if namespace == "VideoPlayer" && name == "VideoPlayer"
                    )
            })
        }));
        let player_surface = module
            .components
            .iter()
            .find(|component| component.name == "PlayerSurface")
            .expect("the outer player wrapper should remain reachable");
        let [
            Node::ComponentCall {
                name, arguments, ..
            },
        ] = player_surface.body.as_slice()
        else {
            panic!("the outer wrapper should pass its native object to the nested component");
        };
        assert_eq!(name, "PlayerMedia");
        assert!(matches!(
            arguments.first(),
            Some((parameter, Expr::State(value, Type::Plugin { namespace, name })))
                if parameter == "player"
                    && value == "player"
                    && namespace == "VideoPlayer"
                    && name == "VideoPlayer"
        ));
        let player_instances = module
            .states
            .iter()
            .filter(|state| {
                matches!(
                    &state.ty,
                    Type::Plugin { namespace, name }
                        if namespace == "VideoPlayer" && name == "VideoPlayer"
                )
            })
            .count();
        assert_eq!(player_instances, 2);
        assert!(
            module
                .components
                .iter()
                .flat_map(|component| &component.states)
                .any(|state| state.name == "tapped"),
            "native component callback state must survive optimization"
        );
        assert!(
            module
                .states
                .iter()
                .any(|state| state.name == "secondTapped"),
            "app callback state must survive optimization"
        );

        let mut found_components = Vec::new();
        let mut component_event_handlers = Vec::new();
        let mut component_controls = Vec::new();
        let mut component_children = Vec::new();
        for nodes in std::iter::once(&module.body)
            .chain(module.components.iter().map(|component| &component.body))
        {
            walk_ir(
                nodes,
                &mut |node| {
                    if let nexa_ir::Node::NativeComponentCall {
                        namespace,
                        name,
                        arguments,
                        children,
                        event_handlers,
                        ..
                    } = node
                    {
                        found_components.push((namespace.clone(), name.clone()));
                        component_controls.push(arguments.iter().find_map(|(name, value)| {
                            if name == "controls" {
                                Some(matches!(value, Expr::Bool(true)))
                            } else {
                                None
                            }
                        }));
                        component_children.push(children.is_some());
                        component_event_handlers.push(
                            event_handlers
                                .iter()
                                .map(|handler| {
                                    (
                                        handler.property.clone(),
                                        matches!(
                                            handler.actions.as_slice(),
                                            [nexa_ir::Action::Assign {
                                                value: Expr::Bool(true),
                                                ..
                                            }]
                                        ),
                                    )
                                })
                                .collect::<Vec<_>>(),
                        );
                    }
                },
                &mut |_| {},
            );
        }
        assert_eq!(
            found_components,
            vec![
                ("VideoPlayer".to_owned(), "VideoView".to_owned()),
                ("VideoPlayer".to_owned(), "VideoView".to_owned()),
            ]
        );
        assert_eq!(component_controls, vec![Some(false), Some(true)]);
        assert_eq!(component_children, vec![true, true]);
        assert_eq!(component_event_handlers.len(), 2);
        assert!(component_event_handlers.iter().all(|handlers| {
            handlers.len() == 1 && handlers[0].0 == "onTapped" && handlers[0].1
        }));

        let handled_prepares = module
            .on_appear
            .as_ref()
            .expect("native preparation should run when the view appears")
            .iter()
            .find_map(|action| match action {
                nexa_ir::Action::TryCatch {
                    body,
                    error_catches,
                    catch_body,
                } => Some((body, error_catches, catch_body)),
                _ => None,
            })
            .expect("the example explicitly recovers from native preparation errors");
        assert_eq!(
                handled_prepares
                    .0
                    .iter()
                    .filter(|action| matches!(
                        action,
                        nexa_ir::Action::Expression(Expr::TryAwait(call))
                            if matches!(call.as_ref(), Expr::NativeCall { name, is_throwing: true, .. } if name == "prepare")
                    ))
                    .count(),
                2
            );
        assert!(handled_prepares.2.is_none());
        assert!(matches!(
            handled_prepares.1.as_slice(),
            [
                nexa_ir::ErrorCatchArm { variant, parameters, .. },
                nexa_ir::ErrorCatchArm {
                    variant: payload_variant,
                    parameters: payload_parameters,
                    ..
                }
            ] if variant == "invalidUrl"
                && parameters.is_empty()
                && payload_variant == "decodingFailed"
                && payload_parameters == &[("message".to_owned(), "message".to_owned(), Type::String)]
        ));

        let mut instance_calls = Vec::new();
        for state in &module.states {
            if let Expr::NativeCall {
                receiver: None,
                name,
                ..
            } = &state.initial
            {
                assert_eq!(name, "VideoPlayer");
            }
        }
        walk_ir(&module.body, &mut |_| {}, &mut |expression| {
            if let Expr::NativeCall {
                receiver: Some(_),
                name,
                ..
            } = expression
            {
                instance_calls.push(name.clone());
            }
        });
        if let Some(actions) = &module.on_appear {
            walk_actions(actions, &mut |expression| {
                if let Expr::NativeCall {
                    receiver: Some(_),
                    name,
                    ..
                } = expression
                {
                    instance_calls.push(name.clone());
                }
            });
        }
        assert!(instance_calls.iter().any(|name| name == "play"));
        assert!(instance_calls.iter().any(|name| name == "pause"));
        assert!(instance_calls.iter().any(|name| name == "prepare"));

        let event_subscriptions = module
            .on_appear
            .as_ref()
            .expect("event handlers are registered when the view appears")
            .iter()
            .filter_map(|action| match action {
                nexa_ir::Action::NativeEventSubscribe {
                    receiver: Expr::State(receiver, Type::Plugin { .. }),
                    property,
                    actions,
                    ..
                } => Some((receiver.as_str(), property.as_str(), actions.as_slice())),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(event_subscriptions.len(), 2);
        assert_eq!(event_subscriptions[0].0, "player1");
        assert_eq!(event_subscriptions[1].0, "player2");
        assert!(event_subscriptions.iter().all(|(_, property, handler)| {
            *property == "onEnded"
                && matches!(
                    handler,
                    [nexa_ir::Action::Assign {
                        value: Expr::Bool(true),
                        ..
                    }]
                )
        }));
    }
}

#[test]
fn compiles_result_type_and_try_propagation() {
    let source = r#"
app TestApp {
    enum AppError {
        NotFound,
        Unauthorized
    }

    fn fetch_code() -> Result<Int32, AppError> {
        return Ok(42);
    }

    fn compute() -> Result<Int32, AppError> {
        let code: Int32 = fetch_code()?;
        return Ok(code);
    }

    state status: String = "Ready"
    state res: Result<Int32, AppError> = Ok(0)

    body {
        Button(status) {
            res = compute();
        }
    }
}
"#;
    let module = compile(source).expect("Result and try propagation should compile successfully");
    assert_eq!(module.functions.len(), 2);
    assert!(matches!(
        module.functions[0].return_type,
        Type::Result(_, _)
    ));
    assert!(matches!(
        module.functions[1].return_type,
        Type::Result(_, _)
    ));
}
