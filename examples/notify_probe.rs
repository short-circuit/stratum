// Standalone notify probe: does inotify deliver file events in this env?
fn main() {
    let dir = std::env::temp_dir().join(format!("notify-probe-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("watch.md");
    std::fs::write(&file, "a").unwrap();
    let (tx, rx) = std::sync::mpsc::channel();
    let mut watcher = notify::RecommendedWatcher::new(
        move |res: notify::Result<notify::Event>| {
            if let Ok(ev) = res { let _ = tx.send(format!("{:?} {:?}", ev.kind, ev.paths)); }
        },
        notify::Config::default(),
    ).unwrap();
    watcher.watch(&dir, notify::RecursiveMode::Recursive).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(500));
    std::fs::write(&file, "b").unwrap();
    std::thread::sleep(std::time::Duration::from_millis(1500));
    match rx.try_recv() {
        Ok(ev) => println!("EVENT_DELIVERED: {ev}"),
        Err(_) => println!("NO_EVENT"),
    }
}
