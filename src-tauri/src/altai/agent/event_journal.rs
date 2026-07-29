//! Thin re-export of the shared, Tauri-independent event journal.
//!
//! The durable SQLite journal implementation lives in `altai-core` so the
//! ALTAI CLI can append to the exact same store. Desktop code keeps
//! importing from this module path; only the implementation moved.

pub use altai_core::journal::*;
