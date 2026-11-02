use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::{Expr, FunctionParameter, NavigationTitleAlignment, NavigationTitleMode, ScreenId};

/// Stable identifier for a route destination, independent of ScreenId index or file order.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct RouteKey(pub String);

impl RouteKey {
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for RouteKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Authoritative identifier for a navigation container (e.g. app root stack, tab stack).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct NavigatorId(pub String);

impl NavigatorId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn root() -> Self {
        Self("app/root".to_string())
    }

    pub fn tab(index: i32) -> Self {
        Self(format!("tab/{index}"))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for NavigatorId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Unique instance identifier for a pushed route entry on a navigation stack.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct EntryId(pub String);

impl EntryId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for EntryId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum NavigatorPresentation {
    #[default]
    Stack,
    Tab,
    Split,
    Modal,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum NavigationTransitionPolicy {
    #[default]
    Standard,
    None,
    Slide,
    Modal,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct NavigationChromePolicy {
    pub show_navigation_bar: bool,
    pub title_mode: NavigationTitleMode,
    pub title_alignment: NavigationTitleAlignment,
}

/// Declaration of a navigation route in the canonical navigation graph.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RouteDecl {
    pub key: RouteKey,
    pub screen_id: ScreenId,
    pub screen_name: String,
    pub parameters: Vec<FunctionParameter>,
    pub schema_version: u32,
}

/// Declaration of a navigator container in the canonical navigation graph.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NavigatorDecl {
    pub id: NavigatorId,
    pub root_route: RouteKey,
    pub presentation: NavigatorPresentation,
    pub chrome: NavigationChromePolicy,
    pub transitions: NavigationTransitionPolicy,
}

/// Canonical navigation graph linking routes and navigators.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct NavigationGraph {
    pub routes: Vec<RouteDecl>,
    pub navigators: Vec<NavigatorDecl>,
}

impl NavigationGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn find_route(&self, key: &RouteKey) -> Option<&RouteDecl> {
        self.routes.iter().find(|r| &r.key == key)
    }

    pub fn find_navigator(&self, id: &NavigatorId) -> Option<&NavigatorDecl> {
        self.navigators.iter().find(|n| &n.id == id)
    }
}

impl crate::Module {
    /// Builds the canonical Navigation v2 graph for this module.
    pub fn navigation_graph(&self) -> NavigationGraph {
        let mut routes = Vec::with_capacity(self.screens.len());
        for screen in &self.screens {
            routes.push(RouteDecl {
                key: RouteKey::new(&screen.name),
                screen_id: screen.id,
                screen_name: screen.name.clone(),
                parameters: screen.parameters.clone(),
                schema_version: 1,
            });
        }

        let mut navigators = Vec::new();
        collect_navigators(&self.body, &self.screens, &mut navigators);

        NavigationGraph { routes, navigators }
    }
}

fn collect_navigators(
    nodes: &[crate::Node],
    screens: &[crate::Screen],
    navigators: &mut Vec<NavigatorDecl>,
) {
    for node in nodes {
        match node {
            crate::Node::NavigationStack {
                root,
                show_navigation_bar,
                title_mode,
                title_alignment,
                ..
            } => {
                let root_name = screens
                    .get(root.0)
                    .map(|s| s.name.as_str())
                    .unwrap_or("Root");
                navigators.push(NavigatorDecl {
                    id: if navigators.is_empty() {
                        NavigatorId::root()
                    } else {
                        NavigatorId::new(format!("stack/{}", navigators.len()))
                    },
                    root_route: RouteKey::new(root_name),
                    presentation: NavigatorPresentation::Stack,
                    chrome: NavigationChromePolicy {
                        show_navigation_bar: *show_navigation_bar,
                        title_mode: *title_mode,
                        title_alignment: *title_alignment,
                    },
                    transitions: NavigationTransitionPolicy::Standard,
                });
            }
            crate::Node::AppBottomBar { tabs, .. } => {
                for tab in tabs {
                    for tab_node in &tab.children {
                        if let crate::Node::NavigationStack {
                            root,
                            show_navigation_bar,
                            title_mode,
                            title_alignment,
                            ..
                        } = tab_node
                        {
                            let root_name = screens
                                .get(root.0)
                                .map(|s| s.name.as_str())
                                .unwrap_or("Root");
                            navigators.push(NavigatorDecl {
                                id: NavigatorId::tab(tab.index),
                                root_route: RouteKey::new(root_name),
                                presentation: NavigatorPresentation::Tab,
                                chrome: NavigationChromePolicy {
                                    show_navigation_bar: *show_navigation_bar,
                                    title_mode: *title_mode,
                                    title_alignment: *title_alignment,
                                },
                                transitions: NavigationTransitionPolicy::Standard,
                            });
                        }
                    }
                }
            }
            crate::Node::If {
                then_body,
                else_body,
                ..
            } => {
                collect_navigators(then_body, screens, navigators);
                if let Some(else_body) = else_body {
                    collect_navigators(else_body, screens, navigators);
                }
            }
            crate::Node::When {
                cases, else_body, ..
            } => {
                for case in cases {
                    collect_navigators(&case.body, screens, navigators);
                }
                if let Some(else_body) = else_body {
                    collect_navigators(else_body, screens, navigators);
                }
            }
            _ => {}
        }
    }
}

/// Canonical atomic navigation commands.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum NavigationCommand {
    Push {
        navigator: NavigatorId,
        route: RouteKey,
        arguments: Vec<Expr>,
    },
    Pop {
        navigator: NavigatorId,
    },
    PopTo {
        navigator: NavigatorId,
        entry: EntryId,
        inclusive: bool,
    },
    ReplaceTop {
        navigator: NavigatorId,
        route: RouteKey,
        arguments: Vec<Expr>,
    },
    Reset {
        navigator: NavigatorId,
        route: RouteKey,
        arguments: Vec<Expr>,
    },
    SelectTab {
        navigator: NavigatorId,
    },
    OpenExternalRoute {
        uri: String,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NavigationEntryState {
    pub entry_id: EntryId,
    pub route: RouteKey,
    pub arguments: Vec<Expr>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NavigatorStackState {
    pub id: NavigatorId,
    pub root_route: RouteKey,
    pub entries: Vec<NavigationEntryState>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NavigationSnapshot {
    pub graph_version: u32,
    pub active_navigator: NavigatorId,
    pub navigators: BTreeMap<NavigatorId, NavigatorStackState>,
}

impl NavigationSnapshot {
    pub fn from_graph(graph: &NavigationGraph) -> Self {
        let mut navigators = BTreeMap::new();
        for nav in &graph.navigators {
            navigators.insert(
                nav.id.clone(),
                NavigatorStackState {
                    id: nav.id.clone(),
                    root_route: nav.root_route.clone(),
                    entries: vec![NavigationEntryState {
                        entry_id: EntryId::new(format!("{}/entry_root", nav.id)),
                        route: nav.root_route.clone(),
                        arguments: vec![],
                    }],
                },
            );
        }
        let active_navigator = graph
            .navigators
            .first()
            .map(|n| n.id.clone())
            .unwrap_or_else(NavigatorId::root);

        Self {
            graph_version: 1,
            active_navigator,
            navigators,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NavigationTransition {
    pub command: NavigationCommand,
    pub affected_navigator: NavigatorId,
    pub previous_top: Option<EntryId>,
    pub new_top: Option<EntryId>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NavigationError {
    NavigatorNotFound(NavigatorId),
    EntryNotFound(EntryId),
    IllegalRootPop(NavigatorId),
    RouteNotFound(RouteKey),
    EmptyStack(NavigatorId),
    InvalidUri(String),
}

impl std::fmt::Display for NavigationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NavigatorNotFound(id) => write!(f, "navigator `{id}` not found"),
            Self::EntryNotFound(id) => write!(f, "entry `{id}` not found on stack"),
            Self::IllegalRootPop(id) => write!(f, "cannot pop root entry of navigator `{id}`"),
            Self::RouteNotFound(key) => write!(f, "route `{key}` not found"),
            Self::EmptyStack(id) => write!(f, "navigator `{id}` has an empty stack"),
            Self::InvalidUri(uri) => write!(f, "invalid external URI `{uri}`"),
        }
    }
}

impl std::error::Error for NavigationError {}

/// Pure navigation reducer that executes a command against a snapshot.
pub fn reduce_navigation_command(
    snapshot: &mut NavigationSnapshot,
    command: NavigationCommand,
    next_entry_id: &mut impl FnMut() -> EntryId,
) -> Result<NavigationTransition, NavigationError> {
    match command {
        NavigationCommand::Push {
            navigator,
            route,
            arguments,
        } => {
            let stack = snapshot
                .navigators
                .get_mut(&navigator)
                .ok_or_else(|| NavigationError::NavigatorNotFound(navigator.clone()))?;
            let previous_top = stack.entries.last().map(|e| e.entry_id.clone());
            let entry_id = next_entry_id();
            stack.entries.push(NavigationEntryState {
                entry_id: entry_id.clone(),
                route: route.clone(),
                arguments: arguments.clone(),
            });
            Ok(NavigationTransition {
                command: NavigationCommand::Push {
                    navigator: navigator.clone(),
                    route,
                    arguments,
                },
                affected_navigator: navigator,
                previous_top,
                new_top: Some(entry_id),
            })
        }
        NavigationCommand::Pop { navigator } => {
            let stack = snapshot
                .navigators
                .get_mut(&navigator)
                .ok_or_else(|| NavigationError::NavigatorNotFound(navigator.clone()))?;
            if stack.entries.len() <= 1 {
                return Err(NavigationError::IllegalRootPop(navigator));
            }
            let previous_top = stack.entries.pop().map(|e| e.entry_id);
            let new_top = stack.entries.last().map(|e| e.entry_id.clone());
            Ok(NavigationTransition {
                command: NavigationCommand::Pop {
                    navigator: navigator.clone(),
                },
                affected_navigator: navigator,
                previous_top,
                new_top,
            })
        }
        NavigationCommand::PopTo {
            navigator,
            entry,
            inclusive,
        } => {
            let stack = snapshot
                .navigators
                .get_mut(&navigator)
                .ok_or_else(|| NavigationError::NavigatorNotFound(navigator.clone()))?;
            let target_idx = stack
                .entries
                .iter()
                .position(|e| e.entry_id == entry)
                .ok_or_else(|| NavigationError::EntryNotFound(entry.clone()))?;

            let previous_top = stack.entries.last().map(|e| e.entry_id.clone());
            let keep_len = if inclusive {
                target_idx
            } else {
                target_idx + 1
            };
            if keep_len < 1 {
                return Err(NavigationError::IllegalRootPop(navigator));
            }
            stack.entries.truncate(keep_len);
            let new_top = stack.entries.last().map(|e| e.entry_id.clone());
            Ok(NavigationTransition {
                command: NavigationCommand::PopTo {
                    navigator: navigator.clone(),
                    entry,
                    inclusive,
                },
                affected_navigator: navigator,
                previous_top,
                new_top,
            })
        }
        NavigationCommand::ReplaceTop {
            navigator,
            route,
            arguments,
        } => {
            let stack = snapshot
                .navigators
                .get_mut(&navigator)
                .ok_or_else(|| NavigationError::NavigatorNotFound(navigator.clone()))?;
            if stack.entries.is_empty() {
                return Err(NavigationError::EmptyStack(navigator));
            }
            let previous_top = stack.entries.pop().map(|e| e.entry_id);
            let entry_id = next_entry_id();
            stack.entries.push(NavigationEntryState {
                entry_id: entry_id.clone(),
                route: route.clone(),
                arguments: arguments.clone(),
            });
            Ok(NavigationTransition {
                command: NavigationCommand::ReplaceTop {
                    navigator: navigator.clone(),
                    route,
                    arguments,
                },
                affected_navigator: navigator,
                previous_top,
                new_top: Some(entry_id),
            })
        }
        NavigationCommand::Reset {
            navigator,
            route,
            arguments,
        } => {
            let stack = snapshot
                .navigators
                .get_mut(&navigator)
                .ok_or_else(|| NavigationError::NavigatorNotFound(navigator.clone()))?;
            let previous_top = stack.entries.last().map(|e| e.entry_id.clone());
            stack.entries.clear();
            let entry_id = next_entry_id();
            stack.entries.push(NavigationEntryState {
                entry_id: entry_id.clone(),
                route: route.clone(),
                arguments: arguments.clone(),
            });
            Ok(NavigationTransition {
                command: NavigationCommand::Reset {
                    navigator: navigator.clone(),
                    route,
                    arguments,
                },
                affected_navigator: navigator,
                previous_top,
                new_top: Some(entry_id),
            })
        }
        NavigationCommand::SelectTab { navigator } => {
            if !snapshot.navigators.contains_key(&navigator) {
                return Err(NavigationError::NavigatorNotFound(navigator));
            }
            let previous_top = snapshot
                .navigators
                .get(&snapshot.active_navigator)
                .and_then(|s| s.entries.last().map(|e| e.entry_id.clone()));
            snapshot.active_navigator = navigator.clone();
            let new_top = snapshot
                .navigators
                .get(&navigator)
                .and_then(|s| s.entries.last().map(|e| e.entry_id.clone()));
            Ok(NavigationTransition {
                command: NavigationCommand::SelectTab {
                    navigator: navigator.clone(),
                },
                affected_navigator: navigator,
                previous_top,
                new_top,
            })
        }
        NavigationCommand::OpenExternalRoute { uri } => {
            if uri.trim().is_empty() || !uri.contains(':') {
                return Err(NavigationError::InvalidUri(uri));
            }
            let active_id = snapshot.active_navigator.clone();
            let current_top = snapshot
                .navigators
                .get(&active_id)
                .and_then(|s| s.entries.last().map(|e| e.entry_id.clone()));
            Ok(NavigationTransition {
                command: NavigationCommand::OpenExternalRoute { uri },
                affected_navigator: active_id,
                previous_top: current_top.clone(),
                new_top: current_top,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_snapshot() -> NavigationSnapshot {
        let root_nav = NavigatorId::root();
        let tab_nav = NavigatorId::tab(1);
        let mut navigators = BTreeMap::new();
        navigators.insert(
            root_nav.clone(),
            NavigatorStackState {
                id: root_nav.clone(),
                root_route: RouteKey::new("Home"),
                entries: vec![NavigationEntryState {
                    entry_id: EntryId::new("entry_1"),
                    route: RouteKey::new("Home"),
                    arguments: vec![],
                }],
            },
        );
        navigators.insert(
            tab_nav.clone(),
            NavigatorStackState {
                id: tab_nav.clone(),
                root_route: RouteKey::new("Library"),
                entries: vec![NavigationEntryState {
                    entry_id: EntryId::new("entry_2"),
                    route: RouteKey::new("Library"),
                    arguments: vec![],
                }],
            },
        );

        NavigationSnapshot {
            graph_version: 2,
            active_navigator: root_nav,
            navigators,
        }
    }

    #[test]
    fn push_and_pop_invariants() {
        let mut snapshot = sample_snapshot();
        let root = NavigatorId::root();
        let mut seq = 3;
        let mut gen_id = || {
            let id = format!("entry_{seq}");
            seq += 1;
            EntryId::new(id)
        };

        // Push Details
        let tr1 = reduce_navigation_command(
            &mut snapshot,
            NavigationCommand::Push {
                navigator: root.clone(),
                route: RouteKey::new("Details"),
                arguments: vec![],
            },
            &mut gen_id,
        )
        .expect("push should succeed");

        assert_eq!(tr1.new_top, Some(EntryId::new("entry_3")));
        assert_eq!(snapshot.navigators[&root].entries.len(), 2);

        // Push duplicate Details(step: 2)
        let tr2 = reduce_navigation_command(
            &mut snapshot,
            NavigationCommand::Push {
                navigator: root.clone(),
                route: RouteKey::new("Details"),
                arguments: vec![],
            },
            &mut gen_id,
        )
        .expect("push duplicate should succeed");

        assert_eq!(tr2.new_top, Some(EntryId::new("entry_4")));
        assert_eq!(snapshot.navigators[&root].entries.len(), 3);

        // Pop once
        let tr_pop1 = reduce_navigation_command(
            &mut snapshot,
            NavigationCommand::Pop {
                navigator: root.clone(),
            },
            &mut gen_id,
        )
        .expect("pop should succeed");

        assert_eq!(tr_pop1.previous_top, Some(EntryId::new("entry_4")));
        assert_eq!(tr_pop1.new_top, Some(EntryId::new("entry_3")));
        assert_eq!(snapshot.navigators[&root].entries.len(), 2);

        // Pop again
        let tr_pop2 = reduce_navigation_command(
            &mut snapshot,
            NavigationCommand::Pop {
                navigator: root.clone(),
            },
            &mut gen_id,
        )
        .expect("pop should succeed");

        assert_eq!(tr_pop2.previous_top, Some(EntryId::new("entry_3")));
        assert_eq!(tr_pop2.new_top, Some(EntryId::new("entry_1")));
        assert_eq!(snapshot.navigators[&root].entries.len(), 1);

        // Root pop must fail
        let err = reduce_navigation_command(
            &mut snapshot,
            NavigationCommand::Pop {
                navigator: root.clone(),
            },
            &mut gen_id,
        );
        assert_eq!(err, Err(NavigationError::IllegalRootPop(root)));
    }

    #[test]
    fn select_tab_and_tab_stack_isolation() {
        let mut snapshot = sample_snapshot();
        let root = NavigatorId::root();
        let tab1 = NavigatorId::tab(1);
        let mut seq = 10;
        let mut gen_id = || {
            let id = format!("entry_{seq}");
            seq += 1;
            EntryId::new(id)
        };

        // Select tab 1
        reduce_navigation_command(
            &mut snapshot,
            NavigationCommand::SelectTab {
                navigator: tab1.clone(),
            },
            &mut gen_id,
        )
        .expect("select tab should succeed");
        assert_eq!(snapshot.active_navigator, tab1);

        // Push to tab 1
        reduce_navigation_command(
            &mut snapshot,
            NavigationCommand::Push {
                navigator: tab1.clone(),
                route: RouteKey::new("TabDetail"),
                arguments: vec![],
            },
            &mut gen_id,
        )
        .expect("push to tab should succeed");

        assert_eq!(snapshot.navigators[&tab1].entries.len(), 2);
        assert_eq!(snapshot.navigators[&root].entries.len(), 1);
    }

    #[test]
    fn open_external_route_does_not_mutate_stack_and_rejects_missing_scheme() {
        let root = NavigatorId::new("app");
        let mut snapshot = NavigationSnapshot {
            graph_version: 1,
            active_navigator: root.clone(),
            navigators: [(
                root.clone(),
                NavigatorStackState {
                    id: root.clone(),
                    root_route: RouteKey::new("Home"),
                    entries: vec![NavigationEntryState {
                        entry_id: EntryId::new("root_0"),
                        route: RouteKey::new("Home"),
                        arguments: vec![],
                    }],
                },
            )]
            .into_iter()
            .collect(),
        };

        let mut next_id = 0;
        let mut gen_id = move || {
            next_id += 1;
            EntryId::new(format!("entry_{next_id}"))
        };

        // Valid external URI produces transition with previous_top == new_top and stack length preserved
        let transition = reduce_navigation_command(
            &mut snapshot,
            NavigationCommand::OpenExternalRoute {
                uri: "https://example.com".to_owned(),
            },
            &mut gen_id,
        )
        .expect("valid uri should succeed");

        assert_eq!(transition.previous_top, Some(EntryId::new("root_0")));
        assert_eq!(transition.new_top, Some(EntryId::new("root_0")));
        assert_eq!(snapshot.navigators[&root].entries.len(), 1);

        // Invalid URI without scheme is rejected with InvalidUri
        let err = reduce_navigation_command(
            &mut snapshot,
            NavigationCommand::OpenExternalRoute {
                uri: "not_a_valid_url".to_owned(),
            },
            &mut gen_id,
        );
        assert!(matches!(err, Err(NavigationError::InvalidUri(_))));
    }
}
