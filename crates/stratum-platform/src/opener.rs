//! Opening paths and revealing files in the OS.
//!
//! The shipped app used the Tauri opener plugin for exactly one user-facing
//! action: "reveal the exported file in the file manager". That is the
//! capability this module replaces (`xdg-open` on Linux, `explorer /select,` on
//! Windows, `open -R` on macOS). Opening a URL is handled by gpui's own
//! `cx.open_url`, which uses the same platform conventions.
//!
//! The argv builders are pure so the exact command line can be asserted in a
//! test; only [`reveal_in_dir`] and [`open_path`] spawn a process.

use std::path::Path;

use pkm_core::PkmResult;

/// The OS families with a distinct reveal command.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OsFamily {
    /// Linux and other freedesktop systems.
    Linux,
    /// Windows.
    Windows,
    /// macOS.
    MacOs,
}

/// The OS family of the running host.
pub fn current_os() -> OsFamily {
    if cfg!(target_os = "windows") {
        OsFamily::Windows
    } else if cfg!(target_os = "macos") {
        OsFamily::MacOs
    } else {
        OsFamily::Linux
    }
}

/// The program and argv that reveal `path` in the file manager.
pub fn reveal_argv(os: OsFamily, path: &Path) -> (String, Vec<String>) {
    match os {
        OsFamily::Windows => (
            "explorer".into(),
            vec![format!("/select,{}", path.display())],
        ),
        OsFamily::MacOs => ("open".into(), vec!["-R".into(), path.display().to_string()]),
        OsFamily::Linux => {
            // xdg-open opens the containing directory; the shipped app revealed
            // the item, and every Linux file manager can open its directory.
            let parent = path.parent().unwrap_or(path);
            ("xdg-open".into(), vec![parent.display().to_string()])
        }
    }
}

/// The program and argv that open `path` with its default application.
pub fn open_argv(os: OsFamily, path: &Path) -> (String, Vec<String>) {
    match os {
        OsFamily::Windows => (
            "cmd".into(),
            vec![
                "/C".into(),
                "start".into(),
                String::new(),
                path.display().to_string(),
            ],
        ),
        OsFamily::MacOs => ("open".into(), vec![path.display().to_string()]),
        OsFamily::Linux => ("xdg-open".into(), vec![path.display().to_string()]),
    }
}

/// Reveal `path` in the OS file manager.
#[cfg(desktop)]
pub fn reveal_in_dir(path: &Path) -> PkmResult<()> {
    let (program, args) = reveal_argv(current_os(), path);
    run(&program, &args)
}

/// Open `path` with its default application.
#[cfg(desktop)]
pub fn open_path(path: &Path) -> PkmResult<()> {
    let (program, args) = open_argv(current_os(), path);
    run(&program, &args)
}

/// Non-desktop hosts have no file manager; opening is a no-op there.
#[cfg(not(desktop))]
pub fn reveal_in_dir(_path: &Path) -> PkmResult<()> {
    Ok(())
}

/// Non-desktop hosts have no file manager; opening is a no-op there.
#[cfg(not(desktop))]
pub fn open_path(_path: &Path) -> PkmResult<()> {
    Ok(())
}

#[cfg(desktop)]
fn run(program: &str, args: &[String]) -> PkmResult<()> {
    let status = std::process::Command::new(program)
        .args(args)
        .status()
        .map_err(|e| pkm_core::PkmError::Internal(format!("{program}: {e}")))?;
    if status.success() {
        Ok(())
    } else {
        Err(pkm_core::PkmError::Internal(format!(
            "{program} exited with {status}"
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn linux_reveal_opens_the_parent_directory() {
        let path = PathBuf::from("/vault/exports/stratum-export/index.html");
        let (program, args) = reveal_argv(OsFamily::Linux, &path);
        assert_eq!(program, "xdg-open");
        assert_eq!(args, ["/vault/exports/stratum-export"]);
    }

    #[test]
    fn windows_reveal_selects_the_item() {
        let path = PathBuf::from(r"C:\vault\export\index.html");
        let (program, args) = reveal_argv(OsFamily::Windows, &path);
        assert_eq!(program, "explorer");
        assert_eq!(args, [r"/select,C:\vault\export\index.html"]);
    }

    #[test]
    fn macos_reveal_uses_open_dash_r() {
        let path = PathBuf::from("/vault/export/index.html");
        let (program, args) = reveal_argv(OsFamily::MacOs, &path);
        assert_eq!(program, "open");
        assert_eq!(args, ["-R", "/vault/export/index.html"]);
    }

    #[test]
    fn open_argv_uses_the_default_application() {
        let path = PathBuf::from("/vault/export/index.html");
        assert_eq!(open_argv(OsFamily::Linux, &path).0, "xdg-open");
        assert_eq!(open_argv(OsFamily::MacOs, &path).0, "open");
        assert_eq!(open_argv(OsFamily::Windows, &path).0, "cmd");
    }
}
