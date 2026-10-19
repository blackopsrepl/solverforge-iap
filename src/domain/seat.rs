use serde::{Deserialize, Serialize};
use solverforge::prelude::*;

use super::Plan;

/// TODO — describe this entity.
#[planning_entity]
#[derive(Serialize, Deserialize)]
pub struct Seat {
    #[planning_id]
    pub id: String,
    pub exam_index: usize,
    pub position: usize,
    pub slot: usize,
    /// Invigilator indexes available at this seat's exam timeslot, derived by
    /// the data layer from the invigilator availability facts. Read by the
    /// candidate-values hook `seat_candidates`; never read as a rule verdict.
    /// Invigilator indexes available at this seat's exam timeslot, derived by the
    /// data layer (and idempotently on every posted plan). Solver-only data:
    /// skipped in serde so API payloads stay at instance-file size.
    #[serde(skip)]
    pub candidates: Vec<usize>,
    /// Objective sense of this plan, projected from `Plan::variant` by the data
    /// layer. Carried on the entity so the `contacts` group key reads it without
    /// needing plan access inside the penalty closure.
    #[serde(default)]
    pub maximize: bool,
    // @solverforge:begin entity-variables
    #[planning_variable(
        value_range_provider = "invigilators",
        allows_unassigned = false,
        candidate_values = "seat_candidates",
        nearby_entity_candidates = "same_slot_seats"
    )]
    pub invigilator_idx: Option<usize>,
    // @solverforge:end entity-variables
}

impl Seat {
    pub fn new(id: impl Into<String>, exam_index: usize, position: usize, slot: usize) -> Self {
        Self {
            id: id.into(),
            exam_index,
            position,
            slot,
            candidates: Vec::new(),
            maximize: false,
            // @solverforge:begin entity-variable-init
            invigilator_idx: None,
            // @solverforge:end entity-variable-init
        }
    }
}

/// Candidate-values hook (declared as `candidate_values = "seat_candidates"`):
/// the invigilators available at the seat's exam timeslot, as projected by the
/// data layer onto `Seat::candidates`. The availability rule itself is still
/// stated — and scored — by the `availability` constraint.
pub(super) fn seat_candidates(
    plan: &Plan,
    entity_index: usize,
    _variable_index: usize,
) -> &[usize] {
    &plan.seats[entity_index].candidates
}

/// Immutable factual neighborhood; SolverForge owns swap generation and scoring.
pub(super) fn same_slot_seats(
    plan: &Plan,
    entity_index: usize,
    _variable_index: usize,
) -> &[usize] {
    &plan.seats_by_slot[plan.seats[entity_index].slot]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_seat_construction() {
        let entity = Seat::new(
            "test-id",
            Default::default(),
            Default::default(),
            Default::default(),
        );
        assert_eq!(entity.id, "test-id");
        let _ = &entity.exam_index;
        let _ = &entity.position;
        let _ = &entity.slot;
    }
}
