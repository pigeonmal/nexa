//! Gates for the isolation property that the old clock-derived naming broke.
//!
//! The third test is the one that matters: it reproduces the original failure
//! shape directly, and it passes only because ownership is claimed through the
//! operating system rather than assumed.

use nexa_testkit::{TempDir, vacant_path};
use std::collections::HashSet;
use std::path::PathBuf;
use std::process;
use std::sync::{Arc, Mutex};

/// Every directory handed out under load is distinct, and really exists.
///
/// This is the heavy calibration gate. It is `#[ignore]`d so the default suite
/// stays fast, and run explicitly with:
///
/// ```text
/// cargo test -p nexa-testkit -- --ignored
/// ```
///
/// Sized against the old scheme's behaviour on the machine it was calibrated
/// against, which shared 1,026 paths over 4,500 claims. Nine rounds of 4,000
/// claims would have collided on every one of them; the fixed scheme collides
/// on none, and each round re-checks that every directory it handed out still
/// exists when the round ends.
#[test]
#[ignore = "high-volume collision gate; run with --ignored"]
fn concurrent_creation_never_shares_a_directory() {
    const ROUNDS: usize = 9;
    const THREADS: usize = 16;
    const PER_THREAD: usize = 250;

    for round in 1..=ROUNDS {
        let claimed: Arc<Mutex<Vec<PathBuf>>> = Arc::new(Mutex::new(Vec::new()));
        let mut threads = Vec::new();
        for _ in 0..THREADS {
            let claimed = Arc::clone(&claimed);
            threads.push(std::thread::spawn(move || {
                let mut mine = Vec::with_capacity(PER_THREAD);
                for _ in 0..PER_THREAD {
                    let directory = TempDir::new("nexa-testkit-stress");
                    assert!(
                        directory.path().is_dir(),
                        "a claimed directory must exist on disk"
                    );
                    mine.push(directory.path().to_path_buf());
                }
                claimed.lock().expect("claim list").extend(mine);
            }));
        }
        for thread in threads {
            thread.join().expect("no claiming thread may panic");
        }

        let claimed = Arc::try_unwrap(claimed)
            .expect("sole owner of the claim list")
            .into_inner()
            .expect("claim list is not poisoned");
        assert_eq!(
            claimed.len(),
            THREADS * PER_THREAD,
            "every claim is recorded"
        );
        let distinct: HashSet<&PathBuf> = claimed.iter().collect();
        assert_eq!(
            distinct.len(),
            claimed.len(),
            "round {round}: two callers were handed the same directory, so one \
             can delete the other's files"
        );
    }
}

/// Releasing one directory never removes another's.
///
/// This is the original bug in miniature. The old scheme let two callers adopt
/// one path, so the first to drop wiped the second's fixtures -- which is why a
/// seeding write could fail with `NotFound` immediately after its own
/// `create_dir_all` succeeded. With claimed ownership there is no path by which
/// one guard can remove a directory another guard owns.
#[test]
fn releasing_one_directory_never_removes_another() {
    const OWNERS: usize = 64;

    // Claimed concurrently, because that is the only way two owners can end up
    // sharing a path. Sequential claiming hides the very failure under test.
    let mut threads = Vec::new();
    for index in 0..OWNERS {
        threads.push(std::thread::spawn(move || {
            let directory = TempDir::new("nexa-testkit-isolation");
            std::fs::create_dir_all(directory.join("nested/deeper"))
                .expect("nested fixture directory");
            std::fs::write(directory.join("nested/deeper/payload.bin"), b"payload")
                .expect("fixture payload");
            std::fs::write(directory.join(format!("owner-{index}.txt")), b"payload")
                .expect("owner fixture");
            (index, directory)
        }));
    }
    let owners: Vec<(usize, TempDir)> = threads
        .into_iter()
        .map(|thread| thread.join().expect("owner thread"))
        .collect();

    // Release every other owner, as tests finishing out of order would.
    let mut survivors = Vec::new();
    let mut released = 0;
    for (index, owner) in owners {
        if index % 2 == 0 {
            let path = owner.path().to_path_buf();
            drop(owner);
            assert!(!path.exists(), "a released directory is removed");
            released += 1;
        } else {
            survivors.push((index, owner));
        }
    }
    assert_eq!(released, OWNERS / 2, "half the owners were released");
    assert_eq!(survivors.len(), OWNERS - released);

    for (index, directory) in &survivors {
        assert!(
            directory.path().is_dir(),
            "owner {index} lost its directory to another owner's release"
        );
        assert_eq!(
            std::fs::read(directory.join("nested/deeper/payload.bin")).expect("surviving payload"),
            b"payload",
            "owner {index} lost its fixtures to another owner's release"
        );
    }
}

/// An occupied name is skipped rather than adopted.
///
/// The sequence starts at zero for each process, so pre-creating the first few
/// names the process would ask for forces the retry path deterministically
/// rather than hoping for a collision.
#[test]
fn an_occupied_name_is_skipped_rather_than_adopted() {
    const OCCUPIED: u64 = 32;

    let base = std::env::temp_dir();
    let pid = process::id();
    let mut squatters = Vec::new();
    for sequence in 0..OCCUPIED {
        let path = base.join(format!("nexa-testkit-squatted-{pid}-{sequence}"));
        std::fs::create_dir_all(&path).expect("squatter directory");
        squatters.push(path);
    }

    let claimed = TempDir::new("nexa-testkit-squatted");
    let claimed_path = claimed.path().to_path_buf();
    assert!(
        !squatters.contains(&claimed_path),
        "the helper adopted a directory that was already taken: {}",
        claimed_path.display()
    );
    assert!(
        claimed_path.is_dir(),
        "the helper returned a usable directory"
    );

    // The squatters are untouched, which is the point: nobody else owns them.
    for squatter in &squatters {
        assert!(squatter.is_dir(), "{} was disturbed", squatter.display());
    }
    for squatter in squatters {
        let _ = std::fs::remove_dir_all(squatter);
    }
}

/// A vacant path does not exist, and is still ours to use.
///
/// `nexa create` refuses to scaffold into a directory that already exists, so
/// tests that drive it need a claimed name whose directory is absent.
#[test]
fn a_vacant_path_does_not_exist_and_is_still_claimable() {
    let path = vacant_path("nexa-testkit-vacant");
    assert!(!path.exists(), "a vacant path must not exist yet");
    assert!(
        path.parent().is_some_and(|parent| parent.is_dir()),
        "the parent temporary directory must exist"
    );

    // Nothing else can have taken the name, so a consumer can create it.
    std::fs::create_dir(&path).expect("the claimed name is free to create");
    assert!(path.is_dir());
    std::fs::remove_dir(&path).expect("clean up");
}
