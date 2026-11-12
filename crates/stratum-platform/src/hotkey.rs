//! Global quick-capture shortcut.
//!
//! The pure half validates a shortcut string against the grammar the OS layer
//! accepts, so the settings tab can reject a typo without touching the desktop
//! APIs. The OS half registers the binding with `global-hotkey`, which the
//! shell polls for presses.
//!
//! Platform note (recorded in `docs/guide/platform-integration.md`):
//! `global-hotkey` grabs keys through X11 (`XGrabKey`), so the shortcut works in
//! X11 and XWayland sessions. A pure Wayland session has no X server to grab
//! from and the registration fails with a logged warning; the tray menu's
//! Quick Capture entry stays available either way. The shipped Tauri app had the
//! same limitation because its plugin wraps the same crate.

#[cfg(desktop)]
use pkm_core::PkmResult;

/// The shipped default binding.
pub const DEFAULT_SHORTCUT: &str = "CmdOrCtrl+Shift+Space";

/// Modifier bits parsed from a shortcut string.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Modifiers {
    /// Control / `CmdOrCtrl` on Linux and Windows.
    pub control: bool,
    /// Alt / Option.
    pub alt: bool,
    /// Shift.
    pub shift: bool,
    /// Super / Command (`CmdOrCtrl` on macOS).
    pub command: bool,
}

/// A parsed shortcut: modifiers plus one key token.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShortcutSpec {
    /// The modifier combination.
    pub modifiers: Modifiers,
    /// The normalized key token (`SPACE`, `F5`, `K`, ...).
    pub key: String,
}

/// Parse a shortcut string.
///
/// Accepts the spelling the shipped app stored (`CmdOrCtrl+Shift+Space`,
/// `Ctrl+Alt+K`, ...). `CmdOrCtrl` maps to Command on macOS and Control
/// everywhere else, matching `global-hotkey`. Returns `None` for anything the
/// OS layer would reject: no key, two keys, an empty token, or an unknown key
/// name.
pub fn parse(shortcut: &str) -> Option<ShortcutSpec> {
    let mut modifiers = Modifiers::default();
    let mut key: Option<String> = None;

    for raw in shortcut.split('+') {
        let token = raw.trim();
        if token.is_empty() {
            return None;
        }
        let upper = token.to_uppercase();
        match upper.as_str() {
            "CTRL" | "CONTROL" => modifiers.control = true,
            "ALT" | "OPTION" => modifiers.alt = true,
            "SHIFT" => modifiers.shift = true,
            "CMD" | "COMMAND" | "SUPER" => modifiers.command = true,
            "CMDORCTRL" | "CMDORCONTROL" | "COMMANDORCTRL" | "COMMANDORCONTROL" => {
                if cfg!(target_os = "macos") {
                    modifiers.command = true;
                } else {
                    modifiers.control = true;
                }
            }
            _ => {
                if key.is_some() || !is_key_token(&upper) {
                    return None;
                }
                key = Some(upper);
            }
        }
    }

    let key = key?;
    let any_modifier = modifiers.control || modifiers.alt || modifiers.shift || modifiers.command;
    if !any_modifier {
        // A bare key is legal for the OS layer, but the shipped UI always had a
        // modifier; keeping the requirement makes a stray "K" a visible typo.
        return None;
    }
    Some(ShortcutSpec { modifiers, key })
}

/// Whether `shortcut` is a binding the OS layer can register.
pub fn is_well_formed(shortcut: &str) -> bool {
    parse(shortcut).is_some()
}

/// Whether a token names a key `global-hotkey` understands.
fn is_key_token(token: &str) -> bool {
    if token.chars().count() == 1 {
        let ch = token.chars().next().unwrap_or(' ');
        return ch.is_ascii_alphanumeric();
    }
    if let Some(rest) = token.strip_prefix('F') {
        if let Ok(n) = rest.parse::<u8>() {
            return (1..=24).contains(&n);
        }
    }
    matches!(
        token,
        "SPACE"
            | "ENTER"
            | "TAB"
            | "BACKSPACE"
            | "DELETE"
            | "INSERT"
            | "HOME"
            | "END"
            | "PAGEUP"
            | "PAGEDOWN"
            | "UP"
            | "DOWN"
            | "LEFT"
            | "RIGHT"
            | "ARROWUP"
            | "ARROWDOWN"
            | "ARROWLEFT"
            | "ARROWRIGHT"
            | "MINUS"
            | "EQUAL"
            | "COMMA"
            | "PERIOD"
            | "SLASH"
            | "SEMICOLON"
            | "QUOTE"
            | "BACKQUOTE"
            | "BACKSLASH"
            | "BRACKETLEFT"
            | "BRACKETRIGHT"
            | "CAPSLOCK"
            | "PRINTSCREEN"
            | "SCROLLLOCK"
            | "PAUSE"
            | "-"
            | "="
            | ","
            | "."
            | "/"
            | ";"
            | "'"
            | "`"
            | "\\"
            | "["
            | "]"
    )
}

/// The registered global shortcut manager.
#[cfg(desktop)]
pub struct HotkeyManager {
    manager: global_hotkey::GlobalHotKeyManager,
    registered: Option<String>,
    registered_hotkey: Option<global_hotkey::hotkey::HotKey>,
}

#[cfg(desktop)]
impl HotkeyManager {
    /// Create the manager.
    ///
    /// Must run on the thread that owns the application event loop on Windows
    /// and macOS; the gpui host creates it on the main thread.
    pub fn new() -> PkmResult<Self> {
        let manager = global_hotkey::GlobalHotKeyManager::new()
            .map_err(|e| pkm_core::PkmError::Internal(format!("global shortcut: {e}")))?;
        Ok(Self {
            manager,
            registered: None,
            registered_hotkey: None,
        })
    }

    /// Register `shortcut`, replacing any previous binding.
    pub fn register(&mut self, shortcut: &str) -> PkmResult<()> {
        self.unregister_all();
        let hotkey: global_hotkey::hotkey::HotKey = shortcut
            .parse()
            .map_err(|e| pkm_core::PkmError::Validation(format!("shortcut '{shortcut}': {e}")))?;
        self.manager
            .register(hotkey)
            .map_err(|e| pkm_core::PkmError::Internal(format!("shortcut '{shortcut}': {e}")))?;
        self.registered = Some(shortcut.to_string());
        self.registered_hotkey = Some(hotkey);
        Ok(())
    }

    /// Drop every registered binding.
    pub fn unregister_all(&mut self) {
        if let Some(hotkey) = self.registered_hotkey.take() {
            let _ = self.manager.unregister_all(&[hotkey]);
        }
        self.registered = None;
    }

    /// The currently registered shortcut, if any.
    pub fn registered(&self) -> Option<&str> {
        self.registered.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_the_shipped_default() {
        let spec = parse(DEFAULT_SHORTCUT).expect("default is valid");
        assert_eq!(spec.key, "SPACE");
        assert!(spec.modifiers.shift);
        assert!(spec.modifiers.control || spec.modifiers.command);
    }

    #[test]
    fn accepts_letters_digits_and_function_keys() {
        assert!(is_well_formed("Ctrl+Alt+K"));
        assert!(is_well_formed("Super+Shift+7"));
        assert!(is_well_formed("Ctrl+F5"));
        assert!(is_well_formed("Alt+ArrowUp"));
    }

    #[test]
    fn rejects_malformed_bindings() {
        assert!(!is_well_formed(""));
        assert!(!is_well_formed("K"), "a bare key needs a modifier");
        assert!(!is_well_formed("Ctrl+"), "empty token");
        assert!(!is_well_formed("Ctrl+K+J"), "two keys");
        assert!(!is_well_formed("Ctrl+Banana"), "unknown key");
        assert!(!is_well_formed("Ctrl+F25"), "out of range");
    }

    #[test]
    fn normalizes_case_and_whitespace() {
        let spec = parse(" ctrl + shift + k ").expect("valid");
        assert_eq!(spec.key, "K");
        assert!(spec.modifiers.control && spec.modifiers.shift);
        assert!(!spec.modifiers.alt && !spec.modifiers.command);
    }
}
