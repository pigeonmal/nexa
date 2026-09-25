use nexa_syntax::catalog;

// Every advertised component must be accepted by the parser. If a component
// is renamed or removed from the grammar, its probe fails here first —
// before stale completions or hover docs can reach users.
#[test]
fn every_catalog_component_probe_parses() {
    for entry in catalog::COMPONENTS {
        nexa_syntax::parse_program(entry.probe)
            .unwrap_or_else(|error| panic!("catalog component `{}` probe failed to parse: {error}", entry.name));
    }
}

#[test]
fn catalog_component_names_are_unique_and_identifier_shaped() {
    let mut seen = std::collections::HashSet::new();
    for entry in catalog::COMPONENTS {
        assert!(
            entry.name.chars().next().is_some_and(|c| c.is_ascii_alphabetic()),
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
    let mut names: Vec<&str> = catalog::DOT_MODIFIERS.iter().map(|entry| entry.name).collect();
    names.sort_unstable();
    assert_eq!(
        names,
        ["onEndReached", "onLongPress", "onPress", "onRefresh", "onScroll", "sectionHeader", "stickyHeader"]
    );
}
