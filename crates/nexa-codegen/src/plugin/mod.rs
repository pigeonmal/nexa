//! Code generation for native plugin contracts across Swift, Kotlin, and C++.

pub mod bindings;
pub mod bindings_cpp;

use nexa_plugin_idl::PluginIdl;

/// Emits Swift binding protocols and structs from the plugin IDL contract.
pub fn render_swift(contract: &PluginIdl) -> String {
    bindings::swift(contract)
}

/// Emits Kotlin binding interfaces and data classes from the plugin IDL contract.
pub fn render_kotlin(contract: &PluginIdl, package: &str) -> String {
    bindings::kotlin(contract, package)
}

/// Emits pure C++ specification headers from the plugin IDL contract.
pub fn render_cpp(contract: &PluginIdl, plugin_id: &str) -> String {
    bindings_cpp::render(contract, plugin_id)
}

/// Emits Swift-to-C++ bridging adapters.
pub fn render_swift_adapters(contract: &PluginIdl, plugin_id: &str) -> Result<String, String> {
    bindings_cpp::render_swift_adapters(contract, plugin_id)
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
        contract,
        plugin_id,
        plugin_namespace,
        package,
        plugin_index,
    )
}
