use nexa_compiler::{Span, resolve_scroll_owner};
use nexa_ir::Node;
use nexa_ir::adaptive_layout::{LayoutEdge, ScrollOwnerId, ScrollOwnerKind};

fn form() -> Node {
    Node::Form {
        show_dividers: true,
        hide_horizontal_scrollbar: false,
        hide_vertical_scrollbar: false,
        background: None,
        children: Vec::new(),
    }
}

fn pager() -> Node {
    Node::PagePager {
        state: "page".to_owned(),
        pages: Vec::new(),
    }
}

#[test]
fn empty_or_axis_incompatible_scopes_need_no_scroll_adjustment() {
    assert!(
        resolve_scroll_owner(&[], LayoutEdge::Bottom, None, Span::default())
            .expect("no owner is valid")
            .is_none()
    );
    assert!(
        resolve_scroll_owner(&[pager()], LayoutEdge::Bottom, None, Span::default())
            .expect("horizontal owner is unrelated to a bottom inset")
            .is_none()
    );
}

#[test]
fn unique_relevant_scroll_owner_is_inferred() {
    let selected = resolve_scroll_owner(&[form()], LayoutEdge::Top, None, Span::default())
        .expect("single owner is unambiguous")
        .expect("Form owns vertical scrolling");
    assert_eq!(selected.kind, ScrollOwnerKind::Form);
    assert_eq!(selected.id, ScrollOwnerId(0));
}

#[test]
fn multiple_relevant_owners_require_an_explicit_typed_selection() {
    let span = Span {
        start: 14,
        end: 28,
        line: 2,
        column: 5,
    };
    let error = resolve_scroll_owner(&[form(), form()], LayoutEdge::Bottom, None, span)
        .expect_err("ambiguous ownership must be diagnosed");
    assert_eq!(error.span, span);
    assert!(error.message.contains("2 compatible scroll owners"));
    assert!(error.message.contains("typed ScrollRef"));
}

#[test]
fn explicit_selection_must_name_a_compatible_owner_in_the_scope() {
    let error = resolve_scroll_owner(
        &[form(), pager()],
        LayoutEdge::Top,
        Some(ScrollOwnerId(1)),
        Span::default(),
    )
    .expect_err("horizontal owner cannot serve a top-edge inset");
    assert!(
        error
            .message
            .contains("does not identify a scroll owner compatible")
    );

    let selected = resolve_scroll_owner(
        &[form(), form()],
        LayoutEdge::Top,
        Some(ScrollOwnerId(1)),
        Span::default(),
    )
    .expect("explicit owner resolves ambiguity")
    .expect("selected owner exists");
    assert_eq!(selected.id, ScrollOwnerId(1));
}
