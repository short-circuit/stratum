pub mod auto_commit;
pub mod conflict;
pub mod git;
pub mod scheduler;

// Re-export key types at crate level for convenience.
pub use auto_commit::AutoCommitEngine;
pub use conflict::{ConflictFile, ConflictHunk};
pub use git::{CommitInfo, GitEngine, PullResult};
pub use scheduler::{SyncResult, SyncScheduler};
