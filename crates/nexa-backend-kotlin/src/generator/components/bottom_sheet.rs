use nexa_codegen::SourceWriter;
use nexa_codegen::names::state_name;
use nexa_ir::{Action, BottomSheetDetent, ColorExpression, Module, Node};

use crate::generator::{components::render_children, engine::features::Features};

pub(crate) struct BottomSheetRenderArgs<'a> {
    pub(crate) state: &'a str,
    pub(crate) content_padding: f32,
    pub(crate) detents: &'a [BottomSheetDetent],
    pub(crate) snap_points: &'a [f32],
    pub(crate) initial_snap_point: Option<f32>,
    pub(crate) selected_snap_point: Option<&'a str>,
    pub(crate) initial_detent: Option<BottomSheetDetent>,
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
        state,
        content_padding,
        detents,
        snap_points,
        initial_snap_point,
        selected_snap_point,
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
    let detents = if detents.is_empty() {
        &[BottomSheetDetent::Medium][..]
    } else {
        detents
    };
    let enabled_values = std::iter::once("androidx.compose.material3.SheetValue.Hidden")
        .chain(detents.iter().map(|detent| match detent {
            BottomSheetDetent::Medium => "androidx.compose.material3.SheetValue.PartiallyExpanded",
            BottomSheetDetent::Expanded => "androidx.compose.material3.SheetValue.Expanded",
        }))
        .collect::<Vec<_>>()
        .join(", ");
    let initial_detent = initial_detent.unwrap_or(detents[0]);
    let initial_value = native_detent(initial_detent);
    let background_color = background_color.map_or_else(
        || "null".to_owned(),
        crate::generator::colors::expression_for_color,
    );
    let scrim_color = scrim_color.map_or_else(
        || "null".to_owned(),
        crate::generator::colors::expression_for_color,
    );
    let snap_points = snap_points
        .iter()
        .map(|point| format!("{point}f"))
        .collect::<Vec<_>>()
        .join(", ");
    let initial_snap_point =
        initial_snap_point.map_or_else(|| "null".to_owned(), |point| format!("{point}f"));
    let selected_snap_point = selected_snap_point.map(state_name);
    let selected_snap_point_value = selected_snap_point
        .as_ref()
        .map_or_else(|| "null".to_owned(), |state| format!("{state}.toFloat()"));
    let on_snap_point_change = selected_snap_point.as_ref().map_or_else(
        || "null".to_owned(),
        |state| format!("{{ point -> {state} = point.toDouble() }}"),
    );
    let state = state_name(state);

    out.line_at(depth, format_args!("NexaBottomSheetPrimitive("));
    out.line_at(depth + 1, format_args!("isPresented = {state},"));
    out.line_at(
        depth + 1,
        format_args!("contentPadding = {content_padding}f.dp,"),
    );
    out.line_at(
        depth + 1,
        format_args!("enabledValues = setOf({enabled_values}),"),
    );
    out.line_at(depth + 1, format_args!("initialDetent = {initial_value},"));
    out.line_at(
        depth + 1,
        format_args!("snapPoints = listOf({snap_points}),"),
    );
    out.line_at(
        depth + 1,
        format_args!("initialSnapPoint = {initial_snap_point},"),
    );
    out.line_at(
        depth + 1,
        format_args!("selectedSnapPoint = {selected_snap_point_value},"),
    );
    out.line_at(
        depth + 1,
        format_args!("onSnapPointChange = {on_snap_point_change},"),
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
    out.line_at(
        depth + 1,
        format_args!("onDismissRequest = {{ {state} = false }},"),
    );
    out.line_at(depth + 1, format_args!("onDismiss = {{"));
    crate::generator::components::controls::render_actions(on_dismiss, depth + 2, out);
    out.line_at(depth + 1, format_args!("}},"));
    out.line_at(depth + 1, format_args!("content = {{"));
    render_children(children, module, features, depth + 2, out);
    out.line_at(depth + 1, format_args!("}},"));
    out.line_at(depth, format_args!(")"));
}

fn native_detent(detent: BottomSheetDetent) -> &'static str {
    match detent {
        BottomSheetDetent::Medium => "androidx.compose.material3.SheetValue.PartiallyExpanded",
        BottomSheetDetent::Expanded => "androidx.compose.material3.SheetValue.Expanded",
    }
}

#[cfg(test)]
mod tests {
    use super::{native_detent, render_bottom_sheet};
    use nexa_codegen::SourceWriter;
    use nexa_ir::{BottomSheetDetent, Color, ColorExpression, ColorValue, Module, Node};

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
    fn emits_native_detents_and_keeps_children_in_the_sheet_content_slot() {
        assert_eq!(
            native_detent(BottomSheetDetent::Medium),
            "androidx.compose.material3.SheetValue.PartiallyExpanded"
        );
        assert_eq!(
            native_detent(BottomSheetDetent::Expanded),
            "androidx.compose.material3.SheetValue.Expanded"
        );

        let scrim_color = ColorExpression::Static(ColorValue::Static(Color {
            red: 0,
            green: 0,
            blue: 0,
            alpha: 0x33,
        }));
        let mut out = SourceWriter::new();
        render_bottom_sheet(
            super::BottomSheetRenderArgs {
                state: "visible",
                content_padding: 8.0,
                detents: &[BottomSheetDetent::Medium, BottomSheetDetent::Expanded],
                snap_points: &[],
                initial_snap_point: None,
                selected_snap_point: None,
                initial_detent: Some(BottomSheetDetent::Expanded),
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
        assert!(output.contains("SheetValue.PartiallyExpanded"));
        assert!(output.contains("initialDetent = androidx.compose.material3.SheetValue.Expanded"));
        assert!(output.contains("allowsDismissal = false"));
        assert!(output.contains("scrimColor = Color(0x33000000)"));
        assert!(output.contains("content = {"));
        assert!(output.contains("Sheet body"));
    }
}
