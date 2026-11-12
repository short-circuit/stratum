//! Single instance enforcement and launch-argument forwarding.
//!
//! A second launch must not start a second Stratum: it hands its argv to the
//! running instance (that is how `stratum://` deep links reach an already open
//! app) and exits. The channel is a local socket from the `interprocess` crate —
//! an abstract Unix socket on Linux, a named pipe on Windows and a filesystem
//! socket on macOS — so no extra daemon or D-Bus service is involved.
//!
//! The primary instance keeps a [`InstanceGuard`] alive for the whole run. Its
//! listener thread decodes each forwarded argv and publishes the resulting
//! [`crate::PlatformEvent`] on the shell's event channel.
//!
//! The listener thread is detached on drop: a blocked `accept` is released when
//! the process exits, which is exactly when the guard's job is over. Tests use
//! unique instance names so they never contend.

#[cfg(desktop)]
use std::io::{Read, Write};
use std::sync::mpsc::Sender;
use std::thread::JoinHandle;

use pkm_core::PkmResult;

use crate::deep_link;
use crate::PlatformEvent;

/// The role this process plays.
pub enum InstanceRole {
    /// This process owns the app; keep the guard alive for the whole run.
    Primary(InstanceGuard),
    /// Another instance is already running and received our argv.
    Secondary,
}

/// The primary instance's listener handle.
pub struct InstanceGuard {
    listener: Option<JoinHandle<()>>,
}

impl Drop for InstanceGuard {
    fn drop(&mut self) {
        if let Some(handle) = self.listener.take() {
            drop(handle);
        }
    }
}

/// Turn a forwarded argv into the event the shell should apply.
///
/// A Stratum deep link wins; any other second launch simply raises the existing
/// window, mirroring the shipped single-instance plugin.
pub fn dispatch_args(args: &[String], events: &Sender<PlatformEvent>) {
    let event = match deep_link::first_from_args(args) {
        Some(link) => PlatformEvent::DeepLink(link),
        None => PlatformEvent::ShowWindow,
    };
    let _ = events.send(event);
}

/// Claim the single-instance slot named `name` and forward `args` when another
/// instance already holds it.
///
/// `name` should be stable per application and per user, e.g.
/// `stratum-<version>-<user>`; the user suffix keeps two accounts on one machine
/// from fighting over the same abstract socket.
#[cfg(desktop)]
pub fn claim(
    name: &str,
    args: Vec<String>,
    events: Sender<PlatformEvent>,
) -> PkmResult<InstanceRole> {
    use interprocess::local_socket::{ConnectOptions, ListenerOptions};

    // 1. A connect that succeeds means somebody else is listening.
    if let Ok(stream) = ConnectOptions::new()
        .name(socket_name(name)?.borrow())
        .connect_sync()
    {
        let mut stream = stream;
        let payload = serde_json::to_vec(&args)
            .map_err(|e| pkm_core::PkmError::Serialization(e.to_string()))?;
        stream
            .write_all(&payload)
            .map_err(|e| pkm_core::PkmError::Internal(format!("single instance: {e}")))?;
        let _ = stream.flush();
        return Ok(InstanceRole::Secondary);
    }

    // 2. Nobody answered: become the primary.
    let listener = match ListenerOptions::new()
        .name(socket_name(name)?)
        .create_sync()
    {
        Ok(listener) => listener,
        Err(bind_error) => {
            // A stale socket file (macOS) or a race with a starting instance.
            #[cfg(target_os = "macos")]
            {
                let _ = std::fs::remove_file(filesystem_socket_path(name));
                if let Ok(listener) = ListenerOptions::new()
                    .name(socket_name(name)?)
                    .create_sync()
                {
                    return Ok(InstanceRole::Primary(spawn_listener(listener, events)));
                }
            }
            // Last chance: if a connect works now, this process is the
            // secondary rather than a failed launch.
            if ConnectOptions::new()
                .name(socket_name(name)?.borrow())
                .connect_sync()
                .is_ok()
            {
                return Ok(InstanceRole::Secondary);
            }
            return Err(pkm_core::PkmError::Internal(format!(
                "single instance '{name}': {bind_error}"
            )));
        }
    };
    Ok(InstanceRole::Primary(spawn_listener(listener, events)))
}

/// Non-desktop hosts (Android) always run a single instance.
#[cfg(not(desktop))]
pub fn claim(
    _name: &str,
    _args: Vec<String>,
    _events: Sender<PlatformEvent>,
) -> PkmResult<InstanceRole> {
    Ok(InstanceRole::Secondary)
}

#[cfg(desktop)]
fn spawn_listener(
    listener: interprocess::local_socket::Listener,
    events: Sender<PlatformEvent>,
) -> InstanceGuard {
    use interprocess::local_socket::traits::ListenerExt;

    let handle = std::thread::Builder::new()
        .name("stratum-single-instance".into())
        .spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { continue };
                let mut payload = String::new();
                if stream.read_to_string(&mut payload).is_err() {
                    continue;
                }
                match serde_json::from_str::<Vec<String>>(&payload) {
                    Ok(args) => dispatch_args(&args, &events),
                    Err(e) => log::warn!("single instance: malformed argv payload: {e}"),
                }
            }
        })
        .ok();
    InstanceGuard { listener: handle }
}

/// The socket name for `name`, including the user so accounts do not collide.
#[cfg(desktop)]
fn socket_name(name: &str) -> PkmResult<interprocess::local_socket::Name<'static>> {
    let user = std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_else(|_| "user".into());
    let socket = format!("{name}-{user}");

    #[cfg(target_os = "macos")]
    {
        use interprocess::local_socket::{GenericFilePath, ToFsName};
        let path = filesystem_socket_path(&socket);
        Ok(path
            .to_fs_name::<GenericFilePath>()
            .map_err(|e| pkm_core::PkmError::Internal(format!("single instance name: {e}")))?
            .into_owned())
    }
    #[cfg(not(target_os = "macos"))]
    {
        use interprocess::local_socket::{GenericNamespaced, ToNsName};
        Ok(socket
            .to_ns_name::<GenericNamespaced>()
            .map_err(|e| pkm_core::PkmError::Internal(format!("single instance name: {e}")))?
            .into_owned())
    }
}

/// The filesystem socket path used on macOS (namespaced sockets are Linux and
/// Windows only).
#[cfg(target_os = "macos")]
fn filesystem_socket_path(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("{name}.sock"))
}

#[cfg(all(test, desktop))]
mod tests {
    use super::*;
    use std::sync::mpsc::channel;
    use std::time::{Duration, Instant};

    fn unique(name: &str) -> String {
        format!("stratum-test-{name}-{}", std::process::id())
    }

    #[test]
    fn first_claim_is_primary_and_second_forwards_args() {
        let name = unique("forward");
        let (tx, rx) = channel();
        let guard = match claim(&name, vec!["stratum".into()], tx) {
            Ok(InstanceRole::Primary(guard)) => guard,
            _ => panic!("expected a primary instance"),
        };

        let (tx2, _rx2) = channel();
        let second = claim(
            &name,
            vec![
                "stratum".into(),
                "stratum://board/Architecture%20sketch#frame-1".into(),
            ],
            tx2,
        );
        assert!(matches!(second, Ok(InstanceRole::Secondary)));

        let deadline = Instant::now() + Duration::from_secs(5);
        let mut event = None;
        while Instant::now() < deadline {
            if let Ok(received) = rx.try_recv() {
                event = Some(received);
                break;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        let event = event.expect("the primary receives the forwarded argv");
        assert_eq!(
            event,
            PlatformEvent::DeepLink(deep_link::DeepLink::Board {
                board: "Architecture sketch".into(),
                element: Some("frame-1".into()),
            })
        );
        drop(guard);
    }

    #[test]
    fn independent_names_are_both_primary() {
        let (tx_a, _rx_a) = channel();
        let (tx_b, _rx_b) = channel();
        let first = claim(&unique("independent-a"), Vec::new(), tx_a);
        let second = claim(&unique("independent-b"), Vec::new(), tx_b);
        assert!(matches!(first, Ok(InstanceRole::Primary(_))));
        assert!(matches!(second, Ok(InstanceRole::Primary(_))));
    }

    #[test]
    fn plain_second_launch_raises_the_window() {
        let (tx, rx) = channel();
        dispatch_args(&["stratum".into()], &tx);
        assert_eq!(rx.try_recv(), Ok(PlatformEvent::ShowWindow));
    }
}
