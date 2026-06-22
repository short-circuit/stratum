//! Command handlers for the Tauri IPC bridge.
//!
//! These are the Rust functions callable from the React frontend via `invoke()`.

pub mod block;
pub mod page;
pub mod query;
pub mod search;
pub mod sync;
pub mod vault;
