//! Reproducibility checks for the sequential and parallel simulation paths and
//! the background worker.

use stratum_graph::{fixture, SimWorker, Simulation};

#[test]
fn sequential_ticks_are_reproducible() {
    let data = fixture(64, 1);
    let mut first = Simulation::new(&data, 1).expect("simulation");
    let mut second = Simulation::new(&data, 1).expect("simulation");
    first.advance_sequential(40);
    second.advance_sequential(40);
    assert_eq!(first.positions(), second.positions());
    assert_eq!(first.ticks(), second.ticks());
    assert_eq!(first.alpha(), second.alpha());
}

#[test]
fn parallel_ticks_are_reproducible() {
    let data = fixture(256, 7);
    let mut first = Simulation::new(&data, 7).expect("simulation");
    let mut second = Simulation::new(&data, 7).expect("simulation");
    first.set_parallel(true);
    second.set_parallel(true);
    first.advance(30);
    second.advance(30);
    assert_eq!(first.positions(), second.positions());
}

#[test]
fn forcing_the_sequential_path_is_reproducible() {
    let data = fixture(128, 3);
    let mut first = Simulation::new(&data, 3).expect("simulation");
    let mut second = Simulation::new(&data, 3).expect("simulation");
    first.set_parallel(false);
    second.set_parallel(false);
    first.advance(25);
    second.advance(25);
    assert_eq!(first.positions(), second.positions());
}

#[test]
fn worker_matches_a_direct_simulation() {
    let data = fixture(96, 5);
    let mut direct = Simulation::new(&data, 5).expect("simulation");
    direct.advance(20);
    let expected = direct.positions();

    let worker = SimWorker::spawn(data, 5).expect("worker");
    worker.tick(20).expect("tick");
    let snapshot = worker.snapshot().expect("snapshot");
    worker.stop().expect("stop");
    assert_eq!(snapshot.positions, expected);
    assert_eq!(snapshot.ticks, 20);
    assert_eq!(snapshot.alpha, direct.alpha());
}

#[test]
fn pinned_nodes_are_stable_across_paths() {
    let data = fixture(32, 2);
    let mut simulation = Simulation::new(&data, 2).expect("simulation");
    let pinned = stratum_graph::Vec3::new(12.0, -4.0, 6.0);
    simulation.pin(0, Some(pinned)).expect("pin");
    simulation.advance(15);
    let node = simulation.node(0).expect("node");
    assert_eq!((node.x, node.y, node.z), (12.0, -4.0, 6.0));
    assert_eq!((node.vx, node.vy, node.vz), (0.0, 0.0, 0.0));
}
