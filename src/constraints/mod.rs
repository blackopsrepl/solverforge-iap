//! Constraint definitions.
//!
//! Add constraint modules with `solverforge generate constraint ...`.

#[cfg(test)]
mod audit_tests;

// @solverforge:begin constraint-modules
mod availability;
mod contacts;
mod slot_conflict;
mod workload_bounds;
// @solverforge:end constraint-modules

mod assemble;

pub use assemble::create_constraints;
