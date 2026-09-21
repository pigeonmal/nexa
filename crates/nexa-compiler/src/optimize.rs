use std::collections::HashSet;

use nexa_ir::{
    Action, BinaryOp, Expr, InterpolatedPart, LayoutKind, ListSource, Module, Node, NumericType,
    ViewStyle,
};

/// Applies small, semantics-preserving optimizations to the typed IR before
/// either native backend sees it. The pass deliberately stays conservative:
/// it folds only pure constant expressions, removes branches whose conditions
/// are statically known, and drops state declarations with no reachable use.
pub(crate) fn optimize(module: &mut Module) {
    module.states.iter_mut().for_each(|state| {
        state.initial = fold_expression(std::mem::replace(&mut state.initial, Expr::Bool(false)));
    });
    module.body = optimize_nodes(std::mem::take(&mut module.body));
    if let Some(actions) = &mut module.on_appear {
        *actions = optimize_actions(std::mem::take(actions));
    }
    if let Some(actions) = &mut module.on_disappear {
        *actions = optimize_actions(std::mem::take(actions));
    }
    for screen in &mut module.screens {
        screen.body = optimize_nodes(std::mem::take(&mut screen.body));
        if let Some(actions) = &mut screen.on_appear {
            *actions = optimize_actions(std::mem::take(actions));
        }
        if let Some(actions) = &mut screen.on_disappear {
            *actions = optimize_actions(std::mem::take(actions));
        }
    }
    for component in &mut module.components {
        component.body = optimize_nodes(std::mem::take(&mut component.body));
        for state in &mut component.states {
            state.initial =
                fold_expression(std::mem::replace(&mut state.initial, Expr::Bool(false)));
        }
    }
    prune_unused_states(module);
}

fn prune_unused_states(module: &mut Module) {
    let mut used = HashSet::new();
    collect_node_state_references(&module.body, &mut used);
    if let Some(actions) = &module.on_appear {
        collect_action_state_references(actions, &mut used);
    }
    if let Some(actions) = &module.on_disappear {
        collect_action_state_references(actions, &mut used);
    }
    for screen in &module.screens {
        collect_node_state_references(&screen.body, &mut used);
        if let Some(actions) = &screen.on_appear {
            collect_action_state_references(actions, &mut used);
        }
        if let Some(actions) = &screen.on_disappear {
            collect_action_state_references(actions, &mut used);
        }
    }
    retain_referenced_states(&mut module.states, used);

    for component in &mut module.components {
        let mut used = HashSet::new();
        collect_node_state_references(&component.body, &mut used);
        retain_referenced_states(&mut component.states, used);
    }
}

fn retain_referenced_states(states: &mut Vec<nexa_ir::State>, mut used: HashSet<String>) {
    loop {
        let previous_len = used.len();
        let initializers = states
            .iter()
            .filter(|state| used.contains(&state.name))
            .map(|state| state.initial.clone())
            .collect::<Vec<_>>();
        for initializer in &initializers {
            collect_expression_state_references(initializer, &mut used);
        }
        if used.len() == previous_len {
            break;
        }
    }
    states.retain(|state| used.contains(&state.name));
}

fn collect_node_state_references(nodes: &[Node], used: &mut HashSet<String>) {
    let mut bindings = Vec::new();
    nexa_ir::walk::walk_ir(
        nodes,
        &mut |node| match node {
            Node::Button { actions, .. } | Node::Pressable { actions, .. } => {
                collect_action_bindings(actions, &mut bindings);
            }
            Node::RefreshControl { state, actions, .. } => {
                collect_action_bindings(actions, &mut bindings);
                bindings.push(state.clone());
            }
            Node::TextInput { state, .. }
            | Node::Switch { state, .. }
            | Node::BottomSheet { state, .. }
            | Node::AppBottomBar { state, .. } => {
                bindings.push(state.clone());
            }
            _ => {}
        },
        &mut |expression| {
            insert_state_reference(expression, used);
        },
    );
    used.extend(bindings);
}

fn collect_action_bindings(actions: &[Action], used: &mut Vec<String>) {
    for action in actions {
        match action {
            Action::Assign { name, .. } => used.push(name.clone()),
            Action::If {
                then_branch,
                else_branch,
                ..
            } => {
                collect_action_bindings(then_branch, used);
                if let Some(else_branch) = else_branch {
                    collect_action_bindings(else_branch, used);
                }
            }
        }
    }
}

fn collect_action_state_references(actions: &[Action], used: &mut HashSet<String>) {
    for action in actions {
        match action {
            Action::Assign { name, value } => {
                used.insert(name.clone());
                collect_expression_state_references(value, used);
            }
            Action::If {
                condition,
                then_branch,
                else_branch,
            } => {
                collect_expression_state_references(condition, used);
                collect_action_state_references(then_branch, used);
                if let Some(else_branch) = else_branch {
                    collect_action_state_references(else_branch, used);
                }
            }
        }
    }
}

fn collect_expression_state_references(expression: &Expr, used: &mut HashSet<String>) {
    nexa_ir::walk::walk_expression(expression, &mut |expression| {
        insert_state_reference(expression, used);
    });
}

fn insert_state_reference(expression: &Expr, used: &mut HashSet<String>) {
    if let Expr::State(name, _) = expression {
        used.insert(name.clone());
    }
}

fn optimize_nodes(nodes: Vec<Node>) -> Vec<Node> {
    let mut optimized = Vec::with_capacity(nodes.len());
    for node in nodes {
        match optimize_node(node) {
            Some(node) => optimized.push(node),
            None => {}
        }
    }
    optimized
}

fn optimize_node(node: Node) -> Option<Node> {
    match node {
        Node::Layout {
            kind,
            spacing,
            style,
            children,
        } => Some(Node::Layout {
            kind,
            spacing,
            style,
            children: optimize_nodes(children),
        }),
        Node::Text { value, style } => Some(Node::Text {
            value: fold_expression(value),
            style,
        }),
        Node::Button { label, actions } => Some(Node::Button {
            label: fold_expression(label),
            actions: optimize_actions(actions),
        }),
        Node::Pressable {
            disabled,
            children,
            actions,
        } => Some(Node::Pressable {
            disabled,
            children: optimize_nodes(children),
            actions: optimize_actions(actions),
        }),
        Node::FastList {
            source,
            index,
            item,
            children,
        } => Some(Node::FastList {
            source: optimize_list_source(source),
            index,
            item,
            children: optimize_nodes(children),
        }),
        Node::If {
            condition,
            then_body,
            else_body,
        } => {
            let condition = fold_expression(condition);
            let then_body = optimize_nodes(then_body);
            let else_body = else_body.map(optimize_nodes);
            match condition {
                Expr::Bool(true) => collapse_nodes(then_body),
                Expr::Bool(false) => collapse_nodes(else_body.unwrap_or_default()),
                condition => Some(Node::If {
                    condition,
                    then_body,
                    else_body,
                }),
            }
        }
        Node::ComponentCall { name, arguments } => Some(Node::ComponentCall {
            name,
            arguments: arguments
                .into_iter()
                .map(|(name, value)| (name, fold_expression(value)))
                .collect(),
        }),
        node @ (Node::TextInput { .. }
        | Node::Switch { .. }
        | Node::Image { .. }
        | Node::StatusBar { .. }
        | Node::Direction { .. }
        | Node::NavigationStack { .. }) => Some(node),
        Node::OnAppear { actions } => Some(Node::OnAppear {
            actions: optimize_actions(actions),
        }),
        Node::OnDisappear { actions } => Some(Node::OnDisappear {
            actions: optimize_actions(actions),
        }),
        Node::NavigationLink {
            destination,
            children,
        } => Some(Node::NavigationLink {
            destination,
            children: optimize_nodes(children),
        }),
        Node::Link { url, children } => Some(Node::Link {
            url,
            children: optimize_nodes(children),
        }),
        Node::Accessibility {
            label,
            role,
            children,
        } => Some(Node::Accessibility {
            label,
            role,
            children: optimize_nodes(children),
        }),
        Node::KeyboardAware { children } => Some(Node::KeyboardAware {
            children: optimize_nodes(children),
        }),
        Node::BottomSheet { state, children } => Some(Node::BottomSheet {
            state,
            children: optimize_nodes(children),
        }),
        Node::RefreshControl {
            state,
            children,
            actions,
        } => Some(Node::RefreshControl {
            state,
            children: optimize_nodes(children),
            actions: optimize_actions(actions),
        }),
        Node::AppBottomBar { state, tabs } => Some(Node::AppBottomBar {
            state,
            tabs: tabs
                .into_iter()
                .map(|tab| nexa_ir::BottomBarTab {
                    index: tab.index,
                    label: tab.label,
                    children: optimize_nodes(tab.children),
                })
                .collect(),
        }),
    }
}

fn collapse_nodes(nodes: Vec<Node>) -> Option<Node> {
    match nodes.len() {
        0 => None,
        1 => Some(nodes.into_iter().next().expect("length checked")),
        _ => Some(Node::Layout {
            kind: LayoutKind::Column,
            spacing: 0.0,
            style: ViewStyle::default(),
            children: nodes,
        }),
    }
}

fn optimize_list_source(source: ListSource) -> ListSource {
    match source {
        ListSource::Count(count) => ListSource::Count(fold_expression(count)),
        ListSource::Items {
            collection,
            element_type,
        } => ListSource::Items {
            collection: fold_expression(collection),
            element_type,
        },
    }
}

fn optimize_actions(actions: Vec<Action>) -> Vec<Action> {
    let mut optimized = Vec::with_capacity(actions.len());
    for action in actions {
        match action {
            Action::Assign { name, value } => optimized.push(Action::Assign {
                name,
                value: fold_expression(value),
            }),
            Action::If {
                condition,
                then_branch,
                else_branch,
            } => {
                let condition = fold_expression(condition);
                let then_branch = optimize_actions(then_branch);
                let else_branch = else_branch.map(optimize_actions);
                match condition {
                    Expr::Bool(true) => optimized.extend(then_branch),
                    Expr::Bool(false) => optimized.extend(else_branch.unwrap_or_default()),
                    condition => optimized.push(Action::If {
                        condition,
                        then_branch,
                        else_branch,
                    }),
                }
            }
        }
    }
    optimized
}

fn fold_expression(expression: Expr) -> Expr {
    match expression {
        Expr::Not(value) => match fold_expression(*value) {
            Expr::Bool(value) => Expr::Bool(!value),
            value => Expr::Not(Box::new(value)),
        },
        Expr::Binary { op, left, right } => {
            let left = fold_expression(*left);
            let right = fold_expression(*right);
            match (op, &left, &right) {
                (BinaryOp::And, Expr::Bool(false), _) => Expr::Bool(false),
                (BinaryOp::And, Expr::Bool(true), _) => right,
                (BinaryOp::Or, Expr::Bool(true), _) => Expr::Bool(true),
                (BinaryOp::Or, Expr::Bool(false), _) => right,
                _ => evaluate_binary(op, &left, &right).unwrap_or(Expr::Binary {
                    op,
                    left: Box::new(left),
                    right: Box::new(right),
                }),
            }
        }
        Expr::Add(left, right, ty) => {
            let left = fold_expression(*left);
            let right = fold_expression(*right);
            fold_numeric_add(&left, &right, ty)
                .unwrap_or_else(|| Expr::Add(Box::new(left), Box::new(right), ty))
        }
        Expr::Array(values) => Expr::Array(values.into_iter().map(fold_expression).collect()),
        Expr::Set(values) => Expr::Set(values.into_iter().map(fold_expression).collect()),
        Expr::Map(entries) => Expr::Map(
            entries
                .into_iter()
                .map(|(key, value)| (fold_expression(key), fold_expression(value)))
                .collect(),
        ),
        Expr::Pair(first, second) => Expr::Pair(
            Box::new(fold_expression(*first)),
            Box::new(fold_expression(*second)),
        ),
        Expr::Triple(first, second, third) => Expr::Triple(
            Box::new(fold_expression(*first)),
            Box::new(fold_expression(*second)),
            Box::new(fold_expression(*third)),
        ),
        Expr::Interpolation(parts) => Expr::Interpolation(
            parts
                .into_iter()
                .map(|part| match part {
                    InterpolatedPart::Literal(value) => InterpolatedPart::Literal(value),
                    InterpolatedPart::Value(value) => {
                        InterpolatedPart::Value(Box::new(fold_expression(*value)))
                    }
                })
                .collect(),
        ),
        expression => expression,
    }
}

fn fold_numeric_add(left: &Expr, right: &Expr, ty: NumericType) -> Option<Expr> {
    let Expr::Number {
        raw: left_raw,
        ty: left_ty,
    } = left
    else {
        return None;
    };
    let Expr::Number {
        raw: right_raw,
        ty: right_ty,
    } = right
    else {
        return None;
    };
    if *left_ty != ty || *right_ty != ty {
        return None;
    }
    let raw = match ty {
        NumericType::Int8 => wrap_signed(
            left_raw.parse::<i128>().ok()? + right_raw.parse::<i128>().ok()?,
            8,
        )
        .to_string(),
        NumericType::Int16 => wrap_signed(
            left_raw.parse::<i128>().ok()? + right_raw.parse::<i128>().ok()?,
            16,
        )
        .to_string(),
        NumericType::Int32 => wrap_signed(
            left_raw.parse::<i128>().ok()? + right_raw.parse::<i128>().ok()?,
            32,
        )
        .to_string(),
        NumericType::Int64 => wrap_signed(
            left_raw.parse::<i128>().ok()? + right_raw.parse::<i128>().ok()?,
            64,
        )
        .to_string(),
        NumericType::UInt8 => wrap_unsigned(
            left_raw.parse::<u128>().ok()? + right_raw.parse::<u128>().ok()?,
            8,
        )
        .to_string(),
        NumericType::UInt16 => wrap_unsigned(
            left_raw.parse::<u128>().ok()? + right_raw.parse::<u128>().ok()?,
            16,
        )
        .to_string(),
        NumericType::UInt32 => wrap_unsigned(
            left_raw.parse::<u128>().ok()? + right_raw.parse::<u128>().ok()?,
            32,
        )
        .to_string(),
        NumericType::UInt64 => wrap_unsigned(
            left_raw.parse::<u128>().ok()? + right_raw.parse::<u128>().ok()?,
            64,
        )
        .to_string(),
        NumericType::Float32 => {
            let value = left_raw.parse::<f32>().ok()? + right_raw.parse::<f32>().ok()?;
            if !value.is_finite() {
                return None;
            }
            value.to_string()
        }
        NumericType::Float64 => {
            let value = left_raw.parse::<f64>().ok()? + right_raw.parse::<f64>().ok()?;
            if !value.is_finite() {
                return None;
            }
            value.to_string()
        }
    };
    if raw.contains('e') || raw.contains('E') {
        return None;
    }
    Some(Expr::Number { raw, ty })
}

fn wrap_signed(value: i128, bits: u32) -> i128 {
    let modulus = 1_i128 << bits;
    let half = 1_i128 << (bits - 1);
    let wrapped = value.rem_euclid(modulus);
    if wrapped >= half {
        wrapped - modulus
    } else {
        wrapped
    }
}

fn wrap_unsigned(value: u128, bits: u32) -> u128 {
    value % (1_u128 << bits)
}

fn evaluate_binary(op: BinaryOp, left: &Expr, right: &Expr) -> Option<Expr> {
    match (op, left, right) {
        (BinaryOp::Equal, Expr::String(left), Expr::String(right)) => {
            Some(Expr::Bool(left == right))
        }
        (BinaryOp::NotEqual, Expr::String(left), Expr::String(right)) => {
            Some(Expr::Bool(left != right))
        }
        (BinaryOp::Equal, Expr::Bool(left), Expr::Bool(right)) => Some(Expr::Bool(left == right)),
        (BinaryOp::NotEqual, Expr::Bool(left), Expr::Bool(right)) => {
            Some(Expr::Bool(left != right))
        }
        (op @ (BinaryOp::Equal | BinaryOp::NotEqual), left, right)
        | (
            op
            @ (BinaryOp::Less | BinaryOp::LessEqual | BinaryOp::Greater | BinaryOp::GreaterEqual),
            left,
            right,
        ) => {
            let (left, right) = numeric_constants(left, right)?;
            let value = match op {
                BinaryOp::Equal => left == right,
                BinaryOp::NotEqual => left != right,
                BinaryOp::Less => left < right,
                BinaryOp::LessEqual => left <= right,
                BinaryOp::Greater => left > right,
                BinaryOp::GreaterEqual => left >= right,
                BinaryOp::And | BinaryOp::Or => return None,
            };
            Some(Expr::Bool(value))
        }
        _ => None,
    }
}

fn numeric_constants(left: &Expr, right: &Expr) -> Option<(ConstantNumber, ConstantNumber)> {
    let Expr::Number {
        raw: left_raw,
        ty: left_ty,
    } = left
    else {
        return None;
    };
    let Expr::Number {
        raw: right_raw,
        ty: right_ty,
    } = right
    else {
        return None;
    };
    if left_ty != right_ty {
        return None;
    }
    Some((
        ConstantNumber::parse(left_raw, *left_ty)?,
        ConstantNumber::parse(right_raw, *right_ty)?,
    ))
}

#[derive(PartialEq, PartialOrd)]
enum ConstantNumber {
    Signed(i128),
    Unsigned(u128),
    Float(f64),
}

impl ConstantNumber {
    fn parse(raw: &str, ty: NumericType) -> Option<Self> {
        Some(match ty {
            NumericType::Int8 | NumericType::Int16 | NumericType::Int32 | NumericType::Int64 => {
                Self::Signed(raw.parse().ok()?)
            }
            NumericType::UInt8
            | NumericType::UInt16
            | NumericType::UInt32
            | NumericType::UInt64 => Self::Unsigned(raw.parse().ok()?),
            NumericType::Float32 => Self::Float(raw.parse::<f32>().ok()? as f64),
            NumericType::Float64 => Self::Float(raw.parse().ok()?),
        })
    }
}
