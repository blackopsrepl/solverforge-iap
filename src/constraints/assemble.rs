//! Assembles the project's constraint set.

use crate::domain::Plan;
use solverforge::prelude::*;

/// Builds the constraint set the solver scores every solution against.
pub fn create_constraints() -> impl ConstraintSet<Plan, HardSoftScore> {
    // @solverforge:begin constraint-calls
    (
        super::availability::constraint(),
        super::contacts::constraint(),
        super::slot_conflict::constraint(),
        super::workload_bounds::constraint(),
    )
    // @solverforge:end constraint-calls
}
