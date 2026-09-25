use nexa_syntax::catalog;

// Every advertised component must be accepted by the parser. If a component
// is renamed or removed from the grammar, its probe fails here first —
// before stale completions or hover docs can reach users.
#[test]
fn every_catalog_component_probe_parses() {
    for entry in catalog::COMPONENTS {
        nexa_syntax::parse_program(entry.probe).unwrap_or_else(|error| {
            panic!(
                "catalog component `{}` probe failed to parse: {error}",
                entry.name
            )
        });
    }
}

#[test]
fn catalog_component_names_are_unique_and_identifier_shaped() {
    let mut seen = std::collections::HashSet::new();
    for entry in catalog::COMPONENTS {
        assert!(
            entry
                .name
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_alphabetic()),
            "component name should start with an ASCII letter: {}",
            entry.name
        );
        assert!(
            entry.name.chars().all(|c| c.is_ascii_alphanumeric()),
            "component name should be identifier-shaped: {}",
            entry.name
        );
        assert!(
            seen.insert(entry.name),
            "duplicate catalog component: {}",
            entry.name
        );
        assert!(!entry.summary.is_empty());
        assert!(!entry.snippet.is_empty());
    }
}

// Guards against the exact drift that motivated the catalog: names with no
// parser production must never be advertised by tooling. Note the parser
// accepts any `Name(...)` as a *custom* component call, so absence here is
// enforced together with semantic rejection in the `nexa-lsp` suite, which
// proves each of these names fails full compilation.
#[test]
fn catalog_advertises_no_unsupported_parser_names() {
    for rejected in [
        "TextField",
        "FastSectionedList",
        "Spacer",
        "Divider",
        "VStack",
        "HStack",
        "ZStack",
    ] {
        assert!(
            catalog::component(rejected).is_none(),
            "{rejected} has no parser production and must stay out of the catalog"
        );
    }
    assert!(catalog::component("TextInput").is_some());
}

#[test]
fn keyword_probe_covers_every_catalog_keyword() {
    nexa_syntax::parse_program(catalog::KEYWORD_PROBE).expect("keyword probe should parse");
    let probe = catalog::KEYWORD_PROBE;
    for entry in catalog::KEYWORDS {
        assert!(
            probe.contains(entry.name),
            "keyword probe should exercise `{}`",
            entry.name
        );
    }
}

#[test]
fn type_probe_covers_every_catalog_type() {
    nexa_syntax::parse_program(catalog::TYPE_PROBE).expect("type probe should parse");
    let probe = catalog::TYPE_PROBE;
    for entry in catalog::TYPES {
        assert!(
            probe.contains(entry.name),
            "type probe should exercise `{}`",
            entry.name
        );
    }
}

#[test]
fn dot_modifiers_match_parser_accepted_names() {
    // The parser only accepts these trailing modifiers (Pressable,
    // RefreshControl, FastList); the catalog must not offer others.
    let mut names: Vec<&str> = catalog::DOT_MODIFIERS
        .iter()
        .map(|entry| entry.name)
        .collect();
    names.sort_unstable();
    assert_eq!(
        names,
        [
            "onEndReached",
            "onLongPress",
            "onPress",
            "onRefresh",
            "onScroll",
            "sectionHeader",
            "stickyHeader"
        ]
    );
}

// ---------------------------------------------------------------------------
// Schema parity: every vocabulary entry has exactly one component schema and
// vice versa, so the parser, semantic lowering, and IDE tooling share one
// definition of each component's shape.
// ---------------------------------------------------------------------------

#[test]
fn component_schemas_cover_every_vocabulary_entry() {
    let vocabulary: std::collections::HashSet<&str> =
        catalog::COMPONENTS.iter().map(|entry| entry.name).collect();
    let schemas: std::collections::HashSet<&str> = catalog::COMPONENT_SCHEMAS
        .iter()
        .map(|schema| schema.name)
        .collect();
    assert_eq!(vocabulary, schemas, "vocabulary and schemas diverged");
    for schema in catalog::COMPONENT_SCHEMAS {
        assert!(
            !schema.doc.is_empty(),
            "{} needs a documentation key",
            schema.name
        );
        assert!(
            !schema.features.is_empty(),
            "{} needs a feature tag",
            schema.name
        );
        let mut options = std::collections::HashSet::new();
        for arg in schema.arguments {
            assert!(
                options.insert(arg.name),
                "duplicate option `{}` on {}",
                arg.name,
                schema.name
            );
        }
        for group in schema.exclusive {
            for option in group.options {
                assert!(
                    options.contains(option),
                    "exclusive option `{option}` is not a declared argument of {}",
                    schema.name
                );
            }
        }
        let mut modifiers = std::collections::HashSet::new();
        for modifier in schema.modifiers {
            assert!(
                modifiers.insert(modifier.name),
                "duplicate modifier `{}` on {}",
                modifier.name,
                schema.name
            );
            assert!(
                catalog::DOT_MODIFIERS
                    .iter()
                    .any(|entry| entry.name == modifier.name),
                "modifier `{}` of {} has no DOT_MODIFIERS vocabulary entry",
                modifier.name,
                schema.name
            );
        }
    }
    for entry in catalog::DOT_MODIFIERS {
        assert!(
            catalog::COMPONENT_SCHEMAS.iter().any(|schema| schema
                .modifiers
                .iter()
                .any(|modifier| modifier.name == entry.name)),
            "dot-modifier `{}` is accepted by no component schema",
            entry.name
        );
    }
}

#[test]
fn fast_list_source_grammar_is_catalogued() {
    let schema = catalog::component_schema("FastList").expect("FastList schema");
    let options: Vec<&str> = schema.arguments.iter().map(|arg| arg.name).collect();
    assert_eq!(options, ["axis", "rowHeight", "scrollPosition"]);
    assert_eq!(catalog::FASTLIST_SOURCE_KEYS, ["count", "sections"]);
    assert_eq!(catalog::FASTLIST_KEY_OPTION, "key");
}

// ---------------------------------------------------------------------------
// Generated snapshots: the VSCode TextMate grammar alternations and the
// syntax audit are rendered from the catalog. Tests assert byte equality;
// run with NEXA_UPDATE_SNAPSHOTS=1 to rewrite the checked-in files.
// ---------------------------------------------------------------------------

fn workspace_path(relative: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative)
}

fn maybe_rewrite(path: &std::path::Path, current: &str, generated: &str, what: &str) {
    if current == generated {
        return;
    }
    if std::env::var_os("NEXA_UPDATE_SNAPSHOTS").is_some() {
        std::fs::write(path, generated).expect("rewrite snapshot");
    } else {
        panic!("{what} drifted from the catalog; run with NEXA_UPDATE_SNAPSHOTS=1 to regenerate");
    }
}

/// Locate the `(a|b|c)` alternation group of a grammar rule's `match` line.
/// Returns the byte range of the group contents.
fn grammar_group_range(text: &str, rule: &str) -> (usize, usize) {
    let name_marker = format!("\"name\": \"{rule}\"");
    let name_at = text.find(&name_marker).expect("grammar rule name");
    let after = &text[name_at..];
    let match_marker = "\"match\": \"\\\\b(";
    let group_start =
        name_at + after.find(match_marker).expect("grammar rule match") + match_marker.len();
    let group_end = group_start
        + text[group_start..]
            .find(")\\\\b\"")
            .expect("alternation group end");
    (group_start, group_end)
}

fn grammar_rule_words(text: &str, rule: &str) -> Vec<String> {
    let (start, end) = grammar_group_range(text, rule);
    text[start..end].split('|').map(str::to_owned).collect()
}

#[test]
fn vscode_grammar_matches_catalog_vocabulary() {
    let path = workspace_path("../../editors/vscode/syntaxes/nexa.tmLanguage.json");
    let text = std::fs::read_to_string(&path).expect("read vscode grammar");
    let expected: Vec<(&str, Vec<&str>)> = vec![
        ("keyword.control.nexa", catalog::grammar_control_keywords()),
        (
            "keyword.declaration.nexa",
            catalog::grammar_declaration_keywords(),
        ),
        (
            "entity.name.type.primitive.nexa",
            catalog::grammar_type_names(),
        ),
        (
            "support.class.component.nexa",
            catalog::grammar_component_names(),
        ),
    ];
    let mut regenerated = text.clone();
    let mut dirty = false;
    for (rule, names) in &expected {
        let actual = grammar_rule_words(&regenerated, rule);
        let expected_words: Vec<String> = names.iter().map(|name| name.to_string()).collect();
        if actual != expected_words {
            let (start, end) = grammar_group_range(&regenerated, rule);
            regenerated.replace_range(start..end, &expected_words.join("|"));
            dirty = true;
        }
    }
    if dirty {
        maybe_rewrite(
            &path,
            &text,
            &regenerated,
            "editors/vscode/syntaxes/nexa.tmLanguage.json",
        );
    }
}

#[test]
fn syntax_audit_matches_catalog() {
    let path = workspace_path("../../docs/syntax-audit.md");
    let generated = catalog::render_syntax_audit();
    match std::fs::read_to_string(&path) {
        Ok(current) => maybe_rewrite(&path, &current, &generated, "docs/syntax-audit.md"),
        Err(_) => maybe_rewrite(&path, "", &generated, "docs/syntax-audit.md"),
    }
}
