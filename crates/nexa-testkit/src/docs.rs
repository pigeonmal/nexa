//! Generated-documentation gates.
//!
//! `docs/syntax-audit.md` and the VSCode TextMate grammar are rendered from
//! `nexa-syntax`'s parser catalog and asserted byte for byte, which is what
//! keeps the accepted language and the published reference in agreement. That
//! mechanism used to live as a private helper inside one test file, so no other
//! crate could gate a generated document without copying it -- and every copy
//! was a chance to drift. It lives here instead.
//!
//! Two shapes are supported:
//!
//! * A **whole-file snapshot**, for a document that is entirely generated.
//! * A **fenced region**, for a document that is mostly prose but contains a
//!   table restating a machine-readable source of truth. The prose stays
//!   hand-written and editable; only the fenced block is rewritten.
//!
//! Both honour `NEXA_UPDATE_SNAPSHOTS=1` to rewrite the checked-in file in
//! place, and both panic with the regeneration command when they differ.

use std::path::Path;
use std::sync::{Mutex, MutexGuard, OnceLock};

/// Serializes read-modify-write cycles across every generated region in the
/// process.
///
/// Two tests that regenerate different regions of the *same* file race in a way
/// that silently loses one of them: both read the pre-rewrite text, both splice
/// in their own region, and the second write discards the first one's work. The
/// symptom is a region that stays empty after a regeneration run that reported
/// success. Tests run in parallel by default, so this is the normal case, not a
/// rare one.
fn region_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

/// Takes the region lock, recovering from poisoning.
///
/// A panic in one gate must not turn every later documentation test into a
/// confusing poison error; the next run re-checks the files from disk anyway.
fn lock_regions() -> MutexGuard<'static, ()> {
    region_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Opening marker for a generated region: `<!-- nexadoc:begin icons -->`.
pub fn begin_marker(name: &str) -> String {
    format!("<!-- nexadoc:begin {name} -->")
}

/// Closing marker for a generated region: `<!-- nexadoc:end icons -->`.
pub fn end_marker(name: &str) -> String {
    format!("<!-- nexadoc:end {name} -->")
}

/// Asserts that `path` contains exactly `generated`, rewriting it when
/// `NEXA_UPDATE_SNAPSHOTS` is set.
///
/// `what` names the artifact in the failure message, so a stale file points at
/// the generator that owns it rather than at this helper.
pub fn assert_snapshot(path: &Path, generated: &str, what: &str) {
    let _guard = lock_regions();
    let current = std::fs::read_to_string(path).unwrap_or_default();
    if current == generated {
        return;
    }
    rewrite_or_panic(path, &current, generated, what);
}

/// Asserts that the `name` region of `path` contains exactly `body`.
///
/// A missing marker is an error rather than something to insert silently: an
/// absent fence means the document was restructured without the generator, and
/// quietly appending a new table would leave the old hand-written one in place
/// next to it -- the exact duplication this exists to prevent.
pub fn assert_region(path: &Path, name: &str, body: &str, what: &str) {
    // Held across the read-modify-write, because a second region of the same
    // file would otherwise overwrite this one's result.
    let _guard = lock_regions();
    let current = std::fs::read_to_string(path).unwrap_or_else(|error| {
        panic!("failed to read {}: {error}", path.display());
    });
    let regenerated = match replace_region(&current, name, body) {
        Ok(regenerated) => regenerated,
        Err(missing) => panic!(
            "{what} has no `{name}` region: expected {} ... {missing} in {}. \
             Add the markers around the generated table; a generated region is \
             never inserted automatically, because that would leave a \
             hand-written copy of the same table in the file.",
            begin_marker(name),
            path.display(),
        ),
    };
    if current == regenerated {
        return;
    }
    rewrite_or_panic(path, &current, &regenerated, what);
}

/// Replaces the body between `name`'s markers, leaving every other byte of
/// `text` untouched.
///
/// Returns the name of the first marker that could not be found, so the caller
/// can report which half of the fence is missing.
pub fn replace_region(text: &str, name: &str, body: &str) -> Result<String, String> {
    let begin = begin_marker(name);
    let end = end_marker(name);
    let open = text
        .find(&begin)
        .ok_or_else(|| format!("`{begin}`"))?
        + begin.len();
    let close = text[open..]
        .find(&end)
        .ok_or_else(|| format!("`{end}`"))?
        + open;
    // The generated body owns its own trailing newline so that the closing
    // marker always starts a line, whatever the renderer's last line was.
    let mut out = String::with_capacity(text.len() + body.len());
    out.push_str(&text[..open]);
    out.push('\n');
    out.push_str(body.trim_end_matches('\n'));
    out.push('\n');
    out.push_str(&text[close..]);
    Ok(out)
}

fn rewrite_or_panic(path: &Path, current: &str, generated: &str, what: &str) {
    if std::env::var_os("NEXA_UPDATE_SNAPSHOTS").is_some() {
        std::fs::write(path, generated)
            .unwrap_or_else(|error| panic!("failed to rewrite {}: {error}", path.display()));
    } else {
        let first_difference = current
            .lines()
            .zip(generated.lines())
            .position(|(left, right)| left != right)
            .map(|index| index + 1)
            .unwrap_or_else(|| {
                current
                    .lines()
                    .count()
                    .min(generated.lines().count())
                    + 1
            });
        panic!(
            "{what} drifted from its source of truth at line {first_difference}; \
             run with NEXA_UPDATE_SNAPSHOTS=1 to regenerate"
        );
    }
}
