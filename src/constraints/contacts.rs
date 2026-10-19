use crate::domain::{Plan, Seat};
use solverforge::prelude::*;
use solverforge::stream::collector::count;
use solverforge::IncrementalConstraint;
use solverforge_scoring::stream::cross::Bi;
use solverforge_scoring::stream::filter::FnBiFilter;

/// SOFT: the distinct invigilator pairs that supervise at least one exam
/// together, counted once per unordered pair however many exams they share.
///
/// IAP-CR penalizes every contact (soft = −contacts); IAP-CM rewards the
/// interdepartmental ones with a negative penalty (soft = +contacts). The
/// objective sense travels on the seats as `Seat::maximize`, so the group key
/// carries it and the penalty closure needs no access to the plan — one
/// constraint serves both variants, and only one contact stream is ever scored.
pub fn constraint() -> impl IncrementalConstraint<Plan, HardSoftScore> {
    Bi::new_with_filter(
        Plan::seats(),
        Plan::seats(),
        seat_exam_key,
        seat_exam_key,
        FnBiFilter::new(
            |plan: &Plan, a: &Seat, b: &Seat, a_index: usize, b_index: usize| {
                // One unordered pair per exam: keep the lower seat index only.
                if a_index >= b_index {
                    return false;
                }
                match (a.invigilator_idx, b.invigilator_idx) {
                    (Some(i), Some(j)) if i != j => {
                        // IAP-CR counts every contact; IAP-CM only the
                        // interdepartmental ones.
                        !a.maximize || plan.invigilators[i].group != plan.invigilators[j].group
                    }
                    _ => false,
                }
            },
        ),
    )
    .group_by(contact_key, count())
    .penalize(contact_penalty)
    .named("contacts")
}

fn seat_exam_key(seat: &Seat) -> usize {
    seat.exam_index
}

/// Both orders of a pair and every shared exam collapse into one group, so the
/// penalty lands once per distinct contact.
fn contact_key(left: &Seat, right: &Seat) -> (usize, usize, bool) {
    let i = left.invigilator_idx.unwrap_or(usize::MAX);
    let j = right.invigilator_idx.unwrap_or(usize::MAX);
    (i.min(j), i.max(j), left.maximize)
}

fn contact_penalty(key: &(usize, usize, bool), _count: &usize) -> HardSoftScore {
    if key.2 {
        HardSoftScore::of_soft(-1)
    } else {
        <HardSoftScore as Score>::one_soft()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Exam, Invigilator};
    use crate::solver::variant::Variant;

    fn exam(index: usize, slot: usize) -> Exam {
        Exam::new(
            format!("exam-{index}"),
            format!("exam {index}"),
            index,
            slot,
            "r".to_string(),
            2,
        )
    }

    fn invig(index: usize, group: usize) -> Invigilator {
        Invigilator::new(
            format!("inv-{index}"),
            format!("inv-{index}"),
            index,
            group,
            0,
            5,
            vec![0, 1, 2],
        )
    }

    fn seat(
        id: &str,
        exam_index: usize,
        position: usize,
        slot: usize,
        assigned: Option<usize>,
    ) -> Seat {
        let mut s = Seat::new(id.to_string(), exam_index, position, slot);
        s.invigilator_idx = assigned;
        s
    }

    fn plan_with(invigs: Vec<Invigilator>, exams: Vec<Exam>, seats: Vec<Seat>, v: Variant) -> Plan {
        let mut plan = Plan::new(invigs, exams, seats);
        plan.variant = v;
        for s in &mut plan.seats {
            s.maximize = v.maximize();
        }
        plan
    }

    #[test]
    fn cm_rewards_each_interdepartmental_pair_once() {
        // Pair (0,1) shares exam 0 and exam 1: one rewarded contact.
        let p = plan_with(
            vec![invig(0, 1), invig(1, 2)],
            vec![exam(0, 0), exam(1, 1)],
            vec![
                seat("s0", 0, 0, 0, Some(0)),
                seat("s1", 0, 1, 0, Some(1)),
                seat("s2", 1, 0, 1, Some(0)),
                seat("s3", 1, 1, 1, Some(1)),
            ],
            Variant::Cm,
        );
        assert_eq!((constraint(),).evaluate_all(&p), HardSoftScore::of_soft(1));
    }

    #[test]
    fn cm_ignores_same_department_pairs() {
        let p = plan_with(
            vec![invig(0, 1), invig(1, 1)],
            vec![exam(0, 0)],
            vec![seat("s0", 0, 0, 0, Some(0)), seat("s1", 0, 1, 0, Some(1))],
            Variant::Cm,
        );
        assert_eq!((constraint(),).evaluate_all(&p), HardSoftScore::ZERO);
    }

    #[test]
    fn cr_penalizes_every_distinct_pair() {
        // Exam 0 holds invigilators 0,1,2 from two departments: 3 contacts.
        let p = plan_with(
            vec![invig(0, 0), invig(1, 0), invig(2, 1)],
            vec![exam(0, 0)],
            vec![
                seat("s0", 0, 0, 0, Some(0)),
                seat("s1", 0, 1, 0, Some(1)),
                seat("s2", 0, 2, 0, Some(2)),
            ],
            Variant::Cr,
        );
        assert_eq!((constraint(),).evaluate_all(&p), HardSoftScore::of_soft(-3));
    }

    #[test]
    fn cr_penalizes_a_shared_pair_once() {
        let p = plan_with(
            vec![invig(0, 0), invig(1, 1)],
            vec![exam(0, 0), exam(1, 1)],
            vec![
                seat("s0", 0, 0, 0, Some(0)),
                seat("s1", 0, 1, 0, Some(1)),
                seat("s2", 1, 0, 1, Some(0)),
                seat("s3", 1, 1, 1, Some(1)),
            ],
            Variant::Cr,
        );
        assert_eq!((constraint(),).evaluate_all(&p), HardSoftScore::of_soft(-1));
    }

    #[test]
    fn unassigned_seats_contribute_no_contact() {
        let p = plan_with(
            vec![invig(0, 1), invig(1, 2)],
            vec![exam(0, 0)],
            vec![seat("s0", 0, 0, 0, Some(0)), seat("s1", 0, 1, 0, None)],
            Variant::Cm,
        );
        assert_eq!((constraint(),).evaluate_all(&p), HardSoftScore::ZERO);
    }
}
