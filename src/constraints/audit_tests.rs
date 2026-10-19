//! Independent score oracles; no stream keys or weight helpers are reused.
use super::*;
use crate::domain::{Exam, Invigilator, Seat};
use crate::solver::variant::Variant;
use solverforge::IncrementalConstraint;
use std::collections::BTreeSet;

fn fixture() -> Plan {
    fixture_with(Variant::Cm)
}

fn fixture_with(v: Variant) -> Plan {
    let invigilators = vec![
        Invigilator::new("i0", "i0", 0, 0, 1, 2, vec![0]),
        Invigilator::new("i1", "i1", 1, 1, 2, 3, vec![1]),
        Invigilator::new("i2", "i2", 2, 1, 0, 1, vec![0, 1]),
    ];
    let exams = vec![
        Exam::new("e0", "e0", 0, 0, "r0".into(), 3),
        Exam::new("e1", "e1", 1, 0, "r1".into(), 2),
        Exam::new("e2", "e2", 2, 1, "r0".into(), 1),
    ];
    let seats: Vec<Seat> = [(0, 0), (0, 1), (0, 2), (1, 0), (1, 1), (2, 0)]
        .into_iter()
        .enumerate()
        .map(|(i, (exam, position))| Seat::new(format!("s{i}"), exam, position, exams[exam].slot))
        .collect();
    let mut plan = Plan::new(invigilators, exams, seats);
    plan.variant = v;
    for seat in &mut plan.seats {
        seat.maximize = v.maximize();
    }
    plan
}

/// Independent contact oracle: distinct unordered pairs, restricted to
/// interdepartmental ones for IAP-CM, rewarded there and penalized for IAP-CR.
fn contacts(plan: &Plan, interdepartmental: bool) -> HardSoftScore {
    let mut pairs = BTreeSet::new();
    for exam in &plan.exams {
        let assigned: BTreeSet<usize> = plan
            .seats
            .iter()
            .filter(|s| s.exam_index == exam.index)
            .filter_map(|s| s.invigilator_idx)
            .collect();
        for &i in &assigned {
            for &j in assigned.range((i + 1)..) {
                if !interdepartmental || plan.invigilators[i].group != plan.invigilators[j].group {
                    pairs.insert((i, j));
                }
            }
        }
    }
    HardSoftScore::of_soft(if interdepartmental {
        pairs.len() as i64
    } else {
        -(pairs.len() as i64)
    })
}

fn availability_score(plan: &Plan) -> HardSoftScore {
    HardSoftScore::of_hard(
        -(plan
            .seats
            .iter()
            .filter(|s| {
                s.invigilator_idx.is_some_and(|i| {
                    !plan.invigilators[i]
                        .available_slots
                        .contains(&plan.exams[s.exam_index].slot)
                })
            })
            .count() as i64),
    )
}

fn workload_score(plan: &Plan) -> HardSoftScore {
    let mut loads = vec![0i64; plan.invigilators.len()];
    for s in &plan.seats {
        if let Some(i) = s.invigilator_idx {
            loads[i] += 1;
        }
    }
    HardSoftScore::of_hard(
        -plan
            .invigilators
            .iter()
            .zip(loads)
            .map(|(i, n)| (i.min_workload - n).max(0) + (n - i.max_workload).max(0))
            .sum::<i64>(),
    )
}

fn duplicate_score(plan: &Plan) -> HardSoftScore {
    let mut occupied = BTreeSet::new();
    let mut violations = 0;
    for seat in &plan.seats {
        if let Some(invigilator) = seat.invigilator_idx {
            if !occupied.insert((invigilator, plan.exams[seat.exam_index].slot)) {
                violations += 1;
            }
        }
    }
    HardSoftScore::of_hard(-violations)
}

/// Exercise retained native state through every assignment of six seats to
/// {None, 0, 1, 2}. Consecutive states include compound changes. Each transition
/// is also undone and replayed, matching rejected candidate and swap lifecycles.
fn exhaustive(
    factory: impl Fn() -> Box<dyn IncrementalConstraint<Plan, HardSoftScore>>,
    oracle: impl Fn(&Plan) -> HardSoftScore,
    plan: Plan,
) {
    let mut plan = plan;
    let mut constraint = factory();
    let mut retained = constraint.initialize(&plan);
    assert_eq!(retained, oracle(&plan));
    for code in 0..4usize.pow(plan.seats.len() as u32) {
        let mut digits = code;
        let target: Vec<_> = (0..plan.seats.len())
            .map(|_| {
                let digit = digits % 4;
                digits /= 4;
                if digit == 0 {
                    None
                } else {
                    Some(digit - 1)
                }
            })
            .collect();
        let old: Vec<_> = plan.seats.iter().map(|s| s.invigilator_idx).collect();
        let changed: Vec<_> = (0..target.len()).filter(|&i| target[i] != old[i]).collect();
        for values in [&target, &old, &target] {
            for &i in &changed {
                retained = retained + constraint.on_retract(&plan, i, 0);
            }
            for &i in &changed {
                plan.seats[i].invigilator_idx = values[i];
            }
            for &i in &changed {
                retained = retained + constraint.on_insert(&plan, i, 0);
            }
            let expected = oracle(&plan);
            assert_eq!(
                factory().evaluate(&plan),
                expected,
                "full score at state {code}"
            );
            assert_eq!(retained, expected, "native delta at state {code}");
        }
    }
}

#[test]
fn contacts_cm_full_and_native_deltas_match_unordered_set_oracle() {
    exhaustive(
        || Box::new(contacts::constraint()),
        |p| contacts(p, true),
        fixture_with(Variant::Cm),
    );
}
#[test]
fn contacts_cr_full_and_native_deltas_match_unordered_set_oracle() {
    exhaustive(
        || Box::new(contacts::constraint()),
        |p| contacts(p, false),
        fixture_with(Variant::Cr),
    );
}
#[test]
fn availability_full_and_native_deltas_match_fact_oracle() {
    exhaustive(
        || Box::new(availability::constraint()),
        availability_score,
        fixture(),
    );
}
#[test]
fn workload_full_and_native_deltas_include_zero_load_and_both_bounds() {
    exhaustive(
        || Box::new(workload_bounds::constraint()),
        workload_score,
        fixture(),
    );
}
#[test]
fn slot_conflict_full_and_native_deltas_match_excess_assignment_oracle() {
    exhaustive(
        || Box::new(slot_conflict::constraint()),
        duplicate_score,
        fixture(),
    );
}
