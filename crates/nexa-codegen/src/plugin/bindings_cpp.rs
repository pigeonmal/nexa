//! C++ implementation contracts generated from the shared native IDL.
//!
//! This module is a façade over [`cpp`], kept as the stable entry point for
//! the CLI, the language server, and the integration tests. The
//! implementation lives in [`cpp::abi`] (specification headers),
//! [`cpp::collections`] (collection facades), [`cpp::errors`] (typed errors
//! and the result carrier), [`cpp::swift`] (Swift adapters), and
//! [`cpp::jni`] (Android JNI adapters).

pub use super::cpp::abi::render;
pub use super::cpp::jni::render_android_adapters;
pub use super::cpp::swift::render_swift_adapters;
