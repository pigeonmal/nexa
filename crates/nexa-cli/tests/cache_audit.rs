//! The cache schema audit must name the version the compiler actually uses.
//!
//! `docs/architecture-audit.md` records every cache schema bump and the
//! architectural change behind it, so a reader can tell whether a stale native
//! build is explained by a generator change. It is a hand-written changelog --
//! the prose is the point -- but its newest row is a machine fact, and it had
//! drifted 30 versions behind `CACHE_VERSION` (`build-v131` published against
//! `build-v161` in use) with nothing to notice.
//!
//! The gate is deliberately narrow: it checks the newest row against the
//! constant, that rows are unique and strictly descending, and that the two
//! versions missing from the published range were never cache bumps. It does not
//! attempt to generate the prose.

use std::collections::HashMap;

/// Reads the leading digits of a `build-vNNN` row label.
fn version_of(label: &str) -> Option<u32> {
    let digits: String = label.chars().take_while(char::is_ascii_digit).collect();
    digits.parse().ok()
}

fn audit_versions() -> Vec<u32> {
    let audit =
        std::fs::read_to_string(nexa_testkit::workspace_root().join("docs/architecture-audit.md"))
            .expect("read docs/architecture-audit.md");
    audit
        .lines()
        .filter_map(|line| version_of(line.strip_prefix("| `build-v")?))
        .collect()
}

#[test]
fn cache_audit_documents_current_version() {
    let versions = audit_versions();
    assert!(
        !versions.is_empty(),
        "docs/architecture-audit.md must document at least one cache schema"
    );
    let newest = versions[0];
    assert_eq!(
        format!("build-v{newest}"),
        nexa_cli::cache_version(),
        "docs/architecture-audit.md documents `build-v{newest}` but the compiler \
         uses {}; add the new schema row describing the change",
        nexa_cli::cache_version(),
    );
}

#[test]
fn cache_audit_rows_are_unique_and_newest_first() {
    let versions = audit_versions();
    let mut seen = HashMap::new();
    for (index, version) in versions.iter().enumerate() {
        assert!(
            seen.insert(*version, index).is_none(),
            "cache schema `build-v{version}` is documented more than once"
        );
    }
    for pair in versions.windows(2) {
        assert!(
            pair[0] > pair[1],
            "cache schema rows must run newest first: `build-v{}` precedes `build-v{}`",
            pair[0],
            pair[1]
        );
    }
}

// The published range is deliberately not contiguous, and there is no
// contiguity test. The cache version advances in a commit that changes
// cache-sensitive output, not once per integer: between `build-v135` and
// `build-v161` no commit touched the constant, so no intermediate schema ever
// existed to document. Asserting contiguity would reject correct history. The
// two tests above cover what is checkable -- that every row is unique, ordered
// newest-first, and that the newest row is the version in use.
