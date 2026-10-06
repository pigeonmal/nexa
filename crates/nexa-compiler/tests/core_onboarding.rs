//! The generic native primitives needed for app-authored onboarding stay in
//! the framework: paging, persisted state, async permission requests, and UI.

use std::fs;

use nexa_compiler::{Target, compile_file_with_warnings_for_target};
use nexa_ir::Node;
use nexa_testkit::TestProject;

#[test]
fn onboarding_example_compiles_for_both_targets_with_dedicated_paging_and_permissions() {
    std::thread::Builder::new()
        .name("nexa-onboarding-compile-test".to_owned())
        .stack_size(8 * 1024 * 1024)
        .spawn(onboarding_example_compiles_for_both_targets)
        .expect("spawn onboarding compile test with compiler-sized stack")
        .join()
        .expect("onboarding compile test should finish");
}

fn onboarding_example_compiles_for_both_targets() {
    let project = TestProject::new("nexa-onboarding");
    let entry = project.join("App.nx");
    fs::write(&entry, include_str!("../../../examples/onboarding.nx"))
        .expect("test source should be written");

    for target in [Target::Swift, Target::Kotlin] {
        let module = compile_file_with_warnings_for_target(&entry, target)
            .expect("the onboarding example should compile for both targets")
            .module;
        assert!(
            module
                .states
                .iter()
                .any(|state| state.name == "hasCompletedOnboarding")
        );
        let mut has_page_pager = false;
        let mut has_centered_copy = false;
        let mut has_permission_request = false;
        let mut visit_node = |node: &Node| {
            has_page_pager |= matches!(node, Node::PagePager { pages, .. } if pages.len() == 3);
            has_centered_copy |= matches!(
                node,
                Node::Text {
                    style: nexa_ir::TextStyle {
                        alignment: Some(nexa_ir::TextAlignment::Center),
                        ..
                    },
                    ..
                }
            );
        };
        let mut visit_expr = |expression: &nexa_ir::Expr| {
            has_permission_request |= matches!(
                expression,
                nexa_ir::Expr::PermissionOp {
                    op: nexa_ir::PermissionOpKind::Request,
                    ..
                }
            );
        };
        nexa_ir::walk::walk_ir(&module.body, &mut visit_node, &mut visit_expr);
        assert!(
            has_page_pager,
            "example should lower PagePager to paging IR"
        );
        assert!(
            has_centered_copy,
            "onboarding copy should center on both targets"
        );
        assert!(
            has_permission_request,
            "notification permission stays a core API"
        );
    }
}
