//! Background simulation worker.
//!
//! The worker owns a [`Simulation`] on its own thread so hosts can advance the
//! layout without blocking the UI thread. Commands are processed in order and
//! snapshots are deterministic: the same command sequence against the same
//! graph always yields identical positions.

use crate::data::GraphData;
use crate::math::Vec3;
use crate::physics::sim::{ForceParams, Simulation};
use pkm_core::{PkmError, PkmResult};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, JoinHandle};

/// A deterministic snapshot of worker state.
#[derive(Debug, Clone, PartialEq)]
pub struct WorkerSnapshot {
    /// Current node positions.
    pub positions: Vec<Vec3>,
    /// Completed tick count.
    pub ticks: u64,
    /// Current alpha.
    pub alpha: f64,
}

enum WorkerCommand {
    Tick(usize),
    Snapshot(Sender<WorkerSnapshot>),
    SetParams(ForceParams),
    SetParallel(bool),
    Stop,
}

/// Handle to a running simulation worker thread.
pub struct SimWorker {
    sender: Sender<WorkerCommand>,
    handle: Option<JoinHandle<()>>,
}

impl SimWorker {
    /// Spawn a worker for `data`, returning a handle immediately.
    ///
    /// The simulation is constructed before the thread starts so graph errors are
    /// reported to the caller synchronously.
    pub fn spawn(data: GraphData, seed: u64) -> PkmResult<Self> {
        let mut simulation = Simulation::new(&data, seed)?;
        simulation.set_parallel(true);
        let (sender, receiver) = mpsc::channel();
        let handle = thread::Builder::new()
            .name("stratum-graph-sim".to_string())
            .spawn(move || worker_loop(simulation, receiver))
            .map_err(|error| PkmError::Internal(format!("graph worker spawn failed: {error}")))?;
        Ok(Self {
            sender,
            handle: Some(handle),
        })
    }

    /// Advance the simulation by `count` ticks.
    pub fn tick(&self, count: usize) -> PkmResult<()> {
        self.sender
            .send(WorkerCommand::Tick(count))
            .map_err(|error| PkmError::Internal(format!("graph worker unavailable: {error}")))
    }

    /// Capture a deterministic snapshot of the current state.
    pub fn snapshot(&self) -> PkmResult<WorkerSnapshot> {
        let (reply, receiver) = mpsc::channel();
        self.sender
            .send(WorkerCommand::Snapshot(reply))
            .map_err(|error| PkmError::Internal(format!("graph worker unavailable: {error}")))?;
        receiver
            .recv()
            .map_err(|error| PkmError::Internal(format!("graph worker snapshot failed: {error}")))
    }

    /// Replace the force parameters.
    pub fn set_params(&self, params: ForceParams) -> PkmResult<()> {
        self.sender
            .send(WorkerCommand::SetParams(params))
            .map_err(|error| PkmError::Internal(format!("graph worker unavailable: {error}")))
    }

    /// Enable or disable the parallel Barnes-Hut path.
    pub fn set_parallel(&self, parallel: bool) -> PkmResult<()> {
        self.sender
            .send(WorkerCommand::SetParallel(parallel))
            .map_err(|error| PkmError::Internal(format!("graph worker unavailable: {error}")))
    }

    /// Stop the worker and join its thread.
    pub fn stop(mut self) -> PkmResult<()> {
        let _ = self.sender.send(WorkerCommand::Stop);
        if let Some(handle) = self.handle.take() {
            handle
                .join()
                .map_err(|_| PkmError::Internal("graph worker thread panicked".to_string()))?;
        }
        Ok(())
    }
}

impl Drop for SimWorker {
    fn drop(&mut self) {
        let _ = self.sender.send(WorkerCommand::Stop);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

fn worker_loop(mut simulation: Simulation, receiver: Receiver<WorkerCommand>) {
    while let Ok(command) = receiver.recv() {
        match command {
            WorkerCommand::Tick(count) => simulation.advance(count),
            WorkerCommand::Snapshot(reply) => {
                let snapshot = WorkerSnapshot {
                    positions: simulation.positions(),
                    ticks: simulation.ticks(),
                    alpha: simulation.alpha(),
                };
                let _ = reply.send(snapshot);
            }
            WorkerCommand::SetParams(params) => simulation.set_params(params),
            WorkerCommand::SetParallel(parallel) => simulation.set_parallel(parallel),
            WorkerCommand::Stop => break,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::fixture;

    #[test]
    fn worker_matches_direct_simulation() {
        let data = fixture(30, 1);
        let mut direct = Simulation::new(&data, 7).expect("simulation");
        direct.advance(12);
        let expected = direct.positions();

        let worker = SimWorker::spawn(data, 7).expect("worker");
        worker.tick(12).expect("tick");
        let snapshot = worker.snapshot().expect("snapshot");
        worker.stop().expect("stop");
        assert_eq!(snapshot.positions, expected);
        assert_eq!(snapshot.ticks, 12);
    }

    #[test]
    fn worker_rejects_bad_graphs() {
        let data = GraphData::new(vec![], vec![], "/v");
        let worker = SimWorker::spawn(data, 1).expect("worker");
        worker.stop().expect("stop");
    }
}
