//! C++ contract generation for native plugin contracts.
//!
//! All render paths consume a validated
//! [`BridgePlan`](super::bridge_plan::BridgePlan): types are already resolved
//! and proven mappable, so every mapping function here is total and code
//! generation cannot panic on user input.
//!
//! The plan is rendered through focused modules rather than one generator:
//!
//! - [`abi`] — the pure C++ specification header and the shared native
//!   vocabulary (type spellings, naming, literals).
//! - [`collections`] — `Array`/`Set`/`Map` facades and Swift collection
//!   bridges.
//! - [`errors`] — the `NexaResult` carrier and typed-error policy per target.
//! - [`swift`] — Swift-to-C++ adapters.
//! - [`jni`] — Android JNI adapters and their Kotlin declarations, with
//!   Android type spelling in [`jni::values`].

pub mod abi;
pub mod collections;
pub mod errors;
pub mod jni;
pub mod swift;
