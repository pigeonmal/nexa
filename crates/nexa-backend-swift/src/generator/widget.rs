//! Target-owned WidgetKit generation.
//!
//! Widget declarations are lowered separately from the application root so
//! project generation can compile these units in an extension target.

use nexa_codegen::{GeneratedSources, SourceUnit, SourceWriter};
use nexa_ir::{LayoutKind, ListPlan, Module, Node, Widget, WidgetFamily};

use crate::generator::{
    components::{self, node_renderer},
    engine::{expressions, features::Features, utils::swift_string},
};

type WidgetConfigurationField<'a> = (&'a str, &'a str, Vec<&'a str>, &'a str);

pub(super) fn generate(
    module: &Module,
) -> Result<crate::WidgetGeneratedSources, crate::WidgetGenerationError> {
    for widget in &module.widgets {
        validate_widget(module, widget)?;
    }
    if module.widgets.is_empty() {
        return Ok(crate::WidgetGeneratedSources {
            sources: nexa_codegen::SourceUnits::new("swift").finish(),
            resources: vec![],
        });
    }
    let widget_modules = module
        .widgets
        .iter()
        .map(|widget| reachable_widget_module(module, widget))
        .collect::<Vec<_>>();
    let mut support_module = widget_modules[0].clone();
    for additional in widget_modules.iter().skip(1) {
        support_module.body.extend(additional.body.clone());
        for function in &additional.functions {
            if !support_module
                .functions
                .iter()
                .any(|current| current.name == function.name)
            {
                support_module.functions.push(function.clone());
            }
        }
        for global in &additional.globals {
            if !support_module
                .globals
                .iter()
                .any(|current| current.name == global.name)
            {
                support_module.globals.push(global.clone());
            }
        }
        support_module.states.extend(additional.states.clone());
    }
    let features = Features::analyze(&support_module);
    // Provider expressions are represented as synthetic state initializers
    // for feature analysis, but must not run at extension process startup.
    support_module.states.clear();
    let support = super::generate_with_analysis(&support_module, features);
    let mut imports = support
        .imports
        .lines()
        .filter(|line| line.starts_with("import "))
        .map(str::to_owned)
        .collect::<std::collections::BTreeSet<_>>();
    imports.insert("import WidgetKit".to_owned());
    for widget_module in &widget_modules {
        let features = Features::analyze(widget_module);
        for line in super::imports::render(&features).lines() {
            if line.starts_with("import ") {
                imports.insert(line.to_owned());
            }
        }
    }
    if module
        .widgets
        .iter()
        .any(|widget| widget.configuration.is_some())
    {
        imports.insert("import AppIntents".to_owned());
    }
    let mut output_units = support
        .units
        .into_iter()
        .filter(|unit| {
            unit.name != "NexaGenerated.swift" && unit.name != "NexaGenerated_list_runtime.swift"
        })
        .collect::<Vec<_>>();
    output_units.push(SourceUnit {
        name: "NexaWidgetFamily.swift".to_owned(),
        contents: "enum NexaWidgetFamily { case Small, Medium, Large, ExtraLarge }\n".to_owned(),
    });
    for (widget, widget_module) in module.widgets.iter().zip(widget_modules.iter()) {
        let features = Features::analyze(widget_module);
        let mut out = SourceWriter::new();
        render_widget(widget, widget_module, &features, &mut out);
        output_units.push(SourceUnit {
            name: format!("{}.swift", widget_type_name(&widget.name)),
            contents: out.finish(),
        });
    }
    let bundle = widget_bundle(&module.widgets);
    output_units.push(SourceUnit {
        name: "NexaWidgetBundle.swift".to_owned(),
        contents: bundle,
    });
    Ok(crate::WidgetGeneratedSources {
        sources: GeneratedSources {
            imports: format!("{}\n\n", imports.into_iter().collect::<Vec<_>>().join("\n")),
            preamble: support.preamble,
            units: output_units,
        },
        resources: vec![],
    })
}

fn validate_widget(module: &Module, widget: &Widget) -> Result<(), crate::WidgetGenerationError> {
    fn fail(widget: &Widget, feature: &str) -> crate::WidgetGenerationError {
        crate::WidgetGenerationError {
            widget: widget.name.clone(),
            message: format!("unsupported widget IR: {feature}"),
        }
    }
    fn nodes(widget: &Widget, children: &[Node]) -> Result<(), crate::WidgetGenerationError> {
        for node in children {
            match node {
                Node::Text { .. }
                | Node::Spacer
                | Node::Divider { .. }
                | Node::SystemIcon { .. } => {}
                Node::Layout { children, .. } => nodes(widget, children)?,
                Node::Accessibility { children, .. } => nodes(widget, children)?,
                Node::Link { url, children } => {
                    if !matches!(url, nexa_ir::Expr::String(_)) {
                        return Err(fail(widget, "Link URL must be a static string"));
                    }
                    nodes(widget, children)?;
                }
                Node::If {
                    then_body,
                    else_body,
                    transition,
                    ..
                } => {
                    if transition.is_some() {
                        return Err(fail(widget, "If transition"));
                    }
                    nodes(widget, then_body)?;
                    if let Some(body) = else_body {
                        nodes(widget, body)?;
                    }
                }
                Node::FastList {
                    plan: ListPlan::Count { common, .. } | ListPlan::Items { common, .. },
                } => {
                    if common.axis != nexa_ir::ListAxis::Vertical
                        || common.native
                        || common.reverse_layout
                        || common.page_snap
                        || common.item_extent.is_some()
                        || common.key.is_some()
                        || common.scroll_position.is_some()
                        || common.on_end_reached.is_some()
                        || common.on_scroll.is_some()
                        || common.on_move.is_some()
                        || common.swipe_actions.is_some()
                        || common.sticky_header.is_some()
                        || common.refresh.is_some()
                    {
                        return Err(fail(
                            widget,
                            "FastList options outside bounded vertical rows",
                        ));
                    }
                    nodes(widget, &common.children)?;
                }
                Node::FastList { .. } => return Err(fail(widget, "sectioned FastList")),
                _ => return Err(fail(widget, "node outside the widget-safe subset")),
            }
        }
        Ok(())
    }
    if widget.configuration.is_some() {
        widget_configuration_fields(widget, module)?;
    }
    nodes(widget, &widget.body)
}

fn reachable_widget_module(module: &Module, widget: &Widget) -> Module {
    let mut scoped = module.clone();
    scoped.app_name = format!("{}WidgetContent", widget.name);
    scoped.body = widget.body.clone();
    scoped.widgets.clear();
    scoped.states = vec![nexa_ir::State {
        name: "__nexaWidgetEntryProvider".to_owned(),
        ty: widget.entry_type.clone(),
        initial: widget.entry_provider.clone(),
        mutable: false,
    }];
    if let Some(placeholder) = &widget.placeholder_provider {
        scoped.states.push(nexa_ir::State {
            name: "__nexaWidgetPlaceholderProvider".to_owned(),
            ty: widget.entry_type.clone(),
            initial: placeholder.clone(),
            mutable: false,
        });
    }
    scoped.screens.clear();
    scoped.background_tasks.clear();
    scoped.on_appear = None;
    scoped.on_appear_async = false;
    scoped.on_disappear = None;
    scoped.on_active = None;
    scoped.on_inactive = None;
    scoped.on_background = None;
    let (functions, globals) = reachable_functions(module, widget);
    scoped.functions = functions;
    scoped.globals = globals;
    // App-owned components are never pulled into an extension target. The
    // frontend rejects ComponentCall from widget bodies until it can provide
    // explicit widget-safe component ownership.
    scoped.components.clear();
    scoped
}

fn reachable_functions(
    module: &Module,
    widget: &Widget,
) -> (Vec<nexa_ir::Function>, Vec<nexa_ir::State>) {
    use nexa_ir::walk::{walk_expression, walk_ir};
    use std::collections::{HashSet, VecDeque};

    let declared = module
        .functions
        .iter()
        .map(|function| function.name.as_str())
        .collect::<HashSet<_>>();
    let mut used = HashSet::<String>::new();
    let mut global_names = HashSet::<String>::new();
    {
        let mut collect = |expr: &nexa_ir::Expr| {
            walk_expression(expr, &mut |nested| match nested {
                nexa_ir::Expr::Call { name, .. } if declared.contains(name.as_str()) => {
                    used.insert(name.clone());
                }
                nexa_ir::Expr::State(name, _) | nexa_ir::Expr::AnimatedState(name, _) => {
                    global_names.insert(name.clone());
                }
                _ => {}
            });
        };
        collect(&widget.entry_provider);
        if let Some(placeholder) = &widget.placeholder_provider {
            collect(placeholder);
        }
        walk_ir(&widget.body, &mut |_| {}, &mut collect);
    }

    let mut pending = used.iter().cloned().collect::<VecDeque<_>>();
    while let Some(name) = pending.pop_front() {
        let Some(function) = module
            .functions
            .iter()
            .find(|function| function.name == name)
        else {
            continue;
        };
        let before = used.clone();
        for local in &function.locals {
            collect_function_refs(
                &local.initial,
                module,
                &declared,
                &mut used,
                &mut global_names,
            );
        }
        for local in &function.class_initializers {
            collect_function_refs(
                &local.initial,
                module,
                &declared,
                &mut used,
                &mut global_names,
            );
        }
        if let Some(actions) = &function.body_actions {
            nexa_ir::walk::walk_actions(actions, &mut |expression| {
                collect_function_refs(expression, module, &declared, &mut used, &mut global_names);
            });
        }
        collect_function_refs(
            &function.body,
            module,
            &declared,
            &mut used,
            &mut global_names,
        );
        pending.extend(used.difference(&before).cloned());
    }
    let functions = module
        .functions
        .iter()
        .filter(|function| used.contains(&function.name))
        .cloned()
        .collect::<Vec<_>>();
    let referenced_classes = functions
        .iter()
        .filter_map(|function| function.receiver.as_ref())
        .filter_map(|ty| match ty {
            nexa_ir::Type::Class { name, .. } => Some(name.as_str()),
            _ => None,
        })
        .collect::<HashSet<_>>();
    let globals = module
        .globals
        .iter()
        .filter(|global| {
            global_names.contains(&global.name)
                || global
                    .name
                    .split_once("::")
                    .is_some_and(|(class, _)| referenced_classes.contains(class))
        })
        .cloned()
        .collect();
    (functions, globals)
}

fn collect_function_refs(
    expression: &nexa_ir::Expr,
    module: &Module,
    declared: &std::collections::HashSet<&str>,
    used: &mut std::collections::HashSet<String>,
    globals: &mut std::collections::HashSet<String>,
) {
    nexa_ir::walk::walk_expression(expression, &mut |nested| match nested {
        nexa_ir::Expr::Call { name, .. } if declared.contains(name.as_str()) => {
            used.insert(name.clone());
        }
        nexa_ir::Expr::NativeCall {
            namespace, name, ..
        } if let Some(class_name) = namespace.strip_prefix("__NexaUserClass:") => {
            for function in &module.functions {
                if matches!(
                    &function.receiver,
                    Some(nexa_ir::Type::Class { name, .. }) if name == class_name
                ) && function.name.rsplit('.').next() == Some(name.as_str())
                {
                    used.insert(function.name.clone());
                }
            }
        }
        nexa_ir::Expr::State(name, _) | nexa_ir::Expr::AnimatedState(name, _) => {
            globals.insert(name.clone());
        }
        _ => {}
    });
}

fn widget_bundle(widgets: &[Widget]) -> String {
    let mut out = String::new();
    if widgets.iter().any(|widget| widget.configuration.is_some()) {
        out.push_str("@available(iOSApplicationExtension 17.0, *)\n");
    }
    out.push_str(
        "@main\nstruct NexaGeneratedWidgetBundle: WidgetBundle {\n    var body: some Widget {\n",
    );
    for widget in widgets {
        out.push_str(&format!("        {}()\n", widget_type_name(&widget.name)));
    }
    out.push_str("    }\n}\n");
    out
}

fn widget_type_name(name: &str) -> String {
    nexa_codegen::names::widget_name(name)
}

fn render_widget(widget: &Widget, module: &Module, features: &Features, out: &mut SourceWriter) {
    let widget_type = widget_type_name(&widget.name);
    let name = widget_type
        .strip_prefix("Nexa")
        .and_then(|name| name.strip_suffix("Widget"))
        .unwrap_or(&widget_type);
    let content_name = format!("{widget_type}Content");
    let entry_type = super::engine::types::swift_type(&widget.entry_type);
    let placeholder_provider = widget
        .placeholder_provider
        .as_ref()
        .unwrap_or(&widget.entry_provider);
    let configuration_type = widget
        .configuration
        .as_ref()
        .map(|configuration| super::engine::types::swift_type(&configuration.ty));
    out.line(format_args!("private struct {content_name}: View {{"));
    out.line(format_args!("    let entry: {entry_type}"));
    if let Some(ty) = &configuration_type {
        out.line(format_args!("    let configuration: {ty}"));
    }
    out.line(format_args!(
        "    @Environment(\\.widgetFamily) private var nativeFamily"
    ));
    out.line(format_args!("{}", "    var body: some View {"));
    out.line(format_args!("{}", "        let family: NexaWidgetFamily = switch nativeFamily { case .systemSmall: .Small; case .systemMedium: .Medium; case .systemLarge: .Large; default: .ExtraLarge }"));
    out.line(format_args!("        let nexa_entry = entry"));
    if configuration_type.is_some() {
        out.line(format_args!(
            "        let nexa_configuration = configuration"
        ));
    }
    out.line(format_args!("        let nexa_family = family"));
    render_body(&widget.body, module, features, 2, out);
    if !out.ends_with("\n") {
        out.push('\n');
    }
    out.line(format_args!("{}", "    }"));
    out.line(format_args!("{}", "}"));
    out.line(format_args!("{}", ""));

    if let Some(configuration) = &widget.configuration {
        render_configuration_intent(widget, module, configuration, out);
        out.line(format_args!(
            "{}",
            "@available(iOSApplicationExtension 17.0, *)"
        ));
    }
    let entry_name = format!("Nexa{name}TimelineEntry");
    let provider_name = format!("Nexa{name}TimelineProvider");
    out.line(format_args!(
        "private struct {entry_name}: TimelineEntry {{"
    ));
    out.line(format_args!("{}", "    let date: Date"));
    out.line(format_args!("    let value: {entry_type}"));
    if let Some(ty) = &configuration_type {
        out.line(format_args!("    let configuration: {ty}"));
    }
    out.line(format_args!("{}", "}"));
    out.line(format_args!("{}", ""));
    if let Some(configuration) = &widget.configuration {
        out.line(format_args!(
            "{}",
            "@available(iOSApplicationExtension 17.0, *)"
        ));
        render_app_intent_provider(
            widget,
            configuration,
            &provider_name,
            &entry_name,
            &entry_type,
            out,
        );
    } else {
        out.line(format_args!(
            "private struct {provider_name}: TimelineProvider {{"
        ));
        out.line(format_args!(
            "    func placeholder(in context: Context) -> {entry_name} {{"
        ));
        render_provider_family_binding("context.family", 2, out);
        out.line(format_args!(
            "        {entry_name}(date: Date(), value: {})",
            expressions::expression(placeholder_provider)
        ));
        out.line(format_args!("{}", "    }"));
        out.line(format_args!("    func getSnapshot(in context: Context, completion: @escaping ({entry_name}) -> Void) {{"));
        render_provider_family_binding("context.family", 2, out);
        if widget.entry_provider_async {
            out.line(format_args!("        Task {{"));
            render_provider_entry_value(widget, &entry_type, 3, out);
            out.line(format_args!(
                "            completion({entry_name}(date: Date(), value: value))"
            ));
            out.line(format_args!("        }}"));
        } else {
            render_provider_entry_value(widget, &entry_type, 2, out);
            out.line(format_args!(
                "        completion({entry_name}(date: Date(), value: value))"
            ));
        }
        out.line(format_args!("{}", "    }"));
        out.line(format_args!("    func getTimeline(in context: Context, completion: @escaping (Timeline<{entry_name}>) -> Void) {{"));
        render_provider_family_binding("context.family", 2, out);
        if widget.entry_provider_async {
            out.line(format_args!("        Task {{"));
            render_provider_entry_value(widget, &entry_type, 3, out);
            out.line(format_args!(
                "            let entry = {entry_name}(date: Date(), value: value)"
            ));
            out.line(format_args!("            completion(Timeline(entries: [entry], policy: .after(Date().addingTimeInterval({}))))", widget.refresh_seconds));
            out.line(format_args!("        }}"));
        } else {
            render_provider_entry_value(widget, &entry_type, 2, out);
            out.line(format_args!(
                "        let entry = {entry_name}(date: Date(), value: value)"
            ));
            out.line(format_args!("        completion(Timeline(entries: [entry], policy: .after(Date().addingTimeInterval({}))))", widget.refresh_seconds));
        }
        out.line(format_args!("{}", "    }"));
        out.line(format_args!("{}", "}"));
        out.line(format_args!("{}", ""));
    }

    if widget.configuration.is_some() {
        out.line(format_args!(
            "{}",
            "@available(iOSApplicationExtension 17.0, *)"
        ));
    }
    out.line(format_args!("public struct {widget_type}: Widget {{"));
    out.line(format_args!("    public let kind = \"{}\"", widget.name));
    out.line(format_args!("{}", "    public init() {}"));
    out.line(format_args!(
        "{}",
        "    public var body: some WidgetConfiguration {"
    ));
    if widget.configuration.is_some() {
        out.line(format_args!("        AppIntentConfiguration(kind: kind, intent: {widget_type}ConfigurationIntent.self, provider: {provider_name}()) {{ entry in"));
    } else {
        out.line(format_args!(
            "        StaticConfiguration(kind: kind, provider: {provider_name}()) {{ entry in"
        ));
    }
    out.line(format_args!(
        "{}",
        "            if #available(iOSApplicationExtension 17.0, *) {"
    ));
    if configuration_type.is_some() {
        out.line(format_args!(
            "                {content_name}(entry: entry.value, configuration: entry.configuration)"
        ));
    } else {
        out.line(format_args!(
            "                {content_name}(entry: entry.value)"
        ));
    }
    out.line(format_args!(
        "                    .containerBackground(.fill.tertiary, for: .widget)"
    ));
    out.line(format_args!("{}", "            } else {"));
    if configuration_type.is_some() {
        out.line(format_args!(
            "                {content_name}(entry: entry.value, configuration: entry.configuration)"
        ));
    } else {
        out.line(format_args!(
            "                {content_name}(entry: entry.value)"
        ));
    }
    out.line(format_args!(
        "                    .background(Color(.systemBackground))"
    ));
    out.line(format_args!("{}", "            }"));
    out.line(format_args!("{}", "        }"));
    let display_name = nexa_ir::localization::widget_display_name(widget);
    let description = nexa_ir::localization::widget_description(widget);
    out.line(format_args!(
        "        .configurationDisplayName(Text({}))",
        swift_string(&display_name)
    ));
    out.line(format_args!(
        "        .description(Text({}))",
        swift_string(&description)
    ));
    out.line(format_args!(
        "{}",
        "        .supportedFamilies(widgetFamilies)"
    ));
    out.line(format_args!("{}", "    }"));
    let fixed_families = widget
        .families
        .iter()
        .filter(|family| **family != WidgetFamily::ExtraLarge)
        .map(swift_family)
        .collect::<Vec<_>>();
    if widget.families.contains(&WidgetFamily::ExtraLarge) {
        out.line(format_args!(
            "    private var widgetFamilies: [WidgetFamily] {{"
        ));
        out.line(format_args!(
            "        var result: [WidgetFamily] = [{}]",
            fixed_families.join(", ")
        ));
        out.line(format_args!("{}", "        if #available(iOSApplicationExtension 17.0, *) { result.append(.systemExtraLarge) }"));
        out.line(format_args!("{}", "        return result"));
        out.line(format_args!("{}", "    }"));
    } else {
        out.line(format_args!(
            "    private var widgetFamilies: [WidgetFamily] {{ [{}] }}",
            fixed_families.join(", ")
        ));
    }
    out.line(format_args!("{}", "}"));
}

fn render_provider_entry_value(
    widget: &Widget,
    entry_type: &str,
    depth: usize,
    out: &mut SourceWriter,
) {
    let provider = expressions::expression(&widget.entry_provider);
    let placeholder = expressions::expression(
        widget
            .placeholder_provider
            .as_ref()
            .unwrap_or(&widget.entry_provider),
    );
    if widget.entry_provider_throws {
        out.line_at(depth, format_args!("let value: {entry_type}"));
        out.line_at(depth, format_args!("do {{"));
        out.line_at(depth + 1, format_args!("value = {provider}"));
        out.line_at(depth, format_args!("}} catch {{"));
        out.line_at(depth + 1, format_args!("value = {placeholder}"));
        out.line_at(depth, format_args!("}}"));
    } else {
        out.line_at(depth, format_args!("let value: {entry_type} = {provider}"));
    }
}

fn widget_configuration_fields<'a>(
    widget: &'a Widget,
    module: &'a Module,
) -> Result<Vec<WidgetConfigurationField<'a>>, crate::WidgetGenerationError> {
    let configuration =
        widget
            .configuration
            .as_ref()
            .ok_or_else(|| crate::WidgetGenerationError {
                widget: widget.name.clone(),
                message: "configuration metadata missing".to_owned(),
            })?;
    let (struct_name, fields) = match &configuration.ty {
        nexa_ir::Type::Struct { name, fields } => (name.as_str(), fields.as_slice()),
        _ => {
            return Err(crate::WidgetGenerationError {
                widget: widget.name.clone(),
                message: "configuration must be a struct".to_owned(),
            });
        }
    };
    let defaults = match &configuration.default { nexa_ir::Expr::Call { name, arguments, .. } if name == struct_name && arguments.len() == fields.len() => arguments, _ => return Err(crate::WidgetGenerationError { widget: widget.name.clone(), message: "configuration default must call its struct constructor with one enum case per field".to_owned() }) };
    let mut result = Vec::with_capacity(fields.len());
    for ((field, ty), default) in fields.iter().zip(defaults) {
        let nexa_ir::Type::Enum(enum_name) = ty else {
            return Err(crate::WidgetGenerationError {
                widget: widget.name.clone(),
                message: format!("configuration field `{field}` must use an enum"),
            });
        };
        let enum_decl = module
            .enums
            .iter()
            .find(|declaration| declaration.name == *enum_name)
            .ok_or_else(|| crate::WidgetGenerationError {
                widget: widget.name.clone(),
                message: format!("configuration enum `{enum_name}` is missing"),
            })?;
        let nexa_ir::Expr::EnumValue {
            enum_name: default_type,
            case_name,
        } = default
        else {
            return Err(crate::WidgetGenerationError {
                widget: widget.name.clone(),
                message: format!("configuration default for `{field}` must be an enum case"),
            });
        };
        if default_type != enum_name || !enum_decl.cases.iter().any(|case| case == case_name) {
            return Err(crate::WidgetGenerationError {
                widget: widget.name.clone(),
                message: format!("invalid default enum case `{case_name}` for `{field}`"),
            });
        }
        result.push((
            struct_name,
            field.as_str(),
            enum_decl.cases.iter().map(String::as_str).collect(),
            case_name.as_str(),
        ));
    }
    Ok(result)
}

fn render_configuration_intent(
    widget: &Widget,
    module: &Module,
    configuration: &nexa_ir::WidgetConfiguration,
    out: &mut SourceWriter,
) {
    let widget_type = widget_type_name(&widget.name);
    let intent = format!("{widget_type}ConfigurationIntent");
    let nexa_ir::Type::Struct {
        name: config_name,
        fields,
    } = &configuration.ty
    else {
        return;
    };
    let Ok(config_fields) = widget_configuration_fields(widget, module) else {
        return;
    };
    for ((field, ty), (struct_name, _, cases, default)) in fields.iter().zip(&config_fields) {
        let nexa_ir::Type::Enum(enum_name) = ty else {
            continue;
        };
        let adapter = format!("{widget_type}{}Option", pascal_field(field));
        let domain_enum = nexa_codegen::names::enum_name(enum_name);
        out.line(format_args!(
            "{}",
            "@available(iOSApplicationExtension 17.0, *)"
        ));
        out.line(format_args!("private enum {adapter}: String, AppEnum {{"));
        for case in cases {
            out.line(format_args!("    case {case} = \"{case}\""));
        }
        out.line(format_args!(
            "    static let typeDisplayRepresentation = TypeDisplayRepresentation(name: LocalizedStringResource({}))",
            swift_string(&nexa_ir::localization::humanize_identifier(field))
        ));
        out.line(format_args!(
            "{}",
            "    static let caseDisplayRepresentations: [Self: DisplayRepresentation] = ["
        ));
        for case in cases {
            out.line(format_args!(
                "            .{case}: DisplayRepresentation(title: LocalizedStringResource({})),",
                swift_string(&nexa_ir::localization::humanize_identifier(case))
            ));
        }
        out.line(format_args!("{}", "    ]"));
        out.line(format_args!(
            "    var nexaValue: {domain_enum} {{ switch self {{"
        ));
        for case in cases {
            out.line(format_args!("        case .{case}: .{case}"));
        }
        out.line(format_args!("{}", "    } }"));
        out.line(format_args!("{}", "}"));
        let _ = struct_name;
        let _ = default;
    }
    out.line(format_args!("@available(iOSApplicationExtension 17.0, *)\nprivate struct {intent}: WidgetConfigurationIntent {{"));
    let title = nexa_ir::localization::widget_configuration_title(widget);
    out.line(format_args!(
        "    static let title: LocalizedStringResource = LocalizedStringResource({})",
        swift_string(&title)
    ));
    if let Some(description) = &widget.configuration_description {
        out.line(format_args!(
            "    static let description = IntentDescription(LocalizedStringResource({}))",
            swift_string(description)
        ));
    }
    for (field, ty) in fields {
        let nexa_ir::Type::Enum(_) = ty else {
            continue;
        };
        let adapter = format!("{widget_type}{}Option", pascal_field(field));
        out.line(format_args!(
            "    @Parameter(title: LocalizedStringResource({})) var {}: {adapter}",
            swift_string(&nexa_ir::localization::humanize_identifier(field)),
            nexa_codegen::names::struct_field_name(field)
        ));
    }
    out.line(format_args!("{}", "    init() {"));
    for (field, ty) in fields {
        let nexa_ir::Type::Enum(_) = ty else {
            continue;
        };
        let native_field = nexa_codegen::names::struct_field_name(field);
        let adapter = format!("{widget_type}{}Option", pascal_field(field));
        if let Some((_, _, _, default)) = config_fields
            .iter()
            .find(|(_, field_name, _, _)| *field_name == field)
        {
            out.line(format_args!(
                "        self.{native_field} = {adapter}(rawValue: \"{default}\")!"
            ));
        }
    }
    out.line(format_args!("{}", "    }"));
    out.line(format_args!("{}", "}"));
    out.line(format_args!("{}", ""));
    let _ = config_name;
}

fn render_app_intent_provider(
    widget: &Widget,
    configuration: &nexa_ir::WidgetConfiguration,
    provider_name: &str,
    entry_name: &str,
    entry_type: &str,
    out: &mut SourceWriter,
) {
    let widget_type = widget_type_name(&widget.name);
    let intent = format!("{widget_type}ConfigurationIntent");
    let nexa_ir::Type::Struct { fields, .. } = &configuration.ty else {
        return;
    };
    let config_type = super::engine::types::swift_type(&configuration.ty);
    let constructor = fields
        .iter()
        .map(|(field, _)| {
            format!(
                "{}: intent.{}.nexaValue",
                nexa_codegen::names::struct_field_name(field),
                nexa_codegen::names::struct_field_name(field)
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    let defaults = expressions::expression(&configuration.default);
    out.line(format_args!(
        "private struct {provider_name}: AppIntentTimelineProvider {{"
    ));
    out.line(format_args!("    typealias Intent = {intent}"));
    out.line(format_args!("    typealias Entry = {entry_name}"));
    out.line(format_args!(
        "    func placeholder(in context: Context) -> Entry {{"
    ));
    out.line(format_args!(
        "        let configuration: {config_type} = {defaults}"
    ));
    out.line(format_args!(
        "        let nexa_configuration = configuration"
    ));
    render_provider_family_binding("context.family", 2, out);
    out.line(format_args!(
        "        return Entry(date: Date(), value: {}, configuration: configuration)",
        expressions::expression(
            widget
                .placeholder_provider
                .as_ref()
                .unwrap_or(&widget.entry_provider)
        )
    ));
    out.line(format_args!("    }}"));
    out.line(format_args!(
        "{}",
        "    func snapshot(for intent: Intent, in context: Context) async -> Entry {"
    ));
    out.line(format_args!(
        "        let configuration = {config_type}({constructor})"
    ));
    out.line(format_args!(
        "        let nexa_configuration = configuration"
    ));
    render_provider_family_binding("context.family", 2, out);
    render_provider_entry_value(widget, entry_type, 2, out);
    out.line(format_args!(
        "        return Entry(date: Date(), value: value, configuration: configuration)"
    ));
    out.line(format_args!("{}", "    }"));
    out.line(format_args!(
        "{}",
        "    func timeline(for intent: Intent, in context: Context) async -> Timeline<Entry> {"
    ));
    out.line(format_args!(
        "        let configuration = {config_type}({constructor})"
    ));
    out.line(format_args!(
        "        let nexa_configuration = configuration"
    ));
    render_provider_family_binding("context.family", 2, out);
    render_provider_entry_value(widget, entry_type, 2, out);
    out.line(format_args!(
        "        let entry = Entry(date: Date(), value: value, configuration: configuration)"
    ));
    out.line(format_args!(
        "        return Timeline(entries: [entry], policy: .after(Date().addingTimeInterval({})))",
        widget.refresh_seconds
    ));
    out.line(format_args!("{}", "    }"));
    out.line(format_args!("{}", "}"));
    let _ = entry_type;
}

fn pascal_field(field: &str) -> String {
    field
        .split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            chars.next().map_or_else(String::new, |first| {
                first.to_ascii_uppercase().to_string() + chars.as_str()
            })
        })
        .collect()
}

fn render_provider_family_binding(native_family: &str, depth: usize, out: &mut SourceWriter) {
    out.line_at(
        depth,
        format_args!(
            "let family: NexaWidgetFamily = switch {native_family} {{ case .systemSmall: .Small; case .systemMedium: .Medium; case .systemLarge: .Large; default: .ExtraLarge }}"
        ),
    );
    out.line_at(depth, format_args!("let nexa_family = family"));
}

fn render_body(
    nodes: &[Node],
    module: &Module,
    features: &Features,
    depth: usize,
    out: &mut SourceWriter,
) {
    if nodes.len() == 1 {
        render_widget_node(&nodes[0], module, features, depth, out);
    } else {
        render_widget_layout(
            LayoutKind::Column,
            0.0,
            &nexa_ir::ViewStyle::default(),
            nodes,
            module,
            features,
            depth,
            out,
        );
    }
}

fn render_widget_nodes(
    nodes: &[Node],
    module: &Module,
    features: &Features,
    depth: usize,
    out: &mut SourceWriter,
) {
    if nodes.len() == 1 {
        render_widget_node(&nodes[0], module, features, depth, out);
    } else {
        render_widget_layout(
            LayoutKind::Column,
            0.0,
            &nexa_ir::ViewStyle::default(),
            nodes,
            module,
            features,
            depth,
            out,
        );
    }
}

fn render_widget_node(
    node: &Node,
    module: &Module,
    features: &Features,
    depth: usize,
    out: &mut SourceWriter,
) {
    match node {
        Node::Layout {
            kind,
            spacing,
            style,
            children,
        } => {
            if contains_custom_widget_node(children) {
                render_widget_layout(
                    *kind, *spacing, style, children, module, features, depth, out,
                );
            } else {
                components::layout::render_layout(
                    *kind,
                    *spacing,
                    style,
                    children,
                    &components::RenderScope { module, features },
                    depth,
                    out,
                );
            }
        }
        Node::If {
            condition,
            then_body,
            else_body,
            ..
        } => {
            out.line_at(
                depth,
                format_args!("if {} {{", expressions::expression(condition)),
            );
            render_widget_nodes(then_body, module, features, depth + 1, out);
            if !out.ends_with("\n") {
                out.push('\n');
            }
            if let Some(children) = else_body {
                out.line_at(depth, format_args!("}} else {{"));
                render_widget_nodes(children, module, features, depth + 1, out);
                if !out.ends_with("\n") {
                    out.push('\n');
                }
            }
            out.line_at(depth, format_args!("}}"));
        }
        Node::Link { url, children } => {
            out.line_at(
                depth,
                format_args!(
                    "Link(destination: URL(string: {})!) {{",
                    expressions::expression(url)
                ),
            );
            render_widget_nodes(children, module, features, depth + 1, out);
            if !out.ends_with("\n") {
                out.push('\n');
            }
            out.line_at(depth, format_args!("}}"));
        }
        Node::Accessibility {
            label,
            hint,
            value,
            role,
            children,
        } => {
            components::accessibility::render_accessibility(
                label,
                hint.as_ref(),
                value.as_ref(),
                *role,
                children,
                &components::RenderScope { module, features },
                depth,
                out,
            );
        }
        Node::FastList {
            plan: ListPlan::Count { count, common },
        } => {
            let index = nexa_codegen::names::state_name(&common.index);
            out.line_at(
                depth,
                format_args!(
                    "ForEach(0..<max(0, {}), id: \\.self) {{ {} in",
                    expressions::expression(count),
                    common.index
                ),
            );
            out.line_at(
                depth + 1,
                format_args!("let {index}: Int32 = Int32(clamping: {})", common.index),
            );
            render_widget_nodes(&common.children, module, features, depth + 1, out);
            if !out.ends_with("\n") {
                out.push('\n');
            }
            out.line_at(depth, format_args!("}}"));
        }
        Node::FastList {
            plan:
                ListPlan::Items {
                    collection,
                    item,
                    common,
                    ..
                },
        } => {
            let values = format!("__nexaWidgetItems{}", common.index);
            let index = nexa_codegen::names::state_name(&common.index);
            let item = nexa_codegen::names::state_name(item);
            out.line_at(
                depth,
                format_args!("let {values} = {}", expressions::expression(collection)),
            );
            out.line_at(
                depth,
                format_args!("ForEach(Array({values}.enumerated()), id: \\.offset) {{ element in"),
            );
            out.line_at(
                depth + 1,
                format_args!("let {index}: Int32 = Int32(clamping: element.offset)"),
            );
            out.line_at(depth + 1, format_args!("let {item} = element.element"));
            render_widget_nodes(&common.children, module, features, depth + 1, out);
            if !out.ends_with("\n") {
                out.push('\n');
            }
            out.line_at(depth, format_args!("}}"));
        }
        _ => node_renderer::render_node(node, module, features, depth, out),
    }
}

fn contains_custom_widget_node(nodes: &[Node]) -> bool {
    nodes.iter().any(|node| match node {
        Node::If { .. } | Node::Link { .. } | Node::FastList { .. } => true,
        Node::Layout { children, .. } | Node::Accessibility { children, .. } => {
            contains_custom_widget_node(children)
        }
        _ => false,
    })
}

#[allow(clippy::too_many_arguments)]
fn render_widget_layout(
    kind: LayoutKind,
    spacing: f32,
    style: &nexa_ir::ViewStyle,
    children: &[Node],
    module: &Module,
    features: &Features,
    depth: usize,
    out: &mut SourceWriter,
) {
    let (name, alignment) = match kind {
        LayoutKind::Column => (
            "VStack",
            match style.alignment {
                Some(nexa_ir::Alignment::Center) => Some(".center"),
                Some(nexa_ir::Alignment::End) => Some(".trailing"),
                _ => Some(".leading"),
            },
        ),
        LayoutKind::Row => (
            "HStack",
            match style.alignment {
                Some(nexa_ir::Alignment::Center) => Some(".center"),
                Some(nexa_ir::Alignment::End) => Some(".bottom"),
                _ => Some(".top"),
            },
        ),
        LayoutKind::Stack => (
            "ZStack",
            match style.alignment {
                Some(nexa_ir::Alignment::Center) => Some(".center"),
                Some(nexa_ir::Alignment::End) => Some(".bottomTrailing"),
                _ => Some(".topLeading"),
            },
        ),
    };
    out.line_at(
        depth,
        format_args!(
            "{name}(alignment: {}, spacing: {spacing}) {{",
            alignment.unwrap_or(".center")
        ),
    );
    for child in children {
        render_widget_node(child, module, features, depth + 1, out);
        if !out.ends_with("\n") {
            out.push('\n');
        }
    }
    out.line_at(depth, format_args!("}}"));
    if let Some(value) = style.padding {
        out.line_at(depth, format_args!(".padding({value})"));
    }
    if let Some(width) = style.width {
        out.line_at(depth, format_args!(".frame(width: {width})"));
    }
    if let Some(height) = style.height {
        out.line_at(depth, format_args!(".frame(height: {height})"));
    }
    if style.min_width.is_some()
        || style.max_width.is_some()
        || style.min_height.is_some()
        || style.max_height.is_some()
    {
        out.line_at(
            depth,
            format_args!(
                ".frame(minWidth: {}, maxWidth: {}, minHeight: {}, maxHeight: {})",
                swift_optional_number(style.min_width),
                swift_optional_number(style.max_width),
                swift_optional_number(style.min_height),
                swift_optional_number(style.max_height)
            ),
        );
    }
    if let Some(background) = style.background {
        out.line_at(
            depth,
            format_args!(
                ".background({})",
                crate::generator::engine::colors::expression(background)
            ),
        );
    }
    if let Some(radius) = style.corner_radius {
        out.line_at(
            depth,
            format_args!(".clipShape(RoundedRectangle(cornerRadius: {radius}))"),
        );
    }
    if let Some(opacity) = style.opacity {
        out.line_at(depth, format_args!(".opacity({opacity})"));
    }
}

fn swift_optional_number(value: Option<f32>) -> String {
    value
        .map(|number| number.to_string())
        .unwrap_or_else(|| "nil".to_owned())
}

fn swift_family(family: &WidgetFamily) -> &'static str {
    match family {
        WidgetFamily::Small => ".systemSmall",
        WidgetFamily::Medium => ".systemMedium",
        WidgetFamily::Large => ".systemLarge",
        WidgetFamily::ExtraLarge => ".systemExtraLarge",
    }
}

#[cfg(test)]
mod tests {
    use super::{generate, widget_bundle};
    use nexa_ir::{Expr, Module, Node, StructDecl, TextStyle, Type, Widget, WidgetFamily};

    #[test]
    fn emits_target_owned_widgetkit_provider_and_declared_families() {
        let mut module = Module {
            app_name: "Example".into(),
            plugins: vec![],
            plugin_assets: vec![],
            enums: vec![],
            structs: vec![],
            functions: vec![],
            background_tasks: vec![],
            states: vec![],
            globals: vec![],
            screens: vec![],
            widgets: vec![],
            components: vec![],
            body: vec![],
            status_bar: None,
            direction: None,
            on_appear: None,
            on_appear_async: false,
            on_disappear: None,
            on_active: None,
            on_inactive: None,
            on_background: None,
        };
        module.structs.push(StructDecl {
            name: "WidgetEntry".into(),
            fields: vec![],
        });
        module.widgets.push(Widget {
            name: "Tasks".into(),
            display_name: None,
            description: None,
            configuration_title: None,
            configuration_description: None,
            configuration: None,
            entry_provider: Expr::Call {
                name: "WidgetEntry.current".into(),
                arguments: vec![],
                is_constructor: false,
                is_async: false,
                is_throwing: false,
                return_type: Type::Struct {
                    name: "WidgetEntry".into(),
                    fields: vec![],
                },
            },
            placeholder_provider: None,
            entry_provider_async: false,
            entry_provider_throws: false,
            entry_type: Type::Struct {
                name: "WidgetEntry".into(),
                fields: vec![],
            },
            families: vec![WidgetFamily::Small, WidgetFamily::Medium],
            refresh_seconds: 1800,
            body: vec![Node::Text {
                value: Expr::String("Tasks".into()),
                style: TextStyle {
                    font_style: Some(nexa_ir::TextFontStyle::Subheadline),
                    ..TextStyle::default()
                },
            }],
        });
        let generated = generate(&module).expect("valid widget IR").sources;
        let source = generated
            .units
            .iter()
            .find(|unit| unit.name == "NexaTasksWidget.swift")
            .map(|unit| unit.contents.as_str())
            .unwrap_or_default();
        assert!(source.contains("StaticConfiguration(kind: kind"));
        assert!(source.contains(".systemSmall, .systemMedium"));
        assert!(source.contains("addingTimeInterval(1800)"));
        assert!(source.contains("let family: NexaWidgetFamily = switch context.family"));
        assert!(source.contains(".font(.subheadline)"));
        assert!(!source.contains("rowLimit"));
        let bundle = generated
            .units
            .iter()
            .find(|unit| unit.name == "NexaWidgetBundle.swift")
            .map(|unit| unit.contents.as_str())
            .unwrap_or_default();
        assert!(bundle.contains("NexaTasksWidget()\n"));
        assert!(!bundle.contains("NexaTasksWidget(),"));

        let mut second_widget = module.widgets[0].clone();
        second_widget.name = "Reminders".into();
        let bundle = widget_bundle(&[module.widgets[0].clone(), second_widget]);
        assert!(bundle.contains("NexaTasksWidget()\n        NexaRemindersWidget()\n"));

        let widget = &mut module.widgets[0];
        widget.entry_provider = Expr::Await(Box::new(Expr::Call {
            name: "WidgetEntry.load".into(),
            arguments: vec![],
            is_constructor: false,
            is_async: true,
            is_throwing: false,
            return_type: widget.entry_type.clone(),
        }));
        widget.placeholder_provider = Some(Expr::Call {
            name: "WidgetEntry.loading".into(),
            arguments: vec![],
            is_constructor: false,
            is_async: false,
            is_throwing: false,
            return_type: widget.entry_type.clone(),
        });
        widget.entry_provider_async = true;
        let generated = generate(&module).expect("valid async widget IR").sources;
        let source = generated
            .units
            .iter()
            .find(|unit| unit.name == "NexaTasksWidget.swift")
            .map(|unit| unit.contents.as_str())
            .unwrap_or_default();
        assert!(source.contains("value: NexaWidgetEntry.nexa_fn_loading()"));
        assert!(source.contains("Task {"));
        assert!(
            source.contains("let value: NexaWidgetEntry = (await NexaWidgetEntry.nexa_fn_load())")
        );
    }
}
