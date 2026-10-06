//! Target-owned Android Glance widget generation.
//!
//! Glance uses its own composables and cannot host ordinary Compose UI nodes;
//! this renderer deliberately supports only the widget-safe IR subset.

use nexa_codegen::{GeneratedSources, SourceUnit, SourceWriter};
use nexa_ir::{Expr, LayoutKind, ListPlan, Module, Node, Widget, WidgetFamily};

use super::engine::{expressions, features::Features};

type WidgetConfigurationParts<'a> = (&'a str, &'a [(String, nexa_ir::Type)], Vec<&'a str>);

pub(super) fn generate(
    module: &Module,
) -> Result<crate::WidgetGeneratedSources, crate::WidgetGenerationError> {
    for widget in &module.widgets {
        validate_widget(widget)?;
    }
    if module.widgets.is_empty() {
        return Ok(crate::WidgetGeneratedSources {
            sources: nexa_codegen::SourceUnits::new("kt").finish(),
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
    let mut features = Features::analyze(&support_module);
    // Widgets can be started by the launcher before MainActivity. Force the
    // small runtime context holder into the target support units so provider
    // and configuration code can bind their own process context.
    features.facts.capabilities.uses_storage_api = true;
    support_module.states.clear();
    let support = super::generate_units(&support_module, &features);
    let mut imports = support
        .imports
        .lines()
        .filter(|line| line.starts_with("import "))
        .map(str::to_owned)
        .collect::<std::collections::BTreeSet<_>>();
    imports.extend([
        "import androidx.compose.runtime.Composable".to_owned(),
        "import androidx.compose.ui.unit.DpSize".to_owned(),
        "import androidx.compose.ui.unit.dp".to_owned(),
        "import androidx.glance.GlanceModifier".to_owned(),
        "import androidx.glance.LocalSize".to_owned(),
        "import androidx.glance.Image as GlanceImage".to_owned(),
        "import androidx.glance.ImageProvider".to_owned(),
        "import androidx.glance.ColorFilter".to_owned(),
        "import androidx.glance.appwidget.GlanceAppWidget".to_owned(),
        "import androidx.glance.appwidget.GlanceAppWidgetReceiver".to_owned(),
        "import androidx.glance.appwidget.SizeMode".to_owned(),
        "import androidx.glance.appwidget.provideContent".to_owned(),
        "import androidx.glance.layout.Box as GlanceBox".to_owned(),
        "import androidx.glance.layout.Column as GlanceColumn".to_owned(),
        "import androidx.glance.layout.Row as GlanceRow".to_owned(),
        "import androidx.glance.layout.Spacer as GlanceSpacer".to_owned(),
        "import androidx.glance.layout.fillMaxSize".to_owned(),
        "import androidx.glance.layout.height".to_owned(),
        "import androidx.glance.layout.width".to_owned(),
        "import androidx.glance.layout.fillMaxWidth".to_owned(),
        "import androidx.glance.layout.padding".to_owned(),
        "import androidx.glance.background".to_owned(),
        "import androidx.glance.appwidget.cornerRadius".to_owned(),
        "import androidx.glance.appwidget.state.updateAppWidgetState".to_owned(),
        "import androidx.glance.appwidget.state.getAppWidgetState".to_owned(),
        "import androidx.datastore.preferences.core.Preferences".to_owned(),
        "import androidx.datastore.preferences.core.stringPreferencesKey".to_owned(),
        "import androidx.activity.ComponentActivity".to_owned(),
        "import androidx.activity.compose.setContent".to_owned(),
        "import androidx.lifecycle.lifecycleScope".to_owned(),
        "import androidx.compose.runtime.*".to_owned(),
        "import androidx.compose.material3.Button".to_owned(),
        "import androidx.compose.material3.Text".to_owned(),
        "import androidx.compose.foundation.layout.*".to_owned(),
        "import androidx.glance.appwidget.GlanceAppWidgetManager".to_owned(),
        "import android.appwidget.AppWidgetManager".to_owned(),
        "import android.os.Bundle".to_owned(),
        "import kotlinx.coroutines.launch".to_owned(),
        "import androidx.glance.action.clickable".to_owned(),
        "import androidx.glance.appwidget.action.actionStartActivity".to_owned(),
        "import androidx.glance.semantics.semantics".to_owned(),
        "import androidx.glance.semantics.contentDescription".to_owned(),
        "import android.content.Intent".to_owned(),
        "import android.net.Uri".to_owned(),
    ]);
    if module
        .widgets
        .iter()
        .any(|widget| widget.configuration.is_some())
    {
        imports.insert("import androidx.compose.ui.res.stringResource".to_owned());
    }
    let mut output_units = support
        .units
        .into_iter()
        .filter(|unit| unit.name != "NexaGenerated.kt")
        .collect::<Vec<_>>();
    output_units.push(SourceUnit {
        name: "NexaWidgetFamily.kt".to_owned(),
        contents: "enum class NexaWidgetFamily { Small, Medium, Large, ExtraLarge }\ntypealias WidgetFamily = NexaWidgetFamily\n\ninternal fun nexaWidgetFamily(size: DpSize): NexaWidgetFamily = when {\n    size.width <= 130.dp && size.height <= 130.dp -> NexaWidgetFamily.Small\n    size.height <= 130.dp -> NexaWidgetFamily.Medium\n    size.width <= 300.dp -> NexaWidgetFamily.Large\n    else -> NexaWidgetFamily.ExtraLarge\n}\n\ninternal fun nexaWidgetColorFromHex(hex: String): androidx.compose.ui.graphics.Color {\n    val digits = hex.removePrefix(\"#\")\n    val hasAlpha = digits.length == 8\n    val parsed = if (digits.length == 6 || hasAlpha) digits.toLongOrNull(16) else null\n    val color = parsed ?: 0xD63031L\n    val red = if (hasAlpha && parsed != null) (color shr 24) and 0xFF else (color shr 16) and 0xFF\n    val green = if (hasAlpha && parsed != null) (color shr 16) and 0xFF else (color shr 8) and 0xFF\n    val blue = if (hasAlpha && parsed != null) (color shr 8) and 0xFF else color and 0xFF\n    val alpha = if (hasAlpha && parsed != null) (color and 0xFF).toFloat() / 255f else 1f\n    return androidx.compose.ui.graphics.Color(red.toFloat() / 255f, green.toFloat() / 255f, blue.toFloat() / 255f, alpha)\n}\n".to_owned(),
    });
    let mut resource_units = Vec::<SourceUnit>::new();
    for widget in &module.widgets {
        let mut out = SourceWriter::new();
        render_widget(widget, module, &mut out)?;
        output_units.push(SourceUnit {
            name: format!("widget-{}.kt", widget.name),
            contents: out.finish(),
        });
        collect_drawables(widget, &widget.body, &mut resource_units)?;
    }
    Ok(crate::WidgetGeneratedSources {
        sources: GeneratedSources {
            imports: format!("{}\n\n", imports.into_iter().collect::<Vec<_>>().join("\n")),
            preamble: support.preamble,
            units: output_units,
        },
        resources: resource_units,
    })
}

fn collect_drawables(
    widget: &Widget,
    nodes: &[Node],
    output: &mut Vec<SourceUnit>,
) -> Result<(), crate::WidgetGenerationError> {
    for node in nodes {
        match node {
            Node::SystemIcon { icon, tint, .. } => {
                let drawable = icon.android_widget_drawable().ok_or_else(|| {
                    crate::WidgetGenerationError {
                        widget: widget.name.clone(),
                        message: format!(
                            "Android Glance has no vector drawable mapping for system icon {icon:?}"
                        ),
                    }
                })?;
                widget_icon_color(tint);
                let resource_name = drawable.resource_name;
                let unit_name = format!("drawable/{resource_name}.xml");
                if !output.iter().any(|unit| unit.name == unit_name) {
                    output.push(SourceUnit {
                        name: unit_name,
                        contents: format!("<vector xmlns:android=\"http://schemas.android.com/apk/res/android\" android:width=\"24dp\" android:height=\"24dp\" android:viewportWidth=\"24\" android:viewportHeight=\"24\"><path android:fillColor=\"#FFFFFFFF\" android:pathData=\"{}\"/></vector>\n", drawable.path_data),
                    });
                }
            }
            Node::Layout { children, .. }
            | Node::Link { children, .. }
            | Node::Accessibility { children, .. } => collect_drawables(widget, children, output)?,
            Node::If {
                then_body,
                else_body,
                ..
            } => {
                collect_drawables(widget, then_body, output)?;
                if let Some(children) = else_body {
                    collect_drawables(widget, children, output)?;
                }
            }
            Node::FastList {
                plan: ListPlan::Count { common, .. } | ListPlan::Items { common, .. },
            } => collect_drawables(widget, &common.children, output)?,
            _ => {}
        }
    }
    Ok(())
}

fn widget_icon_color(tint: &nexa_ir::ColorExpression) -> String {
    match tint {
        nexa_ir::ColorExpression::Static(nexa_ir::ColorValue::Static(color)) => format!(
            "androidx.glance.color.ColorProvider(day = {}, night = {})",
            glance_rgb(*color),
            glance_rgb(*color),
        ),
        nexa_ir::ColorExpression::Static(nexa_ir::ColorValue::Adaptive { light, dark }) => {
            format!(
                "androidx.glance.color.ColorProvider(day = {}, night = {})",
                glance_rgb(*light),
                glance_rgb(*dark),
            )
        }
        nexa_ir::ColorExpression::Dynamic(expression) => format!(
            "androidx.glance.unit.ColorProvider(nexaWidgetColorFromHex({}))",
            expressions::expression(expression)
        ),
    }
}

fn glance_rgb(color: nexa_ir::Color) -> String {
    format!(
        "androidx.compose.ui.graphics.Color(0x{:02X}{:02X}{:02X}{:02X}L)",
        color.alpha, color.red, color.green, color.blue
    )
}

fn validate_widget(widget: &Widget) -> Result<(), crate::WidgetGenerationError> {
    fn fail(widget: &Widget, feature: &str) -> crate::WidgetGenerationError {
        crate::WidgetGenerationError {
            widget: widget.name.clone(),
            message: format!("unsupported widget IR: {feature}"),
        }
    }
    fn validate_nodes(widget: &Widget, nodes: &[Node]) -> Result<(), crate::WidgetGenerationError> {
        for node in nodes {
            match node {
                Node::Text { style, .. } => {
                    if style.padding.is_some()
                        || style.opacity.is_some()
                        || style.effects.has_modifiers()
                        || style.line_height.is_some()
                        || style.letter_spacing.is_some()
                        || style.strikethrough
                        || style.selectable
                    {
                        return Err(fail(
                            widget,
                            "Text style property is not supported by Glance",
                        ));
                    }
                }
                Node::Spacer | Node::Divider { .. } => {}
                Node::SystemIcon { icon, .. } => {
                    if icon.android_widget_drawable().is_none() {
                        return Err(fail(
                            widget,
                            &format!("no Android Glance drawable mapping for system icon {icon:?}"),
                        ));
                    }
                }
                Node::Layout {
                    style, children, ..
                } => {
                    if style.min_width.is_some()
                        || style.max_width.is_some()
                        || style.min_height.is_some()
                        || style.max_height.is_some()
                        || style.border_color.is_some()
                        || style.border_width.is_some()
                        || style.opacity.is_some()
                        || style.effects.has_modifiers()
                        || style.animation.is_some()
                    {
                        return Err(fail(
                            widget,
                            "layout style property is not supported by Glance",
                        ));
                    }
                    validate_nodes(widget, children)?;
                }
                Node::Accessibility {
                    hint,
                    role,
                    children,
                    ..
                } => {
                    if hint.is_some() || !matches!(role, nexa_ir::AccessibilityRole::None) {
                        return Err(fail(
                            widget,
                            "Glance accessibility currently supports a label without a hint or role",
                        ));
                    }
                    validate_nodes(widget, children)?;
                }
                Node::Link { url, children } => {
                    if !matches!(url, Expr::String(_)) {
                        return Err(fail(widget, "Link URL must be a static string"));
                    }
                    validate_nodes(widget, children)?;
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
                    validate_nodes(widget, then_body)?;
                    if let Some(body) = else_body {
                        validate_nodes(widget, body)?;
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
                    validate_nodes(widget, &common.children)?;
                }
                Node::FastList { .. } => return Err(fail(widget, "sectioned FastList")),
                _ => return Err(fail(widget, "node outside the widget-safe subset")),
            }
        }
        Ok(())
    }
    if let Some(configuration) = &widget.configuration {
        let nexa_ir::Type::Struct { fields, .. } = &configuration.ty else {
            return Err(fail(widget, "configuration must be a struct"));
        };
        let Expr::Call { arguments, .. } = &configuration.default else {
            return Err(fail(
                widget,
                "configuration default must be a struct constructor",
            ));
        };
        if fields.len() != arguments.len() {
            return Err(fail(
                widget,
                "configuration default field count does not match its struct",
            ));
        }
        for ((_, ty), default) in fields.iter().zip(arguments) {
            let nexa_ir::Type::Enum(enum_name) = ty else {
                return Err(fail(widget, "configuration fields must be enum values"));
            };
            let Expr::EnumValue {
                enum_name: actual_enum,
                case_name,
            } = default
            else {
                return Err(fail(widget, "configuration defaults must be enum cases"));
            };
            if actual_enum != enum_name || case_name.is_empty() {
                return Err(fail(
                    widget,
                    "configuration default enum type does not match its field",
                ));
            }
        }
    }
    validate_nodes(widget, &widget.body)
}

fn kotlin_string(value: &str) -> String {
    format!(
        "\"{}\"",
        value
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n")
    )
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
    scoped.components.clear();
    scoped.on_appear = None;
    scoped.on_appear_async = false;
    scoped.on_disappear = None;
    scoped.on_active = None;
    scoped.on_inactive = None;
    scoped.on_background = None;
    let (functions, globals) = reachable_functions(module, widget);
    scoped.functions = functions;
    scoped.globals = globals;
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
    let mut globals = HashSet::<String>::new();
    {
        let mut visit = |expr: &Expr| {
            walk_expression(expr, &mut |nested| {
                collect_references(nested, &declared, &mut used, &mut globals)
            })
        };
        visit(&widget.entry_provider);
        if let Some(placeholder) = &widget.placeholder_provider {
            visit(placeholder);
        }
        walk_ir(&widget.body, &mut |_| {}, &mut visit);
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
            nexa_ir::walk::walk_expression(&local.initial, &mut |expr| {
                collect_references(expr, &declared, &mut used, &mut globals)
            });
        }
        for local in &function.class_initializers {
            nexa_ir::walk::walk_expression(&local.initial, &mut |expr| {
                collect_references(expr, &declared, &mut used, &mut globals)
            });
        }
        nexa_ir::walk::walk_expression(&function.body, &mut |expr| {
            collect_references(expr, &declared, &mut used, &mut globals)
        });
        pending.extend(used.difference(&before).cloned());
    }
    let functions = module
        .functions
        .iter()
        .filter(|function| used.contains(&function.name))
        .cloned()
        .collect::<Vec<_>>();
    let classes = functions
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
            globals.contains(&global.name)
                || global
                    .name
                    .split_once("::")
                    .is_some_and(|(class, _)| classes.contains(class))
        })
        .cloned()
        .collect();
    (functions, globals)
}

fn collect_references(
    expr: &Expr,
    declared: &std::collections::HashSet<&str>,
    used: &mut std::collections::HashSet<String>,
    globals: &mut std::collections::HashSet<String>,
) {
    match expr {
        Expr::Call { name, .. } if declared.contains(name.as_str()) => {
            used.insert(name.clone());
        }
        Expr::State(name, _) | Expr::AnimatedState(name, _) => {
            globals.insert(name.clone());
        }
        _ => {}
    }
}

fn configuration_parts<'a>(
    widget: &'a Widget,
    module: &'a Module,
) -> Result<WidgetConfigurationParts<'a>, crate::WidgetGenerationError> {
    let configuration =
        widget
            .configuration
            .as_ref()
            .ok_or_else(|| crate::WidgetGenerationError {
                widget: widget.name.clone(),
                message: "configuration metadata missing".to_owned(),
            })?;
    let (name, fields) = match &configuration.ty {
        nexa_ir::Type::Struct { name, fields } => (name.as_str(), fields.as_slice()),
        _ => {
            return Err(crate::WidgetGenerationError {
                widget: widget.name.clone(),
                message: "configuration must be a struct".to_owned(),
            });
        }
    };
    let args =
        match &configuration.default {
            Expr::Call {
                name: call_name,
                arguments,
                ..
            } if call_name == name && arguments.len() == fields.len() => arguments,
            _ => return Err(crate::WidgetGenerationError {
                widget: widget.name.clone(),
                message:
                    "configuration default must be a struct constructor with enum case arguments"
                        .to_owned(),
            }),
        };
    let mut cases = Vec::with_capacity(fields.len());
    for ((field, ty), default) in fields.iter().zip(args) {
        let nexa_ir::Type::Enum(enum_name) = ty else {
            return Err(crate::WidgetGenerationError {
                widget: widget.name.clone(),
                message: format!("configuration field `{field}` must use an enum"),
            });
        };
        let decl = module
            .enums
            .iter()
            .find(|declaration| declaration.name == *enum_name)
            .ok_or_else(|| crate::WidgetGenerationError {
                widget: widget.name.clone(),
                message: format!("configuration enum `{enum_name}` is missing"),
            })?;
        let Expr::EnumValue {
            enum_name: default_type,
            case_name,
        } = default
        else {
            return Err(crate::WidgetGenerationError {
                widget: widget.name.clone(),
                message: format!("configuration default for `{field}` must be an enum case"),
            });
        };
        if default_type != enum_name || !decl.cases.iter().any(|case| case == case_name) {
            return Err(crate::WidgetGenerationError {
                widget: widget.name.clone(),
                message: format!("invalid default enum case `{case_name}` for `{field}`"),
            });
        }
        cases.push(case_name.as_str());
    }
    Ok((name, fields, cases))
}

fn config_key(widget: &Widget, field: &str) -> String {
    format!(
        "nexa_widget_{}_{}",
        widget.name.to_lowercase(),
        field.to_lowercase()
    )
}

fn render_preference_configuration(
    widget: &Widget,
    module: &Module,
    configuration: &nexa_ir::WidgetConfiguration,
) -> Result<String, crate::WidgetGenerationError> {
    let (_, fields, defaults) = configuration_parts(widget, module)?;
    let config_type = super::engine::types::kotlin_type(&configuration.ty);
    let args = fields.iter().zip(defaults).map(|((field, ty), default)| {
        let enum_type = super::engine::types::kotlin_type(ty);
        format!("{} = {enum_type}.valueOf(preferences[stringPreferencesKey(\"{}\")] ?: \"{default}\")", nexa_codegen::names::struct_field_name(field), config_key(widget, field))
    }).collect::<Vec<_>>().join(", ");
    Ok(format!("{config_type}({args})"))
}

fn render_configuration_activity(
    widget: &Widget,
    module: &Module,
    out: &mut SourceWriter,
) -> Result<(), crate::WidgetGenerationError> {
    let (_, fields, defaults) = configuration_parts(widget, module)?;
    let widget_type = nexa_codegen::names::widget_name(&widget.name);
    let activity = format!("{widget_type}ConfigurationActivity");
    out.line(format_args!("class {activity} : ComponentActivity() {{"));
    out.line(format_args!(
        "{}",
        "    override fun onCreate(savedInstanceState: Bundle?) {"
    ));
    out.line(format_args!(
        "{}",
        "        super.onCreate(savedInstanceState)"
    ));
    out.line(format_args!("{}", "        NexaRuntime.bind(this)"));
    out.line(format_args!("{}", "        val appWidgetId = intent?.extras?.getInt(AppWidgetManager.EXTRA_APPWIDGET_ID, AppWidgetManager.INVALID_APPWIDGET_ID) ?: AppWidgetManager.INVALID_APPWIDGET_ID"));
    out.line(format_args!("{}", "        setResult(RESULT_CANCELED, android.content.Intent().putExtra(AppWidgetManager.EXTRA_APPWIDGET_ID, appWidgetId))"));
    out.line(format_args!(
        "{}",
        "        if (appWidgetId == AppWidgetManager.INVALID_APPWIDGET_ID) { finish(); return }"
    ));
    out.line(format_args!("{}", "        lifecycleScope.launch {"));
    out.line(format_args!("            val glanceId = GlanceAppWidgetManager(this@{activity}).getGlanceIdBy(appWidgetId)"));
    out.line(format_args!(
                "            val existing = {widget_type}().getAppWidgetState<Preferences>(this@{activity}, glanceId)"
    ));
    out.line(format_args!("{}", "            setContent {"));
    out.line(format_args!(
        "{}",
        "                androidx.compose.foundation.layout.Column {"
    ));
    let title = widget
        .configuration_title
        .as_deref()
        .map(str::to_owned)
        .unwrap_or_else(|| nexa_ir::localization::widget_display_name(widget));
    out.line(format_args!(
        "                    Text({})",
        widget_text_resource(&title)
    ));
    if let Some(key) = &widget.configuration_description {
        out.line(format_args!(
            "                    Text({})",
            widget_text_resource(key)
        ));
    }
    for (index, ((field, ty), default)) in fields.iter().zip(defaults).enumerate() {
        let enum_type = super::engine::types::kotlin_type(ty);
        out.line(format_args!("                var selected{index} by remember {{ mutableStateOf({enum_type}.valueOf(existing[stringPreferencesKey({})] ?: \"{default}\")) }}", kotlin_string(&config_key(widget, field))));
        let field_label = nexa_ir::localization::humanize_identifier(field);
        out.line(format_args!(
            "                Text({})",
            widget_text_resource(&field_label)
        ));
        out.line(format_args!(
            "{}",
            "                    androidx.compose.foundation.layout.Row {"
        ));
        if let nexa_ir::Type::Enum(name) = ty {
            let declaration = module
                .enums
                .iter()
                .find(|item| item.name == *name)
                .ok_or_else(|| crate::WidgetGenerationError {
                    widget: widget.name.clone(),
                    message: format!("configuration enum `{name}` is missing"),
                })?;
            for case in &declaration.cases {
                let case_label = nexa_ir::localization::humanize_identifier(case);
                out.line(format_args!("                        Button(onClick = {{ selected{index} = {enum_type}.{case} }}) {{ Text({}) }}", widget_text_resource(&case_label)));
            }
        }
        out.line(format_args!("{}", "                    }"));
        let _ = field;
    }
    out.line(format_args!(
        "                    Button(onClick = {{ lifecycleScope.launch {{"
    ));
    out.line(format_args!(
        "                        updateAppWidgetState(this@{activity}, glanceId) {{ preferences ->"
    ));
    for (index, (field, _)) in fields.iter().enumerate() {
        out.line(format_args!(
            "                            preferences[stringPreferencesKey({})] = selected{index}.name",
            kotlin_string(&config_key(widget, field))
        ));
    }
    out.line(format_args!("{}", "                        }"));
    out.line(format_args!(
        "                        {widget_type}().update(this@{activity}, glanceId)"
    ));
    out.line(format_args!("{}", "                        setResult(RESULT_OK, android.content.Intent().putExtra(AppWidgetManager.EXTRA_APPWIDGET_ID, appWidgetId))"));
    out.line(format_args!("{}", "                        finish()"));
    out.line(format_args!("{}", "                    }"));
    out.line(format_args!(
        "                    }}) {{ Text({}) }}",
        widget_text_resource("Save")
    ));
    out.line(format_args!("{}", "                }"));
    out.line(format_args!("{}", "            }"));
    out.line(format_args!("{}", "        }"));
    out.line(format_args!("{}", "    }"));
    out.line(format_args!("{}", "}"));
    Ok(())
}

fn render_widget(
    widget: &Widget,
    module: &Module,
    out: &mut SourceWriter,
) -> Result<(), crate::WidgetGenerationError> {
    let widget_type = nexa_codegen::names::widget_name(&widget.name);
    let provider = expressions::expression(&widget.entry_provider);
    let placeholder = expressions::expression(
        widget
            .placeholder_provider
            .as_ref()
            .unwrap_or(&widget.entry_provider),
    );
    let entry_type = super::engine::types::kotlin_type(&widget.entry_type);
    out.line(format_args!("class {widget_type} : GlanceAppWidget() {{"));
    out.line(format_args!(
        "    override val sizeMode: SizeMode = SizeMode.Responsive({})",
        glance_sizes(&widget.families)
    ));
    out.line(format_args!("    override suspend fun provideGlance(context: android.content.Context, id: androidx.glance.GlanceId) {{"));
    out.line(format_args!("{}", "        NexaRuntime.bind(context)"));
    if let Some(configuration) = &widget.configuration {
        let preferences = render_preference_configuration(widget, module, configuration)?;
        out.line(format_args!(
            "        val preferences = getAppWidgetState<Preferences>(context, id)"
        ));
        out.line(format_args!(
            "        val nexa_configuration: {} = {preferences}",
            super::engine::types::kotlin_type(&configuration.ty)
        ));
    }
    if widget.entry_provider_throws {
        out.line(format_args!(
            "        val nexa_entry: {entry_type} = try {{"
        ));
        out.line(format_args!("            {provider}"));
        out.line(format_args!(
            "        }} catch (cancelled: kotlinx.coroutines.CancellationException) {{"
        ));
        out.line(format_args!("            throw cancelled"));
        out.line(format_args!("        }} catch (_: Exception) {{"));
        out.line(format_args!("            {placeholder}"));
        out.line(format_args!("        }}"));
    } else {
        out.line(format_args!(
            "        val nexa_entry: {entry_type} = {provider}"
        ));
    }
    out.line(format_args!("{}", "        provideContent {"));
    out.line(format_args!(
        "            val nexa_family = nexaWidgetFamily(LocalSize.current)"
    ));
    if widget.configuration.is_some() {
        out.line(format_args!(
            "            {widget_type}Content(nexa_entry, nexa_configuration, nexa_family)"
        ));
    } else {
        out.line(format_args!(
            "            {widget_type}Content(nexa_entry, nexa_family)"
        ));
    }
    out.line(format_args!("{}", "        }"));
    out.line(format_args!("{}", "    }"));
    out.line(format_args!("{}", "}"));
    out.line(format_args!("{}", ""));
    out.line(format_args!(
        "class {widget_type}Receiver : GlanceAppWidgetReceiver() {{"
    ));
    out.line(format_args!(
        "    override val glanceAppWidget: GlanceAppWidget = {widget_type}()"
    ));
    out.line(format_args!("{}", "}"));
    out.line(format_args!("{}", ""));
    let configuration_parameter = widget
        .configuration
        .as_ref()
        .map(|configuration| {
            format!(
                ", nexa_configuration: {}",
                super::engine::types::kotlin_type(&configuration.ty)
            )
        })
        .unwrap_or_default();
    out.line(format_args!("@Composable\nprivate fun {widget_type}Content(nexa_entry: {entry_type}{configuration_parameter}, nexa_family: NexaWidgetFamily) {{"));
    out.line(format_args!("{}", "    GlanceBox(modifier = GlanceModifier.fillMaxSize().clickable(actionStartActivity(Intent(NexaRuntime.context(), MainActivity::class.java)))) {"));
    render_nodes(&widget.body, 2, out, widget)?;
    out.line(format_args!("{}", "    }"));
    out.line(format_args!("{}", "}"));
    if widget.configuration.is_some() {
        render_configuration_activity(widget, module, out)?;
    }
    Ok(())
}

fn render_nodes(
    nodes: &[Node],
    depth: usize,
    out: &mut SourceWriter,
    widget: &Widget,
) -> Result<(), crate::WidgetGenerationError> {
    if nodes.len() == 1 {
        render_node(&nodes[0], depth, out, widget)?;
    } else {
        render_layout(
            LayoutKind::Column,
            nodes,
            0.0,
            &nexa_ir::ViewStyle::default(),
            depth,
            out,
            widget,
        )?;
    }
    Ok(())
}

fn render_node(
    node: &Node,
    depth: usize,
    out: &mut SourceWriter,
    widget: &Widget,
) -> Result<(), crate::WidgetGenerationError> {
    match node {
        Node::Text { value, style, .. } => {
            let mut parameters = Vec::new();
            if let Some(size) = style.font_size { parameters.push(format!("fontSize = {size}.sp")); }
            if let Some(weight) = style.font_weight {
                let value = match weight { nexa_ir::FontWeight::Normal => "Normal", nexa_ir::FontWeight::Medium => "Medium", nexa_ir::FontWeight::Semibold => "SemiBold", nexa_ir::FontWeight::Bold => "Bold" };
                parameters.push(format!("fontWeight = androidx.glance.text.FontWeight.{value}"));
            }
            if let Some(color) = style.color { parameters.push(format!("color = {}", glance_color(color))); }
            if let Some(alignment) = style.alignment {
                let value = match alignment { nexa_ir::TextAlignment::Leading => "Start", nexa_ir::TextAlignment::Center => "Center", nexa_ir::TextAlignment::Trailing => "End" };
                parameters.push(format!("textAlign = androidx.glance.text.TextAlign.{value}"));
            }
            let suffix = if parameters.is_empty() { String::new() } else { format!(", style = androidx.glance.text.TextStyle({})", parameters.join(", ")) };
            let max_lines = style.line_limit.map(|limit| format!(", maxLines = {limit}")).unwrap_or_default();
            out.line_at(depth, format_args!("androidx.glance.text.Text(text = {}{}{})", expressions::expression(value), suffix, max_lines));
        }
        Node::Spacer => out.line_at(depth, format_args!("GlanceSpacer(modifier = GlanceModifier.defaultWeight())")),
        Node::Divider { thickness, color } => out.line_at(depth, format_args!("GlanceBox(modifier = GlanceModifier.fillMaxWidth().height({thickness}.dp).background({})) {{}}", glance_color(*color))),
        Node::Layout { kind, children, spacing, style } => render_layout(*kind, children, *spacing, style, depth, out, widget)?,
        Node::If { condition, then_body, else_body, .. } => {
            out.line_at(depth, format_args!("if ({}) {{", expressions::expression(condition)));
            render_nodes(then_body, depth + 1, out, widget)?;
            if let Some(children) = else_body {
                out.line_at(depth, format_args!("}} else {{"));
                render_nodes(children, depth + 1, out, widget)?;
            }
            out.line_at(depth, format_args!("}}"));
        }
        Node::Link { url, children } => {
            out.line_at(depth, format_args!("GlanceBox(modifier = GlanceModifier.clickable(actionStartActivity(Intent(Intent.ACTION_VIEW, Uri.parse({}))))) {{", expressions::expression(url)));
            render_nodes(children, depth + 1, out, widget)?;
            out.line_at(depth, format_args!("}}"));
        }
        Node::Accessibility {
            label,
            value,
            children,
            ..
        } => {
            let description = value.as_ref().map_or_else(
                || expressions::expression(label),
                |value| {
                    format!(
                        "run {{ val label = {}; val value = {}; if (value.isEmpty()) label else \"$label, $value\" }}",
                        expressions::expression(label),
                        expressions::expression(value)
                    )
                },
            );
            out.line_at(
                depth,
                format_args!(
                    "GlanceBox(modifier = GlanceModifier.semantics {{ contentDescription = {} }}) {{",
                    description
                ),
            );
            render_nodes(children, depth + 1, out, widget)?;
            out.line_at(depth, format_args!("}}"));
        }
        Node::FastList { plan: ListPlan::Count { count, common } } => {
            let index = nexa_codegen::names::state_name(&common.index);
            out.line_at(depth, format_args!("GlanceColumn {{"));
            out.line_at(depth + 1, format_args!("for ({index} in 0 until maxOf({}, 0)) {{", expressions::expression(count)));
            render_nodes(&common.children, depth + 2, out, widget)?;
            out.line_at(depth + 1, format_args!("}}"));
            out.line_at(depth, format_args!("}}"));
        }
        Node::FastList { plan: ListPlan::Items { collection, item, common, .. } } => {
            let values = format!("__nexaWidgetItems{}", common.index);
            let index = nexa_codegen::names::state_name(&common.index);
            let item = nexa_codegen::names::state_name(item);
            out.line_at(depth, format_args!("GlanceColumn {{"));
            out.line_at(depth + 1, format_args!("val {values} = {}", expressions::expression(collection)));
            out.line_at(depth + 1, format_args!("for ({index} in 0 until {values}.size) {{"));
            out.line_at(depth + 2, format_args!("val {item} = {values}[{index}]"));
            render_nodes(&common.children, depth + 2, out, widget)?;
            out.line_at(depth + 1, format_args!("}}"));
            out.line_at(depth, format_args!("}}"));
        }
        Node::SystemIcon { icon, description, size, tint } => {
            let drawable = icon.android_widget_drawable().ok_or_else(|| crate::WidgetGenerationError { widget: widget.name.clone(), message: format!("no Android Glance drawable mapping for system icon {icon:?}") })?;
            let color = widget_icon_color(tint);
            let resource_name = drawable.resource_name;
            let content_description = if description.is_empty() {
                "null".to_owned()
            } else {
                kotlin_string(description)
            };
            out.line_at(depth, format_args!("GlanceImage(provider = ImageProvider(R.drawable.{resource_name}), contentDescription = {content_description}, alpha = 1f, colorFilter = ColorFilter.tint({color}), modifier = GlanceModifier.width({size}.dp).height({size}.dp))"));
        }
        _ => return Err(crate::WidgetGenerationError { widget: widget.name.clone(), message: "unsupported widget node reached backend renderer".to_owned() }),
    }
    Ok(())
}

fn render_layout(
    kind: LayoutKind,
    children: &[Node],
    spacing: f32,
    style: &nexa_ir::ViewStyle,
    depth: usize,
    out: &mut SourceWriter,
    widget: &Widget,
) -> Result<(), crate::WidgetGenerationError> {
    let mut modifiers = Vec::new();
    if let Some(padding) = style.padding {
        modifiers.push(format!("padding({padding}.dp)"));
    }
    if let Some(width) = style.width {
        modifiers.push(format!("width({width}.dp)"));
    }
    if let Some(height) = style.height {
        modifiers.push(format!("height({height}.dp)"));
    }
    if let Some(background) = style.background {
        modifiers.push(format!("background({})", glance_color(background)));
    }
    if let Some(radius) = style.corner_radius {
        modifiers.push(format!("cornerRadius({radius}.dp)"));
    }
    let modifier = if modifiers.is_empty() {
        String::new()
    } else {
        format!("modifier = GlanceModifier.{}, ", modifiers.join("."))
    };
    let container = match kind {
        LayoutKind::Column => "GlanceColumn",
        LayoutKind::Row => "GlanceRow",
        LayoutKind::Stack => "GlanceBox",
    };
    let alignment = match (kind, style.alignment) {
        (LayoutKind::Column, Some(nexa_ir::Alignment::Start)) => {
            "horizontalAlignment = androidx.glance.layout.Alignment.Horizontal.Start".to_owned()
        }
        (LayoutKind::Column, Some(nexa_ir::Alignment::Center)) => {
            "horizontalAlignment = androidx.glance.layout.Alignment.Horizontal.CenterHorizontally"
                .to_owned()
        }
        (LayoutKind::Column, Some(nexa_ir::Alignment::End)) => {
            "horizontalAlignment = androidx.glance.layout.Alignment.Horizontal.End".to_owned()
        }
        (LayoutKind::Row, Some(nexa_ir::Alignment::Start)) => {
            "verticalAlignment = androidx.glance.layout.Alignment.Vertical.Top".to_owned()
        }
        (LayoutKind::Row, Some(nexa_ir::Alignment::Center)) => {
            "verticalAlignment = androidx.glance.layout.Alignment.Vertical.CenterVertically"
                .to_owned()
        }
        (LayoutKind::Row, Some(nexa_ir::Alignment::End)) => {
            "verticalAlignment = androidx.glance.layout.Alignment.Vertical.Bottom".to_owned()
        }
        (LayoutKind::Stack, Some(nexa_ir::Alignment::Start)) => {
            "contentAlignment = androidx.glance.layout.Alignment.TopStart".to_owned()
        }
        (LayoutKind::Stack, Some(nexa_ir::Alignment::Center)) => {
            "contentAlignment = androidx.glance.layout.Alignment.Center".to_owned()
        }
        (LayoutKind::Stack, Some(nexa_ir::Alignment::End)) => {
            "contentAlignment = androidx.glance.layout.Alignment.BottomEnd".to_owned()
        }
        _ => String::new(),
    };
    let arguments = match (modifier.is_empty(), alignment.is_empty()) {
        (true, true) => String::new(),
        (false, true) => modifier.trim_end_matches(", ").to_owned(),
        (true, false) => alignment,
        (false, false) => format!("{modifier}{alignment}"),
    };
    out.line_at(depth, format_args!("{container}({arguments}) {{"));
    for (index, child) in children.iter().enumerate() {
        if index > 0 && spacing > 0.0 {
            let dimension = match kind {
                LayoutKind::Row => "width",
                _ => "height",
            };
            out.line_at(
                depth + 1,
                format_args!("GlanceSpacer(modifier = GlanceModifier.{dimension}({spacing}.dp))"),
            );
        }
        render_node(child, depth + 1, out, widget)?;
    }
    out.line_at(depth, format_args!("}}"));
    Ok(())
}

fn glance_color(color: nexa_ir::ColorValue) -> String {
    fn value(color: nexa_ir::Color) -> String {
        format!(
            "androidx.compose.ui.graphics.Color(0x{:02X}{:02X}{:02X}{:02X}L)",
            color.alpha, color.red, color.green, color.blue
        )
    }
    match color {
        nexa_ir::ColorValue::Static(color) => format!(
            "androidx.glance.color.ColorProvider(day = {}, night = {})",
            value(color),
            value(color)
        ),
        nexa_ir::ColorValue::Adaptive { light, dark } => format!(
            "androidx.glance.color.ColorProvider(day = {}, night = {})",
            value(light),
            value(dark)
        ),
    }
}

fn glance_sizes(families: &[WidgetFamily]) -> String {
    let sizes = families
        .iter()
        .map(|family| match family {
            WidgetFamily::Small => "DpSize(110.dp, 110.dp)".to_owned(),
            WidgetFamily::Medium => "DpSize(250.dp, 110.dp)".to_owned(),
            WidgetFamily::Large => "DpSize(250.dp, 180.dp)".to_owned(),
            WidgetFamily::ExtraLarge => "DpSize(375.dp, 245.dp)".to_owned(),
        })
        .collect::<Vec<_>>()
        .join(", ");
    format!("setOf({sizes})")
}

fn widget_text_resource(text: &str) -> String {
    format!(
        "stringResource(R.string.{})",
        nexa_codegen::names::localization_resource_name(text)
    )
}

#[cfg(test)]
mod tests {
    use super::generate;
    use nexa_ir::{Expr, Module, Node, StructDecl, TextStyle, Type, Widget, WidgetFamily};

    #[test]
    fn emits_glance_receiver_provider_and_widget_composables() {
        let mut module = Module {
            app_name: "Example".into(),
            plugins: vec![],
            plugin_assets: vec![],
            enums: vec![],
            structs: vec![StructDecl {
                name: "WidgetEntry".into(),
                fields: vec![],
            }],
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
                return_type: Type::Struct {
                    name: "WidgetEntry".into(),
                    fields: vec![],
                },
                is_async: false,
                is_throwing: false,
                is_constructor: false,
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
            body: vec![
                Node::Text {
                    value: Expr::String("Tasks".into()),
                    style: TextStyle::default(),
                },
                Node::Accessibility {
                    label: Expr::String("Calendar".into()),
                    hint: None,
                    value: None,
                    role: nexa_ir::AccessibilityRole::None,
                    children: vec![Node::SystemIcon {
                        icon: nexa_ir::SystemIcon::Shared("calendar".into()),
                        description: String::new(),
                        size: 16.0,
                        tint: nexa_ir::ColorExpression::Static(nexa_ir::ColorValue::Static(
                            nexa_ir::Color {
                                red: 0,
                                green: 0,
                                blue: 0,
                                alpha: 255,
                            },
                        )),
                    }],
                },
            ],
        });
        let generated = generate(&module).expect("valid widget IR");
        let source = generated
            .sources
            .units
            .iter()
            .find(|unit| unit.name == "widget-Tasks.kt")
            .map(|unit| unit.contents.as_str())
            .unwrap_or_default();
        assert!(source.contains("GlanceAppWidgetReceiver"));
        assert!(source.contains("provideGlance"));
        assert!(source.contains("Text(text = \"Tasks\")"));
        assert!(generated.sources.units.iter().any(|unit| {
            unit.name == "NexaWidgetFamily.kt"
                && unit.contents.contains("internal fun nexaWidgetFamily")
                && unit
                    .contents
                    .contains("internal fun nexaWidgetColorFromHex")
        }));
        assert!(source.contains("contentDescription = \"Calendar\""));
        assert!(
            generated
                .resources
                .iter()
                .any(|resource| resource.name == "drawable/nexa_widget_calendar.xml")
        );

        let widget = &mut module.widgets[0];
        widget.entry_provider = Expr::Await(Box::new(Expr::Call {
            name: "WidgetEntry.load".into(),
            arguments: vec![],
            return_type: Type::Struct {
                name: "WidgetEntry".into(),
                fields: vec![],
            },
            is_async: true,
            is_throwing: false,
            is_constructor: false,
        }));
        widget.placeholder_provider = Some(Expr::Call {
            name: "WidgetEntry.loading".into(),
            arguments: vec![],
            return_type: Type::Struct {
                name: "WidgetEntry".into(),
                fields: vec![],
            },
            is_async: false,
            is_throwing: false,
            is_constructor: false,
        });
        widget.entry_provider_async = true;
        let generated = generate(&module).expect("valid async widget IR");
        let source = generated
            .sources
            .units
            .iter()
            .find(|unit| unit.name == "widget-Tasks.kt")
            .map(|unit| unit.contents.as_str())
            .unwrap_or_default();
        assert!(
            source.contains("val nexa_entry: NexaWidgetEntry = NexaWidgetEntry.nexa_fn_load()"),
            "{source}"
        );
        assert!(source.find("val nexa_entry:").unwrap() < source.find("provideContent {").unwrap());
    }
}
