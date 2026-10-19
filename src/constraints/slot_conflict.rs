use crate::domain::{Plan, Seat};
use solverforge::prelude::*;
use solverforge::IncrementalConstraint;

/// HARD: an invigilator occupies at most one seat in a timeslot. This also
/// forbids duplicate assignments within one exam, without a second constraint.
/// Each assignment beyond the first contributes one hard point.
pub fn constraint() -> impl IncrementalConstraint<Plan, HardSoftScore> {
    ConstraintFactory::<Plan, HardSoftScore>::new()
        .for_each(Plan::seats())
        .filter(is_assigned)
        .group_by(invigilator_slot, count())
        .penalize(excess_assignments)
        .named("slot_conflict")
}

fn is_assigned(seat: &Seat) -> bool {
    seat.invigilator_idx.is_some()
}

fn invigilator_slot(seat: &Seat) -> (Option<usize>, usize) {
    (seat.invigilator_idx, seat.slot)
}

fn excess_assignments(key: &(Option<usize>, usize), count: &usize) -> HardSoftScore {
    // Optional-key handling also protects full evaluation on stream versions
    // that evaluate grouped keys before applying the upstream filter.
    HardSoftScore::of_hard(if key.0.is_some() {
        count.saturating_sub(1) as i64
    } else {
        0
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Exam, Invigilator};

    fn plan_with(exam0_slot: usize, exam1_slot: usize, seats: Vec<Seat>) -> Plan {
        let invigs = vec![
            Invigilator::new("inv-0", "inv 0", 0, 0, 0, 5, vec![0, 1]),
            Invigilator::new("inv-1", "inv 1", 1, 1, 0, 5, vec![0, 1]),
        ];
        let exams = vec![
            Exam::new("e0", "exam 0", 0, exam0_slot, "r0".to_string(), 1),
            Exam::new("e1", "exam 1", 1, exam1_slot, "r1".to_string(), 1),
        ];
        Plan::new(invigs, exams, seats)
    }

    fn seat(id: &str, exam_index: usize, slot: usize, assigned: Option<usize>) -> Seat {
        let mut s = Seat::new(id.to_string(), exam_index, 0, slot);
        s.invigilator_idx = assigned;
        s
    }

    #[test]
    fn penalizes_same_slot_double_booking() {
        let p = plan_with(
            0,
            0,
            vec![seat("s0", 0, 0, Some(0)), seat("s1", 1, 0, Some(0))],
        );
        assert_eq!((constraint(),).evaluate_all(&p), HardSoftScore::of_hard(-1));
    }

    #[test]
    fn allows_same_invigilator_in_different_slots() {
        let p = plan_with(
            0,
            1,
            vec![seat("s0", 0, 0, Some(0)), seat("s1", 1, 1, Some(0))],
        );
        assert_eq!((constraint(),).evaluate_all(&p), HardSoftScore::ZERO);
    }

    #[test]
    fn ignores_unassigned_seats() {
        let p = plan_with(0, 0, vec![seat("s0", 0, 0, None), seat("s1", 1, 0, None)]);
        assert_eq!((constraint(),).evaluate_all(&p), HardSoftScore::ZERO);
    }
}
