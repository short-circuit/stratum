//! [`MobileServices`] backed by the vendored gpui-mobile packages.
//!
//! The mobile shell asks the host for capabilities it cannot provide itself:
//! microphone permission and dictation, a system file picker, notifications,
//! clipboard access and external URL opening. Every call routes through a
//! gpui-mobile package, which resolves the Java helper class from the running
//! Activity through JNI.
//!
//! Safety before the platform exists is a hard requirement: the shell may
//! request a capability before `android_main` finished booting (or in a test
//! harness). The gpui-mobile packages return `Err` when the JVM or Activity is
//! missing, so every method below propagates that error and never panics.

use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};

use gpui_mobile::packages::{
    clipboard, file_selector, microphone, notifications, permission_handler, url_launcher,
};
use stratum_ui::mobile::MobileServices;

/// The Android implementation of the shell's host capability seam.
///
/// The struct is stateless apart from the notification counter and the lazy
/// channel initialisation flag, so it can be installed once per process and
/// shared through an `Arc`.
pub struct AndroidServices {
    next_notification_id: AtomicI32,
    notifications_initialised: AtomicBool,
}

impl AndroidServices {
    /// Create the host services.
    pub fn new() -> Self {
        Self {
            next_notification_id: AtomicI32::new(1),
            notifications_initialised: AtomicBool::new(false),
        }
    }

    /// Create the default notification channel once per process.
    ///
    /// A failure is logged and ignored: `show` still attempts to post, and the
    /// Java helper falls back to its own channel when initialisation was not
    /// possible.
    fn ensure_notification_channel(&self) {
        if self.notifications_initialised.swap(true, Ordering::SeqCst) {
            return;
        }
        if let Err(error) = notifications::initialize() {
            log::warn!("services: notification channel unavailable: {error}");
        }
    }
}

impl Default for AndroidServices {
    fn default() -> Self {
        Self::new()
    }
}

impl MobileServices for AndroidServices {
    fn request_mic_permission(&self) -> Result<bool, String> {
        let status =
            permission_handler::request_permission(permission_handler::Permission::Microphone)
                .map_err(|error| format!("microphone permission request failed: {error}"))?;
        log::info!("services: microphone permission -> {status:?}");
        Ok(status.is_granted())
    }

    fn start_dictation(&self) -> Result<(), String> {
        let path = microphone::start_recording(&microphone::RecordingConfig::default())
            .map_err(|error| format!("failed to start dictation: {error}"))?;
        log::info!("services: dictation recording to {path}");
        Ok(())
    }

    fn stop_dictation(&self) -> Result<String, String> {
        let recording = microphone::stop_recording()
            .map_err(|error| format!("failed to stop dictation: {error}"))?;
        log::info!(
            "services: dictation finished after {} ms at {}",
            recording.duration_ms,
            recording.path
        );
        Ok(recording.path)
    }

    fn pick_file(&self) -> Result<Option<String>, String> {
        let options = file_selector::OpenFileOptions::default();
        let selected = file_selector::open_file(&options)
            .map_err(|error| format!("file picker failed: {error}"))?;
        match selected {
            Some(file) => {
                log::info!("services: picked {}", file.path);
                Ok(Some(file.path))
            }
            None => {
                log::info!("services: file picker cancelled");
                Ok(None)
            }
        }
    }

    fn notify(&self, title: &str, body: &str) -> Result<(), String> {
        self.ensure_notification_channel();
        let id = self.next_notification_id.fetch_add(1, Ordering::Relaxed);
        notifications::show(&notifications::Notification {
            id,
            title: title.to_string(),
            body: body.to_string(),
            channel: notifications::NotificationChannel::default(),
            payload: None,
        })
        .map_err(|error| format!("notification failed: {error}"))
    }

    fn set_clipboard(&self, text: &str) -> Result<(), String> {
        clipboard::set_text(text).map_err(|error| format!("clipboard write failed: {error}"))
    }

    fn open_external(&self, url: &str) -> Result<(), String> {
        match url_launcher::launch_url(url)
            .map_err(|error| format!("failed to open {url}: {error}"))?
        {
            true => Ok(()),
            false => Err(format!("no installed app handles {url}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notification_ids_start_at_one_and_increment() {
        let services = AndroidServices::new();
        assert_eq!(services.next_notification_id.load(Ordering::Relaxed), 1);
        let first = services
            .next_notification_id
            .fetch_add(1, Ordering::Relaxed);
        let second = services
            .next_notification_id
            .fetch_add(1, Ordering::Relaxed);
        assert_eq!((first, second), (1, 2));
    }

    #[test]
    fn channel_initialisation_is_recorded_once() {
        let services = AndroidServices::default();
        assert!(!services.notifications_initialised.load(Ordering::SeqCst));
        // The call itself is a no-op when no Activity exists; the flag still
        // flips so later notifications skip the attempt.
        services.ensure_notification_channel();
        assert!(services.notifications_initialised.load(Ordering::SeqCst));
    }
}
