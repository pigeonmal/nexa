//! Validated bridge plans for native plugin contracts.
//!
//! Pipeline: `PluginIdl -> validate -> BridgePlan -> render`.
//!
//! The IDL parser guarantees syntactic validity, but renderers historically
//! re-derived deeper legality (declared-type resolution, generic arities,
//! `Result` shapes, per-target value support) at every use site through
//! `Option`-returning mappers paired with `expect`/`unreachable!`. This
//! module performs that validation exactly once per render path and resolves
//! every type reference into a [`BridgeType`]. Renderers consume the plan:
//! all mapping functions over [`BridgeType`] are total, so code generation
//! cannot panic on user input — unsupported contracts fail in `validate`
//! with a descriptive error instead.

use nexa_plugin_idl::{
    Interface, InterfaceKind, Literal, Method, NamedType, NamedTypeKind, PluginIdl,
};

/// Scalar value kinds shared by every bridge target.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BridgeScalar {
    Void,
    Bool,
    Int8,
    Int16,
    Int32,
    Int64,
    UInt8,
    UInt16,
    UInt32,
    UInt64,
    Float32,
    Float64,
    String,
    Bytes,
}

/// Resolved kind of a named type or interface reference. Carrying the kind
/// removes every "find this declaration to decide how to render it" lookup
/// from the renderers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BridgeNamedKind {
    Struct,
    Enum,
    Error,
    Interface,
    NativeClass,
    NativeComponent,
}

/// Resolved kind of a declared named type. Only these three can appear in
/// the plan's type table, so matches over them are exhaustive.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BridgeTypeKind {
    Struct,
    Enum,
    Error,
}

/// A fully resolved IDL type reference. Resolution proves the name is
/// declared (or builtin) and the generic arity is correct; per-target
/// validators additionally prove the shape is mappable, so mapping
/// functions over this type are total.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BridgeType {
    Scalar(BridgeScalar),
    Named {
        name: String,
        kind: BridgeNamedKind,
    },
    Array(Box<BridgeType>),
    Set(Box<BridgeType>),
    Map(Box<BridgeType>, Box<BridgeType>),
    Pair(Box<BridgeType>, Box<BridgeType>),
    Triple(Box<BridgeType>, Box<BridgeType>, Box<BridgeType>),
    Optional(Box<BridgeType>),
    Result {
        success: Box<BridgeType>,
        /// Declared error type name. Validation proves the declaration
        /// exists; renderers iterate [`BridgePlan::errors`] for its cases.
        failure: String,
    },
}

impl BridgeType {
    /// Whether this type is the `Void` scalar.
    pub fn is_void(&self) -> bool {
        matches!(self, BridgeType::Scalar(BridgeScalar::Void))
    }

    /// Whether this type is `Optional<..>`.
    pub fn is_optional(&self) -> bool {
        matches!(self, BridgeType::Optional(_))
    }
}

/// A validated struct field, property, parameter, or error payload.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BridgeField {
    pub name: String,
    pub ty: BridgeType,
    pub default: Option<Literal>,
}

/// A validated method or constructor parameter.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BridgeParameter {
    pub name: String,
    pub ty: BridgeType,
}

/// A validated error case with resolved payload types.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BridgeVariant {
    pub name: String,
    pub parameters: Vec<BridgeParameter>,
}

/// A validated named type declaration.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BridgeNamedType {
    pub name: String,
    pub kind: BridgeTypeKind,
    pub fields: Vec<BridgeField>,
    pub cases: Vec<BridgeVariant>,
}

/// A validated method with a resolved return type and error reference.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BridgeMethod {
    pub name: String,
    pub is_async: bool,
    pub parameters: Vec<BridgeParameter>,
    pub return_type: BridgeType,
    /// Declared error type name for `throws` methods.
    pub throws: Option<String>,
}

impl BridgeMethod {
    /// The success type of the method: the `Result` payload, if any, else
    /// the return type itself. The plan guarantees the `Result` shape, so
    /// this never needs an expect.
    pub fn success_type(&self) -> &BridgeType {
        match &self.return_type {
            BridgeType::Result { success, .. } => success,
            other => other,
        }
    }

    /// The declared error type name, from `throws` or a `Result` return.
    pub fn error_type(&self) -> Option<&str> {
        self.throws.as_deref().or_else(|| match &self.return_type {
            BridgeType::Result { failure, .. } => Some(failure),
            _ => None,
        })
    }
}

/// A validated property.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BridgeProperty {
    pub name: String,
    pub ty: BridgeType,
    pub mutable: bool,
    pub default: Option<Literal>,
}

/// A validated event.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BridgeEvent {
    pub name: String,
    pub parameters: Vec<BridgeParameter>,
}

/// A validated constructor.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BridgeConstructor {
    pub parameters: Vec<BridgeParameter>,
}

/// A validated interface declaration.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BridgeInterface {
    pub name: String,
    pub kind: InterfaceKind,
    pub has_content_slot: bool,
    pub constructors: Vec<BridgeConstructor>,
    pub methods: Vec<BridgeMethod>,
    pub properties: Vec<BridgeProperty>,
    pub events: Vec<BridgeEvent>,
}

/// A validated config option.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BridgeConfigOption {
    pub name: String,
    pub ty: BridgeType,
    pub default: Option<Literal>,
}

/// A validated plugin contract ready for rendering. Constructible only
/// through the `validate_*` entry points, which prove every contained type
/// reference resolves and is mappable by the corresponding render path.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BridgePlan {
    pub types: Vec<BridgeNamedType>,
    pub interfaces: Vec<BridgeInterface>,
    pub config: Vec<BridgeConfigOption>,
    /// Declared error types referenced by method `Result` returns or
    /// `throws` clauses, deduplicated by name. Renderers emit converters by
    /// iterating this list instead of looking errors up by name.
    pub referenced_errors: Vec<BridgeNamedType>,
}

impl BridgePlan {
    /// Validates the contract for the pure C++ specification renderer.
    pub fn validate_contract(idl: &PluginIdl) -> Result<Self, String> {
        validate(idl, Target::Contract)
    }

    /// Validates the contract for the Swift contract renderer.
    pub fn validate_swift_contract(idl: &PluginIdl) -> Result<Self, String> {
        validate(idl, Target::SwiftContract)
    }

    /// Validates the contract for the Kotlin contract renderer.
    pub fn validate_kotlin_contract(idl: &PluginIdl) -> Result<Self, String> {
        validate(idl, Target::KotlinContract)
    }

    /// Validates the contract for the Swift-to-C++ adapter renderer.
    pub fn validate_swift_cpp(idl: &PluginIdl) -> Result<Self, String> {
        validate(idl, Target::SwiftCpp)
    }

    /// Validates the contract for the Android JNI adapter renderer.
    pub fn validate_android(idl: &PluginIdl) -> Result<Self, String> {
        validate(idl, Target::Android)
    }

    /// Looks up a declared type by name. Validation proves every referenced
    /// name resolves, so renderers iterate [`BridgePlan::types`] or use
    /// references carried inline instead of looking types up here.
    pub fn declared_type(&self, name: &str) -> Option<&BridgeNamedType> {
        self.types.iter().find(|ty| ty.name == name)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Target {
    Contract,
    SwiftContract,
    KotlinContract,
    SwiftCpp,
    Android,
}

fn validate(idl: &PluginIdl, target: Target) -> Result<BridgePlan, String> {
    let resolver = Resolver::new(idl);
    let mut types = Vec::with_capacity(idl.types.len());
    for ty in &idl.types {
        types.push(resolver.named_type(ty)?);
    }
    let mut interfaces = Vec::with_capacity(idl.interfaces.len());
    for interface in &idl.interfaces {
        interfaces.push(resolver.interface(interface)?);
    }
    let mut config = Vec::with_capacity(idl.config.len());
    for option in &idl.config {
        config.push(BridgeConfigOption {
            name: option.name.clone(),
            ty: resolver.resolve(&option.ty, &option.name, Position::Value)?,
            default: option.default.clone(),
        });
    }
    let mut plan = BridgePlan {
        types,
        interfaces,
        config,
        referenced_errors: Vec::new(),
    };
    // Per-target value support previously lived behind `Option` mappers
    // rediscovered at every render call. Check every value position once,
    // with the same messages the renderers used to produce.
    check_target_values(&plan, target)?;
    check_target_structure(&plan, target)?;
    plan.referenced_errors = collect_referenced_errors(&plan);
    Ok(plan)
}

/// Structural contract rules the renderers used to enforce inline while
/// emitting. Like value support, these are checked once during validation
/// so rendering stays total.
fn check_target_structure(plan: &BridgePlan, target: Target) -> Result<(), String> {
    if !matches!(target, Target::Android) {
        return Ok(());
    }
    for interface in &plan.interfaces {
        if interface.kind == InterfaceKind::Service {
            let mut methods = std::collections::HashSet::new();
            for method in &interface.methods {
                if !methods.insert(method.name.as_str()) {
                    return Err(format!(
                        "Android C++ adapters do not support duplicate service method name `{}`",
                        method.name
                    ));
                }
            }
        }
        if interface.kind == InterfaceKind::NativeClass {
            if interface.constructors.len() > 1 {
                return Err(format!(
                    "Android C++ adapters currently support one constructor for `{}`",
                    interface.name
                ));
            }
            let dispose = interface
                .methods
                .iter()
                .find(|method| method.name == "dispose");
            let Some(dispose) = dispose else {
                return Err(format!(
                    "Android C++ native class `{}` must declare `fn dispose()` for deterministic native ownership",
                    interface.name
                ));
            };
            if !dispose.parameters.is_empty()
                || dispose.is_async
                || !matches!(dispose.return_type, BridgeType::Scalar(BridgeScalar::Void))
            {
                return Err(format!(
                    "Android C++ native class disposal must be a synchronous parameterless `fn dispose()` on `{}`",
                    interface.name
                ));
            }
        }
    }
    Ok(())
}

/// Where a type reference occurs. `Result` is only legal in method return
/// position, mirroring the IDL parser's own rules so hand-built contracts
/// get the same diagnostics as parsed ones.
#[derive(Clone, Copy)]
enum Position {
    Value,
    MethodReturn,
}

struct Resolver<'a> {
    types: std::collections::HashMap<&'a str, &'a NamedType>,
    interfaces: std::collections::HashMap<&'a str, &'a Interface>,
    errors: std::collections::HashSet<&'a str>,
}

impl<'a> Resolver<'a> {
    fn new(idl: &'a PluginIdl) -> Self {
        let mut types = std::collections::HashMap::new();
        for ty in &idl.types {
            types.insert(ty.name.as_str(), ty);
        }
        let mut interfaces = std::collections::HashMap::new();
        for interface in &idl.interfaces {
            interfaces.insert(interface.name.as_str(), interface);
        }
        let errors = idl
            .types
            .iter()
            .filter(|ty| ty.kind == NamedTypeKind::Error)
            .map(|ty| ty.name.as_str())
            .collect();
        Self {
            types,
            interfaces,
            errors,
        }
    }

    fn named_type(&self, ty: &NamedType) -> Result<BridgeNamedType, String> {
        let kind = match ty.kind {
            NamedTypeKind::Struct => BridgeTypeKind::Struct,
            NamedTypeKind::Enum => BridgeTypeKind::Enum,
            NamedTypeKind::Error => BridgeTypeKind::Error,
        };
        let mut fields = Vec::with_capacity(ty.fields.len());
        for field in &ty.fields {
            fields.push(BridgeField {
                name: field.name.clone(),
                ty: self.resolve(&field.ty, &field.name, Position::Value)?,
                default: field.default.clone(),
            });
        }
        let mut cases = Vec::with_capacity(ty.cases.len());
        for case in &ty.cases {
            cases.push(BridgeVariant {
                name: case.name.clone(),
                parameters: self.parameters(&case.parameters)?,
            });
        }
        Ok(BridgeNamedType {
            name: ty.name.clone(),
            kind,
            fields,
            cases,
        })
    }

    fn interface(&self, interface: &Interface) -> Result<BridgeInterface, String> {
        let mut constructors = Vec::with_capacity(interface.constructors.len());
        for constructor in &interface.constructors {
            constructors.push(BridgeConstructor {
                parameters: self.parameters(&constructor.parameters)?,
            });
        }
        let mut methods = Vec::with_capacity(interface.methods.len());
        for method in &interface.methods {
            methods.push(self.method(method)?);
        }
        let mut properties = Vec::with_capacity(interface.properties.len());
        for property in &interface.properties {
            properties.push(BridgeProperty {
                name: property.name.clone(),
                ty: self.resolve(&property.ty, &property.name, Position::Value)?,
                mutable: property.mutable,
                default: property.default.clone(),
            });
        }
        let mut events = Vec::with_capacity(interface.events.len());
        for event in &interface.events {
            events.push(BridgeEvent {
                name: event.name.clone(),
                parameters: self.parameters(&event.parameters)?,
            });
        }
        Ok(BridgeInterface {
            name: interface.name.clone(),
            kind: interface.kind,
            has_content_slot: interface.has_content_slot,
            constructors,
            methods,
            properties,
            events,
        })
    }

    fn method(&self, method: &Method) -> Result<BridgeMethod, String> {
        let return_type =
            self.resolve(&method.return_type, &method.name, Position::MethodReturn)?;
        if matches!(return_type, BridgeType::Result { .. }) && method.throws.is_some() {
            return Err(format!(
                "method `{}` cannot combine `Result` with `throws`; declare one error type",
                method.name
            ));
        }
        let throws = method
            .throws
            .as_ref()
            .map(|throws| self.error_reference(throws, &method.name))
            .transpose()?;
        if let BridgeType::Result { failure, .. } = &return_type
            && !self.errors.contains(failure.as_str())
        {
            return Err(format!(
                "failure type `{failure}` in method `{}` must be a declared error type",
                method.name
            ));
        }
        if let Some(throws) = &throws
            && !self.errors.contains(throws.as_str())
        {
            return Err(format!(
                "throws type `{throws}` in method `{}` must be a declared error type",
                method.name
            ));
        }
        Ok(BridgeMethod {
            name: method.name.clone(),
            is_async: method.is_async,
            parameters: self.parameters(&method.parameters)?,
            return_type,
            throws,
        })
    }

    fn error_reference(
        &self,
        ty: &nexa_plugin_idl::TypeRef,
        context: &str,
    ) -> Result<String, String> {
        if ty.optional || !ty.arguments.is_empty() {
            return Err(format!(
                "throws type `{}` in method `{context}` must be a declared error type",
                ty.name
            ));
        }
        Ok(ty.name.clone())
    }

    fn parameters(
        &self,
        parameters: &[nexa_plugin_idl::Parameter],
    ) -> Result<Vec<BridgeParameter>, String> {
        parameters
            .iter()
            .map(|parameter| {
                Ok(BridgeParameter {
                    name: parameter.name.clone(),
                    ty: self.resolve(&parameter.ty, &parameter.name, Position::Value)?,
                })
            })
            .collect()
    }

    fn resolve(
        &self,
        ty: &nexa_plugin_idl::TypeRef,
        context: &str,
        position: Position,
    ) -> Result<BridgeType, String> {
        let resolved = match ty.name.as_str() {
            "Void" => BridgeType::Scalar(BridgeScalar::Void),
            "Bool" => BridgeType::Scalar(BridgeScalar::Bool),
            "Int8" => BridgeType::Scalar(BridgeScalar::Int8),
            "Int16" => BridgeType::Scalar(BridgeScalar::Int16),
            "Int32" => BridgeType::Scalar(BridgeScalar::Int32),
            "Int64" => BridgeType::Scalar(BridgeScalar::Int64),
            "UInt8" => BridgeType::Scalar(BridgeScalar::UInt8),
            "UInt16" => BridgeType::Scalar(BridgeScalar::UInt16),
            "UInt32" => BridgeType::Scalar(BridgeScalar::UInt32),
            "UInt64" => BridgeType::Scalar(BridgeScalar::UInt64),
            "Float32" => BridgeType::Scalar(BridgeScalar::Float32),
            "Float64" => BridgeType::Scalar(BridgeScalar::Float64),
            "String" => BridgeType::Scalar(BridgeScalar::String),
            "Bytes" => BridgeType::Scalar(BridgeScalar::Bytes),
            "Array" => {
                let element = self.generic_argument(ty, context, 1)?.remove(0);
                BridgeType::Array(Box::new(self.resolve(
                    &element,
                    context,
                    Position::Value,
                )?))
            }
            "Set" => {
                let element = self.generic_argument(ty, context, 1)?.remove(0);
                BridgeType::Set(Box::new(self.resolve(
                    &element,
                    context,
                    Position::Value,
                )?))
            }
            "Map" => {
                let mut arguments = self.generic_argument(ty, context, 2)?;
                let value = arguments.pop().expect("two map type arguments");
                let key = arguments.pop().expect("two map type arguments");
                BridgeType::Map(
                    Box::new(self.resolve(&key, context, Position::Value)?),
                    Box::new(self.resolve(&value, context, Position::Value)?),
                )
            }
            "Pair" => {
                let mut arguments = self.generic_argument(ty, context, 2)?;
                let second = arguments.pop().expect("two pair type arguments");
                let first = arguments.pop().expect("two pair type arguments");
                BridgeType::Pair(
                    Box::new(self.resolve(&first, context, Position::Value)?),
                    Box::new(self.resolve(&second, context, Position::Value)?),
                )
            }
            "Triple" => {
                let mut arguments = self.generic_argument(ty, context, 3)?;
                let third = arguments.pop().expect("three triple type arguments");
                let second = arguments.pop().expect("three triple type arguments");
                let first = arguments.pop().expect("three triple type arguments");
                BridgeType::Triple(
                    Box::new(self.resolve(&first, context, Position::Value)?),
                    Box::new(self.resolve(&second, context, Position::Value)?),
                    Box::new(self.resolve(&third, context, Position::Value)?),
                )
            }
            "Result" => {
                if !matches!(position, Position::MethodReturn) {
                    return Err(format!(
                        "{context} uses `Result` outside a method return type"
                    ));
                }
                if ty.optional {
                    return Err(format!(
                        "{context} cannot make `Result` optional; make its success type optional instead"
                    ));
                }
                let mut arguments = self.generic_argument(ty, context, 2)?;
                let failure = arguments.pop().expect("two result type arguments");
                let success = arguments.pop().expect("two result type arguments");
                if failure.optional {
                    return Err(format!("{context} uses an optional `Result` failure type"));
                }
                let failure = self.plain_name(&failure, context)?;
                BridgeType::Result {
                    success: Box::new(self.resolve(&success, context, Position::Value)?),
                    failure,
                }
            }
            name => {
                if let Some(declared) = self.types.get(name) {
                    if !ty.arguments.is_empty() {
                        return Err(format!(
                            "{context} uses declared type `{name}` with unsupported type arguments"
                        ));
                    }
                    let kind = match declared.kind {
                        NamedTypeKind::Struct => BridgeNamedKind::Struct,
                        NamedTypeKind::Enum => BridgeNamedKind::Enum,
                        NamedTypeKind::Error => BridgeNamedKind::Error,
                    };
                    BridgeType::Named {
                        name: name.to_owned(),
                        kind,
                    }
                } else if let Some(interface) = self.interfaces.get(name) {
                    if !ty.arguments.is_empty() {
                        return Err(format!(
                            "{context} uses declared type `{name}` with unsupported type arguments"
                        ));
                    }
                    let kind = match interface.kind {
                        InterfaceKind::Interface => BridgeNamedKind::Interface,
                        InterfaceKind::NativeClass => BridgeNamedKind::NativeClass,
                        InterfaceKind::NativeComponent => BridgeNamedKind::NativeComponent,
                        InterfaceKind::Service => BridgeNamedKind::Interface,
                    };
                    BridgeType::Named {
                        name: name.to_owned(),
                        kind,
                    }
                } else {
                    return Err(format!(
                        "{context} references undeclared plugin type `{name}`"
                    ));
                }
            }
        };
        if ty.optional {
            if matches!(resolved, BridgeType::Scalar(BridgeScalar::Void)) {
                return Err(format!("{context} uses optional `Void`"));
            }
            Ok(BridgeType::Optional(Box::new(resolved)))
        } else {
            Ok(resolved)
        }
    }

    fn generic_argument(
        &self,
        ty: &nexa_plugin_idl::TypeRef,
        context: &str,
        expected: usize,
    ) -> Result<Vec<nexa_plugin_idl::TypeRef>, String> {
        if ty.arguments.len() != expected {
            return Err(format!(
                "{context} uses `{}` with {}, expected {expected} type argument(s)",
                ty.name,
                ty.arguments.len()
            ));
        }
        Ok(ty.arguments.clone())
    }

    fn plain_name(&self, ty: &nexa_plugin_idl::TypeRef, context: &str) -> Result<String, String> {
        if !ty.arguments.is_empty() {
            return Err(format!(
                "{context} uses `{}` with unexpected type arguments",
                ty.name
            ));
        }
        Ok(ty.name.clone())
    }
}

fn collect_referenced_errors(plan: &BridgePlan) -> Vec<BridgeNamedType> {
    // Mirrors the renderers' collection scope and ordering: service and
    // native-class methods only, sorted by error name.
    let mut names = std::collections::BTreeSet::new();
    for interface in &plan.interfaces {
        if !matches!(
            interface.kind,
            InterfaceKind::Service | InterfaceKind::NativeClass
        ) {
            continue;
        }
        for method in &interface.methods {
            if let Some(error) = method.error_type() {
                names.insert(error.to_owned());
            }
        }
    }
    names
        .into_iter()
        .filter_map(|name| {
            plan.types
                .iter()
                .find(|ty| ty.name == name && matches!(ty.kind, BridgeTypeKind::Error))
        })
        .cloned()
        .collect()
}

/// Per-target value support, previously rediscovered through `Option`
/// mappers at every render call. Returns the same errors the renderers
/// used to produce so unsupported contracts fail identically.
fn check_target_values(plan: &BridgePlan, target: Target) -> Result<(), String> {
    let check =
        |interface: &str, member: &str, ty: &BridgeType, allow_void: bool| -> Result<(), String> {
            let supported = match target {
                Target::Contract | Target::SwiftContract | Target::KotlinContract => true,
                Target::SwiftCpp => swift_cpp_supported(ty),
                Target::Android => android_member_supported(plan, ty, allow_void),
            };
            if supported && (allow_void || !ty.is_void()) {
                return Ok(());
            }
            Err(target_value_error(target, interface, member, ty))
        };
    for ty in &plan.types {
        // Every declared struct and enum is validated for the target, so
        // helpers render directly. Unused-but-unsupported declarations fail
        // in validation instead of panicking mid-render.
        for field in &ty.fields {
            // Struct fields, error payloads, and enum payloads share one
            // recursive check; the target predicates encode the nesting
            // rules each renderer assumed.
            check_nested_value(plan, &field.ty, target)?;
        }
        for case in &ty.cases {
            for parameter in &case.parameters {
                check_nested_value(plan, &parameter.ty, target)?;
            }
        }
        if matches!(target, Target::Android)
            && matches!(ty.kind, BridgeTypeKind::Enum)
            && (ty.cases.is_empty() || ty.cases.len() > 256)
        {
            return Err(format!(
                "Android C++ adapters require enums with 1 to 256 cases; `{}` declares {}",
                ty.name,
                ty.cases.len()
            ));
        }
    }
    for interface in &plan.interfaces {
        for constructor in &interface.constructors {
            for parameter in &constructor.parameters {
                check(&interface.name, &parameter.name, &parameter.ty, false)?;
            }
        }
        for property in &interface.properties {
            check(&interface.name, &property.name, &property.ty, false)?;
        }
        for event in &interface.events {
            for parameter in &event.parameters {
                check(&interface.name, &parameter.name, &parameter.ty, false)?;
            }
        }
        for method in &interface.methods {
            for parameter in &method.parameters {
                check(&interface.name, &parameter.name, &parameter.ty, false)?;
            }
            check(&interface.name, &method.name, method.success_type(), true)?;
            // Dispose methods skip error-payload validation exactly like the
            // renderers' shape-only disposal check did.
            let is_dispose =
                interface.kind == InterfaceKind::NativeClass && method.name == "dispose";
            if is_dispose {
                continue;
            }
            if let Some(error) = method.error_type() {
                let referenced = plan
                    .types
                    .iter()
                    .find(|ty| ty.name == error && matches!(ty.kind, BridgeTypeKind::Error));
                let Some(referenced) = referenced else {
                    return Err(format!(
                        "typed error `{error}` for `{}.{}` must be a declared error type",
                        interface.name, method.name
                    ));
                };
                for case in &referenced.cases {
                    for parameter in &case.parameters {
                        check_error_payload(
                            &parameter.ty,
                            target,
                            &interface.name,
                            &method.name,
                            error,
                            &case.name,
                        )?;
                    }
                }
            }
        }
    }
    Ok(())
}

fn check_nested_value(plan: &BridgePlan, ty: &BridgeType, target: Target) -> Result<(), String> {
    // Nested positions (struct fields, payloads) must satisfy the same
    // target predicates as top-level value positions.
    let supported = match target {
        Target::Contract | Target::SwiftContract | Target::KotlinContract => true,
        Target::SwiftCpp => swift_cpp_supported(ty),
        Target::Android => android_member_supported(plan, ty, false),
    };
    if !supported {
        return Err(target_nested_error(target, ty));
    }
    match ty {
        BridgeType::Array(element) | BridgeType::Set(element) | BridgeType::Optional(element) => {
            check_nested_value(plan, element, target)
        }
        BridgeType::Map(key, value) => {
            check_nested_value(plan, key, target)?;
            check_nested_value(plan, value, target)
        }
        BridgeType::Pair(first, second) => {
            check_nested_value(plan, first, target)?;
            check_nested_value(plan, second, target)
        }
        BridgeType::Triple(first, second, third) => {
            check_nested_value(plan, first, target)?;
            check_nested_value(plan, second, target)?;
            check_nested_value(plan, third, target)
        }
        BridgeType::Result { success, .. } => check_nested_value(plan, success, target),
        BridgeType::Scalar(_) | BridgeType::Named { .. } => Ok(()),
    }
}

fn check_error_payload(
    ty: &BridgeType,
    target: Target,
    interface: &str,
    method: &str,
    error: &str,
    case: &str,
) -> Result<(), String> {
    let supported = match target {
        Target::Contract | Target::SwiftContract | Target::KotlinContract => true,
        Target::SwiftCpp => swift_cpp_error_payload_supported(ty),
        Target::Android => android_error_payload_supported(ty),
    };
    if supported {
        return Ok(());
    }
    Err(match target {
        Target::SwiftCpp => format!(
            "C++ Swift typed errors support primitive, `String`, and `Bytes` payloads; `{interface}.{method}` error case `{error}.{case}` uses `{}`",
            bridge_bare_name(ty)
        ),
        Target::Android => format!(
            "Android C++ typed errors support non-optional primitive, `String`, and `Bytes` payloads; `{interface}.{method}` error case `{error}.{case}` uses `{}`",
            bridge_bare_name(ty)
        ),
        _ => format!(
            "typed error payload for `{interface}.{method}` error case `{error}.{case}` uses unsupported type `{}`",
            bridge_bare_name(ty)
        ),
    })
}

fn target_value_error(target: Target, interface: &str, member: &str, ty: &BridgeType) -> String {
    match target {
        Target::SwiftCpp => format!(
            "C++ Swift adapters support primitive, `String`, `Bytes`, nested `Array` values, compatible `Set` values, and supported-key `Map` values; `{interface}.{member}` uses `{}`",
            bridge_bare_name(ty)
        ),
        Target::Android => format!(
            "Android C++ adapters support primitive, `String`, `Bytes`, nested `Array` values, compatible `Set` values, flat primitive/string `Map` values, and maps with array or compatible set values; `{interface}.{member}` uses unsupported type `{}`",
            bridge_bare_name(ty)
        ),
        _ => format!(
            "`{interface}.{member}` uses unsupported type `{}`",
            bridge_bare_name(ty)
        ),
    }
}

fn target_nested_error(target: Target, ty: &BridgeType) -> String {
    match target {
        Target::SwiftCpp => format!(
            "C++ Swift adapters support primitive, `String`, `Bytes`, nested `Array` values, compatible `Set` values, and supported-key `Map` values; nested type `{}` is an unsupported type",
            bridge_bare_name(ty)
        ),
        Target::Android => format!(
            "Android C++ adapters support primitive, `String`, `Bytes`, nested `Array` values, compatible `Set` values, flat primitive/string `Map` values, and maps with array or compatible set values; nested type `{}` is an unsupported type",
            bridge_bare_name(ty)
        ),
        _ => format!(
            "nested type `{}` is an unsupported type",
            bridge_bare_name(ty)
        ),
    }
}

/// Bare source-level name of a resolved type for diagnostics, mirroring
/// the historical `TypeRef::name` spelling the old error messages
/// interpolated (generic arguments and the optional flag lived in separate
/// fields there).
fn bridge_bare_name(ty: &BridgeType) -> &str {
    match ty {
        BridgeType::Scalar(scalar) => bridge_scalar_name(*scalar),
        BridgeType::Named { name, .. } => name,
        BridgeType::Array(_) => "Array",
        BridgeType::Set(_) => "Set",
        BridgeType::Map(..) => "Map",
        BridgeType::Pair(..) => "Pair",
        BridgeType::Triple(..) => "Triple",
        BridgeType::Optional(inner) => bridge_bare_name(inner),
        BridgeType::Result { .. } => "Result",
    }
}

pub(crate) fn bridge_scalar_name(scalar: BridgeScalar) -> &'static str {
    match scalar {
        BridgeScalar::Void => "Void",
        BridgeScalar::Bool => "Bool",
        BridgeScalar::Int8 => "Int8",
        BridgeScalar::Int16 => "Int16",
        BridgeScalar::Int32 => "Int32",
        BridgeScalar::Int64 => "Int64",
        BridgeScalar::UInt8 => "UInt8",
        BridgeScalar::UInt16 => "UInt16",
        BridgeScalar::UInt32 => "UInt32",
        BridgeScalar::UInt64 => "UInt64",
        BridgeScalar::Float32 => "Float32",
        BridgeScalar::Float64 => "Float64",
        BridgeScalar::String => "String",
        BridgeScalar::Bytes => "Bytes",
    }
}

/// Swift-to-C++ value support, ported from the renderer's `Option` mapper
/// into total predicates over resolved types. Shapes the adapter cannot
/// spell (`Pair`, `Triple`, value-position `Result`) are rejected here so
/// mapping functions stay total.
fn swift_cpp_supported(ty: &BridgeType) -> bool {
    match ty {
        BridgeType::Scalar(_) => true,
        BridgeType::Named { .. } => true,
        BridgeType::Array(_) => swift_cpp_array_type_supported(ty),
        BridgeType::Set(_) => swift_cpp_supported_set(ty),
        BridgeType::Map(key, value) => {
            swift_cpp_map_key_supported(key) && swift_cpp_map_value_supported(value)
        }
        BridgeType::Optional(inner) => match inner.as_ref() {
            // Optional collections have no optional-bridge spelling (the
            // historical value mapper returned `None` for them), so they
            // fail validation exactly like the old renderer did.
            BridgeType::Scalar(BridgeScalar::Void) => false,
            BridgeType::Scalar(_) | BridgeType::Named { .. } => swift_cpp_supported(inner),
            _ => false,
        },
        BridgeType::Pair(..) | BridgeType::Triple(..) | BridgeType::Result { .. } => false,
    }
}

fn swift_cpp_array_type_supported(ty: &BridgeType) -> bool {
    let BridgeType::Array(element) = ty else {
        return false;
    };
    if matches!(element.as_ref(), BridgeType::Optional(_)) {
        return false;
    }
    match element.as_ref() {
        BridgeType::Array(_) => swift_cpp_array_leaf_supported(element),
        BridgeType::Set(_) => swift_cpp_supported(element),
        BridgeType::Scalar(_) | BridgeType::Named { .. } => {
            !matches!(element.as_ref(), BridgeType::Scalar(BridgeScalar::Void))
        }
        _ => false,
    }
}

fn swift_cpp_array_leaf_supported(ty: &BridgeType) -> bool {
    let BridgeType::Array(element) = ty else {
        return false;
    };
    if matches!(element.as_ref(), BridgeType::Optional(_)) {
        return false;
    }
    match element.as_ref() {
        BridgeType::Array(_) => swift_cpp_array_leaf_supported(element),
        _ => {
            matches!(
                element.as_ref(),
                BridgeType::Scalar(_) | BridgeType::Named { .. }
            ) && !matches!(element.as_ref(), BridgeType::Scalar(BridgeScalar::Void))
        }
    }
}

fn swift_cpp_supported_set(ty: &BridgeType) -> bool {
    match ty {
        BridgeType::Set(element) => matches!(
            element.as_ref(),
            BridgeType::Scalar(
                BridgeScalar::Bool
                    | BridgeScalar::Int8
                    | BridgeScalar::Int16
                    | BridgeScalar::Int32
                    | BridgeScalar::Int64
                    | BridgeScalar::UInt8
                    | BridgeScalar::UInt16
                    | BridgeScalar::UInt32
                    | BridgeScalar::UInt64
                    | BridgeScalar::Bytes
            )
        ),
        _ => false,
    }
}

fn swift_cpp_map_key_supported(ty: &BridgeType) -> bool {
    matches!(
        ty,
        BridgeType::Scalar(
            BridgeScalar::Bool
                | BridgeScalar::Int8
                | BridgeScalar::Int16
                | BridgeScalar::Int32
                | BridgeScalar::Int64
                | BridgeScalar::UInt8
                | BridgeScalar::UInt16
                | BridgeScalar::UInt32
                | BridgeScalar::UInt64
                | BridgeScalar::Bytes
        )
    )
}

fn swift_cpp_map_value_supported(ty: &BridgeType) -> bool {
    match ty {
        BridgeType::Map(key, value) => {
            swift_cpp_map_key_supported(key) && swift_cpp_map_value_supported(value)
        }
        _ if swift_cpp_supported_set(ty) => true,
        BridgeType::Scalar(scalar) => !matches!(scalar, BridgeScalar::Void),
        BridgeType::Named { .. } => true,
        BridgeType::Array(_) => swift_cpp_array_type_supported(ty),
        // Optional and compound values have no nested map-value spelling.
        _ => false,
    }
}

fn swift_cpp_error_payload_supported(ty: &BridgeType) -> bool {
    matches!(
        ty,
        BridgeType::Scalar(
            BridgeScalar::Bool
                | BridgeScalar::Int8
                | BridgeScalar::Int16
                | BridgeScalar::Int32
                | BridgeScalar::Int64
                | BridgeScalar::UInt8
                | BridgeScalar::UInt16
                | BridgeScalar::UInt32
                | BridgeScalar::UInt64
                | BridgeScalar::Float32
                | BridgeScalar::Float64
                | BridgeScalar::String
                | BridgeScalar::Bytes
        )
    )
}

/// Android C++ member support, ported from `ensure_android_cpp_value`.
/// Optional positions mirror the renderer exactly: optional scalars map
/// through the value table, optional collections are rejected, and
/// optional named values fail the named-declaration rule.
fn android_member_supported(plan: &BridgePlan, ty: &BridgeType, allow_void: bool) -> bool {
    let named_scalar = android_named_supported(plan, ty).is_some();
    let collections_with_named = !named_scalar && android_contains_named(plan, ty);
    let named_ok = !android_requires_named(ty)
        || android_named_supported(plan, ty).is_some_and(|named| {
            !ty.is_optional() && android_named_declaration_supported(plan, named)
        });
    android_cpp_type_supported(plan, ty)
        && named_ok
        && !collections_with_named
        && (allow_void || !android_cpp_is_void(ty))
}

/// Whether the type needs a declared struct/enum behind it: bare unknown
/// names (and optional `Void`, which misses the scalar table).
fn android_requires_named(ty: &BridgeType) -> bool {
    match ty {
        BridgeType::Array(_)
        | BridgeType::Set(_)
        | BridgeType::Map(..)
        | BridgeType::Pair(..)
        | BridgeType::Triple(..)
        | BridgeType::Result { .. } => false,
        BridgeType::Optional(inner) => match inner.as_ref() {
            BridgeType::Scalar(BridgeScalar::Void) => true,
            BridgeType::Scalar(_) => false,
            other => android_requires_named(other),
        },
        BridgeType::Scalar(_) => false,
        BridgeType::Named { .. } => true,
    }
}

/// Declared struct/enum behind a value, ignoring the outer optional flag
/// exactly like the renderer's lookup.
fn android_named_supported<'p>(
    plan: &'p BridgePlan,
    ty: &BridgeType,
) -> Option<&'p BridgeNamedType> {
    let name = match ty {
        BridgeType::Named { name, .. } => name,
        BridgeType::Optional(inner) => match inner.as_ref() {
            BridgeType::Named { name, .. } => name,
            _ => return None,
        },
        _ => return None,
    };
    plan.types.iter().find(|ty| {
        &ty.name == name && matches!(ty.kind, BridgeTypeKind::Struct | BridgeTypeKind::Enum)
    })
}

fn android_named_declaration_supported(plan: &BridgePlan, named: &BridgeNamedType) -> bool {
    match named.kind {
        BridgeTypeKind::Enum => !named.cases.is_empty() && named.cases.len() <= 256,
        BridgeTypeKind::Struct => named.fields.iter().all(|field| {
            if android_scalar_supported(&field.ty) && !field.ty.is_optional() {
                return true;
            }
            !field.ty.is_optional()
                && android_named_supported(plan, &field.ty)
                    .is_some_and(|nested| android_named_declaration_supported(plan, nested))
        }),
        BridgeTypeKind::Error => false,
    }
}

/// Whether any nested generic argument references a declared struct/enum
/// value. Optional wrappers are transparent here because the lookup
/// matches on the inner name.
fn android_contains_named(plan: &BridgePlan, ty: &BridgeType) -> bool {
    if android_named_supported(plan, ty).is_some() {
        return true;
    }
    match ty {
        BridgeType::Array(element) | BridgeType::Set(element) | BridgeType::Optional(element) => {
            android_contains_named(plan, element)
        }
        BridgeType::Map(key, value) | BridgeType::Pair(key, value) => {
            android_contains_named(plan, key) || android_contains_named(plan, value)
        }
        BridgeType::Triple(first, second, third) => {
            android_contains_named(plan, first)
                || android_contains_named(plan, second)
                || android_contains_named(plan, third)
        }
        BridgeType::Result { success, .. } => android_contains_named(plan, success),
        BridgeType::Scalar(_) | BridgeType::Named { .. } => false,
    }
}

/// Whether the scalar table maps this type (ignoring the outer optional
/// flag, like the renderer).
fn android_scalar_supported(ty: &BridgeType) -> bool {
    let inner = match ty {
        BridgeType::Optional(inner) => inner.as_ref(),
        other => other,
    };
    matches!(
        inner,
        BridgeType::Scalar(
            BridgeScalar::Void
                | BridgeScalar::Bool
                | BridgeScalar::Int8
                | BridgeScalar::Int16
                | BridgeScalar::Int32
                | BridgeScalar::Int64
                | BridgeScalar::UInt8
                | BridgeScalar::UInt16
                | BridgeScalar::UInt32
                | BridgeScalar::UInt64
                | BridgeScalar::Float32
                | BridgeScalar::Float64
                | BridgeScalar::String
                | BridgeScalar::Bytes
        )
    )
}

fn android_cpp_type_supported(plan: &BridgePlan, ty: &BridgeType) -> bool {
    match ty {
        BridgeType::Map(key, value) => {
            android_map_supported(plan, ty)
                && android_map_element_supported(key, true)
                && android_map_value_supported(plan, value)
        }
        BridgeType::Array(element) => android_cpp_type_supported(plan, element),
        BridgeType::Set(element) => {
            if !android_set_element_supported(element) {
                return false;
            }
            if ty.is_optional() {
                return false;
            }
            // Mirrors the historical mapper: a set is spelled when the
            // element has a primitive or reference array mapping, or when
            // the element itself is optional (`std::set<std::optional<T>>`).
            android_primitive_array_supported(element)
                || android_reference_array_element_supported(element)
                || element.is_optional()
        }
        BridgeType::Optional(inner) => match inner.as_ref() {
            BridgeType::Map(..) => {
                android_map_supported(plan, inner)
                    && match inner.as_ref() {
                        BridgeType::Map(key, value) => {
                            android_map_element_supported(key, true)
                                && android_map_value_supported(plan, value)
                        }
                        _ => false,
                    }
            }
            BridgeType::Scalar(BridgeScalar::Void) => false,
            BridgeType::Scalar(_) | BridgeType::Named { .. } => true,
            _ => false,
        },
        BridgeType::Scalar(_) | BridgeType::Named { .. } => true,
        BridgeType::Pair(..) | BridgeType::Triple(..) | BridgeType::Result { .. } => false,
    }
}

fn android_set_element_supported(ty: &BridgeType) -> bool {
    let inner = match ty {
        BridgeType::Optional(inner) => inner.as_ref(),
        other => other,
    };
    matches!(
        inner,
        BridgeType::Scalar(
            BridgeScalar::Bool
                | BridgeScalar::Int8
                | BridgeScalar::Int16
                | BridgeScalar::Int32
                | BridgeScalar::Int64
                | BridgeScalar::UInt8
                | BridgeScalar::UInt16
                | BridgeScalar::UInt32
                | BridgeScalar::UInt64
                | BridgeScalar::String
        )
    )
}

fn android_map_element_supported(ty: &BridgeType, is_key: bool) -> bool {
    let inner = match ty {
        BridgeType::Optional(inner) => inner.as_ref(),
        other => other,
    };
    if is_key && ty.is_optional() {
        return false;
    }
    matches!(
        inner,
        BridgeType::Scalar(
            BridgeScalar::Bool
                | BridgeScalar::Int8
                | BridgeScalar::Int16
                | BridgeScalar::Int32
                | BridgeScalar::Int64
                | BridgeScalar::UInt8
                | BridgeScalar::UInt16
                | BridgeScalar::UInt32
                | BridgeScalar::UInt64
                | BridgeScalar::Float32
                | BridgeScalar::Float64
                | BridgeScalar::String
        )
    ) && (!is_key
        || !matches!(
            inner,
            BridgeType::Scalar(BridgeScalar::Float32 | BridgeScalar::Float64)
        ))
}

fn android_map_value_supported(plan: &BridgePlan, ty: &BridgeType) -> bool {
    match ty {
        BridgeType::Map(..) => android_map_supported(plan, ty),
        _ if android_map_element_supported(ty, false)
            || matches!(ty, BridgeType::Scalar(BridgeScalar::Bytes)) =>
        {
            true
        }
        BridgeType::Optional(inner) => match inner.as_ref() {
            // The historical mapper matched maps by name, transparent over
            // the optional flag, so an optional nested map value is
            // supported exactly when its inner map is. Every other optional
            // shape is rejected here, as before.
            BridgeType::Map(..) => android_map_supported(plan, inner),
            _ => false,
        },
        BridgeType::Array(element) => android_map_value_supported(plan, element),
        BridgeType::Set(element) => android_set_element_supported(element),
        _ => false,
    }
}

fn android_map_supported(plan: &BridgePlan, ty: &BridgeType) -> bool {
    match ty {
        BridgeType::Map(key, value) => {
            android_map_element_supported(key, true) && android_map_value_supported(plan, value)
        }
        _ => false,
    }
}

fn android_cpp_is_void(ty: &BridgeType) -> bool {
    matches!(ty, BridgeType::Scalar(BridgeScalar::Void))
}

fn android_primitive_array_supported(ty: &BridgeType) -> bool {
    // Mirrors the renderer's primitive-array table, which requires a
    // non-optional scalar element (strings use the reference path).
    if ty.is_optional() {
        return false;
    }
    matches!(
        ty,
        BridgeType::Scalar(
            BridgeScalar::Bool
                | BridgeScalar::Int8
                | BridgeScalar::Int16
                | BridgeScalar::Int32
                | BridgeScalar::Int64
                | BridgeScalar::UInt8
                | BridgeScalar::UInt16
                | BridgeScalar::UInt32
                | BridgeScalar::UInt64
                | BridgeScalar::Float32
                | BridgeScalar::Float64
        )
    )
}

fn android_reference_array_element_supported(ty: &BridgeType) -> bool {
    if ty.is_optional() {
        return false;
    }
    matches!(
        ty,
        BridgeType::Scalar(BridgeScalar::String | BridgeScalar::Bytes)
    )
}

fn android_error_payload_supported(ty: &BridgeType) -> bool {
    matches!(
        ty,
        BridgeType::Scalar(
            BridgeScalar::Bool
                | BridgeScalar::Int8
                | BridgeScalar::Int16
                | BridgeScalar::Int32
                | BridgeScalar::Int64
                | BridgeScalar::UInt8
                | BridgeScalar::UInt16
                | BridgeScalar::UInt32
                | BridgeScalar::UInt64
                | BridgeScalar::Float32
                | BridgeScalar::Float64
                | BridgeScalar::String
                | BridgeScalar::Bytes
        )
    )
}
