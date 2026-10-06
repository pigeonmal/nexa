//! Adaptive master-detail navigation is a typed framework UI primitive.

use std::fs;

use nexa_compiler::{Target, compile_file_with_warnings_for_target};
use nexa_ir::Node;
use nexa_testkit::TestProject;

#[test]
fn split_view_lowers_both_panes_and_keeps_detail_state_live() {
    let project = TestProject::new("nexa-core-split-view");
    let entry = project.join("App.nx");
    fs::write(
        &entry,
        r#"app SplitExample {
    state showDetail: Bool = false
    body {
        NavigationSplitView(detailVisible: showDetail) {
            Sidebar { Text("Items") }
            Detail { Text("Selection") }
        }
    }
}
"#,
    )
    .expect("test source should be written");

    let compiled = compile_file_with_warnings_for_target(&entry, Target::Swift)
        .expect("split view should compile")
        .module;
    assert_eq!(compiled.states.len(), 1);
    assert_eq!(compiled.states[0].name, "showDetail");
    let Node::NavigationSplitView {
        detail_visible,
        sidebar,
        detail,
    } = &compiled.body[0]
    else {
        panic!("split view should lower to its dedicated IR node");
    };
    assert_eq!(detail_visible, "showDetail");
    assert_eq!(sidebar.len(), 1);
    assert_eq!(detail.len(), 1);
}
