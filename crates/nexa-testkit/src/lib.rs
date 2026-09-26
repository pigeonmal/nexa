//! Test-only support shared across the workspace.
//!
//! This crate is a `dev-dependency` and never reaches a shipped binary, so it
//! can depend on nothing and be depended on by everything without affecting the
//! layering between the real crates. It exists so that `nexa-cli` and
//! `nexa-compiler` can share one correct answer to a question both of them got
//! wrong independently.
//!
//! # Why a clock is not an identity
//!
//! The obvious way to name a temporary directory is the process id plus
//! `SystemTime::now().as_nanos()`. That is not unique. The system clock's
//! granularity is far coarser than a nanosecond -- on the machine this was
//! measured on, 320,000 clock reads produced only 40,838 distinct values, and
//! 35,377 of those were handed out more than once.
//!
//! Reproducing the old naming scheme under concurrency shows what that costs:
//!
//! ```text
//! 4,500 directory creations -> 1,470 distinct paths
//! 1,026 paths shared by two or more owners (worst case: 8)
//! ```
//!
//! A collision is silent, because `create_dir_all` on an existing directory is
//! a no-op rather than an error. Two tests then share one tree, and whichever
//! finishes first deletes the other's files on the way out -- surfacing as
//! `NotFound` on a write whose `create_dir_all` succeeded moments earlier, and
//! only under load. That is not a flaky test; it is two tests sharing a
//! directory.
//!
//! # The rule
//!
//! Ownership comes from the operating system, not from arithmetic. `create_dir`
//! is atomic and fails when a name is taken, so a caller can retry into a
//! different name instead of adopting one that already exists. A counter keeps
//! the process making progress, and the process id keeps concurrent processes
//! apart; a stale directory left by a crashed run is absorbed by the same
//! retry, because the name is claimed rather than assumed.

use std::path::{Path, PathBuf};
use std::process;
use std::sync::atomic::{AtomicU64, Ordering};

/// Hands out a distinct suffix to every caller within this process.
static SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// How many names to try before reporting that no name is free.
///
/// Reaching this means the prefix is unusable in this environment -- a
/// read-only or full `TMPDIR`, most likely -- rather than that a name was
/// briefly contested.
const ATTEMPTS: u32 = 1024;

/// A uniquely owned temporary directory, removed when dropped.
///
/// The guard is what makes the isolation hold: a directory is only ever removed
/// by the caller that successfully claimed it.
#[derive(Debug)]
pub struct TempDir {
    path: PathBuf,
}

impl TempDir {
    /// Claims a fresh directory named after `prefix`.
    ///
    /// # Panics
    ///
    /// Panics if the temporary directory cannot be created, or if
    /// [`ATTEMPTS`] names are all already taken.
    pub fn new(prefix: &str) -> Self {
        let base = std::env::temp_dir();
        let pid = process::id();
        for _ in 0..ATTEMPTS {
            // Read the sequence inside the loop so a retry is guaranteed to ask
            // for a different name, even if the clock never advances.
            let sequence = SEQUENCE.fetch_add(1, Ordering::Relaxed);
            let candidate = base.join(format!("{prefix}-{pid}-{sequence}"));
            match std::fs::create_dir(&candidate) {
                Ok(()) => return Self { path: candidate },
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => panic!("{}: {error}", candidate.display()),
            }
        }
        panic!("no unused temporary directory name available for `{prefix}`");
    }

    /// The directory's path.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Releases ownership without removing the directory.
    ///
    /// The caller becomes responsible for cleanup, which is what a test needs
    /// when it has to inspect the tree after the fact.
    pub fn keep(self) -> PathBuf {
        let path = self.path.clone();
        std::mem::forget(self);
        path
    }
}

/// Claims a name and then releases the directory, yielding a path that does not
/// exist yet.
///
/// Some consumers require the absence of a directory rather than its presence:
/// `nexa create` refuses to scaffold into a directory that already exists. A
/// [`TempDir`] cannot serve those, because claiming the name means creating it.
///
/// The name is still exclusively ours after the release. It is built from the
/// process id and a sequence number that this process never reuses, and
/// [`TempDir::new`] proved the name was free a moment earlier, so releasing the
/// directory cannot hand it to anyone else. Cleanup returns to the caller.
pub fn vacant_path(prefix: &str) -> PathBuf {
    let claimed = TempDir::new(prefix);
    let path = claimed.path().to_path_buf();
    std::fs::remove_dir(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
    debug_assert!(!path.exists(), "a vacant path must not exist");
    path
}

impl Drop for TempDir {
    fn drop(&mut self) {
        // A test that failed mid-way may have left a locked or unreadable file
        // behind, and that must not mask the original failure with a cleanup
        // error.
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

/// Lets call sites use a guard wherever they used a `Path`.
impl AsRef<Path> for TempDir {
    fn as_ref(&self) -> &Path {
        &self.path
    }
}

impl std::ops::Deref for TempDir {
    type Target = Path;

    fn deref(&self) -> &Path {
        &self.path
    }
}
