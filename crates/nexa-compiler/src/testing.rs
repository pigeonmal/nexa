use std::cmp::Ordering;
use std::collections::HashMap;

use nexa_diagnostics::Span;
use nexa_ir::{
    Action, ArithmeticOp, BinaryOp, CollectionMutation, CollectionTransform, CollectionUtilityKind,
    Component, Expr, Function, FunctionLocal, MemberKind, Node, NumericType, TuplePosition, Type,
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
    TypeText {
        placeholder: Expr,
        value: Expr,
        span: Span,
    },
    Toggle {
        label: Expr,
        span: Span,
    },
    Slide {
        state: Expr,
        value: Expr,
        span: Span,
    },
    Select {
        state: Expr,
        value: Expr,
        span: Span,
    },
    Submit {
        placeholder: Expr,
        span: Span,
    },
    AssertComponent {
        name: Expr,
        span: Span,
    },
    Emit {
        component: Expr,
        event: Expr,
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
    Null,
    Void,
    Bool(bool),
    String(String),
    Integer(i128),
    Float(f64),
    Enum(String, String),
    EnumPayload(String, String, Vec<Value>),
    Array(Vec<Value>),
    Set(Vec<Value>),
    Map(Vec<(Value, Value)>),
    Pair(Box<Value>, Box<Value>),
    Triple(Box<Value>, Box<Value>, Box<Value>),
    Struct(String, Vec<(String, Value)>),
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
                sync_root_environment(runtime, &mut environment);
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
            TestStatement::TypeText {
                placeholder,
                value,
                span,
            } => {
                let placeholder = eval_test_string(placeholder, &environment, functions, *span)?;
                let value = eval_test_string(value, &environment, functions, *span)?;
                let runtime = runtime.as_mut().ok_or_else(|| {
                    (
                        *span,
                        "`typeText(...)` requires a component target".to_owned(),
                    )
                })?;
                runtime
                    .type_text(&placeholder, value)
                    .map_err(|message| (*span, message))?;
                sync_root_environment(runtime, &mut environment);
            }
            TestStatement::Toggle { label, span } => {
                let label = eval_test_string(label, &environment, functions, *span)?;
                let runtime = runtime.as_mut().ok_or_else(|| {
                    (
                        *span,
                        "`toggle(...)` requires a component target".to_owned(),
                    )
                })?;
                runtime.toggle(&label).map_err(|message| (*span, message))?;
                sync_root_environment(runtime, &mut environment);
            }
            TestStatement::Slide { state, value, span } => {
                let state = eval_test_string(state, &environment, functions, *span)?;
                let value = match eval_expr(value, &environment, functions, 0)
                    .map_err(|message| (*span, message))?
                {
                    Value::Float(value) => value,
                    _ => return Err((*span, "slider value is not floating point".to_owned())),
                };
                let runtime = runtime.as_mut().ok_or_else(|| {
                    (*span, "`slide(...)` requires a component target".to_owned())
                })?;
                runtime
                    .slide(&state, value)
                    .map_err(|message| (*span, message))?;
                sync_root_environment(runtime, &mut environment);
            }
            TestStatement::Select { state, value, span } => {
                let state = eval_test_string(state, &environment, functions, *span)?;
                let value = eval_test_string(value, &environment, functions, *span)?;
                let runtime = runtime.as_mut().ok_or_else(|| {
                    (
                        *span,
                        "`select(...)` requires a component target".to_owned(),
                    )
                })?;
                runtime
                    .select(&state, &value)
                    .map_err(|message| (*span, message))?;
                sync_root_environment(runtime, &mut environment);
            }
            TestStatement::Submit { placeholder, span } => {
                let placeholder = eval_test_string(placeholder, &environment, functions, *span)?;
                let runtime = runtime.as_mut().ok_or_else(|| {
                    (
                        *span,
                        "`submit(...)` requires a component target".to_owned(),
                    )
                })?;
                runtime
                    .submit(&placeholder)
                    .map_err(|message| (*span, message))?;
                sync_root_environment(runtime, &mut environment);
            }
            TestStatement::AssertComponent { name, span } => {
                let name = eval_test_string(name, &environment, functions, *span)?;
                let runtime = runtime.as_mut().ok_or_else(|| {
                    (
                        *span,
                        "`assertComponent(...)` requires a component target".to_owned(),
                    )
                })?;
                runtime
                    .assert_component(&name)
                    .map_err(|message| (*span, message))?;
            }
            TestStatement::Emit {
                component,
                event,
                span,
            } => {
                let component = eval_test_string(component, &environment, functions, *span)?;
                let event = eval_test_string(event, &environment, functions, *span)?;
                let runtime = runtime
                    .as_mut()
                    .ok_or_else(|| (*span, "`emit(...)` requires a component target".to_owned()))?;
                runtime
                    .emit(&component, &event)
                    .map_err(|message| (*span, message))?;
                sync_root_environment(runtime, &mut environment);
            }
        }
    }
    Ok(())
}

fn eval_test_string(
    expression: &Expr,
    environment: &Environment,
    functions: &FunctionMap<'_>,
    span: Span,
) -> Result<String, (Span, String)> {
    match eval_expr(expression, environment, functions, 0).map_err(|message| (span, message))? {
        Value::String(value) => Ok(value),
        _ => Err((span, "expected a String test argument".to_owned())),
    }
}

fn sync_root_environment(runtime: &HeadlessRuntime<'_>, environment: &mut Environment) {
    if let Some(root) = runtime.instances.get(ROOT_INSTANCE) {
        for (name, value) in &root.environment {
            environment.insert(name.clone(), value.clone());
        }
    }
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

#[derive(Clone)]
enum HeadlessControl {
    TextInput {
        placeholder: String,
        state: String,
        actions: Vec<Action>,
    },
    Switch {
        label: String,
        state: String,
    },
    Slider {
        state: String,
        min: f64,
        max: f64,
        step: f64,
    },
    Selection {
        state: String,
        values: Vec<String>,
    },
}

#[derive(Clone)]
struct HeadlessPluginComponent {
    qualified_name: String,
    instance: String,
    event_handlers: Vec<nexa_ir::NativeComponentEventHandler>,
}

#[derive(Default)]
struct RenderSnapshot {
    texts: Vec<String>,
    buttons: Vec<HeadlessButton>,
    controls: Vec<(String, HeadlessControl)>,
    plugin_components: Vec<HeadlessPluginComponent>,
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

    fn type_text(&mut self, placeholder: &str, value: String) -> Result<(), String> {
        let snapshot = self.render()?;
        let (instance, control) = snapshot
            .controls
            .into_iter()
            .find(|(_, control)| {
                matches!(control, HeadlessControl::TextInput { placeholder: candidate, .. } if candidate == placeholder)
            })
            .ok_or_else(|| format!("no visible TextInput with placeholder `{placeholder}`"))?;
        let HeadlessControl::TextInput { state, .. } = control else {
            return Err("headless TextInput selection changed unexpectedly".to_owned());
        };
        self.set_instance_state(&instance, &state, Value::String(value))
    }

    fn toggle(&mut self, label: &str) -> Result<(), String> {
        let snapshot = self.render()?;
        let (instance, control) = snapshot
            .controls
            .into_iter()
            .find(|(_, control)| {
                matches!(control, HeadlessControl::Switch { label: candidate, .. } if candidate == label)
            })
            .ok_or_else(|| format!("no visible Switch labeled `{label}`"))?;
        let HeadlessControl::Switch { state, .. } = control else {
            return Err("headless Switch selection changed unexpectedly".to_owned());
        };
        let value = self
            .instances
            .get(&instance)
            .and_then(|component| component.environment.get(&state))
            .cloned()
            .ok_or_else(|| format!("Switch state `{state}` is unavailable"))?;
        let Value::Bool(value) = value else {
            return Err(format!("Switch state `{state}` is not Boolean"));
        };
        self.set_instance_state(&instance, &state, Value::Bool(!value))
    }

    fn slide(&mut self, state: &str, value: f64) -> Result<(), String> {
        if !value.is_finite() {
            return Err("slider value must be finite".to_owned());
        }
        let snapshot = self.render()?;
        let (instance, control) = snapshot
            .controls
            .into_iter()
            .find(|(_, control)| {
                matches!(control, HeadlessControl::Slider { state: candidate, .. } if candidate == state)
            })
            .ok_or_else(|| format!("no visible Slider bound to state `{state}`"))?;
        let HeadlessControl::Slider { min, max, step, .. } = control else {
            return Err("headless Slider selection changed unexpectedly".to_owned());
        };
        if value < min || value > max {
            return Err(format!("slider value {value} is outside {min}..{max}"));
        }
        if !step.is_finite() || step <= 0.0 || min > max {
            return Err("Slider has an invalid range or step".to_owned());
        }
        let steps = ((value - min) / step).round();
        let stepped = (min + steps * step).clamp(min, max);
        self.set_instance_state(&instance, state, Value::Float(stepped))
    }

    fn select(&mut self, state: &str, value: &str) -> Result<(), String> {
        let snapshot = self.render()?;
        let (instance, control) = snapshot
            .controls
            .into_iter()
            .find(|(_, control)| {
                matches!(control, HeadlessControl::Selection { state: candidate, .. } if candidate == state)
            })
            .ok_or_else(|| format!("no visible Picker or SegmentedControl bound to `{state}`"))?;
        let HeadlessControl::Selection { values, .. } = control else {
            return Err("headless selection control changed unexpectedly".to_owned());
        };
        if !values.iter().any(|item| item == value) {
            return Err(format!(
                "`{value}` is not an option for control state `{state}`"
            ));
        }
        self.set_instance_state(&instance, state, Value::String(value.to_owned()))
    }

    fn submit(&mut self, placeholder: &str) -> Result<(), String> {
        let snapshot = self.render()?;
        let (instance, actions) = snapshot
            .controls
            .into_iter()
            .find_map(|(instance, control)| match control {
                HeadlessControl::TextInput {
                    placeholder: candidate,
                    actions,
                    ..
                } if candidate == placeholder => Some((instance, actions)),
                _ => None,
            })
            .ok_or_else(|| format!("no visible TextInput with placeholder `{placeholder}`"))?;
        let environment = &mut self
            .instances
            .get_mut(&instance)
            .ok_or_else(|| "headless TextInput instance disappeared during submit".to_owned())?
            .environment;
        run_actions(&actions, environment, self.functions, 0)
    }

    fn assert_component(&mut self, qualified_name: &str) -> Result<(), String> {
        let snapshot = self.render()?;
        if snapshot
            .plugin_components
            .iter()
            .any(|component| component.qualified_name == qualified_name)
        {
            Ok(())
        } else {
            Err(format!(
                "native plugin component `{qualified_name}` is not visible"
            ))
        }
    }

    fn emit(&mut self, qualified_name: &str, property: &str) -> Result<(), String> {
        let snapshot = self.render()?;
        let component = snapshot
            .plugin_components
            .into_iter()
            .find(|component| component.qualified_name == qualified_name)
            .ok_or_else(|| format!("native plugin component `{qualified_name}` is not visible"))?;
        let handler = component
            .event_handlers
            .iter()
            .find(|handler| handler.property == property)
            .ok_or_else(|| {
                format!("native plugin component `{qualified_name}` has no `{property}` event")
            })?;
        if !handler.parameters.is_empty() {
            return Err(format!(
                "headless event injection for `{qualified_name}.{property}` requires a payload and is not supported"
            ));
        }
        let environment = &mut self
            .instances
            .get_mut(&component.instance)
            .ok_or_else(|| "native plugin component instance disappeared during event".to_owned())?
            .environment;
        run_actions(&handler.actions, environment, self.functions, 0)
    }

    fn set_instance_state(
        &mut self,
        instance: &str,
        name: &str,
        value: Value,
    ) -> Result<(), String> {
        let component = self
            .instances
            .get_mut(instance)
            .ok_or_else(|| format!("headless component instance `{instance}` is missing"))?;
        if !component.environment.contains_key(name) {
            return Err(format!("headless state `{name}` is unavailable"));
        }
        component.environment.insert(name.to_owned(), value);
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
                Node::Layout { children, .. }
                | Node::Form { children }
                | Node::Toolbar { children, .. } => self.render_nodes(
                    children,
                    environment,
                    instance_path,
                    content,
                    snapshot,
                    depth + 1,
                )?,
                Node::FormSection {
                    title,
                    footer,
                    children,
                    ..
                } => {
                    for value in title.iter().chain(footer) {
                        snapshot.texts.push(display_value(eval_expr(
                            value,
                            environment,
                            self.functions,
                            0,
                        )?));
                    }
                    self.render_nodes(
                        children,
                        environment,
                        instance_path,
                        content,
                        snapshot,
                        depth + 1,
                    )?;
                }
                Node::Text { value, .. } => {
                    let value = eval_expr(value, environment, self.functions, 0)?;
                    snapshot.texts.push(display_value(value));
                }
                Node::ContentUnavailable {
                    title, description, ..
                } => {
                    snapshot.texts.push(display_value(eval_expr(
                        title,
                        environment,
                        self.functions,
                        0,
                    )?));
                    snapshot.texts.push(display_value(eval_expr(
                        description,
                        environment,
                        self.functions,
                        0,
                    )?));
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
                Node::TextInput {
                    state,
                    placeholder,
                    actions,
                    ..
                } => {
                    let text = match environment.get(state) {
                        Some(Value::String(value)) if !value.is_empty() => value.clone(),
                        Some(Value::String(_)) => placeholder.clone(),
                        Some(_) => {
                            return Err(format!("TextInput state `{state}` is not a String"));
                        }
                        None => {
                            return Err(format!("TextInput state `{state}` is unavailable"));
                        }
                    };
                    snapshot.texts.push(text);
                    snapshot.controls.push((
                        instance_path.to_owned(),
                        HeadlessControl::TextInput {
                            placeholder: placeholder.clone(),
                            state: state.clone(),
                            actions: actions.clone(),
                        },
                    ));
                }
                Node::Switch { state, label, .. } => {
                    if !matches!(environment.get(state), Some(Value::Bool(_))) {
                        return Err(format!("Switch state `{state}` is not a Boolean"));
                    }
                    let label = display_value(eval_expr(label, environment, self.functions, 0)?);
                    snapshot.texts.push(label.clone());
                    snapshot.controls.push((
                        instance_path.to_owned(),
                        HeadlessControl::Switch {
                            label: label.clone(),
                            state: state.clone(),
                        },
                    ));
                }
                Node::Slider {
                    state,
                    min,
                    max,
                    step,
                    ..
                } => {
                    if !matches!(environment.get(state), Some(Value::Float(_))) {
                        return Err(format!("Slider state `{state}` is not a Float64"));
                    }
                    snapshot.controls.push((
                        instance_path.to_owned(),
                        HeadlessControl::Slider {
                            state: state.clone(),
                            min: *min,
                            max: *max,
                            step: *step,
                        },
                    ));
                }
                Node::SegmentedControl { items, state, .. } | Node::Picker { items, state, .. } => {
                    let values = match eval_expr(items, environment, self.functions, 0)? {
                        Value::Array(values) => values
                            .into_iter()
                            .map(|value| match value {
                                Value::String(value) => Ok(value),
                                _ => Err("selection control options must be Strings".to_owned()),
                            })
                            .collect::<Result<Vec<_>, _>>()?,
                        _ => return Err("selection control options are not an Array".to_owned()),
                    };
                    if !matches!(environment.get(state), Some(Value::String(_))) {
                        return Err(format!("selection control state `{state}` is not a String"));
                    }
                    for value in &values {
                        snapshot.texts.push(value.clone());
                    }
                    snapshot.controls.push((
                        instance_path.to_owned(),
                        HeadlessControl::Selection {
                            state: state.clone(),
                            values,
                        },
                    ));
                }
                Node::DatePicker {
                    timestamp_state,
                    has_time_state,
                } => {
                    if !matches!(environment.get(timestamp_state), Some(Value::Integer(_))) {
                        return Err(format!(
                            "DatePicker timestamp state `{timestamp_state}` is not an Int64"
                        ));
                    }
                    if !matches!(environment.get(has_time_state), Some(Value::Bool(_))) {
                        return Err(format!(
                            "DatePicker hasTime state `{has_time_state}` is not a Boolean"
                        ));
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
                    namespace,
                    name,
                    arguments,
                    children,
                    event_handlers,
                } => {
                    for (_, value) in arguments {
                        let _ = eval_expr(value, environment, self.functions, 0)?;
                    }
                    snapshot.plugin_components.push(HeadlessPluginComponent {
                        qualified_name: format!("{namespace}.{name}"),
                        instance: instance_path.to_owned(),
                        event_handlers: event_handlers.clone(),
                    });
                    if let Some(children) = children {
                        self.render_nodes(
                            children,
                            environment,
                            instance_path,
                            content,
                            snapshot,
                            depth + 1,
                        )?;
                    }
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
        Node::DatePicker { .. } => "DatePicker",
        Node::Image { .. } => "Image",
        Node::SystemIcon { .. } => "Icon",
        Node::LinearGradient { .. } => "LinearGradient",
        Node::Pressable { .. } => "Pressable",
        Node::NavigationStack { .. } => "NavigationStack",
        Node::NavigationLink { .. } => "NavigationLink",
        Node::NavigationSplitView { .. } => "NavigationSplitView",
        Node::NavigationBack { .. } => "NavigationBack",
        Node::Link { .. } => "Link",
        Node::Accessibility { .. } => "Accessibility",
        Node::KeyboardAware { .. } => "KeyboardAware",
        Node::BottomSheet { .. } => "BottomSheet",
        Node::Dialog { .. } => "Dialog",
        Node::ConfirmationDialog { .. } => "ConfirmationDialog",
        Node::RefreshControl { .. } => "RefreshControl",
        Node::AppBottomBar { .. } => "AppBottomBar",
        Node::PagePager { .. } => "PagePager",
        Node::Toolbar { .. } => "Toolbar",
        Node::FastList { .. } => "FastList",
        Node::StatusBar { .. } => "StatusBar",
        Node::Direction { .. } => "Direction",
        Node::Appearance { .. } => "Appearance",
        Node::Form { .. } => "Form",
        Node::FormSection { .. } => "Section",
        Node::OnAppear { .. } => "OnAppear",
        Node::OnDisappear { .. } => "OnDisappear",
        Node::OnActive { .. } => "OnActive",
        Node::OnInactive { .. } => "OnInactive",
        Node::OnBackground { .. } => "OnBackground",
        Node::Layout { .. }
        | Node::Text { .. }
        | Node::ContentUnavailable { .. }
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
    match run_action_sequence(actions, environment, functions, depth)? {
        HeadlessActionFlow::Normal => Ok(()),
        HeadlessActionFlow::Break | HeadlessActionFlow::Continue => {
            Err("loop control escaped its headless loop".to_owned())
        }
        HeadlessActionFlow::Return(_) => Err("function return escaped an action block".to_owned()),
    }
}

#[derive(Clone, Debug, PartialEq)]
enum HeadlessActionFlow {
    Normal,
    Break,
    Continue,
    Return(Value),
}

fn run_action_sequence(
    actions: &[Action],
    environment: &mut Environment,
    functions: &FunctionMap<'_>,
    depth: usize,
) -> Result<HeadlessActionFlow, String> {
    if depth >= 128 {
        return Err("headless action depth exceeded".to_owned());
    }
    for action in actions {
        match action {
            Action::Let { name, value, .. } => {
                let value = eval_expr(value, environment, functions, depth + 1)?;
                environment.insert(name.clone(), value);
            }
            Action::Return { value } => {
                return Ok(HeadlessActionFlow::Return(eval_expr(
                    value,
                    environment,
                    functions,
                    depth + 1,
                )?));
            }
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
            Action::WithAnimation { actions, .. } => {
                let flow = run_action_sequence(actions, environment, functions, depth + 1)?;
                if flow != HeadlessActionFlow::Normal {
                    return Ok(flow);
                }
            }
            Action::If {
                condition,
                then_branch,
                else_branch,
            } => match eval_expr(condition, environment, functions, depth + 1)? {
                Value::Bool(true) => {
                    let flow = run_action_sequence(then_branch, environment, functions, depth + 1)?;
                    if flow != HeadlessActionFlow::Normal {
                        return Ok(flow);
                    }
                }
                Value::Bool(false) => {
                    if let Some(else_branch) = else_branch {
                        let flow =
                            run_action_sequence(else_branch, environment, functions, depth + 1)?;
                        if flow != HeadlessActionFlow::Normal {
                            return Ok(flow);
                        }
                    }
                }
                _ => return Err("headless action condition is not a Boolean".to_owned()),
            },
            Action::For {
                name,
                iterable,
                body,
            } => {
                let values = match eval_expr(iterable, environment, functions, depth + 1)? {
                    Value::Array(values) | Value::Set(values) => values,
                    _ => return Err("headless `for` iterable must be an Array or Set".to_owned()),
                };
                for value in values {
                    let previous = environment.insert(name.clone(), value);
                    let result = run_action_sequence(body, environment, functions, depth + 1);
                    restore_binding(environment, name, previous);
                    match result? {
                        HeadlessActionFlow::Normal | HeadlessActionFlow::Continue => {}
                        HeadlessActionFlow::Break => break,
                        returned @ HeadlessActionFlow::Return(_) => return Ok(returned),
                    }
                }
            }
            Action::ForMap {
                key_name,
                value_name,
                iterable,
                body,
            } => {
                let entries = match eval_expr(iterable, environment, functions, depth + 1)? {
                    Value::Map(entries) => entries,
                    _ => return Err("headless `for` map iterable must be a Map".to_owned()),
                };
                for (key, value) in entries {
                    let previous_key = environment.insert(key_name.clone(), key);
                    let previous_value = environment.insert(value_name.clone(), value);
                    let result = run_action_sequence(body, environment, functions, depth + 1);
                    restore_binding(environment, key_name, previous_key);
                    restore_binding(environment, value_name, previous_value);
                    match result? {
                        HeadlessActionFlow::Normal | HeadlessActionFlow::Continue => {}
                        HeadlessActionFlow::Break => break,
                        returned @ HeadlessActionFlow::Return(_) => return Ok(returned),
                    }
                }
            }
            Action::While { condition, body } => {
                let mut iterations = 0usize;
                loop {
                    if iterations >= 10_000 {
                        return Err("headless `while` loop exceeded 10,000 iterations".to_owned());
                    }
                    iterations += 1;
                    match eval_expr(condition, environment, functions, depth + 1)? {
                        Value::Bool(false) => break,
                        Value::Bool(true) => {}
                        _ => return Err("headless `while` condition is not a Boolean".to_owned()),
                    }
                    match run_action_sequence(body, environment, functions, depth + 1)? {
                        HeadlessActionFlow::Normal | HeadlessActionFlow::Continue => {}
                        HeadlessActionFlow::Break => break,
                        returned @ HeadlessActionFlow::Return(_) => return Ok(returned),
                    }
                }
            }
            Action::CollectionMutation {
                name,
                operation,
                arguments,
            } => mutate_collection(
                name,
                *operation,
                arguments,
                environment,
                functions,
                depth + 1,
            )?,
            Action::TryCatch {
                body,
                error_catches,
                catch_body,
            } => {
                if !error_catches.is_empty() {
                    return Err(
                        "headless tests cannot synthesize typed native plugin failures".to_owned(),
                    );
                }
                match run_action_sequence(body, environment, functions, depth + 1) {
                    Ok(HeadlessActionFlow::Normal) => {}
                    Ok(flow) => return Ok(flow),
                    Err(_) => {
                        if let Some(catch_body) = catch_body {
                            let flow =
                                run_action_sequence(catch_body, environment, functions, depth + 1)?;
                            if flow != HeadlessActionFlow::Normal {
                                return Ok(flow);
                            }
                        } else {
                            return Err("headless action failed inside a try block".to_owned());
                        }
                    }
                }
            }
            Action::Break => return Ok(HeadlessActionFlow::Break),
            Action::Continue => return Ok(HeadlessActionFlow::Continue),
            Action::NativePropertyAssign { .. }
            | Action::NativeEventSubscribe { .. }
            | Action::NetworkStatusSubscribe { .. }
            | Action::TaskLaunch { .. }
            | Action::TaskCancel { .. } => {
                return Err(
                    "headless tests cannot execute this platform or async action".to_owned(),
                );
            }
        }
    }
    Ok(HeadlessActionFlow::Normal)
}

fn restore_binding(environment: &mut Environment, name: &str, previous: Option<Value>) {
    if let Some(previous) = previous {
        environment.insert(name.to_owned(), previous);
    } else {
        environment.remove(name);
    }
}

fn mutate_collection(
    name: &str,
    operation: CollectionMutation,
    arguments: &[Expr],
    environment: &mut Environment,
    functions: &FunctionMap<'_>,
    depth: usize,
) -> Result<(), String> {
    let values = arguments
        .iter()
        .map(|argument| eval_expr(argument, environment, functions, depth + 1))
        .collect::<Result<Vec<_>, _>>()?;
    let target = environment
        .get_mut(name)
        .ok_or_else(|| format!("headless collection state `{name}` is unavailable"))?;
    match (operation, target) {
        (CollectionMutation::ArrayAppend, Value::Array(items)) => match values.as_slice() {
            [value] => items.push(value.clone()),
            _ => return Err("headless Array.append expects one value".to_owned()),
        },
        (CollectionMutation::ArrayRemoveAt, Value::Array(items)) => {
            let [Value::Integer(index)] = values.as_slice() else {
                return Err("headless Array.remove expects one integer index".to_owned());
            };
            let index = usize::try_from(*index)
                .map_err(|_| "headless Array.remove index is negative".to_owned())?;
            if index >= items.len() {
                return Err(format!(
                    "headless Array.remove index {index} is out of bounds"
                ));
            }
            items.remove(index);
        }
        (CollectionMutation::ArrayMove, Value::Array(items)) => {
            let [Value::Integer(from), Value::Integer(to)] = values.as_slice() else {
                return Err("headless Array.move expects two integer indexes".to_owned());
            };
            let (Ok(from), Ok(to)) = (usize::try_from(*from), usize::try_from(*to)) else {
                return Ok(());
            };
            if from < items.len() && to < items.len() && from != to {
                let item = items.remove(from);
                items.insert(to, item);
            }
        }
        (CollectionMutation::ArrayMoveSubset, Value::Array(items)) => {
            let [
                Value::Integer(from),
                Value::Integer(to),
                Value::Array(subset),
            ] = values.as_slice()
            else {
                return Err(
                    "headless Array.moveSubset expects two integer indexes and an array subset"
                        .to_owned(),
                );
            };
            let (Ok(from), Ok(to)) = (usize::try_from(*from), usize::try_from(*to)) else {
                return Ok(());
            };
            if from >= subset.len() || to >= subset.len() || from == to {
                return Ok(());
            }
            let mut reordered = subset.clone();
            let item = reordered.remove(from);
            reordered.insert(to, item);
            let mut cursor = 0;
            for backing in items.iter_mut() {
                if cursor < subset.len() && *backing == subset[cursor] {
                    *backing = reordered[cursor].clone();
                    cursor += 1;
                }
            }
        }
        (CollectionMutation::SetInsert, Value::Set(items)) => match values.as_slice() {
            [value] if !items.contains(value) => items.push(value.clone()),
            [_] => {}
            _ => return Err("headless Set.insert expects one value".to_owned()),
        },
        (CollectionMutation::SetRemove, Value::Set(items)) => match values.as_slice() {
            [value] => items.retain(|item| item != value),
            _ => return Err("headless Set.remove expects one value".to_owned()),
        },
        (CollectionMutation::MapSet, Value::Map(items)) => match values.as_slice() {
            [key, value] => {
                if let Some((_, current)) = items.iter_mut().find(|(current, _)| current == key) {
                    *current = value.clone();
                } else {
                    items.push((key.clone(), value.clone()));
                }
            }
            _ => return Err("headless Map.set expects a key and value".to_owned()),
        },
        (CollectionMutation::MapRemove, Value::Map(items)) => match values.as_slice() {
            [key] => items.retain(|(current, _)| current != key),
            _ => return Err("headless Map.remove expects one key".to_owned()),
        },
        (CollectionMutation::MapClear, Value::Map(items)) => {
            if !values.is_empty() {
                return Err("headless Map.clear expects no arguments".to_owned());
            }
            items.clear();
        }
        (CollectionMutation::Replace, Value::Array(items)) => match values.as_slice() {
            [Value::Array(replacement)] => *items = replacement.clone(),
            _ => return Err("headless Array replacement received a non-Array".to_owned()),
        },
        (CollectionMutation::Replace, Value::Set(items)) => match values.as_slice() {
            [Value::Set(replacement)] => *items = replacement.clone(),
            _ => return Err("headless Set replacement received a non-Set".to_owned()),
        },
        (CollectionMutation::Replace, Value::Map(items)) => match values.as_slice() {
            [Value::Map(replacement)] => *items = replacement.clone(),
            _ => return Err("headless Map replacement received a non-Map".to_owned()),
        },
        _ => return Err(format!("invalid headless collection mutation on `{name}`")),
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
    if let Some(actions) = &function.body_actions {
        return match run_action_sequence(actions, &mut environment, functions, depth + 1)? {
            HeadlessActionFlow::Return(value) => Ok(value),
            _ => Err(format!(
                "function `{}` completed without returning",
                function.name
            )),
        };
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
        Expr::LocalizedText { value, .. } => eval_expr(value, environment, functions, depth + 1),
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
        Expr::PluginEnumConstructor {
            namespace,
            enum_name,
            case_name,
            arguments,
            ..
        } => Ok(Value::EnumPayload(
            format!("{namespace}.{enum_name}"),
            case_name.clone(),
            arguments
                .iter()
                .map(|argument| eval_expr(argument, environment, functions, depth + 1))
                .collect::<Result<Vec<_>, _>>()?,
        )),
        Expr::PluginEnumOptionalConstructor {
            namespace,
            enum_name,
            case_name,
            null_case_name,
            value,
            ..
        } => {
            let enum_name = format!("{namespace}.{enum_name}");
            match eval_expr(value, environment, functions, depth + 1)? {
                Value::Null => Ok(Value::Enum(enum_name, null_case_name.clone())),
                value => Ok(Value::EnumPayload(
                    enum_name,
                    case_name.clone(),
                    vec![value],
                )),
            }
        }
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
        Expr::Set(values) => {
            let mut items = Vec::with_capacity(values.len());
            for value in values {
                let value = eval_expr(value, environment, functions, depth + 1)?;
                if !items.contains(&value) {
                    items.push(value);
                }
            }
            Ok(Value::Set(items))
        }
        Expr::Map(entries) => {
            let mut items = Vec::with_capacity(entries.len());
            for (key, value) in entries {
                let key = eval_expr(key, environment, functions, depth + 1)?;
                let value = eval_expr(value, environment, functions, depth + 1)?;
                if let Some((_, current)) =
                    items.iter_mut().find(|(candidate, _)| candidate == &key)
                {
                    *current = value;
                } else {
                    items.push((key, value));
                }
            }
            Ok(Value::Map(items))
        }
        Expr::Pair(first, second) => Ok(Value::Pair(
            Box::new(eval_expr(first, environment, functions, depth + 1)?),
            Box::new(eval_expr(second, environment, functions, depth + 1)?),
        )),
        Expr::Triple(first, second, third) => Ok(Value::Triple(
            Box::new(eval_expr(first, environment, functions, depth + 1)?),
            Box::new(eval_expr(second, environment, functions, depth + 1)?),
            Box::new(eval_expr(third, environment, functions, depth + 1)?),
        )),
        Expr::Contains {
            value, collection, ..
        } => {
            let value = eval_expr(value, environment, functions, depth + 1)?;
            let collection = eval_expr(collection, environment, functions, depth + 1)?;
            let contains = match collection {
                Value::Array(values) | Value::Set(values) => values.contains(&value),
                Value::Map(entries) => entries.iter().any(|(key, _)| key == &value),
                Value::String(haystack) => match value {
                    Value::String(needle) => {
                        haystack.to_lowercase().contains(&needle.to_lowercase())
                    }
                    _ => return Err("String membership requires a String value".to_owned()),
                },
                _ => return Err("membership requires a String, Array, Set, or Map".to_owned()),
            };
            Ok(Value::Bool(contains))
        }
        Expr::Index {
            collection,
            index,
            optional,
            collection_type,
            ..
        } => {
            let collection = eval_expr(collection, environment, functions, depth + 1)?;
            let index = eval_expr(index, environment, functions, depth + 1)?;
            if matches!(collection, Value::Null) && *optional {
                return Ok(Value::Null);
            }
            let is_map = matches!(collection_type, Type::Map(_, _));
            let value = match (collection, index) {
                (Value::Array(values), Value::Integer(index)) => usize::try_from(index)
                    .ok()
                    .and_then(|index| values.get(index).cloned()),
                (Value::Map(entries), key) => entries
                    .into_iter()
                    .find_map(|(candidate, value)| (candidate == key).then_some(value)),
                _ => {
                    return Err(
                        "headless indexing requires a valid Array index or Map key".to_owned()
                    );
                }
            };
            match value {
                Some(value) => Ok(value),
                None if *optional || is_map => Ok(Value::Null),
                None => Err("headless collection index is out of bounds or missing".to_owned()),
            }
        }
        Expr::Member {
            base,
            kind,
            optional,
            ..
        } => {
            let base = eval_expr(base, environment, functions, depth + 1)?;
            let base = if matches!(base, Value::Null) {
                return if *optional {
                    Ok(Value::Null)
                } else {
                    Err("member access on null in headless test".to_owned())
                };
            } else {
                base
            };
            match (kind, base) {
                (MemberKind::StringTrimmed, Value::String(value)) => {
                    Ok(Value::String(value.trim().to_owned()))
                }
                (MemberKind::CollectionCount, Value::Array(values) | Value::Set(values)) => {
                    Ok(Value::Integer(values.len() as i128))
                }
                (MemberKind::CollectionCount, Value::Map(values)) => {
                    Ok(Value::Integer(values.len() as i128))
                }
                (MemberKind::CollectionIsEmpty, Value::Array(values) | Value::Set(values)) => {
                    Ok(Value::Bool(values.is_empty()))
                }
                (MemberKind::CollectionIsEmpty, Value::Map(values)) => {
                    Ok(Value::Bool(values.is_empty()))
                }
                (MemberKind::TupleIndex(TuplePosition::First), Value::Pair(first, _))
                | (MemberKind::TupleIndex(TuplePosition::First), Value::Triple(first, _, _)) => {
                    Ok(*first)
                }
                (MemberKind::TupleIndex(TuplePosition::Second), Value::Pair(_, second))
                | (MemberKind::TupleIndex(TuplePosition::Second), Value::Triple(_, second, _)) => {
                    Ok(*second)
                }
                (MemberKind::TupleIndex(TuplePosition::Third), Value::Triple(_, _, third)) => {
                    Ok(*third)
                }
                (MemberKind::StructField(field), Value::Struct(_, fields)) => fields
                    .into_iter()
                    .find_map(|(name, value)| (name == *field).then_some(value))
                    .ok_or_else(|| format!("headless struct has no field `{field}`")),
                (MemberKind::PluginField(field), _) => Err(format!(
                    "headless tests cannot read native plugin property `{field}`"
                )),
                _ => Err("unsupported headless member access".to_owned()),
            }
        }
        Expr::NativeCall {
            receiver: Some(receiver),
            namespace,
            name,
            arguments,
            ..
        } if namespace == "__NexaMap" => {
            let Value::Map(entries) = eval_expr(receiver, environment, functions, depth + 1)?
            else {
                return Err("headless Map access requires a Map receiver".to_owned());
            };
            let values = arguments
                .iter()
                .map(|(_, argument)| eval_expr(argument, environment, functions, depth + 1))
                .collect::<Result<Vec<_>, _>>()?;
            match name.as_str() {
                "get" => Ok(values
                    .first()
                    .and_then(|key| {
                        entries
                            .iter()
                            .find_map(|(candidate, value)| (candidate == key).then_some(value))
                    })
                    .cloned()
                    .unwrap_or(Value::Null)),
                "contains" => Ok(Value::Bool(values.first().is_some_and(|key| {
                    entries.iter().any(|(candidate, _)| candidate == key)
                }))),
                "keys" if values.is_empty() => Ok(Value::Array(
                    entries.into_iter().map(|(key, _)| key).collect(),
                )),
                "values" if values.is_empty() => Ok(Value::Array(
                    entries.into_iter().map(|(_, value)| value).collect(),
                )),
                _ => Err(format!("unsupported headless Map method `{name}`")),
            }
        }
        Expr::CollectionUtility {
            operation,
            collection,
            start,
            end,
            inclusive,
            ..
        } => {
            let Value::Array(mut values) =
                eval_expr(collection, environment, functions, depth + 1)?
            else {
                return Err("headless collection utility requires an Array".to_owned());
            };
            match operation {
                CollectionUtilityKind::First => Ok(values.first().cloned().unwrap_or(Value::Null)),
                CollectionUtilityKind::Last => Ok(values.last().cloned().unwrap_or(Value::Null)),
                CollectionUtilityKind::Reverse => {
                    values.reverse();
                    Ok(Value::Array(values))
                }
                CollectionUtilityKind::Slice => {
                    let start =
                        eval_range_bound(start.as_deref(), environment, functions, depth + 1)?;
                    let end = eval_range_bound(end.as_deref(), environment, functions, depth + 1)?;
                    let start = normalize_index(start, values.len())?;
                    let mut end = normalize_index(end, values.len())?;
                    if *inclusive {
                        if end == values.len() {
                            return Err("inclusive headless slice end is out of bounds".to_owned());
                        }
                        end += 1;
                    }
                    if start > end {
                        return Err("headless slice start exceeds its end".to_owned());
                    }
                    Ok(Value::Array(
                        values
                            .get(start..end)
                            .ok_or_else(|| "headless slice range is out of bounds".to_owned())?
                            .to_vec(),
                    ))
                }
                CollectionUtilityKind::Take => {
                    let count =
                        eval_range_bound(end.as_deref(), environment, functions, depth + 1)?;
                    let count = usize::try_from(count).unwrap_or(0);
                    Ok(Value::Array(values.into_iter().take(count).collect()))
                }
                CollectionUtilityKind::Random | CollectionUtilityKind::Shuffled => Err(
                    "randomized collection utilities are not deterministic in headless tests"
                        .to_owned(),
                ),
            }
        }
        Expr::CollectionTransform {
            operation,
            collection,
            initial,
            closure,
        } => {
            let Value::Array(values) = eval_expr(collection, environment, functions, depth + 1)?
            else {
                return Err("headless collection transform requires an Array".to_owned());
            };
            let Expr::Closure { parameters, body } = closure.as_ref() else {
                return Err("headless collection transform requires an inline closure".to_owned());
            };
            match operation {
                CollectionTransform::Map
                | CollectionTransform::FlatMap
                | CollectionTransform::Filter => {
                    let [parameter] = parameters.as_slice() else {
                        return Err(
                            "headless map/filter closure must take one parameter".to_owned()
                        );
                    };
                    let mut result = Vec::with_capacity(values.len());
                    for value in values {
                        let mut scope = environment.clone();
                        scope.insert(parameter.clone(), value.clone());
                        let mapped = eval_expr(body, &scope, functions, depth + 1)?;
                        match operation {
                            CollectionTransform::Map => result.push(mapped),
                            CollectionTransform::FlatMap => match mapped {
                                Value::Array(values) => result.extend(values),
                                _ => {
                                    return Err("headless flatMap closure did not return an Array"
                                        .to_owned());
                                }
                            },
                            CollectionTransform::Filter => match mapped {
                                Value::Bool(true) => result.push(value),
                                Value::Bool(false) => {}
                                _ => {
                                    return Err(
                                        "headless filter closure did not return Bool".to_owned()
                                    );
                                }
                            },
                            CollectionTransform::SortedBy => {
                                return Err("invalid headless transform dispatch".to_owned());
                            }
                            CollectionTransform::GroupedBy => {
                                return Err("invalid headless transform dispatch".to_owned());
                            }
                            CollectionTransform::Reduce => {
                                return Err("invalid headless transform dispatch".to_owned());
                            }
                        }
                    }
                    Ok(Value::Array(result))
                }
                CollectionTransform::SortedBy => {
                    let [parameter] = parameters.as_slice() else {
                        return Err("headless sortedBy closure must take one parameter".to_owned());
                    };
                    let mut keyed = Vec::with_capacity(values.len());
                    for value in values {
                        let mut scope = environment.clone();
                        scope.insert(parameter.clone(), value.clone());
                        keyed.push((value, eval_expr(body, &scope, functions, depth + 1)?));
                    }
                    let mut comparison_error = None;
                    keyed.sort_by(|left, right| match compare_sort_values(&left.1, &right.1) {
                        Ok(ordering) => ordering,
                        Err(error) => {
                            comparison_error = Some(error);
                            Ordering::Equal
                        }
                    });
                    if let Some(error) = comparison_error {
                        return Err(error);
                    }
                    Ok(Value::Array(
                        keyed.into_iter().map(|(value, _)| value).collect(),
                    ))
                }
                CollectionTransform::GroupedBy => {
                    let [parameter] = parameters.as_slice() else {
                        return Err("headless groupedBy closure must take one parameter".to_owned());
                    };
                    let mut groups: Vec<(Value, Vec<Value>)> = Vec::new();
                    for value in values {
                        let mut scope = environment.clone();
                        scope.insert(parameter.clone(), value.clone());
                        let key = eval_expr(body, &scope, functions, depth + 1)?;
                        if let Some(position) =
                            groups.iter().position(|(candidate, _)| candidate == &key)
                        {
                            groups[position].1.push(value);
                        } else {
                            groups.push((key, vec![value]));
                        }
                    }
                    Ok(Value::Array(
                        groups
                            .into_iter()
                            .map(|(_, values)| Value::Array(values))
                            .collect(),
                    ))
                }
                CollectionTransform::Reduce => {
                    let [accumulator_parameter, value_parameter] = parameters.as_slice() else {
                        return Err("headless reduce closure must take two parameters".to_owned());
                    };
                    let mut accumulator = initial
                        .as_deref()
                        .ok_or_else(|| "headless reduce is missing its initial value".to_owned())
                        .and_then(|initial| {
                            eval_expr(initial, environment, functions, depth + 1)
                        })?;
                    for value in values {
                        let mut scope = environment.clone();
                        scope.insert(accumulator_parameter.clone(), accumulator);
                        scope.insert(value_parameter.clone(), value);
                        accumulator = eval_expr(body, &scope, functions, depth + 1)?;
                    }
                    Ok(accumulator)
                }
            }
        }
        Expr::Range {
            start,
            end,
            inclusive,
            step,
        } => eval_integer_range(
            start,
            end,
            *inclusive,
            step.as_deref(),
            environment,
            functions,
            depth + 1,
        ),
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
                if let Type::Struct {
                    name: struct_name,
                    fields,
                } = return_type
                {
                    if arguments.len() != fields.len() {
                        return Err(format!(
                            "struct constructor `{name}` expected {} field(s), got {}",
                            fields.len(),
                            arguments.len()
                        ));
                    }
                    return Ok(Value::Struct(
                        struct_name.clone(),
                        fields
                            .iter()
                            .map(|(field, _)| field.clone())
                            .zip(arguments)
                            .collect(),
                    ));
                }
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
        Expr::Null(_) => Ok(Value::Null),
        Expr::Coalesce(value, fallback) => {
            match eval_expr(value, environment, functions, depth + 1)? {
                Value::Null => eval_expr(fallback, environment, functions, depth + 1),
                value => Ok(value),
            }
        }
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

fn compare_sort_values(left: &Value, right: &Value) -> Result<Ordering, String> {
    match (left, right) {
        (Value::String(left), Value::String(right)) => Ok(left.cmp(right)),
        (Value::Integer(left), Value::Integer(right)) => Ok(left.cmp(right)),
        (Value::Float(left), Value::Float(right)) => left
            .partial_cmp(right)
            .ok_or_else(|| "sortedBy key values must not be NaN".to_owned()),
        (Value::Integer(left), Value::Float(right)) => (*left as f64)
            .partial_cmp(right)
            .ok_or_else(|| "sortedBy key values must not be NaN".to_owned()),
        (Value::Float(left), Value::Integer(right)) => left
            .partial_cmp(&(*right as f64))
            .ok_or_else(|| "sortedBy key values must not be NaN".to_owned()),
        _ => Err("sortedBy keys must be String or numeric values".to_owned()),
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

fn eval_range_bound(
    expression: Option<&Expr>,
    environment: &Environment,
    functions: &FunctionMap<'_>,
    depth: usize,
) -> Result<i128, String> {
    match expression {
        Some(expression) => match eval_expr(expression, environment, functions, depth + 1)? {
            Value::Integer(value) => Ok(value),
            _ => Err("headless range bound is not an integer".to_owned()),
        },
        None => Ok(0),
    }
}

fn normalize_index(index: i128, length: usize) -> Result<usize, String> {
    let index =
        usize::try_from(index).map_err(|_| "headless collection index is negative".to_owned())?;
    if index > length {
        return Err("headless collection index is out of bounds".to_owned());
    }
    Ok(index)
}

fn eval_integer_range(
    start: &Expr,
    end: &Expr,
    inclusive: bool,
    step: Option<&Expr>,
    environment: &Environment,
    functions: &FunctionMap<'_>,
    depth: usize,
) -> Result<Value, String> {
    let start = eval_range_bound(Some(start), environment, functions, depth + 1)?;
    let end = eval_range_bound(Some(end), environment, functions, depth + 1)?;
    let step = match step {
        Some(step) => eval_range_bound(Some(step), environment, functions, depth + 1)?,
        None if start <= end => 1,
        None => -1,
    };
    if step == 0 {
        return Err("headless range step cannot be zero".to_owned());
    }
    let mut values = Vec::new();
    let mut current = start;
    while if step > 0 {
        if inclusive {
            current <= end
        } else {
            current < end
        }
    } else if inclusive {
        current >= end
    } else {
        current > end
    } {
        if values.len() >= 100_000 {
            return Err("headless range exceeded 100,000 values".to_owned());
        }
        values.push(Value::Integer(current));
        current = current
            .checked_add(step)
            .ok_or_else(|| "headless range overflowed".to_owned())?;
    }
    Ok(Value::Array(values))
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
        Value::Null => "null".to_owned(),
        Value::Void => "()".to_owned(),
        Value::Bool(value) => value.to_string(),
        Value::String(value) => value,
        Value::Integer(value) => value.to_string(),
        Value::Float(value) => value.to_string(),
        Value::Enum(name, case) => format!("{name}.{case}"),
        Value::EnumPayload(name, case, values) => format!(
            "{name}.{case}({})",
            values
                .into_iter()
                .map(display_value)
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Value::Array(values) => format!(
            "[{}]",
            values
                .into_iter()
                .map(display_value)
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Value::Set(values) => format!(
            "Set([{}])",
            values
                .into_iter()
                .map(display_value)
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Value::Map(values) => format!(
            "[{}]",
            values
                .into_iter()
                .map(|(key, value)| format!("{}: {}", display_value(key), display_value(value)))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Value::Pair(first, second) => {
            format!(
                "Pair({}, {})",
                display_value(*first),
                display_value(*second)
            )
        }
        Value::Triple(first, second, third) => format!(
            "Triple({}, {}, {})",
            display_value(*first),
            display_value(*second),
            display_value(*third)
        ),
        Value::Struct(name, fields) => format!(
            "{}({})",
            name,
            fields
                .into_iter()
                .map(|(field, value)| format!("{field}: {}", display_value(value)))
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}
