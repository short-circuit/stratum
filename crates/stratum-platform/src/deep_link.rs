//! `stratum://` deep links: parsing, route mapping and scheme registration.
//!
//! The shipped webview had no URL scheme; the plan (E9) requires one so a link
//! such as `stratum://board/Architecture%20sketch#frame-1` opens the app on that
//! whiteboard element. Parsing and route mapping are pure and tested on every
//! target; registration is desktop-only:
//!
//! - Linux: a `stratum.desktop` entry with `MimeType=x-scheme-handler/stratum`
//!   in `$XDG_DATA_HOME/applications`, made the default handler with
//!   `xdg-mime`.
//! - Windows: the `HKCU\Software\Classes\stratum` protocol keys, written with
//!   `reg add` so no registry crate is needed.
//! - macOS: best effort — the scheme must be declared in the bundle's
//!   `Info.plist` (`CFBundleURLTypes`), which packaging adds in P11.

use pkm_core::PkmResult;

/// The URL scheme Stratum owns.
pub const SCHEME: &str = "stratum";

/// The desktop-entry file name used for scheme registration on Linux.
pub const DESKTOP_FILE: &str = "stratum.desktop";

/// A parsed deep link.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DeepLink {
    /// A whiteboard, optionally scrolled to one element.
    Board {
        /// The board name.
        board: String,
        /// The element id or frame name to reveal.
        element: Option<String>,
    },
    /// A page in the vault.
    Page {
        /// Vault-relative page path.
        path: String,
    },
    /// The journal, optionally on a specific day.
    Journal {
        /// `YYYY-MM-DD`, when the link names a day.
        date: Option<String>,
    },
    /// A plain panel route.
    Panel {
        /// The panel slug (`search`, `graph`, `settings`, ...).
        panel: String,
    },
}

/// The panel slugs a deep link may address.
pub const PANEL_SLUGS: [&str; 12] = [
    "journal",
    "search",
    "query",
    "ask",
    "templates",
    "flashcards",
    "kanban",
    "reminders",
    "whiteboards",
    "graph",
    "plugins",
    "settings",
];

/// Parse a `stratum://` URL.
///
/// Returns `None` for anything that is not a Stratum link or does not name a
/// known destination, so a stray argv token can never navigate the shell to a
/// blank route.
pub fn parse(url: &str) -> Option<DeepLink> {
    let rest = url.strip_prefix("stratum://")?;
    let (target, fragment) = match rest.split_once('#') {
        Some((target, fragment)) => (target, Some(fragment)),
        None => (rest, None),
    };
    let target = target.trim_end_matches('/');
    let mut segments = target.splitn(2, '/');
    let kind = segments.next()?.to_ascii_lowercase();
    let tail = segments.next().map(percent_decode).unwrap_or_default();
    let element = fragment.map(percent_decode).filter(|f| !f.is_empty());

    match kind.as_str() {
        "board" | "whiteboard" | "whiteboards" => {
            if tail.is_empty() && element.is_none() {
                return Some(DeepLink::Panel {
                    panel: "whiteboards".into(),
                });
            }
            Some(DeepLink::Board {
                board: tail,
                element,
            })
        }
        "page" | "note" => {
            if tail.is_empty() {
                return None;
            }
            Some(DeepLink::Page { path: tail })
        }
        "journal" => Some(DeepLink::Journal {
            date: if tail.is_empty() { None } else { Some(tail) },
        }),
        slug if PANEL_SLUGS.contains(&slug) => Some(DeepLink::Panel {
            panel: slug.to_string(),
        }),
        _ => None,
    }
}

/// The in-app route for a deep link.
///
/// The shell feeds this through its shared router, so a deep link and a
/// sidebar click produce exactly the same panel state.
pub fn route(link: &DeepLink) -> String {
    match link {
        DeepLink::Board { board, element } => {
            let mut route = format!("/whiteboards?board={}", percent_encode(board));
            if let Some(element) = element {
                route.push_str(&format!("&element={}", percent_encode(element)));
            }
            route
        }
        DeepLink::Page { path } => format!("/page/{}", percent_encode(path)),
        DeepLink::Journal { .. } => "/journal".to_string(),
        DeepLink::Panel { panel } => format!("/{panel}"),
    }
}

/// Find the first Stratum deep link in a launch argv.
///
/// Windows passes the URL as one argument, Linux desktops append it after the
/// executable, and a forwarded second-instance argv arrives in the same shape.
pub fn first_from_args(args: &[String]) -> Option<DeepLink> {
    args.iter().find_map(|arg| {
        let trimmed = arg.trim();
        if trimmed.starts_with("stratum://") {
            parse(trimmed)
        } else {
            None
        }
    })
}

/// Whether `date` is a `YYYY-MM-DD` journal day.
pub fn is_journal_date(date: &str) -> bool {
    let bytes = date.as_bytes();
    if bytes.len() != 10 {
        return false;
    }
    bytes.iter().enumerate().all(|(index, byte)| match index {
        4 | 7 => *byte == b'-',
        _ => byte.is_ascii_digit(),
    })
}

/// The Linux desktop entry that registers the scheme.
///
/// `exec_path` is the absolute path of the running executable; `%u` passes the
/// URL as a single argument, which is what [`first_from_args`] expects.
pub fn desktop_entry(exec_path: &str) -> String {
    format!(
        "[Desktop Entry]\n\
         Type=Application\n\
         Name=Stratum\n\
         Comment=Privacy-first personal knowledge management\n\
         Exec=\"{exec_path}\" %u\n\
         Terminal=false\n\
         NoDisplay=true\n\
         MimeType=x-scheme-handler/{SCHEME};\n\
         Categories=Office;\n"
    )
}

/// The commands that register the scheme for the current user.
///
/// Split out from [`register_scheme`] so the exact argv can be asserted in a
/// test without touching the system.
pub fn scheme_commands(exec_path: &str) -> Vec<(String, Vec<String>)> {
    if cfg!(target_os = "windows") {
        vec![
            (
                "reg".into(),
                vec![
                    "add".into(),
                    format!("HKCU\\Software\\Classes\\{SCHEME}"),
                    "/ve".into(),
                    "/d".into(),
                    format!("URL:{SCHEME} protocol"),
                    "/f".into(),
                ],
            ),
            (
                "reg".into(),
                vec![
                    "add".into(),
                    format!("HKCU\\Software\\Classes\\{SCHEME}"),
                    "/v".into(),
                    "URL Protocol".into(),
                    "/d".into(),
                    String::new(),
                    "/f".into(),
                ],
            ),
            (
                "reg".into(),
                vec![
                    "add".into(),
                    format!("HKCU\\Software\\Classes\\{SCHEME}\\shell\\open\\command"),
                    "/ve".into(),
                    "/d".into(),
                    format!("\"{exec_path}\" \"%1\""),
                    "/f".into(),
                ],
            ),
        ]
    } else {
        vec![
            (
                "xdg-mime".into(),
                vec![
                    "default".into(),
                    DESKTOP_FILE.into(),
                    format!("x-scheme-handler/{SCHEME}"),
                ],
            ),
            (
                "update-desktop-database".into(),
                vec![std::env::var("XDG_DATA_HOME")
                    .map(|dir| format!("{dir}/applications"))
                    .unwrap_or_else(|_| {
                        let home = std::env::var("HOME").unwrap_or_default();
                        format!("{home}/.local/share/applications")
                    })],
            ),
        ]
    }
}

/// Register `stratum://` for the current user.
///
/// Idempotent and best effort: a failure is logged by the caller but never
/// stops the app from starting, because the scheme is a convenience, not a
/// requirement for the app to run.
#[cfg(desktop)]
pub fn register_scheme() -> PkmResult<()> {
    let exe = std::env::current_exe()?;
    let exe = exe.display().to_string();

    #[cfg(target_os = "macos")]
    {
        // Declared by the bundle's Info.plist; nothing to write at runtime.
        // This block is the function's tail expression on macOS, so it returns
        // the value directly instead of a `return` (clippy::needless_return).
        let _ = exe;
        Ok(())
    }

    #[cfg(not(target_os = "macos"))]
    {
        #[cfg(target_os = "windows")]
        {
            let _ = exe;
        }
        #[cfg(not(target_os = "windows"))]
        {
            let dir = match std::env::var("XDG_DATA_HOME") {
                Ok(dir) => std::path::PathBuf::from(dir).join("applications"),
                Err(_) => {
                    let home = std::env::var("HOME").map_err(|_| {
                        pkm_core::PkmError::Config("neither XDG_DATA_HOME nor HOME is set".into())
                    })?;
                    std::path::PathBuf::from(home).join(".local/share/applications")
                }
            };
            std::fs::create_dir_all(&dir)?;
            std::fs::write(dir.join(DESKTOP_FILE), desktop_entry(&exe))?;
        }

        for (program, args) in scheme_commands(&exe) {
            let status = std::process::Command::new(&program).args(&args).status();
            match status {
                Ok(status) if status.success() => {}
                Ok(status) => log::warn!("deep link: {program} exited with {status}"),
                Err(e) => log::warn!("deep link: {program} could not run: {e}"),
            }
        }
        Ok(())
    }
}

/// Non-desktop hosts (Android) declare the scheme in the activity manifest, so
/// registering it at runtime is a no-op there.
#[cfg(not(desktop))]
pub fn register_scheme() -> PkmResult<()> {
    Ok(())
}

/// Percent-decode one URL component. Invalid escapes are kept verbatim.
pub fn percent_decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' && index + 2 < bytes.len() {
            let high = hex_value(bytes[index + 1]);
            let low = hex_value(bytes[index + 2]);
            if let (Some(high), Some(low)) = (high, low) {
                out.push(high * 16 + low);
                index += 3;
                continue;
            }
        }
        out.push(bytes[index]);
        index += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Percent-encode a component using RFC 3986 unreserved characters.
pub fn percent_encode(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.as_bytes() {
        let unreserved = byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~');
        if unreserved {
            out.push(*byte as char);
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_board_element_link() {
        let link = parse("stratum://board/Architecture%20sketch#frame-1").expect("board link");
        assert_eq!(
            link,
            DeepLink::Board {
                board: "Architecture sketch".into(),
                element: Some("frame-1".into()),
            }
        );
        assert_eq!(
            route(&link),
            "/whiteboards?board=Architecture%20sketch&element=frame-1"
        );
    }

    #[test]
    fn parses_a_board_without_an_element() {
        let link = parse("stratum://board/Fixture").expect("board link");
        assert_eq!(route(&link), "/whiteboards?board=Fixture");
    }

    #[test]
    fn bare_whiteboards_is_the_panel() {
        assert_eq!(
            parse("stratum://whiteboards"),
            Some(DeepLink::Panel {
                panel: "whiteboards".into()
            })
        );
    }

    #[test]
    fn parses_pages_and_journal_days() {
        let page = parse("stratum://page/pages/Welcome.md").expect("page link");
        assert_eq!(
            page,
            DeepLink::Page {
                path: "pages/Welcome.md".into()
            }
        );
        assert_eq!(route(&page), "/page/pages%2FWelcome.md");

        let journal = parse("stratum://journal/2026-11-12").expect("journal link");
        assert_eq!(
            journal,
            DeepLink::Journal {
                date: Some("2026-11-12".into())
            }
        );
        assert_eq!(route(&journal), "/journal");
        assert!(is_journal_date("2026-11-12"));
        assert!(!is_journal_date("2026-1-2"));
    }

    #[test]
    fn parses_plain_panels_and_rejects_unknown_ones() {
        assert_eq!(
            parse("stratum://settings"),
            Some(DeepLink::Panel {
                panel: "settings".into()
            })
        );
        assert_eq!(parse("stratum://nope"), None);
        assert_eq!(parse("stratum://page"), None);
        assert_eq!(parse("https://example.com"), None);
    }

    #[test]
    fn finds_the_link_in_a_launch_argv() {
        let args = vec![
            "/usr/bin/stratum".to_string(),
            "--flag".to_string(),
            "stratum://graph".to_string(),
        ];
        assert_eq!(
            first_from_args(&args),
            Some(DeepLink::Panel {
                panel: "graph".into()
            })
        );
        assert_eq!(first_from_args(&["/usr/bin/stratum".to_string()]), None);
    }

    #[test]
    fn desktop_entry_registers_the_scheme() {
        let entry = desktop_entry("/usr/bin/stratum");
        assert!(entry.contains("Exec=\"/usr/bin/stratum\" %u"));
        assert!(entry.contains("MimeType=x-scheme-handler/stratum;"));
        assert!(entry.starts_with("[Desktop Entry]"));
    }

    #[test]
    fn percent_round_trip() {
        let raw = "pages/Architecture sketch.md";
        assert_eq!(percent_decode(&percent_encode(raw)), raw);
        assert_eq!(percent_decode("%ZZ"), "%ZZ");
    }
}
