//! Serializable development representation layered over Nexa's typed IR.
//!
//! Release backends continue to consume `nexa_ir::Module` directly. Dev IR
//! adds stable, deterministic identities for state and tree elements so a
//! connected native development renderer can replace the complete module and
//! preserve compatible values across later revisions.

use nexa_ir::{Module, Node};
use serde::{Deserialize, Serialize};

pub const DEV_IR_FORMAT_VERSION: u16 = 1;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DevModule {
    pub protocol_version: u16,
    pub revision: String,
    pub module: Module,
    pub identities: Vec<StableIdentity>,
}

/// A set of path-level edits to a DevModule's typed `Module` JSON.
/// The native runtime applies these only when it already has `base_revision`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DevModulePatch {
    pub protocol_version: u16,
    pub base_revision: String,
    pub revision: String,
    pub operations: Vec<JsonPatchOperation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identities: Option<Vec<StableIdentity>>,
}

/// One operation in the JSON-pointer subset used for development module
/// patches. Array entries are updated by index only when their array shape is
/// unchanged; structural changes replace the containing array.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JsonPatchKind {
    Set,
    Remove,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct JsonPatchOperation {
    pub op: JsonPatchKind,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<serde_json::Value>,
}

/// Build a deterministic patch between compatible Dev IR protocol versions.
/// Object values and same-length arrays are diffed recursively. An array whose
/// length changes is replaced as one unit to avoid shifting positional paths.
pub fn diff(previous: &DevModule, next: &DevModule) -> Option<DevModulePatch> {
    if previous.protocol_version != next.protocol_version
        || next.protocol_version != DEV_IR_FORMAT_VERSION
        || previous.revision == next.revision
    {
        return None;
    }
    let previous_fields = serde_json::to_value(&previous.module)
        .ok()?
        .as_object()?
        .clone();
    let next_fields = serde_json::to_value(&next.module)
        .ok()?
        .as_object()?
        .clone();
    let mut operations = Vec::new();
    diff_json(
        &serde_json::Value::Object(previous_fields),
        &serde_json::Value::Object(next_fields),
        "",
        &mut operations,
    );
    Some(DevModulePatch {
        protocol_version: next.protocol_version,
        base_revision: previous.revision.clone(),
        revision: next.revision.clone(),
        operations,
        identities: (previous.identities != next.identities).then(|| next.identities.clone()),
    })
}

fn diff_json(
    previous: &serde_json::Value,
    next: &serde_json::Value,
    path: &str,
    operations: &mut Vec<JsonPatchOperation>,
) {
    if previous == next {
        return;
    }
    match (previous, next) {
        (serde_json::Value::Object(previous), serde_json::Value::Object(next)) => {
            for name in previous.keys().filter(|name| !next.contains_key(*name)) {
                operations.push(JsonPatchOperation {
                    op: JsonPatchKind::Remove,
                    path: format!("{path}/{}", escape_pointer_segment(name)),
                    value: None,
                });
            }
            for (name, next_value) in next {
                let next_path = format!("{path}/{}", escape_pointer_segment(name));
                if let Some(previous_value) = previous.get(name) {
                    diff_json(previous_value, next_value, &next_path, operations);
                } else {
                    operations.push(JsonPatchOperation {
                        op: JsonPatchKind::Set,
                        path: next_path,
                        value: Some(next_value.clone()),
                    });
                }
            }
        }
        (serde_json::Value::Array(previous), serde_json::Value::Array(next))
            if previous.len() == next.len() =>
        {
            for (index, (previous_value, next_value)) in previous.iter().zip(next).enumerate() {
                diff_json(
                    previous_value,
                    next_value,
                    &format!("{path}/{index}"),
                    operations,
                );
            }
        }
        _ => operations.push(JsonPatchOperation {
            op: JsonPatchKind::Set,
            path: path.to_owned(),
            value: Some(next.clone()),
        }),
    }
}

fn escape_pointer_segment(segment: &str) -> String {
    segment.replace('~', "~0").replace('/', "~1")
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StableIdentity {
    pub id: String,
    pub kind: IdentityKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IdentityKind {
    AppState,
    Screen,
    ScreenState,
    Component,
    ComponentState,
    Node,
}

/// Lower a typed module into a full-replacement dev payload with stable,
/// readable identities. Identity paths use declaration names and per-kind
/// sibling positions, never vector offsets for declarations such as screens.
pub fn lower(module: &Module, revision: impl Into<String>) -> DevModule {
    let mut identities = Vec::new();
    for state in &module.states {
        identities.push(StableIdentity {
            id: format!("app/state/{}", escape_segment(&state.name)),
            kind: IdentityKind::AppState,
        });
    }
    collect_nodes(&mut identities, "app/body", &module.body);

    for screen in &module.screens {
        let screen_path = format!("screen/{}", escape_segment(&screen.name));
        identities.push(StableIdentity {
            id: screen_path.clone(),
            kind: IdentityKind::Screen,
        });
        for state in &screen.states {
            identities.push(StableIdentity {
                id: format!("{screen_path}/state/{}", escape_segment(&state.name)),
                kind: IdentityKind::ScreenState,
            });
        }
        collect_nodes(
            &mut identities,
            &format!("{screen_path}/body"),
            &screen.body,
        );
    }

    for component in &module.components {
        let component_path = format!("component/{}", escape_segment(&component.name));
        identities.push(StableIdentity {
            id: component_path.clone(),
            kind: IdentityKind::Component,
        });
        for state in &component.states {
            identities.push(StableIdentity {
                id: format!("{component_path}/state/{}", escape_segment(&state.name)),
                kind: IdentityKind::ComponentState,
            });
        }
        collect_nodes(
            &mut identities,
            &format!("{component_path}/body"),
            &component.body,
        );
    }

    DevModule {
        protocol_version: DEV_IR_FORMAT_VERSION,
        revision: revision.into(),
        module: module.clone(),
        identities,
    }
}

fn collect_nodes(identities: &mut Vec<StableIdentity>, parent: &str, nodes: &[Node]) {
    let mut ordinals = Vec::<(&'static str, usize)>::new();
    for node in nodes {
        let kind = node_kind(node);
        let ordinal = if let Some((_, count)) = ordinals.iter_mut().find(|(seen, _)| *seen == kind)
        {
            let ordinal = *count;
            *count += 1;
            ordinal
        } else {
            ordinals.push((kind, 1));
            0
        };
        let path = format!("{parent}/{kind}:{ordinal}");
        identities.push(StableIdentity {
            id: path.clone(),
            kind: IdentityKind::Node,
        });
        for (group, children) in node_child_groups(node) {
            collect_nodes(identities, &format!("{path}/{group}"), children);
        }
    }
}

fn node_kind(node: &Node) -> &'static str {
    match node {
        Node::StatusBar { .. } => "StatusBar",
        Node::Direction { .. } => "Direction",
        Node::OnAppear { .. } => "OnAppear",
        Node::OnDisappear { .. } => "OnDisappear",
        Node::OnActive { .. } => "OnActive",
        Node::OnInactive { .. } => "OnInactive",
        Node::OnBackground { .. } => "OnBackground",
        Node::Layout { kind, .. } => match kind {
            nexa_ir::LayoutKind::Column => "Column",
            nexa_ir::LayoutKind::Row => "Row",
            nexa_ir::LayoutKind::Stack => "Stack",
        },
        Node::Text { .. } => "Text",
        Node::Button { .. } => "Button",
        Node::TextInput { .. } => "TextInput",
        Node::Switch { .. } => "Switch",
        Node::Image { .. } => "Image",
        Node::Pressable { .. } => "Pressable",
        Node::NavigationStack { .. } => "NavigationStack",
        Node::NavigationLink { .. } => "NavigationLink",
        Node::NavigationBack { .. } => "NavigationBack",
        Node::Link { .. } => "Link",
        Node::Accessibility { .. } => "Accessibility",
        Node::KeyboardAware { .. } => "KeyboardAware",
        Node::BottomSheet { .. } => "BottomSheet",
        Node::RefreshControl { .. } => "RefreshControl",
        Node::AppBottomBar { .. } => "AppBottomBar",
        Node::FastList { .. } => "FastList",
        Node::If { .. } => "If",
        Node::When { .. } => "When",
        Node::Content => "Content",
        Node::ComponentCall { .. } => "ComponentCall",
        Node::NativeComponentCall { .. } => "NativeComponentCall",
    }
}

fn node_child_groups(node: &Node) -> Vec<(String, &[Node])> {
    match node {
        Node::Layout { children, .. }
        | Node::Pressable { children, .. }
        | Node::NavigationLink { children, .. }
        | Node::Link { children, .. }
        | Node::Accessibility { children, .. }
        | Node::KeyboardAware { children, .. }
        | Node::BottomSheet { children, .. }
        | Node::RefreshControl { children, .. } => vec![("body".to_owned(), children)],
        Node::FastList { plan } => {
            let mut groups = vec![("body".to_owned(), plan.children())];
            if let Some(nodes) = plan.sticky_header() {
                groups.push(("stickyHeader".to_owned(), nodes));
            }
            if let Some(nodes) = plan.section_header() {
                groups.push(("sectionHeader".to_owned(), nodes));
            }
            groups
        }
        Node::If {
            then_body,
            else_body,
            ..
        } => {
            let mut groups = vec![("then".to_owned(), then_body.as_slice())];
            if let Some(nodes) = else_body {
                groups.push(("else".to_owned(), nodes.as_slice()));
            }
            groups
        }
        Node::When {
            cases, else_body, ..
        } => {
            let mut groups = cases
                .iter()
                .enumerate()
                .map(|(index, case)| (format!("case:{index}"), case.body.as_slice()))
                .collect::<Vec<_>>();
            groups.push(("else".to_owned(), else_body.as_slice()));
            groups
        }
        Node::AppBottomBar { tabs, .. } => tabs
            .iter()
            .map(|tab| (format!("tab:{}", tab.index), tab.children.as_slice()))
            .collect(),
        Node::ComponentCall {
            children: Some(children),
            ..
        }
        | Node::NativeComponentCall {
            children: Some(children),
            ..
        } => vec![("content".to_owned(), children.as_slice())],
        _ => Vec::new(),
    }
}

fn escape_segment(value: &str) -> String {
    value
        .replace('%', "%25")
        .replace('/', "%2F")
        .replace(':', "%3A")
}
