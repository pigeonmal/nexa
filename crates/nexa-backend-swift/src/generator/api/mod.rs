//! Optional native API emitters.
//!
//! Each API owns the native helper it emits. The parent generator only decides
//! whether the API is reachable from the optimized IR.

pub(crate) mod clipboard;
pub(crate) mod crypto;
pub(crate) mod json;
pub(crate) mod network;
pub(crate) mod number;
pub(crate) mod permissions;
pub(crate) mod secure_storage;
pub(crate) mod time;
