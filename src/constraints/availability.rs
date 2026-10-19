use crate::domain::{Invigilator, Plan, Seat};
use solverforge::prelude::*;
use solverforge::stream::joiner::equal_bi;
use solverforge::IncrementalConstraint;

/// HARD: a seat's invigilator must be available during the exam's timeslot.
///
/// Availability is factual input owned by the invigilator
/// (`available_slots`, as given by the instance); the rule is stated over the
/// domain objects and scored by this join, never precomputed per seat.
pub fn constraint() -> impl IncrementalConstraint<Plan, HardSoftScore> {
    ConstraintFactory::<Plan, HardSoftScore>::new()
        .for_each(Plan::seats())
        .join((
            Plan::invigilators(),
            equal_bi(assigned_invigilator, fact_index),
        ))
        .filter(slot_not_available)
        .penalize(hard_weight(one_hard))
        .named("availability")
}

fn assigned_invigilator(seat: &Seat) -> Option<usize> {
    seat.invigilator_idx
}

fn fact_index(invigilator: &Invigilator) -> Option<usize> {
    Some(invigilator.index)
}

/// Unassigned seats (left key `None`) never match the fact key, so only
/// assigned seats reach this filter.
fn slot_not_available(seat: &Seat, invigilator: &Invigilator) -> bool {
    !invigilator.available_slots.contains(&seat.slot)
}

fn one_hard(_seat: &Seat, _invigilator: &Invigilator) -> HardSoftScore {
    <HardSoftScore as Score>::one_hard()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::Exam;

    fn plan_with(available: Vec<usize>, seat_slot: usize, assigned: Option<usize>) -> Plan {
        let invigs = vec![Invigilator::new("inv-0", "inv 0", 0, 0, 0, 5, available)];
        let exams = vec![Exam::new("e0", "exam 0", 0, seat_slot, "r0".to_string(), 1)];
        let mut seat = Seat::new("s0", 0, 0, seat_slot);
        seat.invigilator_idx = assigned;
        Plan::new(invigs, exams, vec![seat])
    }

    #[test]
    fn penalizes_unavailable_assignment() {
        let p = plan_with(vec![0, 1], 2, Some(0));
        assert_eq!((constraint(),).evaluate_all(&p), HardSoftScore::of_hard(-1));
    }

    #[test]
    fn allows_available_assignment() {
        let p = plan_with(vec![0, 1], 1, Some(0));
        assert_eq!((constraint(),).evaluate_all(&p), HardSoftScore::ZERO);
    }

    #[test]
    fn ignores_unassigned_seat() {
        let p = plan_with(vec![0, 1], 2, None);
        assert_eq!((constraint(),).evaluate_all(&p), HardSoftScore::ZERO);
    }
}
