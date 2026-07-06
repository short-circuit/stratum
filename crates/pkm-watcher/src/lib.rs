//! File system watcher — monitors vault for on-disk changes.
//!
//! On Android this module is a no-op — inotify does not work reliably
//! in Android's process model. Users should trigger a manual reindex
//! after making external changes.

#[cfg(not(target_os = "android"))]
pub mod watcher;

#[cfg(not(target_os = "android"))]
pub use watcher::*;

#[cfg(target_os = "android")]
pub mod watcher {
    use pkm_core::FileEvent;
    use std::path::PathBuf;
    use std::time::SystemTime;

    #[derive(Debug, Clone)]
    pub struct FileChangeEvent {
        pub path: PathBuf,
        pub kind: FileEvent,
        pub timestamp: SystemTime,
    }

    pub struct FileWatcher;

    impl FileWatcher {
        pub fn new(
            _vault_path: PathBuf,
            _debounce_ms: u64,
            _on_event: Box<dyn Fn(FileChangeEvent) + Send + 'static>,
        ) -> Self {
            Self
        }

        pub fn start(&mut self) -> Result<(), Box<dyn std::error::Error>> {
            eprintln!("[pkm-watcher] file watching not available on Android");
            Ok(())
        }

        pub fn stop(&mut self) {}
    }
}
