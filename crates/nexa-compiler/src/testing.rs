use std::collections::HashMap;

use nexa_diagnostics::Span;
use nexa_ir::{
    Action, ArithmeticOp, BinaryOp, Component, Expr, Function, FunctionLocal, Node, NumericType,
    Type,
};

/// A statically typed `.nx` test body, evaluated only by the host CLI.
#[derive(Clone, Debug)]
pub struct TestCase {
    pub name: String,
    pub source_file: Option<String>,
    pub span: Span,
    pub component: Option<TestComponentMount>,
    pub statements: Vec<TestStatement>,
}

#[derive(Clone, Debug)]
pub struct TestComponentMount {
    pub name: String,
    /// Arguments are ordered by the component's declared parameter list.
    pub arguments: Vec<Expr>,
}

#[derive(Clone, Debug, Default)]
pub struct TestSuite {
    /// Host-only copies of pure app functions referenced by test blocks.
    pub functions: Vec<Function>,
    /// Custom components available to headless test mounts. These copies
    /// exist only in the host-side test suite, never in generated apps.
    pub components: Vec<Component>,
    pub tests: Vec<TestCase>,
}

#[derive(Clone, Debug)]
pub enum TestStatement {
    Let {
        local: FunctionLocal,
        span: Span,
    },
    Assert {
        condition: Expr,
        message: Option<Expr>,
        span: Span,
    },
    Tap {
        label: Expr,
        span: Span,
    },
    AssertText {
        value: Expr,
        span: Span,
    },
}

#[derive(Clone, Debug)]
pub struct TestFailure {
    pub name: String,
    pub source_file: Option<String>,
    pub span: Span,
    pub message: String,
}

#[derive(Clone, Debug, Default)]
pub struct TestRunReport {
    pub passed: usize,
    pub passed_names: Vec<String>,
    pub failures: Vec<TestFailure>,
}

/// Evaluates deterministic, typed test expressions on the host. This evaluator
/// is used by `nexa test` only and is never included in generated apps.
pub fn run_tests(suite: &TestSuite) -> TestRunReport {
    let function_map = suite
        .functions
        .iter()
        .map(|function| (function.name.as_str(), function))
        .collect::<HashMap<_, _>>();
    let component_map = suite
        .components
        .iter()
        .map(|component| (component.name.as_str(), component))
        .collect::<HashMap<_, _>>();
    let mut report = TestRunReport::default();
    for test in &suite.tests {
        match run_test(&function_map, &component_map, test) {
            Ok(()) => {
                report.passed += 1;
                report.passed_names.push(test.name.clone());
            }
            Err((span, message)) => report.failures.push(TestFailure {
                name: test.name.clone(),
                source_file: test.source_file.clone(),
                span,
                message,
            }),
        }
    }
    report
}

#[derive(Clone, Debug, PartialEq)]
enum Value {
    Void,
    Bool(bool),
    String(String),
    Integer(i128),
    Float(f64),
    Enum(String, String),
    Array(Vec<Value>),
}

type Environment = HashMap<String, Value>;
type FunctionMap<'a> = HashMap<&'a str, &'a Function>;

fn run_test(
    functions: &FunctionMap<'_>,
    components: &HashMap<&str, &Component>,
    test: &TestCase,
) -> Result<(), (Span, String)> {
    let mut runtime = test
        .component
        .as_ref()
        .map(|mount| HeadlessRuntime::mount(functions, components, mount))
        .transpose()
        .map_err(|message| (test.span, message))?;
    let mut environment = runtime
        .as_ref()
        .map(|runtime| runtime.instances[ROOT_INSTANCE].environment.clone())
        .unwrap_or_default();

    for statement in &test.statements {
        match statement {
            TestStatement::Let { local, span } => {
                let value = eval_expr(&local.initial, &environment, functions, 0)
                    .map_err(|message| (*span, message))?;
                environment.insert(local.name.clone(), value);
            }
            TestStatement::Assert {
                condition,
                message,
                span,
            } => {
                let condition = eval_expr(condition, &environment, functions, 0)
                    .map_err(|message| (*span, message))?;
                match condition {
                    Value::Bool(true) => {}
                    Value::Bool(false) => {
                        let message = message
                            .as_ref()
                            .map(|message| eval_expr(message, &environment, functions, 0))
                            .transpose()
                            .map_err(|message| (*span, message))?
                            .and_then(|value| match value {
                                Value::String(value) => Some(value),
                                _ => None,
                            })
                            .unwrap_or_else(|| "assertion failed".to_owned());
                        return Err((*span, message));
                    }
                    _ => return Err((*span, "assertion did not evaluate to a Boolean".to_owned())),
                }
            }
            TestStatement::Tap { label, span } => {
                let label = eval_expr(label, &environment, functions, 0)
                    .map_err(|message| (*span, message))?;
                let Value::String(label) = label else {
                    return Err((*span, "tap target did not evaluate to a String".to_owned()));
                };
                let runtime = runtime.as_mut().ok_or_else(|| {
                    (
                        *span,
                        "`tap(...)` requires a component target after `for`".to_owned(),
                    )
                })?;
                runtime.tap(&label).map_err(|message| (*span, message))?;
                if let Some(root) = runtime.instances.get(ROOT_INSTANCE) {
                    for (name, value) in &root.environment {
                        environment.insert(name.clone(), value.clone());
                    }
                }
            }
            TestStatement::AssertText { value, span } => {
                let expected = eval_expr(value, &environment, functions, 0)
                    .map_err(|message| (*span, message))?;
                let Value::String(expected) = expected else {
                    return Err((
                        *span,
                        "expected visible text did not evaluate to a String".to_owned(),
                    ));
                };
                let runtime = runtime.as_mut().ok_or_else(|| {
                    (
                        *span,
                        "`assertText(...)` requires a component target after `for`".to_owned(),
                    )
                })?;
                runtime
                    .assert_text(&expected)
                    .map_err(|message| (*span, message))?;
            }
        }
    }
    Ok(())
}

const ROOT_INSTANCE: &str = "root";

#[derive(Clone)]
struct ComponentInstance {
    component: String,
    environment: Environment,
}

#[derive(Clone)]
struct HeadlessButton {
    label: String,
    instance: String,
    actions: Vec<Action>,
}

#[derive(Default)]
struct RenderSnapshot {
    texts: Vec<String>,
    buttons: Vec<HeadlessButton>,
}

struct HeadlessRuntime<'a> {
    functions: &'a FunctionMap<'a>,
    components: &'a HashMap<&'a str, &'a Component>,
    instances: HashMap<String, ComponentInstance>,
}

impl<'a> HeadlessRuntime<'a> {
    fn mount(
        functions: &'a FunctionMap<'a>,
        components: &'a HashMap<&'a str, &'a Component>,
        mount: &TestComponentMount,
    ) -> Result<Self, String> {
        let component = components.get(mount.name.as_str()).ok_or_else(|| {
            format!(
                "component `{}` is not available to headless tests",
                mount.name
            )
        })?;
        if component.parameters.len() != mount.arguments.len() {
            return Err(format!(
                "component `{}` expects {} argument(s), got {}",
                mount.name,
                component.parameters.len(),
                mount.arguments.len()
            ));
        }

        let empty = Environment::new();
        let arguments = mount
            .arguments
            .iter()
            .map(|argument| eval_expr(argument, &empty, functions, 0))
            .collect::<Result<Vec<_>, _>>()?;
        let environment = Self::new_environment(component, arguments, functions)?;
        Ok(Self {
            functions,
            components,
            instances: HashMap::from([(
                ROOT_INSTANCE.to_owned(),
                ComponentInstance {
                    component: mount.name.clone(),
                    environment,
                },
            )]),
        })
    }

    fn new_environment(
        component: &Component,
        arguments: Vec<Value>,
        functions: &FunctionMap<'_>,
    ) -> Result<Environment, String> {
        let mut environment =
            Environment::with_capacity(component.parameters.len() + component.states.len());
        for (parameter, value) in component.parameters.iter().zip(arguments) {
            environment.insert(parameter.name.clone(), value);
        }
        for state in &component.states {
            let value = eval_expr(&state.initial, &environment, functions, 0)
                .map_err(|message| format!("initializing `{}`: {message}", state.name))?;
            environment.insert(state.name.clone(), value);
        }
        Ok(environment)
    }

    fn tap(&mut self, label: &str) -> Result<(), String> {
        let snapshot = self.render()?;
        let button = snapshot
            .buttons
            .into_iter()
            .find(|button| button.label == label)
            .ok_or_else(|| format!("no visible Button or Pressable labeled `{label}`"))?;
        let instance = self
            .instances
            .get_mut(&button.instance)
            .ok_or_else(|| "headless component instance disappeared during tap".to_owned())?;
        run_actions(
            &button.actions,
            &mut instance.environment,
            self.functions,
            0,
        )?;
        Ok(())
    }

    fn assert_text(&mut self, expected: &str) -> Result<(), String> {
        let snapshot = self.render()?;
        if snapshot.texts.iter().any(|text| text == expected) {
            Ok(())
        } else {
            Err(format!(
                "expected visible text `{expected}`; rendered text: [{}]",
                snapshot.texts.join(" | ")
            ))
        }
    }

    fn render(&mut self) -> Result<RenderSnapshot, String> {
        let mut snapshot = RenderSnapshot::default();
        self.render_component(ROOT_INSTANCE, None, &mut snapshot, 0)?;
        Ok(snapshot)
    }

    fn render_component(
        &mut self,
        instance_path: &str,
        content: Option<(&[Node], &Environment, &str)>,
        snapshot: &mut RenderSnapshot,
        depth: usize,
    ) -> Result<(), String> {
        if depth >= 128 {
            return Err("headless component depth exceeded".to_owned());
        }
        let instance = self
            .instances
            .get(instance_path)
            .ok_or_else(|| format!("headless component instance `{instance_path}` is missing"))?;
        let component = self
            .components
            .get(instance.component.as_str())
            .ok_or_else(|| format!("component `{}` is unavailable", instance.component))?;
        let body = component.body.clone();
        let environment = instance.environment.clone();
        self.render_nodes(
            &body,
            &environment,
            instance_path,
            content,
            snapshot,
            depth + 1,
        )
    }

    fn render_nodes(
        &mut self,
        nodes: &[Node],
        environment: &Environment,
        instance_path: &str,
        content: Option<(&[Node], &Environment, &str)>,
        snapshot: &mut RenderSnapshot,
        depth: usize,
    ) -> Result<(), String> {
        if depth >= 128 {
            return Err("headless component tree depth exceeded".to_owned());
        }
        for (index, node) in nodes.iter().enumerate() {
            match node {
                Node::Layout { children, .. } => self.render_nodes(
                    children,
                    environment,
                    instance_path,
                    content,
                    snapshot,
                    depth + 1,
                )?,
                Node::Text { value, .. } => {
                    let value = eval_expr(value, environment, self.functions, 0)?;
                    snapshot.texts.push(display_value(value));
                }
                Node::Button {
                    label,
                    loading,
                    disabled,
                    actions,
                    ..
                } => {
                    let label = display_value(eval_expr(label, environment, self.functions, 0)?);
                    snapshot.texts.push(label.clone());
                    let is_loading = loading
                        .as_ref()
                        .map(|value| eval_bool(value, environment, self.functions))
                        .transpose()?
                        .unwrap_or(false);
                    let is_disabled = disabled
                        .as_ref()
                        .map(|value| eval_bool(value, environment, self.functions))
                        .transpose()?
                        .unwrap_or(false);
                    if !is_loading && !is_disabled {
                        snapshot.buttons.push(HeadlessButton {
                            label,
                            instance: instance_path.to_owned(),
                            actions: actions.clone(),
                        });
                    }
                }
                Node::Pressable {
                    disabled,
                    children,
                    actions,
                    ..
                } => {
                    let is_disabled = eval_bool(disabled, environment, self.functions)?;
                    let text_start = snapshot.texts.len();
                    let button_start = snapshot.buttons.len();
                    self.render_nodes(
                        children,
                        environment,
                        instance_path,
                        content,
                        snapshot,
                        depth + 1,
                    )?;
                    if !is_disabled && let Some(label) = snapshot.texts.get(text_start).cloned() {
                        snapshot.buttons.insert(
                            button_start,
                            HeadlessButton {
                                label,
                                instance: instance_path.to_owned(),
                                actions: actions.clone(),
                            },
                        );
                    }
                }
                Node::If {
                    condition,
                    then_body,
                    else_body,
                    ..
                } => match eval_expr(condition, environment, self.functions, 0)? {
                    Value::Bool(true) => self.render_nodes(
                        then_body,
                        environment,
                        instance_path,
                        content,
                        snapshot,
                        depth + 1,
                    )?,
                    Value::Bool(false) => {
                        if let Some(else_body) = else_body {
                            self.render_nodes(
                                else_body,
                                environment,
                                instance_path,
                                content,
                                snapshot,
                                depth + 1,
                            )?;
                        }
                    }
                    _ => return Err("headless If condition is not a Boolean".to_owned()),
                },
                Node::When {
                    value,
                    cases,
                    else_body,
                    ..
                } => {
                    let value = eval_expr(value, environment, self.functions, 0)?;
                    let mut selected = None;
                    for case in cases {
                        if eval_expr(&case.value, environment, self.functions, 0)? == value {
                            selected = Some(case.body.as_slice());
                            break;
                        }
                    }
                    self.render_nodes(
                        selected.unwrap_or(else_body),
                        environment,
                        instance_path,
                        content,
                        snapshot,
                        depth + 1,
                    )?;
                }
                Node::ComponentCall {
                    name,
                    arguments,
                    children,
                } => {
                    let component = self
                        .components
                        .get(name.as_str())
                        .ok_or_else(|| format!("headless component `{name}` is unavailable"))?;
                    let mut values = HashMap::with_capacity(arguments.len());
                    for (parameter, value) in arguments {
                        values.insert(
                            parameter.as_str(),
                            eval_expr(value, environment, self.functions, 0)?,
                        );
                    }
                    let arguments = component
                        .parameters
                        .iter()
                        .map(|parameter| {
                            values.remove(parameter.name.as_str()).ok_or_else(|| {
                                format!(
                                    "headless component `{name}` is missing `{}`",
                                    parameter.name
                                )
                            })
                        })
                        .collect::<Result<Vec<_>, _>>()?;
                    if !values.is_empty() {
                        return Err(format!(
                            "headless component `{name}` received unknown properties"
                        ));
                    }
                    let child_path = format!("{instance_path}/{index}:{name}");
                    if let Some(instance) = self.instances.get_mut(&child_path) {
                        for (parameter, value) in component.parameters.iter().zip(arguments) {
                            instance.environment.insert(parameter.name.clone(), value);
                        }
                    } else {
                        let child_environment =
                            Self::new_environment(component, arguments, self.functions)?;
                        self.instances.insert(
                            child_path.clone(),
                            ComponentInstance {
                                component: name.clone(),
                                environment: child_environment,
                            },
                        );
                    }
                    self.render_component(
                        &child_path,
                        children
                            .as_deref()
                            .map(|children| (children, environment, instance_path)),
                        snapshot,
                        depth + 1,
                    )?;
                }
                Node::Content => {
                    if let Some((content, content_environment, content_instance)) = content {
                        self.render_nodes(
                            content,
                            content_environment,
                            content_instance,
                            None,
                            snapshot,
                            depth + 1,
                        )?;
                    }
                }
                Node::Accessibility { children, .. }
                | Node::KeyboardAware { children, .. }
                | Node::Link { children, .. }
                | Node::RefreshControl { children, .. } => self.render_nodes(
                    children,
                    environment,
                    instance_path,
                    content,
                    snapshot,
                    depth + 1,
                )?,
                Node::Image { .. }
                | Node::Spacer
                | Node::Divider { .. }
                | Node::ProgressBar { .. }
                | Node::ProgressRing { .. } => {}
                Node::NativeComponentCall {
                    namespace, name, ..
                } => {
                    return Err(format!(
                        "native plugin component `{namespace}.{name}` cannot execute headlessly"
                    ));
                }
                _ => {
                    return Err(format!(
                        "headless rendering does not support {} yet",
                        node_name(node)
                    ));
                }
            }
        }
        Ok(())
    }
}

fn node_name(node: &Node) -> &'static str {
    match node {
        Node::TextInput { .. } => "TextInput",
        Node::Switch { .. } => "Switch",
        Node::Slider { .. } => "Slider",
        Node::ProgressBar { .. } => "ProgressBar",
        Node::ProgressRing { .. } => "ProgressRing",
        Node::SegmentedControl { .. } => "SegmentedControl",
        Node::Picker { .. } => "Picker",
        Node::Image { .. } => "Image",
        Node::Pressable { .. } => "Pressable",
        Node::NavigationStack { .. } => "NavigationStack",
        Node::NavigationLink { .. } => "NavigationLink",
        Node::NavigationBack { .. } => "NavigationBack",
        Node::Link { .. } => "Link",
        Node::Accessibility { .. } => "Accessibility",
        Node::KeyboardAware { .. } => "KeyboardAware",
        Node::BottomSheet { .. } => "BottomSheet",
        Node::Dialog { .. } => "Dialog",
        Node::RefreshControl { .. } => "RefreshControl",
        Node::AppBottomBar { .. } => "AppBottomBar",
        Node::FastList { .. } => "FastList",
        Node::StatusBar { .. } => "StatusBar",
        Node::Direction { .. } => "Direction",
        Node::OnAppear { .. } => "OnAppear",
        Node::OnDisappear { .. } => "OnDisappear",
        Node::OnActive { .. } => "OnActive",
        Node::OnInactive { .. } => "OnInactive",
        Node::OnBackground { .. } => "OnBackground",
        Node::Layout { .. }
        | Node::Text { .. }
        | Node::Spacer
        | Node::Divider { .. }
        | Node::Button { .. }
        | Node::If { .. }
        | Node::When { .. }
        | Node::Content
        | Node::ComponentCall { .. }
        | Node::NativeComponentCall { .. } => "component node",
    }
}

fn run_actions(
    actions: &[Action],
    environment: &mut Environment,
    functions: &FunctionMap<'_>,
    depth: usize,
) -> Result<(), String> {
    if depth >= 128 {
        return Err("headless action depth exceeded".to_owned());
    }
    for action in actions {
        match action {
            Action::Assign { name, value } => {
                let value = eval_expr(value, environment, functions, depth + 1)?;
                if !environment.contains_key(name) {
                    return Err(format!("headless action assigned unknown state `{name}`"));
                }
                environment.insert(name.clone(), value);
            }
            Action::Expression(expression) => {
                let _ = eval_expr(expression, environment, functions, depth + 1)?;
            }
            Action::If {
                condition,
                then_branch,
                else_branch,
            } => match eval_expr(condition, environment, functions, depth + 1)? {
                Value::Bool(true) => run_actions(then_branch, environment, functions, depth + 1)?,
                Value::Bool(false) => {
                    if let Some(else_branch) = else_branch {
                        run_actions(else_branch, environment, functions, depth + 1)?;
                    }
                }
                _ => return Err("headless action condition is not a Boolean".to_owned()),
            },
            Action::Break
            | Action::Continue
            | Action::CollectionMutation { .. }
            | Action::NativePropertyAssign { .. }
            | Action::NativeEventSubscribe { .. }
            | Action::NetworkStatusSubscribe { .. }
            | Action::TaskLaunch { .. }
            | Action::TaskCancel { .. }
            | Action::For { .. }
            | Action::ForMap { .. }
            | Action::While { .. }
            | Action::TryCatch { .. } => {
                return Err("headless tests do not support this action yet".to_owned());
            }
        }
    }
    Ok(())
}

fn eval_function(
    function: &Function,
    arguments: Vec<Value>,
    functions: &FunctionMap<'_>,
    depth: usize,
) -> Result<Value, String> {
    if depth >= 128 {
        return Err(format!(
            "function call depth exceeded while evaluating `{}`",
            function.name
        ));
    }
    if function.is_async {
        return Err(format!(
            "async function `{}` cannot run in a synchronous test",
            function.name
        ));
    }
    if arguments.len() != function.parameters.len() {
        return Err(format!(
            "function `{}` expected {} argument(s), got {}",
            function.name,
            function.parameters.len(),
            arguments.len()
        ));
    }
    let mut environment = function
        .parameters
        .iter()
        .zip(arguments)
        .map(|(parameter, value)| (parameter.name.clone(), value))
        .collect::<Environment>();
    for local in &function.locals {
        let value = eval_expr(&local.initial, &environment, functions, depth + 1)?;
        environment.insert(local.name.clone(), value);
    }
    eval_expr(&function.body, &environment, functions, depth + 1)
}

fn eval_expr(
    expression: &Expr,
    environment: &Environment,
    functions: &FunctionMap<'_>,
    depth: usize,
) -> Result<Value, String> {
    if depth >= 128 {
        return Err("expression evaluation depth exceeded".to_owned());
    }
    match expression {
        Expr::String(value) => Ok(Value::String(value.clone())),
        Expr::Bool(value) => Ok(Value::Bool(*value)),
        Expr::Number { raw, ty } => parse_number(raw, *ty),
        Expr::State(name, _) => environment
            .get(name)
            .cloned()
            .ok_or_else(|| format!("unknown test value `{name}`")),
        Expr::EnumValue {
            enum_name,
            case_name,
        } => Ok(Value::Enum(enum_name.clone(), case_name.clone())),
        Expr::PluginEnumValue {
            namespace,
            enum_name,
            case_name,
        } => Ok(Value::Enum(
            format!("{namespace}.{enum_name}"),
            case_name.clone(),
        )),
        Expr::Add(left, right, ty) => {
            let left = eval_expr(left, environment, functions, depth + 1)?;
            let right = eval_expr(right, environment, functions, depth + 1)?;
            numeric_add(left, right, *ty)
        }
        Expr::Concat(left, right) => match (
            eval_expr(left, environment, functions, depth + 1)?,
            eval_expr(right, environment, functions, depth + 1)?,
        ) {
            (Value::String(left), Value::String(right)) => Ok(Value::String(left + &right)),
            _ => Err("string concatenation received a non-string value".to_owned()),
        },
        Expr::Arithmetic {
            op,
            left,
            right,
            ty,
        } => {
            let left = eval_expr(left, environment, functions, depth + 1)?;
            let right = eval_expr(right, environment, functions, depth + 1)?;
            numeric_arithmetic(left, right, *op, *ty)
        }
        Expr::Negate { value, ty } => match eval_expr(value, environment, functions, depth + 1)? {
            Value::Integer(value) => value
                .checked_neg()
                .map(Value::Integer)
                .ok_or_else(|| "integer negation overflowed".to_owned()),
            Value::Float(value) => Ok(Value::Float(-value)),
            _ => Err(format!("unary `-` received a non-{ty:?} value")),
        },
        Expr::Not(value) => match eval_expr(value, environment, functions, depth + 1)? {
            Value::Bool(value) => Ok(Value::Bool(!value)),
            _ => Err("logical negation received a non-Boolean value".to_owned()),
        },
        Expr::Binary { op, left, right } => {
            let left = eval_expr(left, environment, functions, depth + 1)?;
            if *op == BinaryOp::And && left == Value::Bool(false) {
                return Ok(Value::Bool(false));
            }
            if *op == BinaryOp::Or && left == Value::Bool(true) {
                return Ok(Value::Bool(true));
            }
            let right = eval_expr(right, environment, functions, depth + 1)?;
            match op {
                BinaryOp::And => bool_pair(left, right, |left, right| left && right),
                BinaryOp::Or => bool_pair(left, right, |left, right| left || right),
                BinaryOp::Equal => Ok(Value::Bool(left == right)),
                BinaryOp::NotEqual => Ok(Value::Bool(left != right)),
                BinaryOp::Less
                | BinaryOp::LessEqual
                | BinaryOp::Greater
                | BinaryOp::GreaterEqual => compare_values(left, right, *op),
                BinaryOp::Contains => Err("membership tests are not supported yet".to_owned()),
            }
        }
        Expr::Array(values) => Ok(Value::Array(
            values
                .iter()
                .map(|value| eval_expr(value, environment, functions, depth + 1))
                .collect::<Result<Vec<_>, _>>()?,
        )),
        Expr::Call {
            name,
            arguments,
            return_type,
            is_constructor,
            ..
        } => {
            let arguments = arguments
                .iter()
                .map(|argument| eval_expr(argument, environment, functions, depth + 1))
                .collect::<Result<Vec<_>, _>>()?;
            if *is_constructor {
                return Err(format!(
                    "value constructor `{name}` cannot run in a test expression"
                ));
            }
            if let Some(function) = functions.get(name.as_str()) {
                return eval_function(function, arguments, functions, depth + 1);
            }
            cast_builtin(name, arguments, return_type)
        }
        Expr::Conditional {
            condition,
            then_value,
            else_value,
            ..
        } => match eval_expr(condition, environment, functions, depth + 1)? {
            Value::Bool(true) => eval_expr(then_value, environment, functions, depth + 1),
            Value::Bool(false) => eval_expr(else_value, environment, functions, depth + 1),
            _ => Err("conditional test expression has a non-Boolean condition".to_owned()),
        },
        Expr::Interpolation(parts) => {
            let mut output = String::new();
            for part in parts {
                match part {
                    nexa_ir::InterpolatedPart::Literal(value) => output.push_str(value),
                    nexa_ir::InterpolatedPart::Value(value) => output.push_str(&display_value(
                        eval_expr(value, environment, functions, depth + 1)?,
                    )),
                }
            }
            Ok(Value::String(output))
        }
        _ => Err(format!(
            "`nexa test` cannot evaluate this expression yet: {}",
            expression_name(expression)
        )),
    }
}

fn expression_name(expression: &Expr) -> &'static str {
    match expression {
        Expr::Member { .. } => "member access",
        Expr::Index { .. } => "collection indexing",
        Expr::Map(_) => "map literal",
        Expr::Set(_) => "set literal",
        Expr::Pair(_, _) | Expr::Triple(_, _, _) => "tuple literal",
        Expr::NativeCall { .. } => "native API call",
        Expr::CollectionTransform { .. } => "collection transform",
        Expr::CollectionUtility { .. } => "collection utility",
        Expr::Await(_) => "async expression",
        _ => "expression",
    }
}

fn eval_bool(
    expression: &Expr,
    environment: &Environment,
    functions: &FunctionMap<'_>,
) -> Result<bool, String> {
    match eval_expr(expression, environment, functions, 0)? {
        Value::Bool(value) => Ok(value),
        _ => Err("headless component condition is not a Boolean".to_owned()),
    }
}

fn parse_number(raw: &str, ty: NumericType) -> Result<Value, String> {
    if matches!(ty, NumericType::Float32 | NumericType::Float64) {
        raw.parse::<f64>()
            .map(Value::Float)
            .map_err(|_| format!("invalid floating point literal `{raw}`"))
    } else {
        raw.parse::<i128>()
            .map(Value::Integer)
            .map_err(|_| format!("invalid integer literal `{raw}`"))
    }
}

fn numeric_add(left: Value, right: Value, ty: NumericType) -> Result<Value, String> {
    match (left, right) {
        (Value::Integer(left), Value::Integer(right)) => left
            .checked_add(right)
            .map(Value::Integer)
            .ok_or_else(|| "integer addition overflowed".to_owned()),
        (Value::Float(left), Value::Float(right)) => Ok(Value::Float(left + right)),
        _ => Err(format!("`+` received values incompatible with {ty:?}")),
    }
}

fn numeric_arithmetic(
    left: Value,
    right: Value,
    op: ArithmeticOp,
    ty: NumericType,
) -> Result<Value, String> {
    match (left, right) {
        (Value::Integer(left), Value::Integer(right)) => {
            let value = match op {
                ArithmeticOp::Subtract => left.checked_sub(right),
                ArithmeticOp::Multiply => left.checked_mul(right),
                ArithmeticOp::Divide if right != 0 => left.checked_div(right),
                ArithmeticOp::Remainder if right != 0 => left.checked_rem(right),
                ArithmeticOp::Divide | ArithmeticOp::Remainder => {
                    return Err("division by zero in test expression".to_owned());
                }
            };
            value
                .map(Value::Integer)
                .ok_or_else(|| "integer arithmetic overflowed".to_owned())
        }
        (Value::Float(left), Value::Float(right)) => {
            if right == 0.0 && matches!(op, ArithmeticOp::Divide | ArithmeticOp::Remainder) {
                return Err("division by zero in test expression".to_owned());
            }
            Ok(Value::Float(match op {
                ArithmeticOp::Subtract => left - right,
                ArithmeticOp::Multiply => left * right,
                ArithmeticOp::Divide => left / right,
                ArithmeticOp::Remainder => left % right,
            }))
        }
        _ => Err(format!(
            "arithmetic received values incompatible with {ty:?}"
        )),
    }
}

fn bool_pair(
    left: Value,
    right: Value,
    operation: impl FnOnce(bool, bool) -> bool,
) -> Result<Value, String> {
    match (left, right) {
        (Value::Bool(left), Value::Bool(right)) => Ok(Value::Bool(operation(left, right))),
        _ => Err("logical operator received a non-Boolean value".to_owned()),
    }
}

fn compare_values(left: Value, right: Value, op: BinaryOp) -> Result<Value, String> {
    let ordering = match (left, right) {
        (Value::Integer(left), Value::Integer(right)) => left.partial_cmp(&right),
        (Value::Float(left), Value::Float(right)) => left.partial_cmp(&right),
        (Value::String(left), Value::String(right)) => left.partial_cmp(&right),
        _ => return Err("comparison received values of incompatible types".to_owned()),
    }
    .ok_or_else(|| "comparison is undefined for these values".to_owned())?;
    Ok(Value::Bool(match op {
        BinaryOp::Less => ordering.is_lt(),
        BinaryOp::LessEqual => !ordering.is_gt(),
        BinaryOp::Greater => ordering.is_gt(),
        BinaryOp::GreaterEqual => !ordering.is_lt(),
        _ => return Err("invalid ordered comparison".to_owned()),
    }))
}

fn cast_builtin(name: &str, arguments: Vec<Value>, return_type: &Type) -> Result<Value, String> {
    if arguments.len() != 1 {
        return Err(format!("unknown test function `{name}`"));
    }
    let value = arguments.into_iter().next().unwrap_or(Value::Void);
    if matches!(name, "String" | "toString") || *return_type == Type::String {
        return Ok(Value::String(display_value(value)));
    }
    if matches!(
        return_type,
        Type::Numeric(NumericType::Float32 | NumericType::Float64)
    ) {
        return match value {
            Value::Integer(value) => Ok(Value::Float(value as f64)),
            Value::Float(value) => Ok(Value::Float(value)),
            _ => Err(format!("cannot convert this value to `{name}`")),
        };
    }
    if matches!(return_type, Type::Numeric(_)) {
        return match value {
            Value::Integer(value) => Ok(Value::Integer(value)),
            Value::Float(value) if value.is_finite() => Ok(Value::Integer(value as i128)),
            _ => Err(format!("cannot convert this value to `{name}`")),
        };
    }
    Err(format!(
        "test expression called unsupported function `{name}`"
    ))
}

fn display_value(value: Value) -> String {
    match value {
        Value::Void => "()".to_owned(),
        Value::Bool(value) => value.to_string(),
        Value::String(value) => value,
        Value::Integer(value) => value.to_string(),
        Value::Float(value) => value.to_string(),
        Value::Enum(name, case) => format!("{name}.{case}"),
        Value::Array(values) => format!(
            "[{}]",
            values
                .into_iter()
                .map(display_value)
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}
