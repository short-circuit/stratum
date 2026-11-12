//! Host seam for mobile operating-system services.
//!
//! The gpui shell owns the layout and the panel lifecycle, but microphone
//! permission, dictation capture, the file picker, notifications, clipboard
//! access and external links are host capabilities. [`MobileServices`] is the
//! seam the Android host implements (run C); [`NoopServices`] keeps desktop and
//! headless captures compiling without a host.
//!
//! Every method returns a `Result` so a denied permission or an unavailable
//! capability is reported to the UI instead of panicking. The trait is
//! object-safe on purpose: the shell stores it as `Arc<dyn MobileServices>`.

/// Host capabilities the mobile shell can request.
///
/// The default implementation ([`NoopServices`]) answers success without doing
/// anything, so a desktop capture still renders every mobile surface.
pub trait MobileServices: Send + Sync {
    /// Ask the host for microphone permission.
    ///
    /// Returns `Ok(true)` when the app may record, `Ok(false)` when the user
    /// declined and `Err` when the host could not ask.
    fn request_mic_permission(&self) -> Result<bool, String>;

    /// Start a dictation recording.
    fn start_dictation(&self) -> Result<(), String>;

    /// Stop the recording and return the recorded file path.
    fn stop_dictation(&self) -> Result<String, String>;

    /// Open the host file picker and return the chosen path.
    fn pick_file(&self) -> Result<Option<String>, String>;

    /// Post a local notification.
    fn notify(&self, title: &str, body: &str) -> Result<(), String>;

    /// Copy `text` to the system clipboard.
    fn set_clipboard(&self, text: &str) -> Result<(), String>;

    /// Open `url` in the platform browser (or the matching app).
    fn open_external(&self, url: &str) -> Result<(), String>;
}

/// A [`MobileServices`] implementation that succeeds without a host.
///
/// Used by desktop builds, unit tests and the capture harness.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NoopServices;

impl MobileServices for NoopServices {
    fn request_mic_permission(&self) -> Result<bool, String> {
        Ok(false)
    }

    fn start_dictation(&self) -> Result<(), String> {
        Ok(())
    }

    fn stop_dictation(&self) -> Result<String, String> {
        Ok(String::new())
    }

    fn pick_file(&self) -> Result<Option<String>, String> {
        Ok(None)
    }

    fn notify(&self, _title: &str, _body: &str) -> Result<(), String> {
        Ok(())
    }

    fn set_clipboard(&self, _text: &str) -> Result<(), String> {
        Ok(())
    }

    fn open_external(&self, _url: &str) -> Result<(), String> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ::core::prelude::v1::test;

    #[test]
    fn noop_services_report_success_without_a_host() {
        let services = NoopServices;
        assert_eq!(services.request_mic_permission(), Ok(false));
        assert_eq!(services.start_dictation(), Ok(()));
        assert_eq!(services.stop_dictation(), Ok(String::new()));
        assert_eq!(services.pick_file(), Ok(None));
        assert_eq!(services.notify("Stratum", "Reminder due"), Ok(()));
        assert_eq!(services.set_clipboard("note"), Ok(()));
        assert_eq!(services.open_external("https://example.com"), Ok(()));
    }

    #[test]
    fn noop_services_are_object_safe() {
        let services: std::sync::Arc<dyn MobileServices> = std::sync::Arc::new(NoopServices);
        assert_eq!(services.request_mic_permission(), Ok(false));
    }
}
