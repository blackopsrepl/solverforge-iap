/* Constraint definitions.

Add constraint modules with `solverforge generate constraint ...`.
The neutral shell starts with an empty constraint set. */

use crate::domain::Plan;
use solverforge::prelude::*;

pub use self::assemble::create_constraints;

#[cfg(test)]
mod audit_tests;

// @solverforge:begin constraint-modules
mod availability;
mod contacts;
mod slot_conflict;
mod workload_bounds;
// @solverforge:end constraint-modules

mod assemble {
    use super::*;

    pub fn create_constraints() -> impl ConstraintSet<Plan, HardSoftScore> {
        // @solverforge:begin constraint-calls
        (
            availability::constraint(),
            contacts::constraint(),
            slot_conflict::constraint(),
            workload_bounds::constraint(),
        )
        // @solverforge:end constraint-calls
    }
}
