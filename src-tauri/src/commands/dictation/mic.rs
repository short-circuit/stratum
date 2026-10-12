//! Microphone permission commands for the one-tap voice-capture button.
//!
//! The mobile permission dialog is driven entirely from the platform bridge
//! (Kotlin `StratumMic` on Android; AVAudioSession on iOS/desktop), so these
//! commands are intentionally thin. On desktop they always report granted;
//! on Android the real state is owned by the JS-side `StratumMic` bridge, so
//! these commands only provide a safe fallback for non-mobile platforms and
//! a trigger event for any platform listener.

use serde::{Deserialize, Serialize};
#[cfg(target_os = "android")]
use tauri::Emitter;

/// Result of a microphone permission check.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct MicPermissionDto {
    /// True when the app may open the microphone input stream.
    pub granted: bool,
    /// True when permission is granted but partially restricted (e.g. OS-level
    /// "ask every time"). Reserved for future platforms; always false today.
    pub restricted: bool,
}

/// Report whether microphone permission is granted for this platform.
///
/// Desktop and iOS: always granted (iOS prompts on first audio stream, so the
/// app never fails here). Android: the Kotlin `StratumMic` bridge owns the
/// authoritative state; we only fall back to a static flag that defaults to
/// false so callers can still reason about an unset permission. The frontend
/// reads the bridge directly on Android (see `useMicPermission`).
#[tauri::command]
pub fn mic_permission_status() -> MicPermissionDto {
    #[cfg(target_os = "android")]
    {
        MicPermissionDto {
            granted: static_mic_granted(),
            restricted: false,
        }
    }
    #[cfg(not(target_os = "android"))]
    {
        MicPermissionDto {
            granted: true,
            restricted: false,
        }
    }
}

/// Static record used only as a fallback on Android. The Kotlin bridge never
/// writes it (the JS `StratumMic` object is authoritative); it exists so the
/// command has a defined answer when no bridge is present.
#[cfg(target_os = "android")]
static MIC_GRANTED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

#[cfg(target_os = "android")]
fn static_mic_granted() -> bool {
    use std::sync::atomic::Ordering;
    MIC_GRANTED.load(Ordering::Relaxed)
}

/// Trigger a microphone permission request on platforms that need a system
/// dialog. Non-mobile platforms return immediately (treated as granted).
///
/// On Android this emits a `mic-permission-request` event; the frontend
/// listener calls the injected `window.__stratumRequestMic()` which launches
/// the system dialog via the Kotlin `ActivityResultLauncher`. The result
/// arrives back through `window.__stratumMicPermissionResolve(granted)`.
#[tauri::command]
pub fn request_mic_permission(app: tauri::AppHandle) {
    #[cfg(target_os = "android")]
    {
        let _ = app.emit("mic-permission-request", ());
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = &app; // no-op; the JS side treats the call as granted
    }
}
