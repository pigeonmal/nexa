//! Native plugin binding generation shared by project scaffolding.
//!
//! Every entry point validates the IDL contract into a [`BridgePlan`] first
//! (`PluginIdl -> validate -> BridgePlan -> render`); rendering itself is
//! total over the plan and cannot panic on user input.

use nexa_codegen::plugin::{bindings, bindings_cpp, bridge_plan::BridgePlan};
use nexa_plugin_idl as idl;

/// Render the platform contract used by generated native projects.
pub(crate) fn render_swift_bindings(contract: &idl::PluginIdl) -> Result<String, String> {
    Ok(bindings::swift(&BridgePlan::validate_swift_contract(
        contract,
    )?))
}

pub(crate) fn render_kotlin_bindings(
    contract: &idl::PluginIdl,
    package: &str,
) -> Result<String, String> {
    Ok(bindings::kotlin(
        &BridgePlan::validate_kotlin_contract(contract)?,
        package,
    ))
}

pub(crate) fn render_cpp_bindings(
    contract: &idl::PluginIdl,
    plugin_id: &str,
) -> Result<String, String> {
    Ok(bindings_cpp::render(
        &BridgePlan::validate_contract(contract)?,
        plugin_id,
    ))
}

pub(crate) fn render_cpp_swift_adapters(
    contract: &idl::PluginIdl,
    plugin_id: &str,
) -> Result<String, String> {
    bindings_cpp::render_swift_adapters(&BridgePlan::validate_swift_cpp(contract)?, plugin_id)
}

pub(crate) fn render_cpp_android_adapters(
    contract: &idl::PluginIdl,
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
