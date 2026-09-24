//! Native plugin binding generation shared by project scaffolding.

use nexa_codegen::plugin::{bindings, bindings_cpp};
use nexa_plugin_idl as idl;

/// Render the platform contract used by generated native projects.
pub(crate) fn render_swift_bindings(contract: &idl::PluginIdl) -> String {
    bindings::swift(contract)
}

pub(crate) fn render_kotlin_bindings(contract: &idl::PluginIdl, package: &str) -> String {
    bindings::kotlin(contract, package)
}

pub(crate) fn render_cpp_bindings(contract: &idl::PluginIdl, plugin_id: &str) -> String {
    bindings_cpp::render(contract, plugin_id)
}

pub(crate) fn render_cpp_swift_adapters(
    contract: &idl::PluginIdl,
    plugin_id: &str,
) -> Result<String, String> {
    bindings_cpp::render_swift_adapters(contract, plugin_id)
}

pub(crate) fn render_cpp_android_adapters(
    contract: &idl::PluginIdl,
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
