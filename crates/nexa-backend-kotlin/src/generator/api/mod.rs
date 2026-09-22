//! Optional native API emitters.
//!
//! Each API owns the native helper it emits. The parent generator only decides
//! whether the API is reachable from the optimized IR.

pub(crate) mod network;
pub(crate) mod permissions;
