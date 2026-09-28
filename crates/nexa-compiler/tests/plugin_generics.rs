//! Generic plugin methods: binding a value type per call site.
//!
//! A plugin method may declare value type parameters. Every call site has to
//! bind all of them, and the bound type decides which generated value codec
//! the call carries, so these tests pin the binding, the substitution, and the
//! diagnostics for the ways a binding can go wrong.

use std::fs;

use nexa_ir::{Expr, PluginCodec, Type};
use nexa_testkit::TestProject;

use nexa_compiler::{Target, compile_file_with_warnings_for_target};

/// Writes a plugin with `contract` plus an app with `body` and compiles it for
/// every target.
fn compile_with_plugin(name: &str, contract: &str, body: &str) -> Result<nexa_ir::Module, String> {
    let project = TestProject::new(name);
    let plugin = project.join("store");
    fs::create_dir_all(&plugin).expect("plugin directory should be created");
    fs::write(
        plugin.join("plugin.config.nx"),
        "plugin { schema: 2 id: \"dev.test.store\" version: \"1.0.0\" sources { native: \"native.nxid\" } }\n",
    )
    .expect("plugin manifest should be written");
    fs::write(plugin.join("native.nxid"), contract).expect("plugin contract should be written");
    let entry = project.join("App.nx");
    fs::write(&entry, body).expect("app source should be written");
    compile_file_with_warnings_for_target(&entry, Target::Swift)
        .map(|compiled| compiled.module)
        .map_err(|error| error.to_string())
}

/// A contract with the generic method shapes a storage plugin needs.
fn storage_contract() -> String {
    "native class Store {\n\
       init(id: String)\n\
       fn setObject<T>(key: String, value: T) -> Bool\n\
       fn getObject<T>(key: String) -> T?\n\
       fn setMap<K, V>(key: String, values: Map<K, V>) -> Bool\n\
       fn getMap<K, V>(key: String) -> Map<K, V>?\n\
       fn dispose()\n\
     }\n"
    .to_owned()
}

/// Every `NativeCall` in the module, so a test can assert on the codec each
/// call carries.
fn native_calls(module: &nexa_ir::Module) -> Vec<(String, Vec<PluginCodec>)> {
    let mut calls = Vec::new();
    let mut visit = |expr: &Expr| {
        if let Expr::NativeCall { name, codecs, .. } = expr {
            calls.push((name.clone(), codecs.clone()));
        }
    };
    let mut noop = |_: &nexa_ir::Node| {};
    for node in &module.body {
        walk(module, std::slice::from_ref(node), &mut noop, &mut visit);
    }
    for screen in &module.screens {
        walk(module, &screen.body, &mut noop, &mut visit);
    }
    for component in &module.components {
        walk(module, &component.body, &mut noop, &mut visit);
    }
    calls
}

fn walk(
    _module: &nexa_ir::Module,
    nodes: &[nexa_ir::Node],
    visit_node: &mut impl FnMut(&nexa_ir::Node),
    visit_expr: &mut impl FnMut(&Expr),
) {
    nexa_ir::walk::walk_ir(nodes, visit_node, visit_expr);
}

fn struct_body(action: &str) -> String {
    format!(
        "plugin \"store\" as Store\n\
         struct Options {{\n    autoplay: Bool,\n    volume: Float64,\n    tags: Array<String>,\n}}\n\
         app Demo {{\n    let store = Store.Store(\"id\")\n    state saved: Options? = null\n    body {{ Button(\"Save\") {{ {action} }} }}\n}}\n"
    )
}

#[test]
fn a_setter_binds_its_value_type_from_the_argument() {
    let module = compile_with_plugin(
        "nexa-generic-setter",
        &storage_contract(),
        &struct_body("store.setObject(\"options\", Options(true, 0.5, [\"a\"]))"),
    )
    .expect("an app struct should bind as a value type");

    let calls = native_calls(&module);
    let codec = calls
        .iter()
        .find(|(name, _)| name == "setObject")
        .and_then(|(_, codecs)| codecs.first())
        .expect("the setter should carry one value codec");
    assert!(!codec.decodes, "a setter encodes its value");
    assert!(
        matches!(&codec.ty, Type::Struct { name, .. } if name == "Options"),
        "the codec should carry the bound struct type, found {:?}",
        codec.ty
    );
}

#[test]
fn a_getter_binds_its_value_type_from_the_binding() {
    let module = compile_with_plugin(
        "nexa-generic-getter-inferred",
        &storage_contract(),
        &struct_body("saved = store.getObject(\"options\")"),
    )
    .expect("an optional binding should bind the getter's value type");

    let calls = native_calls(&module);
    let codec = calls
        .iter()
        .find(|(name, _)| name == "getObject")
        .and_then(|(_, codecs)| codecs.first())
        .expect("the getter should carry one value codec");
    assert!(codec.decodes, "a getter decodes its value");
    assert!(
        matches!(&codec.ty, Type::Struct { name, .. } if name == "Options"),
        "the codec should carry the bound struct type, found {:?}",
        codec.ty
    );
}

#[test]
fn an_explicit_type_argument_binds_a_getter() {
    let module = compile_with_plugin(
        "nexa-generic-getter-explicit",
        &storage_contract(),
        "plugin \"store\" as Store\n\
         struct Options {\n    autoplay: Bool,\n    volume: Float64,\n    tags: Array<String>,\n}\n\
         app Demo {\n    let store = Store.Store(\"id\")\n    state saved: Options? = null\n    state volume: Float64 = 0\n\
         body { Button(\"Save\") { volume = store.getObject<Float64>(\"volume\") ?? 0\n saved = store.getObject(\"options\") } }\n}\n",
    )
    .expect("an explicit type argument should bind a getter");

    let codecs = native_calls(&module)
        .into_iter()
        .filter(|(name, _)| name == "getObject")
        .flat_map(|(_, codecs)| codecs)
        .collect::<Vec<_>>();
    assert_eq!(codecs.len(), 2, "both reads should carry a codec");
    assert!(
        codecs
            .iter()
            .any(|codec| matches!(&codec.ty, Type::Numeric(_))),
        "the explicit argument should bind Float64, found {:?}",
        codecs.iter().map(|codec| &codec.ty).collect::<Vec<_>>()
    );
}

#[test]
fn a_map_binds_both_of_its_type_parameters() {
    let module = compile_with_plugin(
        "nexa-generic-map",
        &storage_contract(),
        "plugin \"store\" as Store\n\
         app Demo {\n    let store = Store.Store(\"id\")\n    state scores: Map<String, Int32> = [\"a\": 1]\n    body { Button(\"Save\") { store.setMap(\"scores\", scores) } }\n}\n",
    )
    .expect("a map state should bind as a value type");

    let codec = native_calls(&module)
        .into_iter()
        .find(|(name, _)| name == "setMap")
        .and_then(|(_, codecs)| codecs.into_iter().next())
        .expect("a map setter should carry one codec for the whole map");
    assert!(
        matches!(&codec.ty, Type::Map(key, value)
            if matches!(key.as_ref(), Type::String)
                && matches!(value.as_ref(), Type::Numeric(_))),
        "the codec should carry the bound map type, found {:?}",
        codec.ty
    );
}

#[test]
fn pair_and_triple_values_bind_generic_plugin_methods() {
    let module = compile_with_plugin(
        "nexa-generic-tuples",
        &storage_contract(),
        "plugin \"store\" as Store\n\
         app Demo {\n    let store = Store.Store(\"id\")\n\
             state pair: Pair<String, Int32> = Pair(\"key\", 7)\n\
             state triple: Triple<String, Int32, Bool> = Triple(\"key\", 7, true)\n\
             body { Button(\"Save\") { store.setObject(\"pair\", pair)\n store.setObject(\"triple\", triple) } }\n}\n",
    )
    .expect("pair and triple values should have generic plugin codecs");

    let codecs = native_calls(&module)
        .into_iter()
        .filter(|(name, _)| name == "setObject")
        .flat_map(|(_, codecs)| codecs.into_iter())
        .map(|codec| codec.ty)
        .collect::<Vec<_>>();
    assert!(
        codecs.iter().any(|ty| matches!(ty, Type::Pair(_, _))),
        "the generic pair call should carry a Pair codec: {codecs:?}"
    );
    assert!(
        codecs.iter().any(|ty| matches!(ty, Type::Triple(_, _, _))),
        "the generic triple call should carry a Triple codec: {codecs:?}"
    );
}

#[test]
fn an_unbound_value_type_is_reported_with_its_binding_syntax() {
    let error = compile_with_plugin(
        "nexa-generic-unbound",
        &storage_contract(),
        "plugin \"store\" as Store\n\
         app Demo {\n    let store = Store.Store(\"id\")\n    state volume: Float64 = 0\n    body { Button(\"Read\") { store.getObject(\"volume\") } }\n}\n",
    )
    .expect_err("a read with no expected type and no explicit argument cannot bind");
    assert!(
        error.contains("cannot infer value type `T`"),
        "unexpected diagnostic: {error}"
    );
    assert!(
        error.contains("getObject<T>"),
        "the diagnostic should show how to bind it: {error}"
    );
}

#[test]
fn a_wrong_type_argument_count_is_reported() {
    let error = compile_with_plugin(
        "nexa-generic-arity",
        &storage_contract(),
        "plugin \"store\" as Store\n\
         app Demo {\n    let store = Store.Store(\"id\")\n    state volume: Float64 = 0\n    body { Button(\"Read\") { volume = store.getObject<String, Int32>(\"volume\") ?? 0 } }\n}\n",
    )
    .expect_err("a method with one type parameter cannot take two");
    assert!(
        error.contains("expects 1 value type argument(s), found 2"),
        "unexpected diagnostic: {error}"
    );
}

#[test]
fn an_unknown_value_type_is_reported() {
    let error = compile_with_plugin(
        "nexa-generic-unknown",
        &storage_contract(),
        "plugin \"store\" as Store\n\
         app Demo {\n    let store = Store.Store(\"id\")\n    state volume: Float64 = 0\n    body { Button(\"Read\") { volume = store.getObject<Missing>(\"volume\") ?? 0 } }\n}\n",
    )
    .expect_err("a value type that is neither a struct nor an enum cannot be stored");
    assert!(
        error.contains("Missing"),
        "the diagnostic should name the unknown type: {error}"
    );
}

#[test]
fn a_result_value_type_is_reported() {
    let error = compile_with_plugin(
        "nexa-generic-non-value",
        &storage_contract(),
        "plugin \"store\" as Store\n\
         app Demo {\n    let store = Store.Store(\"id\")\n    body { Button(\"Read\") { store.getObject<Result<Float64, String>>(\"k\") } }\n}\n",
    )
    .expect_err("a Result is not a storable plugin value type");
    assert!(
        error.contains("must be a scalar, `Bytes`, an enum, a struct"),
        "unexpected diagnostic: {error}"
    );
}

#[test]
fn a_type_argument_on_a_plain_method_is_reported() {
    let error = compile_with_plugin(
        "nexa-generic-plain",
        "native class Store {\n    init(id: String)\n    fn setString(key: String, value: String) -> Bool\n    fn dispose()\n}\n",
        "plugin \"store\" as Store\n\
         app Demo {\n    let store = Store.Store(\"id\")\n    body { Button(\"Save\") { store.setString<Int32>(\"k\", \"v\") } }\n}\n",
    )
    .expect_err("a method without type parameters cannot take one");
    assert!(
        error.contains("does not declare value type parameters"),
        "unexpected diagnostic: {error}"
    );
}
