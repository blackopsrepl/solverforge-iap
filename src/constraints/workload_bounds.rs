use crate::domain::{Invigilator, Plan, Seat};
use solverforge::prelude::*;
use solverforge::stream::joiner::equal_bi;
use solverforge::IncrementalConstraint;

/// HARD: every invigilator must be assigned a workload inside
/// `[min_workload, max_workload]`.
///
/// Seats are grouped by their assigned invigilator; the group key carries the
/// invigilator's own bounds so the weight reads the whole rule from the group.
/// The count is the invigilator's number of assigned exams.
///
/// `complement` fills a zero-count row for every invigilator with no assigned
/// seat: grouping only sees rows that exist, and a `min_workload >= 1`
/// invigilator with zero seats is nonetheless a violation.
pub fn constraint() -> impl IncrementalConstraint<Plan, HardSoftScore> {
    ConstraintFactory::<Plan, HardSoftScore>::new()
        .for_each(Plan::seats())
        .join((
            Plan::invigilators(),
            equal_bi(assigned_invigilator, fact_index),
        ))
        .project(invigilator_bounds)
        .group_by(bounds_key, count())
        .complement(Plan::invigilators(), complement_key, no_assigned_exams)
        .penalize(hard_weight(outside_bounds))
        .named("workload_bounds")
}

#[derive(Clone)]
struct InvigilatorBounds {
    index: usize,
    min_workload: i64,
    max_workload: i64,
}

fn assigned_invigilator(seat: &Seat) -> Option<usize> {
    seat.invigilator_idx
}

fn fact_index(invigilator: &Invigilator) -> Option<usize> {
    Some(invigilator.index)
}

fn invigilator_bounds(_seat: &Seat, invigilator: &Invigilator) -> InvigilatorBounds {
    InvigilatorBounds {
        index: invigilator.index,
        min_workload: invigilator.min_workload,
        max_workload: invigilator.max_workload,
    }
}

fn bounds_key(entry: &InvigilatorBounds) -> (usize, i64, i64) {
    (entry.index, entry.min_workload, entry.max_workload)
}

fn outside_bounds(key: &(usize, i64, i64), count: &usize) -> HardSoftScore {
    let (_index, min, max) = *key;
    let count = *count as i64;
    // Magnitude, not membership: the feasible set is identical (any violation
    // yields a negative hard level), but the distance from the bound gives the
    // search a real gradient — with a fixed 1-per-group penalty the
    // min/max coupling has long zero-score plateaus where every single move
    // is net-zero and the walk can only drift.
    let distance = (min - count).max(0) + (count - max).max(0);
    if distance > 0 {
        HardSoftScore::of_hard(distance)
    } else {
        <HardSoftScore as Score>::zero()
    }
}

fn complement_key(invigilator: &Invigilator) -> (usize, i64, i64) {
    (
        invigilator.index,
        invigilator.min_workload,
        invigilator.max_workload,
    )
}

/// The collector result for a complemented (unassigned) invigilator: zero
/// assigned exams.
fn no_assigned_exams(_invigilator: &Invigilator) -> usize {
    0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::Exam;

    /// One exam needing two seats, one invigilator with bounds [min, max]:
    /// both seats assigned to that invigilator.
    fn plan_with(min: i64, max: i64, extra_invigilators: usize, assigned: Option<usize>) -> Plan {
        let mut invigs = vec![Invigilator::new(
            "inv-0",
            "inv 0",
            0,
            0,
            min,
            max,
            vec![0, 1, 2],
        )];
        for i in 0..extra_invigilators {
            invigs.push(Invigilator::new(
                format!("inv-{}", i + 1),
                format!("inv {}", i + 1),
                i + 1,
                0,
                min,
                max,
                vec![0, 1, 2],
            ));
        }
        let exams = vec![Exam::new("e0", "exam 0", 0, 0, "r0".to_string(), 2)];
        let seats: Vec<Seat> = (0..2)
            .map(|pos| {
                let mut s = Seat::new(format!("seat-{pos}"), 0, pos, 0);
                s.invigilator_idx = assigned;
                s
            })
            .collect();
        Plan::new(invigs, exams, seats)
    }

    #[test]
    fn penalizes_workload_above_maximum() {
        let p = plan_with(0, 1, 0, Some(0));
        // Both seats on one invigilator whose max is 1: group (0, 0, 1), count
        // 2, one seat over the bound.
        assert_eq!((constraint(),).evaluate_all(&p), HardSoftScore::of_hard(-1));
    }

    #[test]
    fn penalizes_far_above_maximum_proportionally() {
        // A concentration of five seats on a max-1 invigilator: distance 4.
        let mut invigs = vec![Invigilator::new(
            "inv-0",
            "inv 0",
            0,
            0,
            0,
            1,
            vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9],
        )];
        for i in 1..6 {
            invigs.push(Invigilator::new(
                format!("inv-{i}"),
                format!("inv {i}"),
                i,
                0,
                0,
                0,
                vec![0, 1, 2],
            ));
        }
        let exams: Vec<Exam> = (0..5)
            .map(|slot| {
                Exam::new(
                    format!("e{slot}"),
                    format!("exam {slot}"),
                    slot,
                    slot,
                    "r".to_string(),
                    1,
                )
            })
            .collect();
        let seats: Vec<Seat> = (0..5)
            .map(|slot| {
                let mut s = Seat::new(format!("seat-{slot}"), slot, 0, slot);
                s.invigilator_idx = Some(0);
                s
            })
            .collect();
        let p = Plan::new(invigs, exams, seats);
        assert_eq!((constraint(),).evaluate_all(&p), HardSoftScore::of_hard(-4));
    }

    #[test]
    fn penalizes_workload_below_minimum() {
        // Invigilator 0 (min 2, max 3) gets no seats; the two seats sit on
        // invigilator 1 whose bounds [0, 5] hold. The complement surfaces
        // invigilator 0's zero count: the below-minimum violation, alone.
        let mut invigs = vec![Invigilator::new(
            "inv-0",
            "inv 0",
            0,
            0,
            2,
            3,
            vec![0, 1, 2],
        )];
        invigs.push(Invigilator::new(
            "inv-1",
            "inv 1",
            1,
            0,
            0,
            5,
            vec![0, 1, 2],
        ));
        let exams = vec![Exam::new("e0", "exam 0", 0, 0, "r0".to_string(), 2)];
        let seats: Vec<Seat> = (0..2)
            .map(|pos| {
                let mut s = Seat::new(format!("seat-{pos}"), 0, pos, 0);
                s.invigilator_idx = Some(1);
                s
            })
            .collect();
        let p = Plan::new(invigs, exams, seats);
        // min 2, count 0: distance 2.
        assert_eq!((constraint(),).evaluate_all(&p), HardSoftScore::of_hard(-2));
    }

    #[test]
    fn penalizes_below_minimum_and_above_maximum_together() {
        // Invigilator 0 (min 2 max 3, zero seats) and invigilator 1
        // (max 0, two seats): distances 2 and 2, four hard points.
        let mut invigs = vec![Invigilator::new(
            "inv-0",
            "inv 0",
            0,
            0,
            2,
            3,
            vec![0, 1, 2],
        )];
        invigs.push(Invigilator::new(
            "inv-1",
            "inv 1",
            1,
            0,
            0,
            0,
            vec![0, 1, 2],
        ));
        let exams = vec![Exam::new("e0", "exam 0", 0, 0, "r0".to_string(), 2)];
        let seats: Vec<Seat> = (0..2)
            .map(|pos| {
                let mut s = Seat::new(format!("seat-{pos}"), 0, pos, 0);
                s.invigilator_idx = Some(1);
                s
            })
            .collect();
        let p = Plan::new(invigs, exams, seats);
        assert_eq!((constraint(),).evaluate_all(&p), HardSoftScore::of_hard(-4));
    }

    #[test]
    fn allows_workload_within_bounds() {
        let p = plan_with(1, 2, 0, Some(0));
        assert_eq!((constraint(),).evaluate_all(&p), HardSoftScore::ZERO);
    }

    /// Incremental-delta oracle: move one seat off an overloaded invigilator
    /// onto one with slack, and require the retract/insert event sequence to
    /// reproduce the full-evaluation delta exactly. The runtime's acceptor
    /// consumes these deltas; a full-eval-only suite cannot see a defect here.
    #[test]
    fn incremental_delta_matches_full_evaluation_on_reassignment() {
        // Invigilators: 0 min0 max1, 1 min0 max1, 2 min0 max9. Five seats at
        // slot-free exams: init loads invigilator 0 beyond max (distance 4);
        // moving one seat to invigilator 2 repairs distance to 3.
        let mut invigs = vec![
            Invigilator::new("inv-0", "inv 0", 0, 0, 0, 1, vec![0]),
            Invigilator::new("inv-1", "inv 1", 1, 0, 0, 1, vec![0]),
            Invigilator::new("inv-2", "inv 2", 2, 0, 0, 9, vec![0]),
        ];
        invigs.push(Invigilator::new("inv-3", "inv 3", 3, 0, 0, 9, vec![0]));
        let exams: Vec<Exam> = (0..5)
            .map(|slot| {
                Exam::new(
                    format!("e{slot}"),
                    format!("exam {slot}"),
                    slot,
                    slot,
                    "unused".to_string(),
                    1,
                )
            })
            .collect();
        let seats: Vec<Seat> = (0..5)
            .map(|slot| {
                let mut s = Seat::new(format!("seat-{slot}"), slot, 0, slot);
                s.invigilator_idx = Some(0);
                s
            })
            .collect();
        let before = Plan::new(invigs, exams, seats);

        // Full-evaluation delta of the single reassignment: same plan, only
        // seat 4's invigilator changes from 0 to 2.
        let mut after_seats: Vec<Seat> = before
            .seats
            .iter()
            .enumerate()
            .map(|(i, s)| {
                let mut seat: Seat = Seat::new(s.id.clone(), s.exam_index, s.position, s.slot);
                seat.invigilator_idx = before.seats[i].invigilator_idx;
                seat
            })
            .collect();
        after_seats[4].invigilator_idx = Some(2);
        let before_invigs: Vec<Invigilator> = before
            .invigilators
            .iter()
            .map(|i| {
                Invigilator::new(
                    i.id.clone(),
                    i.name.clone(),
                    i.index,
                    i.group,
                    i.min_workload,
                    i.max_workload,
                    i.available_slots.clone(),
                )
            })
            .collect();
        let before_exams: Vec<Exam> = before
            .exams
            .iter()
            .map(|e| {
                Exam::new(
                    e.id.clone(),
                    e.name.clone(),
                    e.index,
                    e.slot,
                    e.room.clone(),
                    e.required,
                )
            })
            .collect();
        let after = Plan::new(before_invigs, before_exams, after_seats);

        let mut set = (constraint(),);
        let score_before = set.initialize_all(&before);

        // The runtime sequence for a scalar change: report the seat's CURRENT
        // (old) row, mutate the solution, then report the new row — all on the
        // same retained constraint state.
        let retract_delta = set.on_retract_all(&before, 4, 0);
        let insert_delta = set.on_insert_all(&after, 4, 0);

        let expected_after = (constraint(),).evaluate_all(&after);
        let actual_after = score_before + retract_delta + insert_delta;
        assert_eq!(actual_after, expected_after);
    }
}
