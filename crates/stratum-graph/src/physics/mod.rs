//! Deterministic force simulation, Barnes-Hut octree and initialization helpers.
//!
//! Public users normally construct a [`Simulation`] and advance it with
//! [`Simulation::tick`]. The [`barnes_hut`] and [`init`] modules are exposed for
//! tests and for hosts that need to inspect the force kernels and LCG behaviour
//! directly; the octree and tick phases are crate-internal.

pub mod barnes_hut;
pub mod init;
mod octree;
pub mod sim;
mod tick;

pub use init::Lcg;
pub use sim::{ForceParams, SimNode, Simulation};
