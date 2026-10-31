use nexa_codegen::SourceWriter;
use nexa_ir::{Action, ColorExpression, Expr, Module, Node};

use crate::generator::{components::render_children, engine::features::Features};

pub(crate) struct BottomSheetRenderArgs<'a> {
    pub(crate) component_ref: &'a Expr,
    pub(crate) content_sized: bool,
    pub(crate) content_padding: f32,
    pub(crate) corner_radius: Option<f32>,
    pub(crate) detents: &'a [f32],
    pub(crate) initial_detent: Option<f32>,
    pub(crate) show_drag_indicator: bool,
    pub(crate) allows_dismissal: bool,
    pub(crate) allows_background_interaction: bool,
    pub(crate) should_dismiss_on_back_press: bool,
    pub(crate) background_color: Option<&'a ColorExpression>,
    pub(crate) scrim_color: Option<&'a ColorExpression>,
    pub(crate) on_dismiss: &'a [Action],
    pub(crate) children: &'a [Node],
    pub(crate) module: &'a Module,
    pub(crate) features: &'a Features,
    pub(crate) depth: usize,
}

pub(crate) fn render_bottom_sheet(args: BottomSheetRenderArgs<'_>, out: &mut SourceWriter) {
    let BottomSheetRenderArgs {
        component_ref,
        content_sized,
        content_padding,
        corner_radius,
        detents,
        initial_detent,
        show_drag_indicator,
        allows_dismissal,
        allows_background_interaction,
        should_dismiss_on_back_press,
        background_color,
        scrim_color,
        on_dismiss,
        children,
        module,
        features,
        depth,
    } = args;
    let detents = if content_sized {
        &[][..]
    } else if detents.is_empty() {
        &[0.5][..]
    } else {
        detents
    };
    let background_color = background_color.map_or_else(
        || "null".to_owned(),
        crate::generator::colors::expression_for_color,
    );
    let scrim_color = scrim_color.map_or_else(
        || "null".to_owned(),
        crate::generator::colors::expression_for_color,
    );
    let detent_values = detents
        .iter()
        .map(|point| format!("{point}f"))
        .collect::<Vec<_>>()
        .join(", ");
    let initial_detent = if content_sized {
        1.0
    } else {
        initial_detent.unwrap_or(detents[0])
    };
    out.line_at(depth, format_args!("NexaBottomSheetPrimitive("));
    out.line_at(
        depth + 1,
        format_args!(
            "componentRef = {},",
            crate::generator::engine::expressions::expression(component_ref)
        ),
    );
    out.line_at(depth + 1, format_args!("contentSized = {content_sized},"));
    out.line_at(
        depth + 1,
        format_args!("contentPadding = {content_padding}f.dp,"),
    );
    out.line_at(
        depth + 1,
        format_args!(
            "cornerRadius = {},",
            corner_radius.map_or_else(|| "null".to_owned(), |radius| format!("{radius}f.dp"))
        ),
    );
    out.line_at(
        depth + 1,
        format_args!("detents = listOf({detent_values}),"),
    );
    out.line_at(
        depth + 1,
        format_args!("initialDetent = {initial_detent}f,"),
    );
    out.line_at(
        depth + 1,
        format_args!("showDragIndicator = {show_drag_indicator},"),
    );
    out.line_at(
        depth + 1,
        format_args!("allowsDismissal = {allows_dismissal},"),
    );
    out.line_at(
        depth + 1,
        format_args!("allowsBackgroundInteraction = {allows_background_interaction},"),
    );
    out.line_at(
        depth + 1,
        format_args!("shouldDismissOnBackPress = {should_dismiss_on_back_press},"),
    );
    out.line_at(
        depth + 1,
        format_args!("backgroundColor = {background_color},"),
    );
    out.line_at(depth + 1, format_args!("scrimColor = {scrim_color},"));
    out.line_at(depth + 1, format_args!("onDismiss = {{"));
    crate::generator::components::controls::render_actions(on_dismiss, depth + 2, out);
    out.line_at(depth + 1, format_args!("}},"));
    out.line_at(depth + 1, format_args!("content = {{"));
    render_children(children, module, features, depth + 2, out);
    out.line_at(depth + 1, format_args!("}},"));
    out.line_at(depth, format_args!(")"));
}

#[cfg(test)]
mod tests {
    use super::render_bottom_sheet;
    use nexa_codegen::SourceWriter;
    use nexa_ir::{Color, ColorExpression, ColorValue, Module, Node};

    fn empty_module() -> Module {
        Module {
            app_name: "BottomSheet".to_owned(),
            plugins: Vec::new(),
            plugin_assets: Vec::new(),
            enums: Vec::new(),
            structs: Vec::new(),
            functions: Vec::new(),
            background_tasks: Vec::new(),
            states: Vec::new(),
            globals: Vec::new(),
            screens: Vec::new(),
            widgets: Vec::new(),
            components: Vec::new(),
            body: Vec::new(),
            status_bar: None,
            direction: None,
            on_appear: None,
            on_appear_async: false,
            on_disappear: None,
            on_active: None,
            on_inactive: None,
            on_background: None,
        }
    }

    #[test]
    fn emits_fractional_detents_and_keeps_children_in_the_sheet_content_slot() {
        let scrim_color = ColorExpression::Static(ColorValue::Static(Color {
            red: 0,
            green: 0,
            blue: 0,
            alpha: 0x33,
        }));
        let mut out = SourceWriter::new();
        render_bottom_sheet(
            super::BottomSheetRenderArgs {
                component_ref: &nexa_ir::Expr::ComponentRefCreate {
                    namespace: "Nexa".to_owned(),
                    name: "BottomSheet".to_owned(),
                    identity: "sheet-ref".to_owned(),
                },
                content_sized: false,
                content_padding: 0.0,
                corner_radius: None,
                detents: &[0.5, 1.0],
                initial_detent: Some(1.0),
                show_drag_indicator: true,
                allows_dismissal: false,
                allows_background_interaction: false,
                should_dismiss_on_back_press: false,
                background_color: None,
                scrim_color: Some(&scrim_color),
                on_dismiss: &[],
                children: &[Node::Text {
                    value: nexa_ir::Expr::String("Sheet body".to_owned()),
                    style: nexa_ir::TextStyle::default(),
                }],
                module: &empty_module(),
                features: &crate::generator::engine::features::Features::default(),
                depth: 0,
            },
            &mut out,
        );
        let output = out.finish();
        assert!(output.contains("detents = listOf(0.5f, 1f)"));
        assert!(output.contains("initialDetent = 1f"));
        assert!(output.contains("allowsDismissal = false"));
        assert!(output.contains("scrimColor = Color(0x33000000)"));
        assert!(output.contains("content = {"));
        assert!(output.contains("Sheet body"));
    }
}
