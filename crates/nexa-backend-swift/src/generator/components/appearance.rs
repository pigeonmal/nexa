use nexa_ir::{Expr, Module, Node, ScreenId, walk::walk_ir};

/// Finds the root screen's appearance mode so native navigation destinations
/// inherit the same local color-scheme environment as the root screen.
pub(crate) fn module_mode(module: &Module, root: ScreenId) -> Option<Expr> {
    let root_screen = module.screens.iter().find(|screen| screen.id == root)?;
    let mut mode = None;
    walk_ir(
        &root_screen.body,
        &mut |node| {
            if mode.is_none()
                && let Node::Appearance {
                    mode: expression, ..
                } = node
            {
                mode = Some(expression.clone());
            }
        },
        &mut |_| {},
    );
    mode
}
