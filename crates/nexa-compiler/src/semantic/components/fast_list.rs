//! Typed fast_list component lowering. Keep recursive nodes in separate stack frames.
use super::*;

#[inline(never)]
pub(super) fn lower_fast_list_component(
    inv: ast::ComponentInvocation,
    cx: &SemanticContext<'_>,
) -> Result<Node, CompileError> {
    let symbols = cx.symbols;
    let themes = cx.themes;
    let functions = cx.functions;
    let native_aliases = cx.native_aliases;
    let child_cx = cx.with_navigation(false, cx.allow_navigation_back);

    let span = inv.span;
    let mut args = inv.arguments;
    let rows = match inv.children {
        ast::ChildBody::Rows(rows) => rows,
        _ => return Err(child_mismatch(span)),
    };
    let ast::ListRows {
        source,
        key,
        item,
        index,
        section,
        children,
    } = rows;
    let axis = args.remove("axis");
    let item_extent = args.remove("rowHeight");
    let gap = optional_dimension(
        args.remove("gap"),
        "FastList gap",
        Some(ast::ThemeTokenKind::Spacing),
        themes,
    )?
    .unwrap_or(0.0);
    let scroll_position = args.remove("scrollPosition");
    let scroll_enabled = args
        .remove("scrollEnabled")
        .map(|enabled| lower_expr(&enabled, Some(&Type::Bool), &cx.exprs(false)))
        .transpose()?;
    let reverse_layout = optional_bool(
        args.remove("reverseLayout"),
        false,
        "FastList reverseLayout",
    )?;
    let page_snap = optional_bool(args.remove("pageSnap"), false, "FastList pageSnap")?;
    let show_dividers = optional_bool(args.remove("showDividers"), true, "FastList showDividers")?;
    let hide_horizontal_scrollbar = optional_bool(
        args.remove("hideHorizontalScrollbar"),
        false,
        "FastList hideHorizontalScrollbar",
    )?;
    let hide_vertical_scrollbar = optional_bool(
        args.remove("hideVerticalScrollbar"),
        page_snap,
        "FastList hideVerticalScrollbar",
    )?;
    let component_ref = args
        .remove("ref")
        .map(|reference| {
            lower_expr(
                &reference,
                Some(&Type::ComponentRef {
                    namespace: "Nexa".to_owned(),
                    name: "FastList".to_owned(),
                }),
                &cx.exprs(false),
            )
        })
        .transpose()?;
    let mut modifiers = inv.modifiers;
    let on_end_reached = take_modifier_actions(&mut modifiers, span, "onEndReached")?;
    let on_scroll = take_modifier_actions(&mut modifiers, span, "onScroll")?;
    let on_scroll_begin_drag = take_modifier_actions(&mut modifiers, span, "onScrollBeginDrag")?;
    let on_move = match take_modifier_decl(&mut modifiers, "onMove") {
        Some(mut modifier) => {
            let enabled = match modifier.arguments.remove("enabled") {
                Some(expression) => lower_expr(&expression, Some(&Type::Bool), &cx.exprs(false))?,
                None => Expr::Bool(true),
            };
            if let Some(unknown) = modifier.arguments.keys().next() {
                return Err(CompileError::new(
                    modifier.span,
                    format!("unknown `.onMove` argument `{unknown}`; expected `enabled`"),
                ));
            }
            match modifier.body {
                ast::ModifierBody::EventActions {
                    parameters,
                    actions,
                } => {
                    if parameters.len() != 2 {
                        return Err(CompileError::new(
                            span,
                            ".onMove requires two bindings: from and to",
                        ));
                    }
                    let from = parameters[0].clone();
                    let to = parameters[1].clone();
                    if from == to {
                        return Err(CompileError::new(
                            span,
                            ".onMove bindings must have different names",
                        ));
                    }
                    let mut move_symbols = symbols.clone();
                    for parameter in [&from, &to] {
                        if symbols.contains_key(parameter) {
                            return Err(CompileError::new(
                                span,
                                format!(".onMove binding `{parameter}` shadows an existing value"),
                            ));
                        }
                        move_symbols.insert(
                            parameter.clone(),
                            (Type::Numeric(NumericType::Int32), false),
                        );
                    }
                    let actions = lower_actions_with_aliases(
                        actions,
                        &move_symbols,
                        functions,
                        false,
                        native_aliases,
                        cx.type_registries(),
                    )?;
                    Some(FastListMove {
                        from,
                        to,
                        enabled,
                        actions,
                    })
                }
                _ => return Err(child_mismatch(span)),
            }
        }
        None => None,
    };
    let sticky_header = take_modifier_nodes(&mut modifiers, span, "stickyHeader")?;
    let section_header = take_modifier_nodes(&mut modifiers, span, "sectionHeader")?;
    let swipe_actions = take_modifier_nodes(&mut modifiers, span, "swipeActions")?;
    let sections_source = matches!(&source, ast::ListSource::Sections(_));
    let array_source = matches!(&source, ast::ListSource::Items(_));
    let uses_platform_list = on_move.is_some() || swipe_actions.is_some();
    let axis = match axis {
        None => ListAxis::Vertical,
        Some(ast::Expr::Name(name, axis_span)) => match name.as_str() {
            "Vertical" => ListAxis::Vertical,
            "Horizontal" => ListAxis::Horizontal,
            _ => {
                return Err(CompileError::new(
                    axis_span,
                    "FastList `axis` must be `Vertical`, `Horizontal`, `Grid(columns)`, `AutoGrid(minCellWidth)`, or `Masonry(columns)`",
                ));
            }
        },
        Some(ast::Expr::Call(name, _, arguments, axis_span)) if name == "Grid" => {
            match arguments.as_slice() {
                [ast::Expr::Number(raw, columns_span)] => {
                    let Ok(columns) = raw.parse::<u32>() else {
                        return Err(CompileError::new(
                            *columns_span,
                            "FastList grid columns must be a positive integer literal",
                        ));
                    };
                    if columns == 0 {
                        return Err(CompileError::new(
                            *columns_span,
                            "FastList grid columns must be greater than zero",
                        ));
                    }
                    ListAxis::Grid { columns }
                }
                _ => {
                    return Err(CompileError::new(
                        axis_span,
                        "FastList `Grid` requires one positive integer column count",
                    ));
                }
            }
        }
        Some(ast::Expr::Call(name, _, arguments, axis_span)) if name == "AutoGrid" => {
            match arguments.as_slice() {
                [ast::Expr::Number(raw, width_span)] => {
                    let Ok(min_cell_width) = raw.parse::<u32>() else {
                        return Err(CompileError::new(
                            *width_span,
                            "FastList AutoGrid minimum cell width must be a positive integer literal",
                        ));
                    };
                    if min_cell_width == 0 {
                        return Err(CompileError::new(
                            *width_span,
                            "FastList AutoGrid minimum cell width must be greater than zero",
                        ));
                    }
                    ListAxis::AutoGrid { min_cell_width }
                }
                _ => {
                    return Err(CompileError::new(
                        axis_span,
                        "FastList `AutoGrid` requires one positive integer minimum cell width",
                    ));
                }
            }
        }
        Some(ast::Expr::Call(name, _, arguments, axis_span)) if name == "Masonry" => {
            match arguments.as_slice() {
                [ast::Expr::Number(raw, columns_span)] => {
                    let Ok(columns) = raw.parse::<u32>() else {
                        return Err(CompileError::new(
                            *columns_span,
                            "FastList Masonry columns must be a positive integer literal",
                        ));
                    };
                    if columns == 0 {
                        return Err(CompileError::new(
                            *columns_span,
                            "FastList Masonry columns must be greater than zero",
                        ));
                    }
                    ListAxis::Masonry { columns }
                }
                _ => {
                    return Err(CompileError::new(
                        axis_span,
                        "FastList `Masonry` requires one positive integer column count",
                    ));
                }
            }
        }
        Some(expression) => {
            return Err(CompileError::new(
                expression.span(),
                "FastList `axis` must be `Vertical`, `Horizontal`, `Grid(columns)`, `AutoGrid(minCellWidth)`, or `Masonry(columns)`",
            ));
        }
    };
    if uses_platform_list
        && (!matches!(axis, ListAxis::Vertical)
            || reverse_layout
            || page_snap
            || item_extent.is_some()
            || on_end_reached.is_some()
            || on_scroll.is_some()
            || on_scroll_begin_drag.is_some()
            || sticky_header.is_some())
    {
        return Err(CompileError::new(
            span,
            "FastList `.onMove` and `.swipeActions` require a vertical list without custom scroll behavior",
        ));
    }
    if on_move.is_some() && !array_source {
        return Err(CompileError::new(
            span,
            "FastList `.onMove` requires an array source",
        ));
    }
    if sticky_header.is_some() && !matches!(axis, ListAxis::Vertical) {
        return Err(CompileError::new(
            span,
            "FastList `stickyHeader` is supported only for vertical lists",
        ));
    }
    if sections_source && !matches!(axis, ListAxis::Vertical) {
        return Err(CompileError::new(
            span,
            "FastList `sections` is supported only for vertical lists",
        ));
    }
    if matches!(axis, ListAxis::Masonry { .. })
        && (sections_source
            || reverse_layout
            || page_snap
            || item_extent.is_some()
            || sticky_header.is_some()
            || on_move.is_some()
            || swipe_actions.is_some())
    {
        return Err(CompileError::new(
            span,
            "FastList `Masonry` requires flat variable-height rows without reverseLayout, pageSnap, rowHeight, stickyHeader, reorder, or swipe actions",
        ));
    }
    if reverse_layout && !matches!(axis, ListAxis::Vertical) {
        return Err(CompileError::new(
            span,
            "FastList `reverseLayout` is supported only for vertical lists",
        ));
    }
    if reverse_layout && sections_source {
        return Err(CompileError::new(
            span,
            "FastList `reverseLayout` is supported only for flat count or collection sources",
        ));
    }
    if reverse_layout && sticky_header.is_some() {
        return Err(CompileError::new(
            span,
            "FastList `reverseLayout` cannot be combined with `stickyHeader`",
        ));
    }
    if page_snap && !matches!(axis, ListAxis::Vertical) {
        return Err(CompileError::new(
            span,
            "FastList `pageSnap` is supported only for vertical lists",
        ));
    }
    if page_snap && sections_source {
        return Err(CompileError::new(
            span,
            "FastList `pageSnap` is supported only for flat count or collection sources",
        ));
    }
    if page_snap && reverse_layout {
        return Err(CompileError::new(
            span,
            "FastList `pageSnap` cannot be combined with `reverseLayout`",
        ));
    }
    if page_snap && sticky_header.is_some() {
        return Err(CompileError::new(
            span,
            "FastList `pageSnap` cannot be combined with `stickyHeader`",
        ));
    }
    if page_snap && item_extent.is_some() {
        return Err(CompileError::new(
            span,
            "FastList `pageSnap` sets each row to the viewport height; omit `rowHeight`",
        ));
    }
    if sections_source && sticky_header.is_some() {
        return Err(CompileError::new(
            span,
            "FastList `sections` uses `sectionHeader` instead of `stickyHeader`",
        ));
    }
    if !sections_source && section.is_some() {
        return Err(CompileError::new(
            span,
            "FastList `section` is only available with a `sections` source",
        ));
    }
    if !sections_source && section_header.is_some() {
        return Err(CompileError::new(
            span,
            "FastList `sectionHeader` is only available with a `sections` source",
        ));
    }
    if sections_source
        && (scroll_position.is_some()
            || on_end_reached.is_some()
            || on_scroll.is_some()
            || on_scroll_begin_drag.is_some())
    {
        return Err(CompileError::new(
            span,
            "FastList `sections` does not yet support scrollPosition, onEndReached, onScroll, or onScrollBeginDrag",
        ));
    }
    if on_scroll_begin_drag.is_some()
        && (!matches!(axis, ListAxis::Vertical) || page_snap || reverse_layout)
    {
        return Err(CompileError::new(
            span,
            "FastList `.onScrollBeginDrag` currently requires a forward vertical list",
        ));
    }
    let item_extent = optional_dimension(item_extent, "FastList rowHeight", None, themes)?;
    if item_extent.is_some_and(|value| value <= 0.0) {
        return Err(CompileError::new(
            span,
            "FastList `rowHeight` must be greater than zero",
        ));
    }
    if gap < 0.0 {
        return Err(CompileError::new(
            span,
            "FastList `gap` must be zero or greater",
        ));
    }
    if page_snap && gap > 0.0 {
        return Err(CompileError::new(
            span,
            "FastList `pageSnap` fills the viewport; `gap` must be zero",
        ));
    }
    let row_index_type = Type::Numeric(NumericType::Int32);
    let section_index_type = Type::Numeric(NumericType::Int32);
    let has_section_binding = section.is_some();
    let section_name = binding_name(section, "section", "FastList section")?;
    let index = binding_name(index, "index", "FastList index")?;
    let scroll_position = scroll_position
        .map(|position| {
            require_mutable_binding(
                &position,
                &Type::Numeric(NumericType::Int32),
                symbols,
                position.span(),
                "FastList scrollPosition",
            )
        })
        .transpose()?;
    let (parts, section, item, item_type) = match source {
        ast::ListSource::Count(count) => {
            if item.is_some() || has_section_binding {
                return Err(CompileError::new(
                    span,
                    "FastList item and section bindings are only available with an array or `sections` source",
                ));
            }
            let count_value = lower_expr(&count, Some(&row_index_type), &cx.exprs(false))?;
            if let ast::Expr::Number(raw, count_span) = &count
                && raw.parse::<i32>().is_ok_and(|value| value < 0)
            {
                return Err(CompileError::new(
                    *count_span,
                    "FastList count must be non-negative",
                ));
            }
            (
                ListSourceParts::Count { count: count_value },
                None,
                None,
                None,
            )
        }
        ast::ListSource::Items(collection) => {
            if has_section_binding {
                return Err(CompileError::new(
                    span,
                    "FastList `section` is only available with a `sections` source",
                ));
            }
            let collection_type =
                infer_expr_type(&collection, symbols, functions).ok_or_else(|| {
                    CompileError::new(
                        collection.span(),
                        "FastList positional source must have a statically known Array<T> type",
                    )
                })?;
            let Type::Array(element_type) = &collection_type else {
                return Err(CompileError::new(
                    collection.span(),
                    "FastList collection must have type Array<T>",
                ));
            };
            let item_name = binding_name(item, "item", "FastList item")?;
            if item_name == index {
                return Err(CompileError::new(
                    collection.span(),
                    "FastList item and index bindings must have different names",
                ));
            }
            let collection = lower_expr(&collection, Some(&collection_type), &cx.exprs(false))?;
            (
                ListSourceParts::Items {
                    collection,
                    element_type: (**element_type).clone(),
                    item: item_name.clone(),
                },
                None,
                Some(item_name),
                Some((**element_type).clone()),
            )
        }
        ast::ListSource::Sections(collection) => {
            let collection_type =
                infer_expr_type(&collection, symbols, functions).ok_or_else(|| {
                    CompileError::new(
                        collection.span(),
                        "FastList `sections` must have a statically known Array<Array<T>> type",
                    )
                })?;
            let Type::Array(section_type) = &collection_type else {
                return Err(CompileError::new(
                    collection.span(),
                    "FastList `sections` must have type Array<Array<T>>",
                ));
            };
            let Type::Array(element_type) = &**section_type else {
                return Err(CompileError::new(
                    collection.span(),
                    "FastList `sections` must have type Array<Array<T>>",
                ));
            };
            let item_name = binding_name(item, "item", "FastList item")?;
            if item_name == index || item_name == section_name || index == section_name {
                return Err(CompileError::new(
                    collection.span(),
                    "FastList section, item, and index bindings must have different names",
                ));
            }
            let collection = lower_expr(&collection, Some(&collection_type), &cx.exprs(false))?;
            (
                ListSourceParts::Sections {
                    collection,
                    element_type: (**element_type).clone(),
                    section: section_name.clone(),
                    item: item_name.clone(),
                },
                Some(section_name.clone()),
                Some(item_name),
                Some((**element_type).clone()),
            )
        }
    };
    let mut row_symbols = symbols.clone();
    row_symbols.insert(index.clone(), (row_index_type, false));
    if let Some(section_name) = &section {
        row_symbols.insert(section_name.clone(), (section_index_type, false));
    }
    if let (Some(item_name), Some(item_type)) = (&item, &item_type) {
        row_symbols.insert(item_name.clone(), (item_type.clone(), false));
    }
    let row_cx = child_cx.with_symbols(&row_symbols);
    let key = key
        .map(|key| {
            let item_name = item.as_deref().ok_or_else(|| {
                CompileError::new(
                    span,
                    "FastList `key` requires a collection source with an item binding",
                )
            })?;
            item_type.as_ref().ok_or_else(|| {
                CompileError::new(
                    span,
                    "FastList `key` requires a collection source with an item binding",
                )
            })?;
            let key = match key {
                ast::ListKey::SelfValue(id_span) => ast::Expr::Name(item_name.to_owned(), id_span),
                ast::ListKey::Member {
                    name,
                    span: id_span,
                } => ast::Expr::Member {
                    base: Box::new(ast::Expr::Name(item_name.to_owned(), id_span)),
                    name,
                    optional: false,
                    span: id_span,
                },
            };
            let key_type =
                super::super::expressions::infer_expr_type(&key, &row_symbols, functions)
                    .ok_or_else(|| {
                        CompileError::new(
                            key.span(),
                            "FastList `key` must resolve to a statically known scalar type",
                        )
                    })?;
            super::super::expressions::require_hashable_key(
                &key_type,
                key.span(),
                "FastList keys",
            )?;
            lower_expr(&key, Some(&key_type), &row_cx.exprs(false))
        })
        .transpose()?;
    let lowered_children = lower_nodes(children, &row_cx)?;
    if lowered_children.is_empty() {
        return Err(CompileError::new(
            span,
            "FastList requires at least one row component",
        ));
    }
    if sticky_header.as_ref().is_some_and(Vec::is_empty) {
        return Err(CompileError::new(
            span,
            "FastList `stickyHeader` requires at least one header component",
        ));
    }
    if section_header.as_ref().is_some_and(Vec::is_empty) {
        return Err(CompileError::new(
            span,
            "FastList `sectionHeader` requires at least one header component",
        ));
    }
    if swipe_actions.as_ref().is_some_and(Vec::is_empty) {
        return Err(CompileError::new(
            span,
            "FastList `swipeActions` requires at least one action component",
        ));
    }
    if swipe_actions.is_some() && lowered_children.len() != 1 {
        return Err(CompileError::new(
            span,
            "FastList `swipeActions` requires exactly one row root component",
        ));
    }
    let swipe_actions = swipe_actions
        .map(|actions| lower_nodes(actions, &row_cx))
        .transpose()?;
    if swipe_actions.as_ref().is_some_and(|actions| {
        actions.len() != 1 || !matches!(actions.first(), Some(Node::Button { .. }))
    }) {
        return Err(CompileError::new(
            span,
            "FastList `swipeActions` currently requires one Button action",
        ));
    }
    let on_end_reached = on_end_reached
        .map(|actions| {
            lower_actions_with_aliases(
                actions,
                symbols,
                functions,
                false,
                native_aliases,
                cx.type_registries(),
            )
        })
        .transpose()?;
    let on_scroll = on_scroll
        .map(|actions| {
            lower_actions_with_aliases(
                actions,
                symbols,
                functions,
                false,
                native_aliases,
                cx.type_registries(),
            )
        })
        .transpose()?;
    let on_scroll_begin_drag = on_scroll_begin_drag
        .map(|actions| {
            lower_actions_with_aliases(
                actions,
                symbols,
                functions,
                false,
                native_aliases,
                cx.type_registries(),
            )
        })
        .transpose()?;
    let sticky_header = sticky_header
        .map(|header| lower_nodes(header, &child_cx))
        .transpose()?;
    let section_header = section_header
        .map(|header| {
            let mut header_symbols = symbols.clone();
            if let Some(section_name) = &section {
                header_symbols.insert(
                    section_name.clone(),
                    (Type::Numeric(NumericType::Int32), false),
                );
            }
            if sections_source && let Some(item_type) = &item_type {
                header_symbols.insert(
                    "sectionItems".to_owned(),
                    (Type::Array(Box::new(item_type.clone())), false),
                );
            }
            lower_nodes(header, &child_cx.with_symbols(&header_symbols))
        })
        .transpose()?;
    Ok(Node::FastList {
        plan: Box::new(match parts {
            ListSourceParts::Count { count } => ListPlan::Count {
                count,
                common: ListCommon {
                    axis,
                    show_dividers,
                    hide_horizontal_scrollbar,
                    hide_vertical_scrollbar,
                    reverse_layout,
                    page_snap,
                    item_extent,
                    gap,
                    scroll_enabled,
                    index,
                    key,
                    scroll_position: scroll_position.clone(),
                    component_ref: component_ref.clone(),
                    children: lowered_children,
                    on_end_reached,
                    on_scroll,
                    on_scroll_begin_drag,
                    on_move: None,
                    swipe_actions,
                    sticky_header,
                    refresh: None,
                },
            },
            ListSourceParts::Items {
                collection,
                element_type,
                item,
            } => ListPlan::Items {
                collection,
                element_type,
                item,
                common: ListCommon {
                    axis,
                    show_dividers,
                    hide_horizontal_scrollbar,
                    hide_vertical_scrollbar,
                    reverse_layout,
                    page_snap,
                    item_extent,
                    gap,
                    scroll_enabled,
                    index,
                    key,
                    scroll_position: scroll_position.clone(),
                    component_ref: component_ref.clone(),
                    children: lowered_children,
                    on_end_reached,
                    on_scroll,
                    on_scroll_begin_drag,
                    on_move,
                    swipe_actions,
                    sticky_header,
                    refresh: None,
                },
            },
            ListSourceParts::Sections {
                collection,
                element_type,
                section,
                item,
            } => {
                // Lowering rejects scroll observers and sticky headers
                // for sectioned lists above, so only the sectioned
                // options can be populated here.
                debug_assert!(
                    on_end_reached.is_none()
                        && on_scroll.is_none()
                        && on_scroll_begin_drag.is_none()
                        && sticky_header.is_none()
                );
                ListPlan::Sections {
                    collection,
                    element_type,
                    section,
                    item,
                    common: SectionedListCommon {
                        show_dividers,
                        hide_horizontal_scrollbar,
                        hide_vertical_scrollbar,
                        item_extent,
                        gap,
                        scroll_enabled,
                        index,
                        key,
                        component_ref,
                        children: lowered_children,
                        section_header,
                        swipe_actions,
                        refresh: None,
                    },
                }
            }
        }),
    })
}
