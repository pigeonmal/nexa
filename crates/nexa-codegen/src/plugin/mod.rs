//! Code generation for native plugin contracts across Swift, Kotlin, and C++.

pub mod bindings;
pub mod bindings_cpp;
pub mod bridge_plan;
pub mod cpp;
pub mod type_visit;

use nexa_plugin_idl::PluginIdl;

use bridge_plan::BridgePlan;

/// Emits Swift binding protocols and structs from the plugin IDL contract.
pub fn render_swift(contract: &PluginIdl) -> Result<String, String> {
    Ok(bindings::swift(&BridgePlan::validate_swift_contract(
        contract,
    )?))
}

/// Emits Kotlin binding interfaces and data classes from the plugin IDL contract.
pub fn render_kotlin(contract: &PluginIdl, package: &str) -> Result<String, String> {
    Ok(bindings::kotlin(
        &BridgePlan::validate_kotlin_contract(contract)?,
        package,
    ))
}

/// Emits pure C++ specification headers from the plugin IDL contract.
pub fn render_cpp(contract: &PluginIdl, plugin_id: &str) -> Result<String, String> {
    Ok(bindings_cpp::render(
        &BridgePlan::validate_contract(contract)?,
        plugin_id,
    ))
}

/// Emits Swift-to-C++ bridging adapters.
pub fn render_swift_adapters(contract: &PluginIdl, plugin_id: &str) -> Result<String, String> {
    bindings_cpp::render_swift_adapters(&BridgePlan::validate_swift_cpp(contract)?, plugin_id)
}

/// Emits Android JNI C++ adapters and Kotlin JNI external function bindings.
pub fn render_android_adapters(
    contract: &PluginIdl,
    plugin_id: &str,
    plugin_namespace: &str,
    package: &str,
    plugin_index: usize,
) -> Result<(String, String), String> {
    bindings_cpp::render_android_adapters(
        &BridgePlan::validate_android(contract)?,
        plugin_id,
        plugin_namespace,
        package,
        plugin_index,
    )
}
